//! Thin legacy adapter for the portable physical Save/Conflict byte oracle.
use masterdata_app::*;
use masterdata_core::AuthoringValue;
use serde_json::Value;
use std::{fs, path::Path};

#[test]
fn portable_save_scope_topology_and_fresh_identity_oracle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for id in manifest["saveScenarios"].as_array().unwrap() {
        let id = id.as_str().unwrap();
        let case = root.join(id);
        let scenario: Value =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let operation = &scenario["operation"];
        assert_eq!(operation["intent"], "save-current-table");
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("sources")).unwrap();
        fs::copy(
            case.join("input/masterdata.toml"),
            temp.path().join("masterdata.toml"),
        )
        .unwrap();
        for entry in fs::read_dir(case.join("input/sources")).unwrap() {
            let entry = entry.unwrap();
            fs::copy(
                entry.path(),
                temp.path().join("sources").join(entry.file_name()),
            )
            .unwrap();
        }
        let workspace = WorkspaceAuthoringSession::open(Some(temp.path()), temp.path()).unwrap();
        let selected = operation["selectedRecordSource"].as_str().unwrap();
        let view = workspace.select_source(selected).unwrap();
        let context = view.context.unwrap();
        let schema_draft = (!operation["schemaDraft"].is_null()).then(|| TableSchemaSaveDraft {
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
                    nullable: if field.name == operation["schemaDraft"]["field"].as_str().unwrap() {
                        operation["schemaDraft"]["nullable"].as_bool().unwrap()
                    } else {
                        field.nullable
                    },
                    array: field.array,
                })
                .collect(),
        });
        let record_draft = (!operation["recordDraft"].is_null()).then(|| {
            let data = view.data.unwrap();
            TableRecordSaveDraft {
                base_source: data.base_source,
                base_content_identity: data.base_content_identity,
                mutation: AuthoringRecordMutation {
                    edits: vec![AuthoringEdit {
                        record_index: operation["recordDraft"]["occurrence"].as_u64().unwrap()
                            as usize
                            - 1,
                        field: operation["recordDraft"]["field"].as_str().unwrap().into(),
                        value: AuthoringValue::String {
                            value: {
                                assert_eq!(operation["recordDraft"]["value"]["meaning"], "text");
                                operation["recordDraft"]["value"]["text"]
                                    .as_str()
                                    .unwrap()
                                    .into()
                            },
                        },
                    }],
                    ..Default::default()
                },
            }
        });
        let external = &operation["externalChangeAfterBaseCapture"];
        if !external.is_null() {
            fs::write(
                temp.path().join(external["source"].as_str().unwrap()),
                external["bytes"].as_str().unwrap(),
            )
            .unwrap();
        }
        let report = TableAuthoringSession::default()
            .save_current_table_context(
                temp.path(),
                &TableContextSaveRequest {
                    schema_path: context.schema_path,
                    selected_record_source: context.selected_record_source,
                    schema_draft,
                    inline_record_draft: None,
                    record_draft,
                },
            )
            .unwrap();
        let success = scenario["expected"]["outcome"] == "success";
        let mut committed = report
            .files
            .iter()
            .filter(|file| file.status == TableContextFileSaveStatus::Success)
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        committed.sort();
        let expected = scenario["expected"]["committedPhysicalSources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|path| path.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(committed, expected, "{id}: physical Save scope");
        if !success {
            assert!(
                report
                    .files
                    .iter()
                    .any(|file| file.status == TableContextFileSaveStatus::Conflict),
                "{id}: stale identity accepted"
            );
        }
        for entry in fs::read_dir(case.join("expected/sources")).unwrap() {
            let entry = entry.unwrap();
            assert_eq!(
                fs::read(temp.path().join("sources").join(entry.file_name())).unwrap(),
                fs::read(entry.path()).unwrap(),
                "{id}: {:?}",
                entry.file_name()
            );
        }
        assert_eq!(
            fs::read(temp.path().join("masterdata.toml")).unwrap(),
            fs::read(case.join("expected/masterdata.toml")).unwrap(),
            "{id}: Save changed project config"
        );
    }
}

#[test]
fn portable_empty_table_context_never_materializes_record_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/empty-table/input");
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    fs::copy(
        root.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    let source = fs::read(root.join("sources/schema.yaml")).unwrap();
    fs::write(temp.path().join("sources/schema.yaml"), &source).unwrap();
    let workspace = WorkspaceAuthoringSession::open(Some(temp.path()), temp.path()).unwrap();
    let view = workspace.select_source("sources/schema.yaml").unwrap();
    let context = view.context.unwrap();
    assert!(context.record_sources.is_empty());
    assert!(context.selected_record_source.is_none());
    assert!(view.data.is_none());
    assert_eq!(
        fs::read_dir(temp.path().join("sources")).unwrap().count(),
        1
    );
    assert_eq!(
        fs::read(temp.path().join("sources/schema.yaml")).unwrap(),
        source
    );
}

