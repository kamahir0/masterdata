use masterdata_app::*;
use masterdata_core::*;
use serde_json::json;
use std::fs;
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    initialize_project(
        dir.path(),
        &InitOptions {
            project_id: "test.table".into(),
            name: "Table".into(),
            version: "0.1.0".into(),
        },
    )
    .unwrap();
    fs::write(dir.path().join("sources/schema.yaml"),"kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\n").unwrap();
    fs::write(
        dir.path().join("sources/data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: text\n",
    )
    .unwrap();
    dir
}
fn input(value: serde_json::Value) -> TableOperationInput {
    serde_json::from_value(value).unwrap()
}
fn rename() -> TableOperationInput {
    input(json!({"operation":"rename","table":"item","field":"note","newName":"description"}))
}
#[test]
fn plan_is_read_only_and_apply_checks_exact_reviewed_snapshot() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let before = fs::read(dir.path().join("sources/data.yaml")).unwrap();
    let plan = session.plan(dir.path(), rename()).unwrap();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(
        before,
        fs::read(dir.path().join("sources/data.yaml")).unwrap()
    );
    fs::write(
        dir.path().join("sources/data.yaml"),
        "kind: data\ntable: item\nrecords: []\n",
    )
    .unwrap();
    let result = session.apply(dir.path(), &plan.token, false).unwrap();
    assert_eq!(result.state, "not_started");
    assert_eq!(result.diagnostic.unwrap().code, "E-MIGRATION-PATCH-INVALID");
}
#[test]
fn destructive_authorization_is_separate_from_plan() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let plan = session
        .plan(
            dir.path(),
            input(json!({"operation":"drop","table":"item","field":"note"})),
        )
        .unwrap();
    assert!(plan.destructive);
    assert_eq!(
        session.apply(dir.path(), &plan.token, false).unwrap().state,
        "not_started"
    );
    assert_eq!(
        session.apply(dir.path(), &plan.token, true).unwrap().state,
        "success"
    );
    assert!(
        !fs::read_to_string(dir.path().join("sources/data.yaml"))
            .unwrap()
            .contains("note:")
    );
}
#[test]
fn recovery_requires_complete_source_set_and_gates_all_session_mutations() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let plan = session.plan(dir.path(), rename()).unwrap();
    let result = session
        .apply_with_failures(
            dir.path(),
            &plan.token,
            false,
            &[
                MigrationCommitFailureInjection::write_file(1),
                MigrationCommitFailureInjection::rollback_file(0),
            ],
        )
        .unwrap();
    assert_eq!(result.state, "recovery_required");
    assert!(session.ensure_mutation_allowed(dir.path()).is_err());
    // A mixed set that parses is insufficient evidence for recovery.
    for file in &plan.files {
        fs::write(
            dir.path().join(&file.path),
            if file.path.ends_with("schema.yaml") {
                &file.before
            } else {
                &file.after
            },
        )
        .unwrap();
    }
    assert!(session.recheck(dir.path()).unwrap().is_some());
    for file in &plan.files {
        fs::write(dir.path().join(&file.path), &file.before).unwrap();
    }
    assert!(session.recheck(dir.path()).unwrap().is_none());
    assert!(session.ensure_mutation_allowed(dir.path()).is_ok());
}
#[test]
fn initializer_preserves_ulong_and_rejects_unsupported_yaml_syntax() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let plan=session.plan(dir.path(),input(json!({"operation":"add","table":"item","field":{"key":2,"name":"amount","type":"ulong"},"initializer":"18446744073709551615"}))).unwrap();
    assert!(
        plan.files
            .iter()
            .any(|file| file.after.contains("18446744073709551615"))
    );
    assert!(session.plan(dir.path(),input(json!({"operation":"add","table":"item","field":{"key":2,"name":"amount","type":"ulong"},"initializer":"&value 1"}))).is_err());
}

#[test]
fn rolled_back_rename_keeps_old_bytes_and_new_plan_invalidates_previous_token() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let first = session.plan(dir.path(), rename()).unwrap();
    let plan = session.plan(dir.path(), rename()).unwrap();
    assert!(session.apply(dir.path(), &first.token, false).is_err());
    let result = session
        .apply_with_failures(
            dir.path(),
            &plan.token,
            false,
            &[MigrationCommitFailureInjection::write_file(1)],
        )
        .unwrap();
    assert_eq!(result.state, "rolled_back");
    for file in &plan.files {
        assert_eq!(
            fs::read_to_string(dir.path().join(&file.path)).unwrap(),
            file.before
        );
    }
    assert!(session.recovery_status(dir.path()).unwrap().is_none());
    assert_eq!(
        session.apply(dir.path(), &plan.token, false).unwrap().state,
        "success"
    );
    assert_eq!(
        session.apply(dir.path(), &plan.token, false).unwrap().state,
        "not_started"
    );
}

#[test]
fn reference_authoring_uses_shared_plan_and_refreshes_snapshot() {
    let dir = project();
    fs::write(
        dir.path().join("sources/category.yaml"),
        "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("sources/category-data.yaml"),
        "kind: data\ntable: category\nrecords:\n  - id: 1\n",
    )
    .unwrap();

    let mut session = TableAuthoringSession::default();
    let plan = session
        .plan(
            dir.path(),
            input(json!({
                "operation": "add_reference",
                "table": "item",
                "reference": {
                    "name": "category",
                    "csharpName": "GetCategoryMaster",
                    "fields": ["id"],
                    "target": {"table": "category", "fields": ["id"]}
                }
            })),
        )
        .unwrap();
    assert_eq!(plan.operation, "AddReference");
    assert_eq!(
        session.apply(dir.path(), &plan.token, false).unwrap().state,
        "success"
    );

    let snapshot = session
        .open_table(dir.path(), "sources/schema.yaml")
        .unwrap();
    assert_eq!(snapshot.references.len(), 1);
    assert_eq!(snapshot.references[0].name, "category");
    assert_eq!(
        snapshot.references[0].csharp_name.as_deref(),
        Some("GetCategoryMaster")
    );
    assert_eq!(
        snapshot.references[0].effective_csharp_name,
        "GetCategoryMaster"
    );
    assert_eq!(
        snapshot.references[0].cardinality,
        Some(ReferenceCardinality::Single)
    );
    assert_eq!(
        snapshot.references[0].optionality,
        Some(ReferenceOptionality::Required)
    );
}
