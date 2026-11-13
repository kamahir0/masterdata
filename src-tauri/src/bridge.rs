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
    fn wrong_case_project_file_keeps_the_original_draft_saveable() {
        let original = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let config = original.path().join("masterdata.toml");
        let misnamed = other.path().join("MASTERDATA.TOML");
        std::fs::write(&config, "name = 'Original'").unwrap();
        std::fs::write(&misnamed, "name = 'Other'").unwrap();

        let bridge = Bridge::start(Path::new("")).unwrap();
        let open = |dir: &Path| {
            bridge.request(
                "open",
                json!([[dir.join("masterdata.toml")], []]).to_string(),
            )
        };
        let loaded: Value = serde_json::from_str(&open(original.path()).unwrap()).unwrap();
        let mut draft = loaded[0]["snapshot"]["config"].clone();
        draft["name"] = json!("Edited");
        let error = open(other.path()).unwrap_err();
        assert!(error.contains(&other.path().join("masterdata.toml").display().to_string()));

        let request =
            json!({"token": loaded[0]["token"], "config": draft, "files": [], "folders": []});
        bridge
            .request("save", json!([[request], []]).to_string())
            .unwrap();
        let saved: toml::Table = std::fs::read_to_string(config).unwrap().parse().unwrap();
        assert_eq!(saved["name"].as_str(), Some("Edited"));
        assert_eq!(std::fs::read_to_string(misnamed).unwrap(), "name = 'Other'");
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