#[test]
fn portable_paste_lossless_targets_and_unsafe_all_or_none_oracle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for id in manifest["pasteScenarios"].as_array().unwrap() {
        let case = root.join(id.as_str().unwrap());
        let scenario: Value =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let operation = &scenario["operation"];
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("sources")).unwrap();
        fs::copy(
            case.join("input/masterdata.toml"),
            temp.path().join("masterdata.toml"),
        )
        .unwrap();
        for entry in fs::read_dir(case.join("input/sources")).unwrap() {
            let entry = entry.unwrap();
            fs::copy(
                entry.path(),
                temp.path().join("sources").join(entry.file_name()),
            )
            .unwrap();
        }
        let path = operation["source"].as_str().unwrap();
        let base = fs::read_to_string(temp.path().join(path)).unwrap();
        let targets = operation["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|occurrence| {
                operation["fields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(move |field| BatchTarget {
                        record_index: Some(occurrence.as_u64().unwrap() as usize - 1),
                        added_record_index: None,
                        field: field.as_str().unwrap().into(),
                    })
            })
            .collect();
        let result = NativeApplicationService::new().preview_data_file_batch(
            Some(temp.path()),
            temp.path(),
            path,
            &base,
            &AuthoringRecordMutation::default(),
            &AuthoringBatchRequest {
                targets,
                clipboard_text: fs::read_to_string(
                    case.join(operation["clipboard"].as_str().unwrap()),
                )
                .unwrap(),
                fill: false,
            },
        );
        if scenario["expected"]["outcome"] == "candidate" {
            let result = result.unwrap();
            assert_eq!(
                result.source.candidate_source.as_bytes(),
                fs::read(case.join("expected/data.yaml")).unwrap(),
                "{id}: candidate bytes"
            );
            assert_eq!(
                result.source.validation.valid,
                scenario["expected"]["semanticValid"].as_bool().unwrap(),
                "{id}: semantics"
            );
        } else {
            assert!(result.is_err(), "{id}: unsafe paste accepted");
        }
        assert_eq!(
            fs::read(temp.path().join(path)).unwrap(),
            base.as_bytes(),
            "{id}: implicit Save"
        );
    }
}

