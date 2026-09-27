use masterdata_app::*;
use masterdata_core::*;
use serde_json::json;
use std::{fs, path::Path};
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
fn apply_current(
    session: &mut TableAuthoringSession,
    root: &Path,
    intent: TableOperationInput,
    dirty_paths: &[String],
) -> masterdata_core::Result<TableApplyView> {
    let context = session.open_context(root, "sources/schema.yaml")?;
    session.apply_intent(
        root,
        intent,
        &[TableSourceIdentity {
            path: context.schema_path,
            content_identity: context.schema_content_identity,
        }],
        dirty_paths,
    )
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

#[test]
fn reference_aware_target_rename_surfaces_inbound_schema_in_reviewed_plan() {
    let dir = project();
    fs::write(
        dir.path().join("sources/schema.yaml"),
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\n  - key: 2\n    name: categoryId\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("sources/data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: text\n    categoryId: 10\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("sources/category.yaml"),
        "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("sources/category-data.yaml"),
        "kind: data\ntable: category\nrecords:\n  - id: 10\n",
    )
    .unwrap();

    let mut session = TableAuthoringSession::default();
    let plan = session
        .plan(
            dir.path(),
            input(json!({
                "operation": "rename",
                "table": "category",
                "field": "id",
                "newName": "categoryKey"
            })),
        )
        .expect("Reference-aware RenameField plan");
    let paths = plan
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"sources/category.yaml"));
    assert!(paths.contains(&"sources/category-data.yaml"));
    assert!(paths.contains(&"sources/schema.yaml"));
    let inbound = plan
        .files
        .iter()
        .find(|file| file.path == "sources/schema.yaml")
        .expect("inbound Reference schema");
    assert!(inbound.after.contains("fields: [categoryKey]"));
}

#[test]
fn table_context_and_direct_intent_cover_separate_record_source() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let context = session
        .open_context(dir.path(), "sources/schema.yaml")
        .unwrap();
    assert_eq!(context.table, "item");
    assert_eq!(
        context.selected_record_source.as_deref(),
        Some("sources/data.yaml")
    );
    assert_eq!(context.record_sources.len(), 1);
    let add = || input(json!({"operation":"add_default","table":"item"}));
    assert_eq!(
        apply_current(
            &mut session,
            dir.path(),
            add(),
            &["sources/data.yaml".into()]
        )
        .unwrap_err()
        .diagnostic()
        .code,
        "E-TABLE-AFFECTED-DIRTY"
    );
    assert_eq!(
        apply_current(&mut session, dir.path(), add(), &[])
            .unwrap()
            .state,
        "success"
    );
    assert!(
        fs::read_to_string(dir.path().join("sources/schema.yaml"))
            .unwrap()
            .contains("name: field3")
    );
    assert!(
        fs::read_to_string(dir.path().join("sources/data.yaml"))
            .unwrap()
            .contains("field3: null")
    );
    assert_eq!(
        apply_current(
            &mut session,
            dir.path(),
            input(json!({"operation":"rename","table":"item","field":"field3","newName":"label"})),
            &[],
        )
        .unwrap()
        .state,
        "success"
    );
    assert_eq!(apply_current(&mut session, dir.path(), input(json!({"operation":"change_declaration","table":"item","field":"label","type":"int","nullable":true,"array":false})), &[]).unwrap().state, "success");
    assert!(
        fs::read_to_string(dir.path().join("sources/schema.yaml"))
            .unwrap()
            .contains("type: int")
    );
}

