use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};
type Reply = Result<String, String>;
type Pending = Arc<Mutex<HashMap<u64, mpsc::Sender<Reply>>>>;
pub struct Bridge {
    child: Mutex<Child>,
    input: Mutex<Option<ChildStdin>>,
    pending: Pending,
    sequence: AtomicU64,
}
impl Bridge {
    pub fn start(executable: &Path) -> Result<Self, String> {
        let mut command = Command::new(executable);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        // NODE_OPTIONS must not turn the bundled private host into a debugger or load external code.
        command.env_remove("NODE_OPTIONS").env_remove("NODE_PATH");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("編集hostを開始できません: {e}"))?;
        let input = child.stdin.take();
        let output = child.stdout.take().ok_or("host stdout unavailable")?;
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let listeners = pending.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str::<Value>(&line) {
                    if let Some(id) = value["id"].as_u64() {
                        if let Some(reply) = listeners.lock().unwrap().remove(&id) {
                            let result = if let Some(error) = value["error"].as_str() {
                                Err(error.to_owned())
                            } else {
                                value["payload"]
                                    .as_str()
                                    .map(str::to_owned)
                                    .ok_or("host response invalid".into())
                            };
                            let _ = reply.send(result);
                        }
                    }
                }
            }
            for (_, reply) in listeners.lock().unwrap().drain() {
                let _ = reply.send(Err(
                    "編集hostが終了しました。入力を保持しています。アプリを再起動してください。"
                        .into(),
                ));
            }
        });
        Ok(Self {
            child: Mutex::new(child),
            input: Mutex::new(input),
            pending,
            sequence: AtomicU64::new(1),
        })
    }
    pub fn request(&self, method: &str, payload: String) -> Reply {
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, sender);
        let line = json!({"id": id, "method": method, "payload": payload}).to_string() + "\n";
        let written = self
            .input
            .lock()
            .unwrap()
            .as_mut()
            .ok_or("編集hostは終了しています。".to_owned())
            .and_then(|input| input.write_all(line.as_bytes()).map_err(|e| e.to_string()));
        if let Err(error) = written {
            self.pending.lock().unwrap().remove(&id);
            return Err(error);
        }
        let result = receiver
            .recv_timeout(Duration::from_secs(400))
            .map_err(|_| "編集hostの応答を取得できません。".to_owned());
        self.pending.lock().unwrap().remove(&id);
        result?
    }
    pub fn stop(&self) {
        self.input.lock().unwrap().take(); // EOF asks the TS host to stop its own .NET child before exit.
        let mut child = self.child.lock().unwrap();
        for _ in 0..50 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_host_round_trip_and_errors() {
        let exe = std::env::var("MASTERDATA_HOST_TEST").expect("prepare:host supplies executable");
        let bridge = Bridge::start(Path::new(&exe)).unwrap();
        assert_eq!(
            bridge.request("initial", "[[],[]]".into()).unwrap(),
            "[null,[]]"
        );
        assert!(bridge
            .request("not-an-operation", "[[],[]]".into())
            .unwrap_err()
            .contains("許可"));
        bridge.stop();
    }
    #[test]
    fn bundled_host_load_save_conflict_build_and_publish() {
        fn copy(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let target = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &target);
                } else {
                    std::fs::copy(entry.path(), target).unwrap();
                }
            }
        }
        let temp = tempfile::tempdir().unwrap();
        copy(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/Adventure"),
            temp.path(),
        );
        let exe = std::env::var("MASTERDATA_HOST_TEST").unwrap();
        let bridge = Bridge::start(Path::new(&exe)).unwrap();
        let result = bridge
            .request(
                "open",
                json!([[temp.path().join("masterdata.toml")], []]).to_string(),
            )
            .unwrap();
        let loaded: Value = serde_json::from_str(&result).unwrap();
        assert!(loaded[1]
            .as_array()
            .unwrap()
            .iter()
            .any(|pair| pair[1].as_str() == Some("18446744073709551615")));
        let project = &loaded[0];
        let request = json!({"token":project["token"], "files":project["saved"], "folders":project["snapshot"]["folders"], "config":project["snapshot"]["config"]});
        bridge
            .request("save", json!([[request], []]).to_string())
            .unwrap();
        let stale = json!({"token":"stale", "files":[], "folders":[], "config":project["snapshot"]["config"]});
        assert!(bridge
            .request("save", json!([[stale], []]).to_string())
            .unwrap_err()
            .contains("切り替わって"));
        let integers: Vec<Value> = loaded[1]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|pair| {
                let old = pair[0].as_array().unwrap();
                if old.first().and_then(Value::as_str) != Some("snapshot") {
                    return None;
                }
                let mut path = vec![json!(1)];
                path.extend_from_slice(&old[1..]);
                let mut rebased = pair.as_array().unwrap().clone();
                rebased[0] = json!(path);
                Some(json!(rebased))
            })
            .collect();
        if std::env::var_os("SKIP_NATIVE_BUILD").is_none() {
            let built = bridge
                .request(
                    "build",
                    json!([[project["token"], project["snapshot"], "", 4], integers]).to_string(),
                )
                .unwrap();
            let built: Value = serde_json::from_str(&built).unwrap();
            assert!(built[0]["log"]
                .as_str()
                .unwrap()
                .contains("PASS MasterMemory build / reload / every selected row"));
            bridge
                .request(
                    "publish",
                    json!([
                        [project["token"], project["snapshot"]["config"]["publish"]],
                        []
                    ])
                    .to_string(),
                )
                .unwrap();
        }
        let mut changed = request.clone();
        let name = changed["files"][0]["path"].as_str().unwrap().to_owned();
        changed["files"][0]["text"] = json!("kind: type\nname: Updated\ncustom: {fields: []}\n");
        std::fs::write(temp.path().join("sources").join(&name), "external").unwrap();
        assert!(bridge
            .request("save", json!([[changed], []]).to_string())
            .unwrap_err()
            .contains("外部で変更"));
        bridge.stop();
    }
}