#[test]
fn portable_recovery_required_disk_and_write_gate_oracle() {
    use masterdata_core::{FieldDefinition, MigrationCommitFailureInjection};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let faults: Value =
        serde_json::from_slice(&fs::read(root.join("faults.json")).unwrap()).unwrap();
    let case = &faults["scenarios"][2];
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    fs::copy(
        root.join("save-record/input/masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    for entry in fs::read_dir(root.join(case["input"].as_str().unwrap())).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    let mut session = TableAuthoringSession::default();
    let plan = session
        .plan(
            temp.path(),
            TableOperationInput::Add {
                table: "item".into(),
                field: FieldDefinition {
                    key: 2,
                    name: "rank".into(),
                    type_name: "int".into(),
                    nullable: false,
                    array: false,
                },
                initializer: Some("7".into()),
            },
        )
        .unwrap();
    let index = |name: &str| {
        plan.files
            .iter()
            .position(|file| file.path.ends_with(name))
            .unwrap()
    };
    // Legacy fault indices are translated here, never stored in the oracle.
    let report = session
        .apply_with_failures(
            temp.path(),
            &plan.token,
            false,
            &[
                MigrationCommitFailureInjection::write_file(index("two.yaml")),
                MigrationCommitFailureInjection::rollback_file(index("one.yaml")),
            ],
        )
        .unwrap();
    assert_eq!(report.state, "recovery_required");
    for (name, expected) in case["expected"]["disk"].as_object().unwrap() {
        assert_eq!(
            fs::read(temp.path().join("sources").join(name)).unwrap(),
            fs::read(root.join(expected.as_str().unwrap())).unwrap(),
            "{name}: Recovery disk bytes"
        );
    }
    assert!(session.ensure_mutation_allowed(temp.path()).is_err());
    assert!(session.recheck(temp.path()).unwrap().is_some());
    assert!(
        WorkspaceAuthoringSession::open(Some(temp.path()), temp.path())
            .unwrap()
            .select_source("sources/one.yaml")
            .is_ok()
    );
}

#[cfg(unix)]
#[test]
fn portable_precommit_failure_keeps_exact_disk_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let faults: Value =
        serde_json::from_slice(&fs::read(root.join("faults.json")).unwrap()).unwrap();
    let case = &faults["scenarios"][0];
    let input = root.join(case["input"].as_str().unwrap());
    let temp = tempfile::tempdir().unwrap();
    let sources = temp.path().join("sources");
    fs::create_dir(&sources).unwrap();
    fs::copy(
        input.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    for entry in fs::read_dir(input.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), sources.join(entry.file_name())).unwrap();
    }
    let service = NativeApplicationService::new();
    let base = service
        .open_data_file(Some(temp.path()), temp.path(), "sources/data.yaml")
        .unwrap();
    let operation: Value =
        serde_json::from_slice(&fs::read(root.join("save-record/scenario.json")).unwrap()).unwrap();
    let draft = &operation["operation"]["recordDraft"];
    let original_permissions = fs::metadata(&sources).unwrap().permissions();
    // A real filesystem failure at the semantic precommit boundary, only in
    // this disposable fixture. The corpus does not prescribe permissions.
    fs::set_permissions(&sources, fs::Permissions::from_mode(0o555)).unwrap();
    let report = service.save_data_file_mutation(
        Some(temp.path()),
        temp.path(),
        &base.path,
        &base.base_source,
        &base.base_content_identity,
        &AuthoringRecordMutation {
            edits: vec![AuthoringEdit {
                record_index: draft["occurrence"].as_u64().unwrap() as usize - 1,
                field: draft["field"].as_str().unwrap().into(),
                value: AuthoringValue::String {
                    value: draft["value"]["text"].as_str().unwrap().into(),
                },
            }],
            ..Default::default()
        },
        None,
    );
    fs::set_permissions(&sources, original_permissions).unwrap();
    assert_eq!(report.unwrap().status, SourceSaveStatus::Failure);
    for entry in fs::read_dir(input.join("sources")).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            fs::read(sources.join(entry.file_name())).unwrap(),
            fs::read(entry.path()).unwrap()
        );
    }
}

#[cfg(feature = "authoring-test-faults")]
#[test]
fn portable_unknown_confirmation_keeps_candidate_and_rejects_stale_retry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let faults: Value =
        serde_json::from_slice(&fs::read(root.join("faults.json")).unwrap()).unwrap();
    let case = &faults["scenarios"][1];
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    let input = root.join(case["input"].as_str().unwrap());
    fs::copy(
        input.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    for entry in fs::read_dir(input.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    let service = NativeApplicationService::new();
    let base = service
        .open_data_file(Some(temp.path()), temp.path(), "sources/data.yaml")
        .unwrap();
    let scenario: Value =
        serde_json::from_slice(&fs::read(root.join("save-record/scenario.json")).unwrap()).unwrap();
    let draft = &scenario["operation"]["recordDraft"];
    let mutation = AuthoringRecordMutation {
        edits: vec![AuthoringEdit {
            record_index: draft["occurrence"].as_u64().unwrap() as usize - 1,
            field: draft["field"].as_str().unwrap().into(),
            value: AuthoringValue::String {
                value: draft["value"]["text"].as_str().unwrap().into(),
            },
        }],
        ..Default::default()
    };
    let expected = fs::read(
        root.join(case["candidate"].as_str().unwrap())
            .join("sources/data.yaml"),
    )
    .unwrap();
    let save = || {
        service
            .save_data_file_mutation(
                Some(temp.path()),
                temp.path(),
                &base.path,
                &base.base_source,
                &base.base_content_identity,
                &mutation,
                None,
            )
            .unwrap()
    };
    let guard =
        oracle_faults::fail_source_observation_after_reads(&temp.path().join(&base.path), 1);
    let report = save();
    drop(guard);
    assert_eq!(report.status, SourceSaveStatus::OutcomeUnknown);
    assert!(report.snapshot.is_none());
    assert!(report.current.is_none());
    assert_eq!(fs::read(temp.path().join(&base.path)).unwrap(), expected);
    // A subsequent request with the stale base cannot silently commit again.
    assert_eq!(save().status, SourceSaveStatus::Conflict);
    assert_eq!(fs::read(temp.path().join(&base.path)).unwrap(), expected);
    let observed = service
        .source_content(Some(temp.path()), temp.path(), &base.path)
        .unwrap();
    assert_eq!(observed.source.as_bytes(), expected);
    assert_ne!(observed.content_identity, base.base_content_identity);
}
