use masterdata_app::{CreationRequest, CreationStatus, NativeApplicationService};
use masterdata_core::{InitOptions, initialize_project};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    initialize_project(
        dir.path(),
        &InitOptions {
            project_id: "test.creation".into(),
            name: "Creation".into(),
            version: "0.1.0".into(),
        },
    )
    .unwrap();
    dir
}
fn request(dir: &Path, destination: &str, artifact: serde_json::Value) -> CreationRequest {
    CreationRequest {
        source_root: dir.join("sources").to_string_lossy().into_owned(),
        destination: destination.into(),
        artifact: serde_json::from_value(artifact).unwrap(),
    }
}
fn folder(dir: &Path, destination: &str) -> CreationRequest {
    request(dir, destination, json!({"category":"folder"}))
}
fn table(dir: &Path, destination: &str, name: &str) -> CreationRequest {
    request(
        dir,
        destination,
        json!({"category":"table","table":name,"fields":[{"key":0,"name":"id","type":"int"}],"primaryKey":{"fields":["id"]}}),
    )
}
#[test]
fn creates_one_artifact_and_folder_without_touching_existing_sources() {
    let dir = project();
    let app = NativeApplicationService::new();
    let before = fs::read(dir.path().join("masterdata.toml")).unwrap();
    let created = app
        .create_source(
            Some(dir.path()),
            dir.path(),
            &folder(dir.path(), "custom-folder"),
        )
        .unwrap();
    assert_eq!(created.status, CreationStatus::Success);
    let created = app
        .create_source(
            Some(dir.path()),
            dir.path(),
            &table(dir.path(), "custom-folder/independent.yml", "weapon"),
        )
        .unwrap();
    assert_eq!(created.status, CreationStatus::Success);
    let data = request(
        dir.path(),
        "custom-folder/records.yaml",
        json!({"category":"data","table":"weapon"}),
    );
    assert_eq!(
        app.create_source(Some(dir.path()), dir.path(), &data)
            .unwrap()
            .status,
        CreationStatus::Success
    );
    let loaded = app
        .open_data_file(
            Some(dir.path()),
            dir.path(),
            "sources/custom-folder/records.yaml",
        )
        .unwrap();
    assert!(loaded.rows.is_empty());
    assert_eq!(
        before,
        fs::read(dir.path().join("masterdata.toml")).unwrap()
    );
    assert!(!dir.path().join(".masterdata/output").exists());
    let workspace = app
        .authoring_workspace(Some(dir.path()), dir.path())
        .unwrap();
    assert!(
        workspace
            .folders
            .iter()
            .any(|folder| folder.path == "sources/custom-folder")
    );
}
#[test]
fn rejects_path_escape_missing_parent_and_non_yaml_extension_without_mutation() {
    let dir = project();
    let app = NativeApplicationService::new();
    for path in [
        "../escape.yaml",
        "/escape.yaml",
        "missing/file.yaml",
        "file.txt",
        "C:\\escape.yaml",
        "stream.yaml:bad",
    ] {
        assert!(
            app.create_source(
                Some(dir.path()),
                dir.path(),
                &table(dir.path(), path, "weapon")
            )
            .is_err(),
            "{path}"
        );
    }
    assert!(!dir.path().join("sources/missing").exists());
}
#[test]
fn existing_files_and_folders_are_conflicts_without_overwrite() {
    let dir = project();
    let app = NativeApplicationService::new();
    fs::write(dir.path().join("sources/existing.yaml"), "retain exactly\n").unwrap();
    let report = app
        .create_source(
            Some(dir.path()),
            dir.path(),
            &table(dir.path(), "existing.yaml", "weapon"),
        )
        .unwrap();
    assert_eq!(report.status, CreationStatus::Conflict);
    assert_eq!(
        fs::read_to_string(dir.path().join("sources/existing.yaml")).unwrap(),
        "retain exactly\n"
    );
    fs::create_dir(dir.path().join("sources/existing")).unwrap();
    assert_eq!(
        app.create_source(
            Some(dir.path()),
            dir.path(),
            &folder(dir.path(), "existing")
        )
        .unwrap()
        .status,
        CreationStatus::Conflict
    );
}
#[test]
fn invalid_request_and_generated_name_collision_do_not_create_destination() {
    let dir = project();
    let app = NativeApplicationService::new();
    let invalid = request(
        dir.path(),
        "invalid.yaml",
        json!({"category":"data","table":"unknown"}),
    );
    assert!(
        app.create_source(Some(dir.path()), dir.path(), &invalid)
            .is_err()
    );
    app.create_source(
        Some(dir.path()),
        dir.path(),
        &table(dir.path(), "first.yaml", "weapon"),
    )
    .unwrap();
    let second = request(
        dir.path(),
        "second.yaml",
        json!({"category":"value_object","name":"Weapon","underlying":"int","conversions":{}}),
    );
    assert!(
        app.create_source(Some(dir.path()), dir.path(), &second)
            .is_err()
    );
    assert!(!dir.path().join("sources/second.yaml").exists());
    assert!(!dir.path().join("sources/invalid.yaml").exists());
}
#[cfg(unix)]
#[test]
fn symlink_parent_cannot_write_outside_selected_root() {
    let dir = project();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("sources/link")).unwrap();
    let app = NativeApplicationService::new();
    assert!(
        app.create_source(
            Some(dir.path()),
            dir.path(),
            &table(dir.path(), "link/escape.yaml", "weapon")
        )
        .is_err()
    );
    assert!(!outside.path().join("escape.yaml").exists());
}
#[test]
fn concurrent_exclusive_creation_has_one_winner() {
    let dir = project();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let tasks = (0..2)
        .map(|_| {
            let root = dir.path().to_path_buf();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                NativeApplicationService::new()
                    .create_source(Some(&root), &root, &folder(&root, "race"))
                    .unwrap()
                    .status
            })
        })
        .collect::<Vec<_>>();
    let results = tasks
        .into_iter()
        .map(|task| task.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|status| **status == CreationStatus::Success)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|status| **status == CreationStatus::Conflict)
            .count(),
        1
    );
}
#[test]
fn creation_context_and_recheck_are_read_only_and_allow_unrelated_parse_errors() {
    let dir = project();
    fs::write(dir.path().join("sources/broken.yaml"), "invalid: [").unwrap();
    let app = NativeApplicationService::new();
    let context = app.creation_context(Some(dir.path()), dir.path()).unwrap();
    assert!(
        context
            .choices
            .value_object_underlyings
            .contains(&"string".into())
    );
    assert!(
        !context
            .choices
            .value_object_underlyings
            .contains(&"bool".into())
    );
    let req = table(dir.path(), "new.yaml", "weapon");
    assert!(
        !app.recheck_creation(Some(dir.path()), dir.path(), &req)
            .unwrap()
            .exists
    );
    assert_eq!(
        app.create_source(Some(dir.path()), dir.path(), &req)
            .unwrap()
            .status,
        CreationStatus::Success
    );
    assert!(
        app.recheck_creation(Some(dir.path()), dir.path(), &req)
            .unwrap()
            .source
            .unwrap()
            .contains("weapon")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("sources/broken.yaml")).unwrap(),
        "invalid: ["
    );
}
