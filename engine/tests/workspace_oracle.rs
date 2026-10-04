use masterdata_engine::{instrument, native::Outcome, source::Value, workspace::Workspace};
use serde_json::Value as Json;
use std::{
    fs,
    path::{Path, PathBuf},
};
fn copy(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &dst.join(e.file_name()));
        } else {
            fs::copy(e.path(), dst.join(e.file_name())).unwrap();
        }
    }
}
fn oracle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1")
}
fn compare(expected: &Path, actual: &Path) {
    for e in fs::read_dir(expected).unwrap() {
        let e = e.unwrap();
        let other = actual.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            compare(&e.path(), &other);
        } else {
            assert_eq!(
                fs::read(e.path()).unwrap(),
                fs::read(&other).unwrap(),
                "exact disk bytes {}",
                other.display()
            );
        }
    }
}
#[test]
fn independent_table_save_scope_and_fresh_conflicts() {
    let root = oracle();
    let manifest: Json =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for id in manifest["saveScenarios"].as_array().unwrap() {
        let id = id.as_str().unwrap();
        let case = root.join(id);
        let temp = tempfile::tempdir().unwrap();
        copy(&case.join("input"), temp.path());
        let scenario: Json =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let op = &scenario["operation"];
        let schema = op["schemaSource"].as_str().unwrap();
        let selected = op["selectedRecordSource"].as_str();
        let mut w = Workspace::open(temp.path()).unwrap();
        let view = w.select(selected.unwrap_or(schema), 0, 64).unwrap();
        let table = view.table.name.clone();
        if !op["schemaDraft"].is_null() {
            w.schema_modifier(
                schema,
                w.drafts[schema].revision,
                op["schemaDraft"]["field"].as_str().unwrap(),
                op["schemaDraft"]["nullable"].as_bool().unwrap(),
                false,
                None,
            )
            .unwrap();
        }
        if !op["recordDraft"].is_null() {
            let path = selected.unwrap();
            let d = &w.drafts[path];
            let row =
                d.row_ids[op["recordDraft"]["occurrence"].as_u64().unwrap() as usize - 1].clone();
            w.edit(
                path,
                d.revision,
                &row,
                &[op["recordDraft"]["field"].as_str().unwrap().into()],
                &Value::Text(op["recordDraft"]["value"]["text"].as_str().unwrap().into()),
            )
            .unwrap();
        }
        if let Some(change) = op["externalChangeAfterBaseCapture"].as_object() {
            fs::write(
                temp.path().join(change["source"].as_str().unwrap()),
                change["bytes"].as_str().unwrap(),
            )
            .unwrap();
        }
        let results = w.save_table(&table, selected).unwrap();
        let mut committed = results
            .iter()
            .filter(|r| r.outcome == Outcome::Success)
            .map(|r| r.source.clone())
            .collect::<Vec<_>>();
        committed.sort();
        let expected = scenario["expected"]["committedPhysicalSources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(committed, expected, "{id}");
        if scenario["expected"]["outcome"] == "conflict" {
            assert!(results.iter().any(|r| r.outcome == Outcome::Conflict));
            assert!(!w.dirty_paths().is_empty());
        }
        compare(&case.join("expected"), temp.path());
        println!("PASS {id}");
    }
}
#[test]
fn source_history_survives_navigation_and_success_clears_only_saved_file() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-both/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    let a = w.select("sources/data.yaml", 0, 32).unwrap();
    let row = a.rows[0].id.clone();
    w.edit_text("sources/data.yaml", a.revision, &row, "note", "draft")
        .unwrap();
    let b = w.select("sources/inactive.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/inactive.yaml",
        b.revision,
        &b.rows[0].id,
        "note",
        "other draft",
    )
    .unwrap();
    let before = fs::read(temp.path().join("sources/data.yaml")).unwrap();
    let (a, measure) = instrument::measure(|| w.select("sources/data.yaml", 0, 32).unwrap());
    assert!(a.rows[0].cells.iter().any(|c| c.display == "draft"));
    assert_eq!(measure.work.project_discovery, 0);
    assert_eq!(measure.work.project_enumeration, 0);
    assert_eq!(measure.work.project_yaml_parse, 0);
    assert_eq!(measure.work.project_validation, 0);
    w.undo("sources/data.yaml", false).unwrap();
    assert!(!w.drafts["sources/data.yaml"].dirty());
    assert!(w.drafts["sources/data.yaml"].can_redo());
    w.undo("sources/data.yaml", true).unwrap();
    assert_eq!(
        fs::read(temp.path().join("sources/data.yaml")).unwrap(),
        before,
        "Undo does not write disk"
    );
    w.save_table("item", Some("sources/data.yaml")).unwrap();
    assert!(!w.drafts["sources/data.yaml"].can_undo());
    assert!(w.drafts["sources/inactive.yaml"].dirty());
    assert!(w.drafts["sources/inactive.yaml"].can_undo());
}
#[test]
fn clean_external_refresh_and_invalid_or_deleted_source_never_returns_stale_editor() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    w.select("sources/data.yaml", 0, 32).unwrap();
    let path = temp.path().join("sources/data.yaml");
    fs::write(
        &path,
        "kind: data\ntable: item\nrecords:\n  - id: 9\n    note: external\n",
    )
    .unwrap();
    assert_eq!(
        w.select("sources/data.yaml", 0, 32).unwrap().rows[0].cells[0].display,
        "9"
    );
    fs::write(&path, "records: [").unwrap();
    assert!(w.select("sources/data.yaml", 0, 32).is_err());
    fs::remove_file(path).unwrap();
    assert!(w.select("sources/data.yaml", 0, 32).is_err());
}
#[test]
fn schema_reinterpretation_keeps_record_bytes_and_rejects_old_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    w.select("sources/data.yaml", 0, 32).unwrap();
    let bytes = w.current_doc("sources/data.yaml").unwrap().bytes.clone();
    let old = w.generation;
    w.schema_modifier("sources/schema.yaml", 0, "note", false, false, Some("int"))
        .unwrap();
    let v = w.select("sources/data.yaml", 0, 32).unwrap();
    assert_eq!(w.current_doc("sources/data.yaml").unwrap().bytes, bytes);
    assert!(!v.rows[0].cells[1].valid);
    assert!(!w.drafts["sources/data.yaml"].dirty());
    assert!(!w.accept_diagnostics(old, vec![]));
    w.undo("sources/schema.yaml", false).unwrap();
    assert!(w.select("sources/data.yaml", 0, 32).unwrap().rows[0].cells[1].valid);
}

