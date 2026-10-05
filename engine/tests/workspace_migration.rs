use masterdata_engine::{
    creation::Declaration,
    migration::Command,
    native::{self, Outcome},
    source::Value,
    workspace::Workspace,
};
use std::{fs, path::PathBuf};
fn oracle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1")
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(oracle().join("migration-add/input")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    fs::copy(
        oracle().join("save-both/input/masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::write(temp.path().join("sources/other.yaml"),"kind: schema\ntable: other\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n    note: original\n").unwrap();
    temp
}
fn add() -> Command {
    Command::AddField {
        table: "item".into(),
        declaration: Declaration {
            key: "2".into(),
            name: "rank".into(),
            type_name: "int".into(),
            nullable: false,
            array: false,
        },
        initializer: Some(Value::Literal("7".into())),
        position: None,
    }
}
#[test]
fn ordinary_field_intents_use_captured_scope_default_null_and_reject_a_stale_or_changed_command() {
    use masterdata_engine::workspace::{FieldIntent, FieldOperation};
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select("sources/one.yaml", 0, 32).unwrap();
    let operation = FieldOperation::Add {
        neighbor: Some("id".into()),
        after: true,
    };
    let scope = w
        .field_operation_scope(
            &p.table.source,
            p.schema_revision,
            p.generation,
            operation.clone(),
        )
        .unwrap();
    let (review, result) = w
        .direct_field_operation(
            &p.table.source,
            p.schema_revision,
            p.generation,
            FieldIntent {
                token: scope["token"].as_str().unwrap().into(),
                operation,
                authorize_destructive: false,
            },
        )
        .unwrap();
    assert_eq!(result.outcome, Outcome::Success);
    assert_eq!(review.affected_records, 2);
    let p = w.select("sources/one.yaml", 0, 32).unwrap();
    assert_eq!(p.columns[1].field.name, "field");
    assert!(p.columns[1].field.nullable);
    assert_eq!(p.rows[0].cells[1].value, Some(Value::Null));
    assert_eq!(
        p.columns.iter().map(|c| c.field.key).collect::<Vec<_>>(),
        [0, 2, 1]
    );
    let operation = FieldOperation::Rename {
        field: "note".into(),
        new_name: "memo".into(),
    };
    let scope = w
        .field_operation_scope(
            &p.table.source,
            p.schema_revision,
            p.generation,
            operation.clone(),
        )
        .unwrap();
    let changed = FieldOperation::Rename {
        field: "note".into(),
        new_name: "otherName".into(),
    };
    assert_eq!(
        w.direct_field_operation(
            &p.table.source,
            p.schema_revision,
            p.generation,
            FieldIntent {
                token: scope["token"].as_str().unwrap().into(),
                operation: changed,
                authorize_destructive: false
            }
        )
        .unwrap_err()
        .code,
        "E-MIGRATION-STALE"
    );
    let file = temp.path().join("sources/two.yaml");
    let untouched = fs::read(temp.path().join("sources/one.yaml")).unwrap();
    fs::write(
        &file,
        fs::read_to_string(&file).unwrap() + "# external source change\n",
    )
    .unwrap();
    assert_eq!(
        w.direct_field_operation(
            &p.table.source,
            p.schema_revision,
            p.generation,
            FieldIntent {
                token: scope["token"].as_str().unwrap().into(),
                operation,
                authorize_destructive: false
            }
        )
        .unwrap_err()
        .code,
        "E-MIGRATION-STALE"
    );
    assert_eq!(
        fs::read(temp.path().join("sources/one.yaml")).unwrap(),
        untouched
    );
}
fn edit(w: &mut Workspace, path: &str, text: &str) {
    let p = w.select(path, 0, 64).unwrap();
    w.edit(
        path,
        p.revision,
        &p.rows[0].id,
        &["note".into()],
        &Value::Text(text.into()),
    )
    .unwrap();
}

#[test]
fn migration_preserves_unrelated_draft_history_search_and_exact_occurrence_selection() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    edit(&mut w, "sources/other.yaml", "unsaved");
    w.set_search("sources/other.yaml", "unsaved").unwrap();
    let before = w.select("sources/one.yaml", 0, 64).unwrap();
    let row = before.rows[0].id.clone();
    w.views
        .entry("sources/one.yaml".into())
        .or_default()
        .selected_row = Some(row.clone());
    let review = w.prepare_migration(add()).unwrap();
    assert_eq!(
        w.recheck_migration_result(&review.token).unwrap().outcome,
        Outcome::NotAttempted
    );
    assert_eq!(review.files.len(), 3);
    assert_eq!(review.affected_records, 2);
    assert_eq!(w.dirty_paths(), ["sources/other.yaml"]);
    let compare = w
        .migration_compare(&review.token, "sources/one.yaml")
        .unwrap();
    assert!(compare.1.contains("rank: 7"));
    let result = w.apply_migration(&review.token, false).unwrap();
    assert_eq!(result.outcome, Outcome::Success);
    let after = w.select("sources/one.yaml", 0, 64).unwrap();
    assert_eq!(after.rows[0].id, row);
    assert_eq!(after.columns.last().unwrap().field.name, "rank");
    assert_eq!(
        w.views["sources/one.yaml"].selected_row.as_ref(),
        Some(&row)
    );
    assert_eq!(w.dirty_paths(), ["sources/other.yaml"]);
    assert_eq!(w.views["sources/other.yaml"].search, "unsaved");
    assert!(w.undo("sources/other.yaml", false).unwrap());
    assert!(!w.drafts["sources/other.yaml"].dirty());
    assert_eq!(
        w.recheck_migration_result(&review.token).unwrap().outcome,
        Outcome::Success
    );
    assert_eq!(
        w.apply_migration(&review.token, false).unwrap_err().code,
        "E-MIGRATION-APPLIED"
    );
}

#[test]
fn affected_dirty_sources_are_scoped_and_a_saved_review_is_stale() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let review = w.prepare_migration(add()).unwrap();
    edit(&mut w, "sources/one.yaml", "changed");
    edit(&mut w, "sources/other.yaml", "unrelated");
    assert_eq!(
        w.apply_migration(&review.token, false).unwrap_err().code,
        "E-MIGRATION-DIRTY"
    );
    w.save_paths(vec!["sources/one.yaml".into()], native::Fault::None)
        .unwrap();
    assert_eq!(
        w.apply_migration(&review.token, false).unwrap_err().code,
        "E-MIGRATION-STALE"
    );
    assert_eq!(w.dirty_paths(), ["sources/other.yaml"]);
    let fresh = w.prepare_migration(add()).unwrap();
    assert_eq!(
        w.apply_migration(&fresh.token, false).unwrap().outcome,
        Outcome::Success
    );
    assert_eq!(w.dirty_paths(), ["sources/other.yaml"]);
}