#[test]
fn table_context_prefers_inline_records_and_type_change_checks_dirty_data() {
    let dir = project();
    fs::write(dir.path().join("sources/schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 2\n    note: inline\n").unwrap();
    let mut session = TableAuthoringSession::default();
    let context = session
        .open_context(dir.path(), "sources/schema.yaml")
        .unwrap();
    assert_eq!(
        context.selected_record_source.as_deref(),
        Some("sources/schema.yaml")
    );
    assert_eq!(context.record_sources.len(), 2);
    let change = || {
        input(
            json!({"operation":"change_declaration","table":"item","field":"note","type":"string","nullable":true,"array":false}),
        )
    };
    assert_eq!(
        apply_current(
            &mut session,
            dir.path(),
            change(),
            &["sources/data.yaml".into()]
        )
        .unwrap_err()
        .diagnostic()
        .code,
        "E-TABLE-AFFECTED-DIRTY"
    );
    assert_eq!(
        apply_current(&mut session, dir.path(), change(), &[])
            .unwrap()
            .state,
        "success"
    );
    assert_eq!(
        apply_current(
            &mut session,
            dir.path(),
            input(json!({"operation":"add_default","table":"item"})),
            &[],
        )
        .unwrap()
        .state,
        "success"
    );
    assert!(
        fs::read_to_string(dir.path().join("sources/schema.yaml"))
            .unwrap()
            .contains("field3: null")
    );
    assert!(
        fs::read_to_string(dir.path().join("sources/data.yaml"))
            .unwrap()
            .contains("field3: null")
    );
}

#[test]
fn table_context_rejects_ambiguous_schema_authority() {
    let dir = project();
    fs::write(
        dir.path().join("sources/duplicate.yaml"),
        fs::read_to_string(dir.path().join("sources/schema.yaml")).unwrap(),
    )
    .unwrap();
    let session = TableAuthoringSession::default();
    assert_eq!(
        session
            .open_context(dir.path(), "sources/data.yaml")
            .unwrap_err()
            .diagnostic()
            .code,
        "E-TABLE-DUPLICATE-SCHEMA"
    );
}

#[test]
fn default_column_uses_an_available_key_when_the_highest_key_is_occupied() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    let source = fs::read_to_string(&schema_path).unwrap();
    fs::write(&schema_path, source.replace("key: 1", "key: 4294967295")).unwrap();
    let mut session = TableAuthoringSession::default();
    assert_eq!(
        apply_current(
            &mut session,
            dir.path(),
            input(json!({"operation":"add_default","table":"item"})),
            &[],
        )
        .unwrap()
        .state,
        "success"
    );
    assert!(
        fs::read_to_string(schema_path)
            .unwrap()
            .contains("key: 1\n    name: field3")
    );
}

#[test]
fn direct_intent_rejects_a_schema_changed_since_the_visible_context() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let context = session
        .open_context(dir.path(), "sources/schema.yaml")
        .unwrap();
    let schema_path = dir.path().join("sources/schema.yaml");
    let mut source = fs::read_to_string(&schema_path).unwrap();
    source.push_str("# external edit\n");
    fs::write(&schema_path, &source).unwrap();
    let error = session
        .apply_intent(
            dir.path(),
            input(json!({"operation":"add_default","table":"item"})),
            &[TableSourceIdentity {
                path: context.schema_path,
                content_identity: context.schema_content_identity,
            }],
            &[],
        )
        .unwrap_err();
    assert_eq!(error.diagnostic().code, "E-TABLE-STALE-SOURCE");
    assert_eq!(fs::read_to_string(schema_path).unwrap(), source);
}

#[test]
fn direct_intent_rejects_a_record_source_changed_since_the_visible_grid() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let context = session
        .open_context(dir.path(), "sources/data.yaml")
        .unwrap();
    let data_path = dir.path().join("sources/data.yaml");
    let original = fs::read_to_string(&data_path).unwrap();
    let changed = original.replace("note: text", "note: external");
    fs::write(&data_path, &changed).unwrap();
    let error = session
        .apply_intent(
            dir.path(),
            input(json!({"operation":"add_default","table":"item"})),
            &[
                TableSourceIdentity {
                    path: context.schema_path,
                    content_identity: context.schema_content_identity,
                },
                TableSourceIdentity {
                    path: "sources/data.yaml".into(),
                    content_identity: source_content_identity(&original),
                },
            ],
            &[],
        )
        .unwrap_err();
    assert_eq!(error.diagnostic().code, "E-TABLE-STALE-SOURCE");
    assert_eq!(fs::read_to_string(data_path).unwrap(), changed);
}
