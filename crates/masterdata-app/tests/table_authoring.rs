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
    let context = open_context(root, "sources/schema.yaml")?;
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

fn schema_save_draft(context: &TableContext) -> TableSchemaSaveDraft {
    TableSchemaSaveDraft {
        base_source: context.schema_source.clone(),
        base_content_identity: context.schema_content_identity.clone(),
        fields: context
            .schema
            .schema
            .fields
            .iter()
            .map(|field| SchemaDraftField {
                name: field.name.clone(),
                type_name: field.type_name.clone(),
                nullable: field.name == "note" || field.nullable,
                array: field.array,
            })
            .collect(),
    }
}

fn record_save_draft(root: &Path, path: &str) -> TableRecordSaveDraft {
    let snapshot = NativeApplicationService::new()
        .open_data_file(Some(root), root, path)
        .unwrap();
    TableRecordSaveDraft {
        base_source: snapshot.base_source,
        base_content_identity: snapshot.base_content_identity,
        mutation: AuthoringRecordMutation {
            edits: vec![AuthoringEdit {
                record_index: 0,
                field: "note".into(),
                value: AuthoringValue::String {
                    value: "changed".into(),
                },
            }],
            ..AuthoringRecordMutation::default()
        },
    }
}

#[test]
fn current_context_saves_separate_dirty_schema_and_selected_record_only() {
    let dir = project();
    let other = dir.path().join("sources/other.yaml");
    fs::write(&other, "kind: data\ntable: item\nrecords: []\n").unwrap();
    let other_before = fs::read(&other).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path.clone(),
                selected_record_source: context.selected_record_source.clone(),
                schema_draft: Some(schema_save_draft(&context)),
                inline_record_draft: None,
                record_draft: Some(record_save_draft(dir.path(), "sources/data.yaml")),
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 2);
    assert!(
        report
            .files
            .iter()
            .all(|file| file.status == TableContextFileSaveStatus::Success)
    );
    assert!(
        fs::read_to_string(dir.path().join("sources/schema.yaml"))
            .unwrap()
            .contains("nullable: true")
    );
    assert!(
        fs::read_to_string(dir.path().join("sources/data.yaml"))
            .unwrap()
            .contains("note: changed")
    );
    assert_eq!(fs::read(&other).unwrap(), other_before);
}

#[test]
fn current_context_known_conflict_prevents_every_commit() {
    let dir = project();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
    let record = record_save_draft(dir.path(), "sources/data.yaml");
    let schema_before = fs::read(dir.path().join("sources/schema.yaml")).unwrap();
    fs::write(
        dir.path().join("sources/data.yaml"),
        "kind: data\ntable: item\nrecords: []\n",
    )
    .unwrap();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path.clone(),
                selected_record_source: context.selected_record_source.clone(),
                schema_draft: Some(schema_save_draft(&context)),
                inline_record_draft: None,
                record_draft: Some(record),
            },
        )
        .unwrap();
    assert!(
        report
            .files
            .iter()
            .any(|file| file.status == TableContextFileSaveStatus::Conflict)
    );
    assert!(
        report
            .files
            .iter()
            .any(|file| file.status == TableContextFileSaveStatus::NotAttempted)
    );
    assert_eq!(
        fs::read(dir.path().join("sources/schema.yaml")).unwrap(),
        schema_before
    );
}

#[test]
fn current_context_composes_schema_and_inline_records_into_one_candidate() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    fs::write(&schema_path, "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n    note: text # preserve\n").unwrap();
    fs::remove_file(dir.path().join("sources/data.yaml")).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path.clone(),
                selected_record_source: context.selected_record_source.clone(),
                schema_draft: Some(schema_save_draft(&context)),
                inline_record_draft: Some(record_save_draft(dir.path(), "sources/schema.yaml")),
                record_draft: None,
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
    let source = fs::read_to_string(&schema_path).unwrap();
    assert!(source.contains("nullable: true"));
    assert!(source.contains("note: changed # preserve"));
}

