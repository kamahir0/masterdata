use masterdata_lib::service::ProjectService;
use std::path::Path;

pub struct Bridge {
    service: ProjectService,
}

impl Bridge {
    pub fn start(_executable: &Path) -> Result<Self, String> {
        Ok(Self {
            service: ProjectService::new(),
        })
    }

    pub fn request(&self, method: &str, payload: String) -> Result<String, String> {
        self.service.request_wire(method, &payload)
    }

    pub fn stop(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn bundled_host_round_trip_and_errors() {
        let exe = std::env::var("MASTERDATA_HOST_TEST").unwrap_or_default();
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
        let exe = std::env::var("MASTERDATA_HOST_TEST").unwrap_or_default();
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