#[cfg(feature = "oracle-faults")]
#[test]
fn recovery_survives_reopen_blocks_all_mutations_allows_read_and_requires_fresh_authorized_repair()
{
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    edit(&mut w, "sources/other.yaml", "draft survives");
    let review = w.prepare_migration(add()).unwrap();
    let result = w
        .apply_migration_with_fault(
            &review.token,
            false,
            native::SetFault::CommitFailure {
                source: "sources/two.yaml".into(),
                rollback_failure: Some("sources/one.yaml".into()),
            },
        )
        .unwrap();
    assert_eq!(result.outcome, Outcome::RecoveryRequired);
    assert!(w.recovery_required);
    assert_eq!(
        w.save_config(w.configuration.revision, native::Fault::None)
            .unwrap_err()
            .code,
        "E-RECOVERY-REQUIRED"
    );
    assert!(w.configuration.saved_compare().0.contains("[project]"));
    let id = w.recovery_information[0].id.clone();
    let p = w.select("sources/one.yaml", 0, 64).unwrap();
    assert!(!p.rows[0].cells[1].editable);
    assert_eq!(
        w.edit(
            "sources/one.yaml",
            p.revision,
            &p.rows[0].id,
            &["note".into()],
            &Value::Text("unsafe".into())
        )
        .unwrap_err()
        .code,
        "E-RECOVERY-REQUIRED"
    );
    assert_eq!(
        w.undo("sources/other.yaml", false).unwrap_err().code,
        "E-RECOVERY-REQUIRED"
    );
    assert_eq!(w.save_all().unwrap_err().code, "E-RECOVERY-REQUIRED");
    assert!(
        w.prepare_path_move("sources/one.yaml", "sources/moved.yaml")
            .is_err()
    );
    assert!(
        w.prepare_migration(Command::RenameField {
            table: "item".into(),
            field: "note".into(),
            new_name: "memo".into()
        })
        .is_err()
    );
    assert!(w.recheck_migration_recovery(&id, false, false).is_err());
    assert_eq!(
        w.recheck_migration_recovery(&id, true, false)
            .unwrap_err()
            .code,
        "E-RECOVERY-AUTHORIZATION"
    );
    let reopened = Workspace::open(temp.path()).unwrap();
    assert!(reopened.recovery_required);
    assert_eq!(reopened.recovery_information[0].id, id);
    let info = w.recheck_migration_recovery(&id, true, true).unwrap();
    assert!(info.files.iter().all(|f| f.state == "OLD"));
    assert!(!w.recovery_required);
    assert_eq!(w.dirty_paths(), ["sources/other.yaml"]);
    assert!(w.undo("sources/other.yaml", false).unwrap());
    assert!(!Workspace::open(temp.path()).unwrap().recovery_required);
    for file in ["one.yaml", "schema.yaml", "two.yaml"] {
        assert_eq!(
            fs::read(temp.path().join("sources").join(file)).unwrap(),
            fs::read(oracle().join("migration-add/input").join(file)).unwrap()
        );
    }
}

