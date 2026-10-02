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
    }
}