#[test]
fn current_context_composes_field_and_inline_record_order_in_one_file() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    fs::write(&schema_path, "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n    note: first\n  # separator stays here\n  - id: 2\n    note: second\n").unwrap();
    fs::remove_file(dir.path().join("sources/data.yaml")).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    let schema_draft = TableSchemaSaveDraft {
        base_source: context.schema_source.clone(),
        base_content_identity: context.schema_content_identity.clone(),
        fields: vec![
            SchemaDraftField {
                name: "note".into(),
                type_name: "string".into(),
                nullable: false,
                array: false,
            },
            SchemaDraftField {
                name: "id".into(),
                type_name: "int".into(),
                nullable: false,
                array: false,
            },
        ],
    };
    let snapshot = NativeApplicationService::new()
        .open_data_file(Some(dir.path()), dir.path(), "sources/schema.yaml")
        .unwrap();
    let record_draft = TableRecordSaveDraft {
        base_source: snapshot.base_source,
        base_content_identity: snapshot.base_content_identity,
        mutation: AuthoringRecordMutation {
            record_order: Some(vec![
                AuthoringRecordOccurrence::Existing(1),
                AuthoringRecordOccurrence::Existing(0),
            ]),
            ..Default::default()
        },
    };
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path,
                selected_record_source: context.selected_record_source,
                schema_draft: Some(schema_draft),
                inline_record_draft: Some(record_draft),
                record_draft: None,
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
    let source = fs::read_to_string(schema_path).unwrap();
    assert!(source.find("name: note").unwrap() < source.find("name: id").unwrap());
    assert!(source.find("note: second").unwrap() < source.find("note: first").unwrap());
    assert!(source.contains("  # separator stays here\n"));
}

#[test]
fn current_context_mixed_saves_inline_and_selected_separate_but_not_inactive() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    let original = fs::read_to_string(&schema_path).unwrap();
    fs::write(
        &schema_path,
        format!("{original}records:\n  - id: 2\n    note: inline\n"),
    )
    .unwrap();
    let inactive_path = dir.path().join("sources/inactive.yaml");
    fs::write(&inactive_path, "kind: data\ntable: item\nrecords: []\n").unwrap();
    let inactive_before = fs::read(&inactive_path).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path.clone(),
                selected_record_source: Some("sources/data.yaml".into()),
                schema_draft: Some(schema_save_draft(&context)),
                inline_record_draft: Some(record_save_draft(dir.path(), "sources/schema.yaml")),
                record_draft: Some(record_save_draft(dir.path(), "sources/data.yaml")),
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 2);
    assert!(
        report
            .files
            .iter()
            .all(|file| file.status == TableContextFileSaveStatus::Success)
    );
    let schema = fs::read_to_string(schema_path).unwrap();
    assert!(schema.contains("nullable: true"));
    assert!(schema.contains("note: changed"));
    assert!(
        fs::read_to_string(dir.path().join("sources/data.yaml"))
            .unwrap()
            .contains("note: changed")
    );
    assert_eq!(fs::read(inactive_path).unwrap(), inactive_before);
}

#[test]
fn current_context_schema_only_does_not_write_clean_diagnostic_source() {
    let dir = project();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
    let data_path = dir.path().join("sources/data.yaml");
    let data_before = fs::read(&data_path).unwrap();
    let mut draft = schema_save_draft(&context);
    draft.fields[1].type_name = "bool".into();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path.clone(),
                selected_record_source: context.selected_record_source.clone(),
                schema_draft: Some(draft),
                inline_record_draft: None,
                record_draft: None,
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
    assert_eq!(fs::read(data_path).unwrap(), data_before);
}

#[test]
fn current_context_record_only_saves_selected_document() {
    let dir = project();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
    let schema_before = fs::read(dir.path().join("sources/schema.yaml")).unwrap();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path,
                selected_record_source: context.selected_record_source,
                schema_draft: None,
                inline_record_draft: None,
                record_draft: Some(record_save_draft(dir.path(), "sources/data.yaml")),
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, "sources/data.yaml");
    assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
    assert_eq!(
        fs::read(dir.path().join("sources/schema.yaml")).unwrap(),
        schema_before
    );
}

#[test]
fn current_context_inline_row_only_uses_schema_physical_source() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    let original = fs::read_to_string(&schema_path).unwrap();
    fs::write(
        &schema_path,
        format!("{original}records:\n  - id: 2\n    note: inline\n"),
    )
    .unwrap();
    fs::remove_file(dir.path().join("sources/data.yaml")).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path,
                selected_record_source: Some("sources/schema.yaml".into()),
                schema_draft: None,
                inline_record_draft: Some(record_save_draft(dir.path(), "sources/schema.yaml")),
                record_draft: None,
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, "sources/schema.yaml");
    assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
    assert!(
        fs::read_to_string(schema_path)
            .unwrap()
            .contains("note: changed")
    );
}

