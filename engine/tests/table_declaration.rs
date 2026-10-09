use masterdata_engine::{
    native::{self, Outcome, SourceSetPlan},
    project::Project,
    source::Value,
    table_declaration::{self, Change, Command, ReferenceInput},
    workspace::Workspace,
};
use std::{fs, path::PathBuf};

const SCHEMA: &str = "# table heading\nkind: schema\ntable: item\nfields:\n  - {name: id, key: 0, type: int} # existing flow\n  - key: 1\n    name: note\n    type: string\n  - key: 2\n    name: category\n    type: int\nprimaryKey: {fields: ['id']} # key context\nsecondaryKeys:\n  - {fields: [note], nonUnique: true} # note query\n  - fields: [category] # category query\n\nrecords:\n  - {id: 1, note: 'one', category: 7}\n  - {id: 1, note: \"two\", category: 8}\n# trailing presentation\n";
fn fixture(schema: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    let oracle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/rewrite-oracle/v1/save-both/input/masterdata.toml");
    fs::copy(oracle, temp.path().join("masterdata.toml")).unwrap();
    fs::write(temp.path().join("sources/item.yaml"), schema).unwrap();
    fs::write(
        temp.path().join("sources/data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 3\n    note: original\n    category: 9\n",
    )
    .unwrap();
    temp
}
fn command(project: &Project, change: Change) -> Command {
    let detail = table_declaration::detail(project, "item").unwrap();
    Command {
        table: detail.table,
        source: detail.source,
        identity: detail.identity,
        change,
    }
}
fn reference(name: &str, fields: &[&str]) -> ReferenceInput {
    ReferenceInput {
        name: name.into(),
        fields: fields.iter().map(|s| (*s).into()).collect(),
        target_table: "item".into(),
        target_fields: fields.iter().map(|s| (*s).into()).collect(),
        csharp_name: None,
    }
}
fn commit(temp: &tempfile::TempDir, change: Change, authorized: bool) -> table_declaration::Detail {
    let project = Project::open(temp.path()).unwrap();
    let plan = table_declaration::derive(&project, command(&project, change)).unwrap();
    assert_eq!(plan.affected_records, 0);
    assert!(plan.candidates.keys().all(|p| p == "sources/item.yaml"));
    assert_eq!(
        SourceSetPlan::prepare(&project, plan)
            .unwrap()
            .commit(authorized, native::SetFault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    table_declaration::detail(&Project::open(temp.path()).unwrap(), "item").unwrap()
}
#[test]
fn messagepack_key_edit_preserves_exact_flow_crlf_inline_and_sibling_bytes() {
    let schema = SCHEMA.replace('\n', "\r\n");
    let temp = fixture(&schema);
    let data = fs::read(temp.path().join("sources/data.yaml")).unwrap();
    // Duplicate PK records are intentionally invalid; declaration authoring
    // does not make record validity a Save / Plan success precondition.
    assert!(
        !Project::open(temp.path())
            .unwrap()
            .validate(None)
            .unwrap()
            .0
            .is_empty()
    );
    commit(
        &temp,
        Change::SetFieldKey {
            occurrence: 0,
            key: "8".into(),
        },
        false,
    );
    assert_eq!(
        fs::read(temp.path().join("sources/item.yaml")).unwrap(),
        schema.replace("key: 0", "key: 8").as_bytes()
    );
    assert_eq!(
        fs::read(temp.path().join("sources/data.yaml")).unwrap(),
        data
    );
}
#[test]
fn ordered_primary_and_secondary_edits_preserve_existing_style_and_require_removal_authorization() {
    let temp = fixture(SCHEMA);
    let detail = commit(
        &temp,
        Change::SetPrimaryKey {
            fields: vec!["category".into(), "id".into()],
        },
        false,
    );
    assert_eq!(detail.primary.fields, ["category", "id"]);
    let bytes = fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap();
    assert!(bytes.contains("primaryKey: {fields: ['category', id]} # key context"));
    assert_eq!(
        bytes.split("records:\n").nth(1),
        SCHEMA.split("records:\n").nth(1)
    );
    commit(
        &temp,
        Change::EditSecondaryKey {
            occurrence: 0,
            fields: vec!["note".into()],
            non_unique: false,
        },
        false,
    );
    assert!(
        fs::read_to_string(temp.path().join("sources/item.yaml"))
            .unwrap()
            .contains("{fields: [note], nonUnique: false} # note query")
    );
    commit(
        &temp,
        Change::AddSecondaryKey {
            fields: vec!["id".into()],
            non_unique: false,
        },
        false,
    );
    let project = Project::open(temp.path()).unwrap();
    let plan = SourceSetPlan::prepare(
        &project,
        table_declaration::derive(
            &project,
            command(&project, Change::RemoveSecondaryKey { occurrence: 1 }),
        )
        .unwrap(),
    )
    .unwrap();
    let before = fs::read(temp.path().join("sources/item.yaml")).unwrap();
    assert_eq!(
        plan.commit(false, native::SetFault::None).unwrap_err().code,
        "E-MIGRATION-AUTHORIZATION"
    );
    assert_eq!(
        fs::read(temp.path().join("sources/item.yaml")).unwrap(),
        before
    );
    assert_eq!(
        plan.commit(true, native::SetFault::None).unwrap().outcome,
        Outcome::Success
    );
}
#[test]
fn reference_add_edit_remove_uses_native_resolution_and_new_block_mappings() {
    let temp = fixture(SCHEMA);
    let detail = commit(
        &temp,
        Change::AddReference {
            declaration: reference("byNote", &["note"]),
        },
        false,
    );
    assert_eq!(
        (detail.references[0].multi, detail.references[0].optional),
        (Some(true), Some(false))
    );
    assert_eq!(detail.references[0].helper, "GetByNote");
    let before = fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap();
    assert_eq!(before, SCHEMA.replace("# trailing presentation\n", "references:\n  - name: byNote\n    fields:\n      - note\n    target:\n      table: item\n      fields:\n        - note\n# trailing presentation\n"));
    assert!(before.contains("references:\n  - name: byNote\n    fields:"));
    let mut edited = reference("byNote", &["note"]);
    edited.csharp_name = Some("FindRelated".into());
    commit(
        &temp,
        Change::EditReference {
            occurrence: 0,
            declaration: edited,
        },
        false,
    );
    let edited = fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap();
    assert_eq!(edited.replace("    csharpName: FindRelated\n", ""), before);
    assert!(edited.contains("csharpName: FindRelated"));
    commit(
        &temp,
        Change::EditReference {
            occurrence: 0,
            declaration: reference("byNote", &["note"]),
        },
        false,
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap(),
        before
    );
    commit(&temp, Change::RemoveReference { occurrence: 0 }, true);
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap(),
        SCHEMA.replace(
            "# trailing presentation\n",
            "references:\n  []\n# trailing presentation\n"
        )
    );
}
#[test]
fn invalid_or_stale_declarations_fail_without_mutating_any_source() {
    let temp = fixture(SCHEMA);
    let project = Project::open(temp.path()).unwrap();
    for change in [
        Change::SetFieldKey {
            occurrence: 0,
            key: "1".into(),
        },
        Change::SetFieldKey {
            occurrence: 0,
            key: "-1".into(),
        },
        Change::SetPrimaryKey {
            fields: vec!["missing".into()],
        },
        Change::AddSecondaryKey {
            fields: vec!["id".into()],
            non_unique: false,
        },
        Change::AddReference {
            declaration: reference("byNote", &["missing"]),
        },
    ] {
        assert!(table_declaration::derive(&project, command(&project, change)).is_err());
    }
    let old = command(
        &project,
        Change::SetFieldKey {
            occurrence: 0,
            key: "9".into(),
        },
    );
    fs::write(
        temp.path().join("sources/item.yaml"),
        format!("{SCHEMA}# external\n"),
    )
    .unwrap();
    assert_eq!(
        table_declaration::derive(&Project::open(temp.path()).unwrap(), old)
            .unwrap_err()
            .code,
        "E-MIGRATION-STALE"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap(),
        format!("{SCHEMA}# external\n")
    );
}
#[test]
fn incoming_reference_blocks_key_removal_but_unrelated_invalid_source_does_not_gate_authoring() {
    let temp = fixture(SCHEMA);
    fs::write(
        temp.path().join("sources/invalid.yaml"),
        "not: a masterdata document\n",
    )
    .unwrap();
    commit(
        &temp,
        Change::SetFieldKey {
            occurrence: 1,
            key: "7".into(),
        },
        false,
    );
    commit(
        &temp,
        Change::AddReference {
            declaration: reference("byNote", &["note"]),
        },
        false,
    );
    let project = Project::open(temp.path()).unwrap();
    assert!(
        table_declaration::derive(
            &project,
            command(&project, Change::RemoveSecondaryKey { occurrence: 0 })
        )
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/invalid.yaml")).unwrap(),
        "not: a masterdata document\n"
    );
}
#[test]
fn workspace_saved_plan_preserves_unrelated_dirty_occurrences_and_history() {
    let temp = fixture(SCHEMA);
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select("sources/data.yaml", 0, 10).unwrap();
    w.edit(
        "sources/data.yaml",
        p.revision,
        &p.rows[0].id,
        &["note".into()],
        &Value::Text("draft".into()),
    )
    .unwrap();
    let dirty_before = w.dirty_paths();
    let detail = w
        .table_declaration_detail("sources/item.yaml", "item")
        .unwrap();
    assert_eq!(w.dirty_paths(), dirty_before);
    assert_eq!(detail.dirty_sources, ["sources/data.yaml"]);
    let review = w
        .prepare_table_declaration(Command {
            table: detail.detail.table,
            source: detail.detail.source,
            identity: detail.detail.identity,
            change: Change::SetFieldKey {
                occurrence: 1,
                key: "8".into(),
            },
        })
        .unwrap();
    assert_eq!(review.dirty_dependencies, ["sources/data.yaml"]);
    assert!(review.plan.dirty_sources.is_empty());
    assert_eq!(
        w.apply_migration(&review.plan.token, false)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    let p = w.select("sources/data.yaml", 0, 10).unwrap();
    assert!(p.dirty && p.can_undo);
    assert_eq!(p.rows[0].cells[1].value, Some(Value::Text("draft".into())));
    assert!(w.undo("sources/data.yaml", false).unwrap());
    assert!(!w.select("sources/data.yaml", 0, 10).unwrap().dirty);
    assert!(
        fs::read_to_string(temp.path().join("sources/item.yaml"))
            .unwrap()
            .contains("key: 8")
    );
}
#[test]
fn fresh_native_closure_rejects_non_target_replacement_and_schema_draft_requires_guard() {
    let temp = fixture(SCHEMA);
    let mut w = Workspace::open(temp.path()).unwrap();
    let d = w
        .table_declaration_detail("sources/item.yaml", "item")
        .unwrap()
        .detail;
    let command = Command {
        table: d.table,
        source: d.source,
        identity: d.identity,
        change: Change::SetFieldKey {
            occurrence: 1,
            key: "8".into(),
        },
    };
    let review = w.prepare_table_declaration(command.clone()).unwrap();
    let file = temp.path().join("sources/data.yaml");
    let bytes = fs::read(&file).unwrap();
    fs::rename(&file, temp.path().join("data-original")).unwrap();
    fs::write(&file, bytes).unwrap();
    assert_eq!(
        w.apply_migration(&review.plan.token, false)
            .unwrap_err()
            .code,
        "E-MIGRATION-STALE"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap(),
        SCHEMA
    );
    let p = w.select("sources/item.yaml", 0, 10).unwrap();
    w.edit(
        "sources/item.yaml",
        p.revision,
        &p.rows[0].id,
        &["note".into()],
        &Value::Text("dirty inline".into()),
    )
    .unwrap();
    let review = w.prepare_table_declaration(command).unwrap();
    assert_eq!(review.plan.dirty_sources, ["sources/item.yaml"]);
    assert_eq!(
        w.apply_migration(&review.plan.token, false)
            .unwrap_err()
            .code,
        "E-MIGRATION-DIRTY"
    );
    assert!(w.select("sources/item.yaml", 0, 10).unwrap().dirty);
}
#[test]
fn existing_flow_reference_edits_preserve_quotes_and_new_root_flow_mapping_fails_closed() {
    let old = "  - {name: 'byNote', fields: [note], target: {table: 'item', fields: [note]}, csharpName: 'FindOld'} # ref trivia\n";
    let schema = SCHEMA.replace(
        "# trailing presentation\n",
        &format!("references:\n{old}# trailing presentation\n"),
    );
    let temp = fixture(&schema);
    let mut input = reference("byCategory", &["category"]);
    input.csharp_name = Some("FindNew".into());
    commit(
        &temp,
        Change::EditReference {
            occurrence: 0,
            declaration: input,
        },
        false,
    );
    let new = "  - {name: 'byCategory', fields: [category], target: {table: 'item', fields: [category]}, csharpName: 'FindNew'} # ref trivia\n";
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap(),
        schema.replace(old, new)
    );
    let flow = "{kind: schema, table: item, fields: [{key: 0, name: id, type: int}], primaryKey: {fields: [id]}}\n";
    let temp = fixture(flow);
    let project = Project::open(temp.path()).unwrap();
    assert_eq!(
        table_declaration::derive(
            &project,
            command(
                &project,
                Change::AddReference {
                    declaration: reference("self", &["id"])
                }
            )
        )
        .unwrap_err()
        .code,
        "E-SOURCE-UNSAFE"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("sources/item.yaml")).unwrap(),
        flow
    );
}
#[test]
fn resolved_reference_projection_keeps_optional_nonunique_and_ordered_composite_semantics_in_rust()
{
    let schema = SCHEMA.replace(
        "primaryKey:",
        "  - key: 9\n    name: optionalNote\n    type: string\n    nullable: true\nprimaryKey:",
    );
    let temp = fixture(&schema);
    let mut input = reference("optionalPeer", &["note"]);
    input.fields = vec!["optionalNote".into()];
    let d = commit(&temp, Change::AddReference { declaration: input }, false);
    assert_eq!(
        (
            d.references[0].multi,
            d.references[0].optional,
            d.references[0].selected_key
        ),
        (Some(true), Some(true), Some(1))
    );
    commit(
        &temp,
        Change::AddSecondaryKey {
            fields: vec!["category".into(), "id".into()],
            non_unique: false,
        },
        false,
    );
    let d = commit(
        &temp,
        Change::AddReference {
            declaration: reference("compositePeer", &["category", "id"]),
        },
        false,
    );
    assert_eq!(
        d.references[1].declaration.target_fields,
        ["category", "id"]
    );
    assert_eq!(
        (d.references[1].multi, d.references[1].optional),
        (Some(false), Some(false))
    );
}
#[test]
fn detail_and_plan_reuse_parsed_saved_generations_but_commit_requires_fresh_namespace_work() {
    let temp = fixture(SCHEMA);
    let mut w = Workspace::open(temp.path()).unwrap();
    let (d, m) = masterdata_engine::instrument::measure(|| {
        w.table_declaration_detail("sources/item.yaml", "item")
    });
    let d = d.unwrap().detail;
    assert_eq!(
        (
            m.work.project_discovery,
            m.work.project_enumeration,
            m.work.project_yaml_parse,
            m.work.project_validation
        ),
        (0, 0, 0, 0)
    );
    let command = Command {
        table: d.table,
        source: d.source,
        identity: d.identity,
        change: Change::SetFieldKey {
            occurrence: 1,
            key: "8".into(),
        },
    };
    let (review, m) =
        masterdata_engine::instrument::measure(|| w.prepare_table_declaration(command));
    review.unwrap();
    assert_eq!(
        (
            m.work.project_discovery,
            m.work.project_yaml_parse,
            m.work.project_validation
        ),
        (0, 0, 0)
    );
    assert!(
        m.work.project_enumeration > 0,
        "native Plan preparation must witness actual membership"
    );
    let p = w.select("sources/data.yaml", 0, 10).unwrap();
    assert_eq!(
        (
            p.measurement.work.project_discovery,
            p.measurement.work.project_enumeration,
            p.measurement.work.project_yaml_parse,
            p.measurement.work.project_validation
        ),
        (0, 0, 0, 0)
    );
}
#[cfg(feature = "oracle-faults")]
#[test]
fn failed_or_unknown_table_declaration_attempt_never_retries_and_preserves_other_history() {
    for fault in [
        native::SetFault::CommitFailure {
            source: "sources/item.yaml".into(),
            rollback_failure: None,
        },
        native::SetFault::ObservationUnknown {
            source: "sources/item.yaml".into(),
        },
    ] {
        let temp = fixture(SCHEMA);
        let mut w = Workspace::open(temp.path()).unwrap();
        let p = w.select("sources/data.yaml", 0, 10).unwrap();
        w.edit(
            "sources/data.yaml",
            p.revision,
            &p.rows[0].id,
            &["note".into()],
            &Value::Text("draft".into()),
        )
        .unwrap();
        let d = w
            .table_declaration_detail("sources/item.yaml", "item")
            .unwrap()
            .detail;
        let r = w
            .prepare_table_declaration(Command {
                table: d.table,
                source: d.source,
                identity: d.identity,
                change: Change::SetFieldKey {
                    occurrence: 1,
                    key: "8".into(),
                },
            })
            .unwrap();
        let result = w
            .apply_migration_with_fault(&r.plan.token, false, fault)
            .unwrap();
        assert_ne!(result.outcome, Outcome::Success);
        assert!(matches!(
            result.files[0].commit.as_ref().unwrap().outcome,
            Outcome::Failure | Outcome::OutcomeUnknown
        ));
        let before = fs::read(temp.path().join("sources/item.yaml")).unwrap();
        assert_eq!(
            w.apply_migration(&r.plan.token, false).unwrap_err().code,
            "E-MIGRATION-APPLIED"
        );
        assert_eq!(
            fs::read(temp.path().join("sources/item.yaml")).unwrap(),
            before
        );
        let p = w.select("sources/data.yaml", 0, 10).unwrap();
        assert!(p.dirty && p.can_undo);
        assert_eq!(p.rows[0].cells[1].value, Some(Value::Text("draft".into())));
    }
}