#[cfg(feature = "oracle-faults")]
#[test]
fn lost_recovery_reply_can_be_rechecked_without_repeating_writes_or_discarding_new_drafts() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let review = w.prepare_migration(add()).unwrap();
    w.apply_migration_with_fault(
        &review.token,
        false,
        native::SetFault::CommitFailure {
            source: "sources/two.yaml".into(),
            rollback_failure: Some("sources/one.yaml".into()),
        },
    )
    .unwrap();
    let id = w.recovery_information[0].id.clone();
    let completed = w.recheck_migration_recovery(&id, true, true).unwrap();
    let actual = native::capture(&w.read.root, &w.read.roots, "sources/one.yaml").unwrap();
    edit(&mut w, "sources/one.yaml", "new draft");
    let repeated = w.recheck_migration_recovery(&id, false, false).unwrap();
    assert_eq!(completed.message, repeated.message);
    assert_eq!(w.dirty_paths(), ["sources/one.yaml"]);
    native::preflight(&w.read.root, &w.read.roots, "sources/one.yaml", &actual).unwrap();
    assert!(w.undo("sources/one.yaml", false).unwrap());
    let path = temp.path().join("sources/one.yaml");
    let replacement = temp.path().join("external.yaml");
    fs::write(&replacement, fs::read(&path).unwrap()).unwrap();
    fs::remove_file(&path).unwrap();
    fs::rename(replacement, path).unwrap();
    assert_eq!(
        w.recheck_migration_recovery(&id, false, false)
            .unwrap_err()
            .code,
        "E-SOURCE-CONFLICT"
    );
}

#[cfg(feature = "oracle-faults")]
#[test]
fn another_session_recovery_marker_is_freshly_respected_by_existing_workspace() {
    let temp = fixture();
    let mut first = Workspace::open(temp.path()).unwrap();
    edit(&mut first, "sources/other.yaml", "draft");
    let mut second = Workspace::open(temp.path()).unwrap();
    let review = second.prepare_migration(add()).unwrap();
    second
        .apply_migration_with_fault(
            &review.token,
            false,
            native::SetFault::CommitFailure {
                source: "sources/two.yaml".into(),
                rollback_failure: Some("sources/one.yaml".into()),
            },
        )
        .unwrap();
    assert_eq!(first.save_all().unwrap_err().code, "E-RECOVERY-REQUIRED");
    assert_eq!(
        first.undo("sources/other.yaml", false).unwrap_err().code,
        "E-RECOVERY-REQUIRED"
    );
    first.detect_recovery();
    assert!(first.recovery_required);
    assert_eq!(first.recovery_information.len(), 1);
}