#[test]
fn current_context_without_record_source_saves_schema_only() {
    let dir = project();
    fs::remove_file(dir.path().join("sources/data.yaml")).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    assert!(context.selected_record_source.is_none());
    let report = session
        .save_current_table_context(
            dir.path(),
            &TableContextSaveRequest {
                schema_path: context.schema_path.clone(),
                selected_record_source: None,
                schema_draft: Some(schema_save_draft(&context)),
                inline_record_draft: None,
                record_draft: None,
            },
        )
        .unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, "sources/schema.yaml");
    assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
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

    let snapshot = open_table(dir.path(), "sources/schema.yaml").unwrap();
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
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
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
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
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
fn table_context_lists_inline_and_multiple_separate_record_sources_without_changing_schema_identity()
 {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    let mut schema = fs::read_to_string(&schema_path).unwrap();
    schema.push_str("records: []\n");
    fs::write(&schema_path, schema).unwrap();
    fs::write(
        dir.path().join("sources/other.yaml"),
        "kind: data\ntable: item\nrecords: []\n",
    )
    .unwrap();
    let inline = open_context(dir.path(), "sources/schema.yaml").unwrap();
    let separate = open_context(dir.path(), "sources/other.yaml").unwrap();
    let paths = inline
        .record_sources
        .iter()
        .map(|source| (source.path.as_str(), source.inline))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            ("sources/schema.yaml", true),
            ("sources/data.yaml", false),
            ("sources/other.yaml", false),
        ]
    );
    assert_eq!(
        inline.selected_record_source.as_deref(),
        Some("sources/schema.yaml")
    );
    assert_eq!(
        separate.selected_record_source.as_deref(),
        Some("sources/other.yaml")
    );
    assert_eq!(inline.schema_path, separate.schema_path);
    assert_eq!(
        inline.schema_content_identity,
        separate.schema_content_identity
    );
}

#[test]
fn table_context_without_record_source_retains_the_same_schema_header() {
    let dir = project();
    fs::remove_file(dir.path().join("sources/data.yaml")).unwrap();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    assert_eq!(context.table, "item");
    assert_eq!(context.schema_path, "sources/schema.yaml");
    assert_eq!(context.schema.schema.fields.len(), 2);
    assert!(context.record_sources.is_empty());
    assert!(context.selected_record_source.is_none());
}

#[test]
fn table_context_rejects_ambiguous_schema_authority() {
    let dir = project();
    fs::write(
        dir.path().join("sources/duplicate.yaml"),
        fs::read_to_string(dir.path().join("sources/schema.yaml")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        open_context(dir.path(), "sources/data.yaml")
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
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
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
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
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

#[test]
fn schema_draft_reinterprets_all_record_sources_and_saves_only_schema_bytes() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    let first_path = dir.path().join("sources/data.yaml");
    let second_path = dir.path().join("sources/other.yaml");
    fs::write(
        &first_path,
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: true # preserve\n",
    )
    .unwrap();
    fs::write(
        &second_path,
        "kind: data\ntable: item\nrecords:\n  - id: 2\n    note: false\n",
    )
    .unwrap();
    let first = fs::read(&first_path).unwrap();
    let second = fs::read(&second_path).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/data.yaml").unwrap();
    let fields = |type_name: &str| {
        context
            .schema
            .schema
            .fields
            .iter()
            .map(|field| SchemaDraftField {
                name: field.name.clone(),
                type_name: if field.name == "note" {
                    type_name.into()
                } else {
                    field.type_name.clone()
                },
                nullable: field.nullable,
                array: field.array,
            })
            .collect::<Vec<_>>()
    };

    let invalid = session
        .preview_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &fields("int"),
            &[],
            Some("sources/data.yaml"),
        )
        .unwrap();
    assert_eq!(
        invalid
            .validation
            .diagnostics
            .iter()
            .filter(|d| d.code == "E-TABLE-INVALID-RECORD-VALUE")
            .count(),
        2
    );
    assert!(invalid.selected_snapshot.is_some());
    assert_eq!(
        fs::read(&schema_path).unwrap(),
        context.schema_source.as_bytes()
    );
    let with_record_draft = session
        .preview_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &fields("int"),
            &[SchemaDraftRecordSource {
                path: "sources/data.yaml".into(),
                candidate_source:
                    "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: 1 # preserve\n".into(),
            }],
            None,
        )
        .unwrap();
    assert_eq!(
        with_record_draft
            .validation
            .diagnostics
            .iter()
            .filter(|d| d.code == "E-TABLE-INVALID-RECORD-VALUE")
            .count(),
        1
    );
    let restored = session
        .preview_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &fields("string"),
            &[],
            None,
        )
        .unwrap();
    assert!(restored.validation.valid);
    assert_eq!(restored.candidate_source, context.schema_source);

    let saved = session
        .save_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &context.schema_content_identity,
            &fields("int"),
        )
        .unwrap();
    assert_eq!(saved.status, SourceSaveStatus::Success);
    assert_eq!(fs::read(&first_path).unwrap(), first);
    assert_eq!(fs::read(&second_path).unwrap(), second);
    assert!(
        fs::read_to_string(&schema_path)
            .unwrap()
            .contains("name: note\n    type: int")
    );
    let app = NativeApplicationService::new();
    let validation = app.validate(Some(dir.path()), dir.path()).unwrap();
    assert_eq!(
        validation
            .diagnostics
            .iter()
            .filter(|d| d.code == "E-TABLE-INVALID-RECORD-VALUE")
            .count(),
        2
    );
    assert!(app.prepare_build(Some(dir.path()), dir.path()).is_err());
    let snapshot = app
        .open_data_file(Some(dir.path()), dir.path(), "sources/data.yaml")
        .unwrap();
    let cell = snapshot.rows[0]
        .cells
        .iter()
        .find(|cell| cell.field == "note")
        .unwrap();
    assert!(cell.editable, "semantic-invalid scalar remains repairable");
    assert_eq!(
        cell.value,
        AuthoringValue::String {
            value: "true".into()
        }
    );
    let repair = app
        .save_data_file(
            Some(dir.path()),
            dir.path(),
            "sources/data.yaml",
            &snapshot.base_source,
            &snapshot.base_content_identity,
            &[AuthoringEdit {
                record_index: 0,
                field: "note".into(),
                value: AuthoringValue::Number { value: "7".into() },
            }],
            None,
        )
        .unwrap();
    assert_eq!(repair.status, SourceSaveStatus::Success);
    assert!(
        fs::read_to_string(&first_path)
            .unwrap()
            .contains("note: 7 # preserve")
    );
    assert_eq!(
        app.validate(Some(dir.path()), dir.path())
            .unwrap()
            .diagnostics
            .iter()
            .filter(|d| d.code == "E-TABLE-INVALID-RECORD-VALUE")
            .count(),
        1
    );
}

