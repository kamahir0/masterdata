use masterdata_desktop::actor::{Intent, Session};
use masterdata_engine::{config::Operation, project::Project};
use serde_json::Value;
use std::{fs, path::Path, sync::atomic::Ordering};
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
#[tokio::test(flavor = "current_thread")]
async fn settings_draft_protects_session_saved_profile_updates_and_obsolete_project_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/rewrite-oracle/v1/save-both/input"),
        t.path(),
    );
    let session = Session::default();
    let open = || Intent::Open {
        path: t.path().to_string_lossy().into(),
        discard: false,
    };
    let reply: Value = serde_json::from_str(&session.request_at(open(), 0).await.unwrap()).unwrap();
    let epoch = reply["host"]["epoch"].as_u64().unwrap();
    let original = fs::read(t.path().join("sources/data.yaml")).unwrap();
    session
        .request_at(
            Intent::ConfigEdit {
                revision: 0,
                operation: Operation::AddProfile {
                    name: "release".into(),
                },
            },
            epoch,
        )
        .await
        .unwrap();
    assert!(session.protected.load(Ordering::Acquire));
    assert_eq!(
        session.request_at(open(), epoch).await.unwrap_err().code,
        "E-PROJECT-DIRTY"
    );
    let v: Value = serde_json::from_str(
        &session
            .request_at(
                Intent::ConfigView {
                    profile: Some("release".into()),
                    starts: [0; 4],
                },
                epoch,
            )
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(v["data"]["dirty"], true);
    let nav: Value = serde_json::from_str(
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
    for counter in [
        "projectDiscovery",
        "projectEnumeration",
        "projectYamlParse",
        "projectValidation",
    ] {
        assert_eq!(nav["host"]["work"][counter], 0);
    }
    let revision = v["data"]["revision"].as_u64().unwrap();
    let saved: Value = serde_json::from_str(
        &session
            .request_at(Intent::ConfigSave { revision }, epoch)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(saved["data"]["outcome"], "Success");
    assert!(!session.protected.load(Ordering::Acquire));
    assert!(
        Project::open(t.path())
            .unwrap()
            .config
            .build
            .profiles
            .contains_key("release")
    );
    assert_eq!(
        fs::read(t.path().join("sources/data.yaml")).unwrap(),
        original
    );
    session.request_at(open(), epoch).await.unwrap();
    let config = fs::read(t.path().join("masterdata.toml")).unwrap();
    assert_eq!(
        session
            .request_at(
                Intent::ConfigEdit {
                    revision: 0,
                    operation: Operation::AddProfile {
                        name: "obsolete".into()
                    }
                },
                epoch
            )
            .await
            .unwrap_err()
            .code,
        "E-PROJECT-OBSOLETE"
    );
    assert_eq!(fs::read(t.path().join("masterdata.toml")).unwrap(), config);
    session.stop();
}