#[test]
fn overwrite_rechecks_reviewed_file_identity_and_keeps_the_draft_on_conflict() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    let v = w.select("sources/data.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/data.yaml",
        v.revision,
        &v.rows[0].id,
        "note",
        "draft",
    )
    .unwrap();
    let path = temp.path().join("sources/data.yaml");
    let external = "kind: data\ntable: item\nrecords:\n  - id: 7\n    note: external\n";
    fs::write(&path, external).unwrap();
    assert_eq!(
        w.save_table("item", Some("sources/data.yaml")).unwrap()[0].outcome,
        Outcome::Conflict
    );
    let (reviewed, _, _) = w.compare("sources/data.yaml").unwrap();
    // Identical bytes in a replaced file still have a different physical identity.
    let replacement = tempfile::NamedTempFile::new_in(path.parent().unwrap()).unwrap();
    fs::write(replacement.path(), external).unwrap();
    replacement.persist(&path).unwrap();
    assert_eq!(
        w.overwrite("sources/data.yaml", &reviewed).unwrap().outcome,
        Outcome::Conflict
    );
    assert!(w.drafts["sources/data.yaml"].dirty());
    assert!(w.drafts["sources/data.yaml"].can_undo());
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    let (reviewed, _, candidate) = w.compare("sources/data.yaml").unwrap();
    assert_eq!(
        w.overwrite("sources/data.yaml", &reviewed).unwrap().outcome,
        Outcome::Success
    );
    assert_eq!(fs::read_to_string(path).unwrap(), candidate);
}

#[test]
fn read_only_write_failure_keeps_exact_disk_bytes_and_local_history() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let path = temp.path().join("sources/data.yaml");
    let before = fs::read(&path).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let v = w.select("sources/data.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/data.yaml",
        v.revision,
        &v.rows[0].id,
        "note",
        "draft",
    )
    .unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    let results = w.save_table("item", Some("sources/data.yaml")).unwrap();
    fs::set_permissions(&path, original_permissions).unwrap();
    assert_eq!(results[0].outcome, Outcome::Failure);
    assert_eq!(fs::read(path).unwrap(), before);
    assert!(w.drafts["sources/data.yaml"].dirty());
    assert!(w.drafts["sources/data.yaml"].can_undo());
}
