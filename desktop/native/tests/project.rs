use masterdata_desktop::actor::{Intent, Session};
use masterdata_engine::{config::Operation, project::Metadata};
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
fn metadata() -> Metadata {
    Metadata {
        id: "new.masterdata".into(),
        name: "New Project".into(),
        version: "0.1.0".into(),
    }
}
#[tokio::test(flavor = "current_thread")]
async fn project_creation_guard_failure_and_obsolete_requests_preserve_the_old_workspace() {
    let t = tempfile::tempdir().unwrap();
    let parent = t.path().canonicalize().unwrap();
    let old = parent.join("old");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/rewrite-oracle/v1/save-both/input"),
        &old,
    );
    let yaml = fs::read(old.join("sources/data.yaml")).unwrap();
    let session = Session::default();
    let opened: Value = serde_json::from_str(
        &session
            .request_at(
                Intent::Open {
                    path: old.to_string_lossy().into(),
                    discard: false,
                },
                0,
            )
            .await
            .unwrap(),
    )
    .unwrap();
    let epoch = opened["host"]["epoch"].as_u64().unwrap();
    let new = parent.join("new");
    session
        .request_at(
            Intent::ConfigEdit {
                revision: 0,
                operation: Operation::AddProfile {
                    name: "draft".into(),
                },
            },
            epoch,
        )
        .await
        .unwrap();
    let create = |target: &Path, discard| Intent::CreateProject {
        path: target.to_string_lossy().into(),
        metadata: metadata(),
        discard,
    };
    assert_eq!(
        session
            .request_at(create(&new, false), epoch)
            .await
            .unwrap_err()
            .code,
        "E-PROJECT-DIRTY"
    );
    assert!(!new.exists());
    assert!(session.protected.load(Ordering::Acquire));
    let occupied = parent.join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join(".hidden"), b"untouched").unwrap();
    assert!(
        session
            .request_at(create(&occupied, true), epoch)
            .await
            .is_err()
    );
    assert_eq!(fs::read(occupied.join(".hidden")).unwrap(), b"untouched");
    assert!(
        session
            .request_at(
                Intent::Open {
                    path: parent.join("missing").to_string_lossy().into(),
                    discard: true
                },
                epoch
            )
            .await
            .is_err()
    );
    let retained: Value = serde_json::from_str(
        &session
            .request_at(
                Intent::ConfigView {
                    profile: Some("draft".into()),
                    starts: [0; 4],
                },
                epoch,
            )
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(retained["data"]["dirty"], true);
    let comparison: Value = serde_json::from_str(
        &session
            .request_at(Intent::ConfigCompare { external: false }, epoch)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        comparison["data"]["after"]
            .as_str()
            .unwrap()
            .contains("draft")
    );
    let created: Value =
        serde_json::from_str(&session.request_at(create(&new, true), epoch).await.unwrap())
            .unwrap();
    assert_eq!(created["data"]["creation"]["outcome"], "Success");
    assert_eq!(
        created["data"]["inventory"]["project"]["name"],
        "New Project"
    );
    assert!(
        created["data"]["inventory"]["sources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let next = created["host"]["epoch"].as_u64().unwrap();
    assert_ne!(epoch, next);
    let obsolete = parent.join("obsolete");
    assert_eq!(
        session
            .request_at(create(&obsolete, true), epoch)
            .await
            .unwrap_err()
            .code,
        "E-PROJECT-OBSOLETE"
    );
    assert!(!obsolete.exists());
    assert_eq!(fs::read(old.join("sources/data.yaml")).unwrap(), yaml);
    assert!(
        !fs::read_to_string(old.join("masterdata.toml"))
            .unwrap()
            .contains("draft")
    );
    session.stop();
}