#[test]
fn schema_draft_save_rejects_stale_identity_without_overwriting() {
    let dir = project();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    let fields = context
        .schema
        .schema
        .fields
        .iter()
        .map(|field| SchemaDraftField {
            name: field.name.clone(),
            type_name: if field.name == "note" {
                "int".into()
            } else {
                field.type_name.clone()
            },
            nullable: field.nullable,
            array: field.array,
        })
        .collect::<Vec<_>>();
    let path = dir.path().join(&context.schema_path);
    let external = format!("{}# external\n", context.schema_source);
    fs::write(&path, &external).unwrap();
    let report = session
        .save_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &context.schema_content_identity,
            &fields,
        )
        .unwrap();
    assert_eq!(report.status, SourceSaveStatus::Conflict);
    assert_eq!(fs::read_to_string(path).unwrap(), external);
}

#[test]
fn inline_record_draft_composes_for_preview_without_being_saved_with_schema() {
    let dir = project();
    let path = dir.path().join("sources/schema.yaml");
    let schema = format!(
        "{}records:\n  - id: 2\n    note: true # inline\n",
        fs::read_to_string(&path).unwrap()
    );
    fs::write(&path, &schema).unwrap();
    let session = TableAuthoringSession::default();
    let context = open_context(dir.path(), "sources/schema.yaml").unwrap();
    let fields = context
        .schema
        .schema
        .fields
        .iter()
        .map(|field| SchemaDraftField {
            name: field.name.clone(),
            type_name: if field.name == "note" {
                "int".into()
            } else {
                field.type_name.clone()
            },
            nullable: field.nullable,
            array: field.array,
        })
        .collect::<Vec<_>>();
    let inline_candidate = schema.replace("note: true # inline", "note: 7 # inline");
    let preview = session
        .preview_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &fields,
            &[SchemaDraftRecordSource {
                path: context.schema_path.clone(),
                candidate_source: inline_candidate,
            }],
            Some(&context.schema_path),
        )
        .unwrap();
    assert!(preview.candidate_source.contains("note: 7 # inline"));
    assert!(
        preview
            .candidate_source
            .contains("name: note\n    type: int")
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), schema);
    let result = session
        .save_schema_draft(
            dir.path(),
            &context.schema_path,
            &context.schema_source,
            &context.schema_content_identity,
            &fields,
        )
        .unwrap();
    assert_eq!(result.status, SourceSaveStatus::Success);
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("name: note\n    type: int"));
    assert!(saved.contains("note: true # inline"));
}

fn open_context(root: &Path, path: &str) -> masterdata_core::Result<TableContext> {
    let workspace = WorkspaceAuthoringSession::open(Some(root), root)?;
    workspace.validate();
    Ok(workspace
        .select_source(path)?
        .context
        .expect("Table context"))
}
fn open_table(root: &Path, path: &str) -> masterdata_core::Result<TableSnapshot> {
    Ok(open_context(root, path)?.schema)
}
