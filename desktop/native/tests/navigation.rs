use masterdata_desktop::actor::{Intent, Session};
use serde_json::Value;
use std::path::PathBuf;
#[test]
fn rapid_navigation_accepts_only_latest_target_and_warm_selection_reuses_project() {
    let session = Session::default();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/save-both/input");
    session
        .request_blocking(Intent::Open {
            path: root.to_string_lossy().into(),
            discard: false,
        })
        .unwrap();
    let paths = [
        "sources/schema.yaml",
        "sources/data.yaml",
        "sources/inactive.yaml",
        "sources/data.yaml",
    ];
    let mut pending = vec![];
    for token in 1..=50 {
        pending.push(session.submit(Intent::Select {
            path: paths[(token - 1) as usize % 4].into(),
            start: 0,
            count: 32,
            token,
        }));
    }
    let mut latest = None;
    for reply in pending {
        if let Ok(result) = reply.blocking_recv().unwrap() {
            let value: Value = serde_json::from_str(&result).unwrap();
            latest = Some(value);
        }
    }
    let latest = latest.unwrap();
    assert_eq!(latest["data"]["clicked"], paths[49 % 4]);
    assert_eq!(latest["host"]["token"], 50);
    for counter in [
        "projectDiscovery",
        "projectEnumeration",
        "projectYamlParse",
        "projectValidation",
    ] {
        assert_eq!(latest["host"]["work"][counter], 0, "{counter}");
    }
    assert!(latest["data"]["rows"].as_array().unwrap().len() <= 32);
    assert_eq!(
        session
            .request_blocking(Intent::Select {
                path: paths[0].into(),
                start: 0,
                count: 32,
                token: 49
            })
            .unwrap_err()
            .code,
        "E-SELECTION-OBSOLETE"
    );
    session.stop();
}

#[tokio::test(flavor = "current_thread")]
async fn previous_project_authoring_cannot_mutate_a_new_session_with_identical_source_bytes() {
    let session = Session::default();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/save-both/input");
    let open = || Intent::Open {
        path: root.to_string_lossy().into(),
        discard: false,
    };
    let a: Value = serde_json::from_str(&session.request_at(open(), 0).await.unwrap()).unwrap();
    let epoch = a["host"]["epoch"].as_u64().unwrap();
    let v: Value = serde_json::from_str(
        &session
            .request_at(
                Intent::Select {
                    path: "sources/data.yaml".into(),
                    start: 0,
                    count: 32,
                    token: 1,
                },
                epoch,
            )
            .await
            .unwrap(),
    )
    .unwrap();
    let row = v["data"]["rows"][0]["id"].as_str().unwrap().to_owned();
    let b: Value = serde_json::from_str(&session.request_at(open(), epoch).await.unwrap()).unwrap();
    let next = b["host"]["epoch"].as_u64().unwrap();
    assert_ne!(epoch, next);
    let error = session
        .request_at(
            Intent::EditText {
                source: "sources/data.yaml".into(),
                revision: 0,
                generation: v["data"]["generation"].as_u64().unwrap(),
                row,
                field: "note".into(),
                text: "late clipboard / typing".into(),
            },
            epoch,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "E-PROJECT-OBSOLETE");
    let inventory: Value =
        serde_json::from_str(&session.request_at(Intent::Inventory, next).await.unwrap()).unwrap();
    assert!(inventory["data"]["dirty"].as_array().unwrap().is_empty());
    session.stop();
}

#[test]
fn filesystem_changes_refresh_clean_sources_conflict_dirty_sources_and_invalidate_unavailable_views()
 {
    use std::{
        fs,
        sync::{Arc, mpsc},
        time::{Duration, Instant},
    };
    fn copy(src: &std::path::Path, dst: &std::path::Path) {
        fs::create_dir_all(dst).unwrap();
        for entry in fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &dst.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/save-both/input");
    let temp = tempfile::tempdir().unwrap();
    copy(&root, temp.path());
    let (send, receive) = mpsc::channel();
    let session = Session::new(Arc::new(move |status| {
        let _ = send.send(status);
    }));
    session
        .request_blocking(Intent::Open {
            path: temp.path().to_string_lossy().into(),
            discard: false,
        })
        .unwrap();
    let select = |token| -> Value {
        serde_json::from_str(
            &session
                .request_blocking(Intent::Select {
                    path: "sources/data.yaml".into(),
                    start: 0,
                    count: 32,
                    token,
                })
                .unwrap(),
        )
        .unwrap()
    };
    let first = select(1);
    let file = temp.path().join("sources/data.yaml");
    let base = fs::read_to_string(&file).unwrap();
    fs::write(&file, format!("{base}\n# external clean source\n")).unwrap();
    let observed = |after| {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let status = receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("native watcher did not publish actual change");
            if status.external_version > after {
                break status;
            }
        }
    };
    let clean = observed(0);
    assert!(clean.environment_error.is_none());
    let current = select(2);
    assert!(
        current["data"]["generation"].as_u64().unwrap()
            > first["data"]["generation"].as_u64().unwrap()
    );
    assert!(!current["data"]["dirty"].as_bool().unwrap());
    session
        .request_blocking(Intent::EditText {
            source: "sources/data.yaml".into(),
            revision: current["data"]["revision"].as_u64().unwrap(),
            generation: current["data"]["generation"].as_u64().unwrap(),
            row: current["data"]["rows"][0]["id"].as_str().unwrap().into(),
            field: "note".into(),
            text: "local draft".into(),
        })
        .unwrap();
    fs::write(&file, format!("{base}\n# external dirty source\n")).unwrap();
    let dirty = observed(clean.external_version);
    let conflict = select(3);
    assert!(conflict["data"]["conflict"].as_bool().unwrap());
    assert!(conflict["data"]["dirty"].as_bool().unwrap());
    let local = conflict["data"]["rows"].clone();
    assert!(
        serde_json::to_string(&local)
            .unwrap()
            .contains("local draft")
    );
    fs::remove_file(&file).unwrap();
    let removed = observed(dirty.external_version);
    assert_eq!(removed.dirty, ["sources/data.yaml"]);
    assert!(
        session
            .request_blocking(Intent::Select {
                path: "sources/data.yaml".into(),
                start: 0,
                count: 32,
                token: 4
            })
            .is_err()
    );
    fs::write(&file, format!("{base}\n# restored source\n")).unwrap();
    observed(removed.external_version);
    assert_eq!(select(5)["data"]["rows"], local);
    let config = temp.path().join("masterdata.toml");
    fs::write(
        &config,
        format!(
            "{}\n# changed config\n",
            fs::read_to_string(&config).unwrap()
        ),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let status = receive
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if status.environment_error.is_some() {
            break;
        }
    }
    assert!(
        session
            .request_blocking(Intent::Select {
                path: "sources/data.yaml".into(),
                start: 0,
                count: 32,
                token: 6
            })
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        format!("{base}\n# restored source\n")
    );
    session.stop();
}
