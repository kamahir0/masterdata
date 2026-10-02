//! Current implementation adapter; fixtures contain only domain intent/bytes.
use masterdata_core::{
    AuthoringMember, AuthoringValue, ProjectDocuments, RecordValueEdit, dry_run_source_edit,
    parse_yaml_document,
};
use serde_json::Value;
use std::{fs, path::Path};

fn value(input: &Value) -> AuthoringValue {
    match input["meaning"].as_str().unwrap() {
        "text" => AuthoringValue::String {
            value: input["text"].as_str().unwrap().into(),
        },
        "integer" => AuthoringValue::Number {
            value: input["decimal"].as_str().unwrap().into(),
        },
        "null" => AuthoringValue::Null,
        "mapping" => AuthoringValue::Mapping {
            entries: input["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|member| AuthoringMember {
                    name: member["name"].as_str().unwrap().into(),
                    value: value(&member["value"]),
                })
                .collect(),
        },
        meaning => panic!("unknown portable value meaning {meaning}"),
    }
}

#[test]
fn independent_source_byte_oracle_and_explicit_legacy_gap() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let mut verified = 0;
    let mut gaps = Vec::new();
    for id in manifest["byteScenarios"].as_array().unwrap() {
        let id = id.as_str().unwrap();
        let case = root.join(id);
        let scenario: Value =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let files = fs::read_dir(case.join("input"))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                parse_yaml_document(
                    entry.file_name().into(),
                    &fs::read_to_string(entry.path()).unwrap(),
                )
                .unwrap()
            })
            .collect();
        let documents = ProjectDocuments { files };
        let expected =
            fs::read(case.join(scenario["expected"]["source"].as_str().unwrap())).unwrap();
        let operation = &scenario["operation"];
        if operation["intent"] == "read-source" {
            assert_eq!(
                fs::read(case.join("input/data.yaml")).unwrap(),
                expected,
                "{id}"
            );
            verified += 1;
            continue;
        }
        let result = dry_run_source_edit(
            &documents,
            Path::new("data.yaml"),
            &[RecordValueEdit {
                record_index: operation["occurrence"].as_u64().unwrap() as usize - 1,
                field: operation["field"].as_str().unwrap().into(),
                value: value(&operation["value"]),
            }],
        );
        if scenario["expected"]["outcome"] == "unsafe-localization" {
            assert!(result.is_err(), "{id}: unsafe source accepted");
            assert_eq!(
                fs::read(case.join("input/data.yaml")).unwrap(),
                expected,
                "{id}: changed bytes"
            );
            verified += 1;
            continue;
        }
        let result = result.unwrap_or_else(|error| panic!("{id}: {error}"));
        if id == "new-mapping-block" {
            assert_ne!(
                result.plan.candidate_source.as_bytes(),
                expected,
                "{id}: gap resolved; remove gap classification"
            );
            assert!(
                result.plan.candidate_source.contains("profile: {"),
                "{id}: unexpected new gap"
            );
            gaps.push(id);
        } else {
            assert_eq!(result.plan.candidate_source.as_bytes(), expected, "{id}");
            verified += 1;
        }
    }
    assert_eq!(gaps, ["new-mapping-block"]);
    println!("portable byte oracle: {verified} verified; target-only legacy gaps: {gaps:?}");
}

#[test]
fn portable_schema_directed_interpretation_oracle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let oracle: Value =
        serde_json::from_slice(&fs::read(root.join("interpretation.json")).unwrap()).unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let schema = format!(
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: value\n    type: {}\n    nullable: {}\nprimaryKey:\n  fields: [id]\n",
            case["target"].as_str().unwrap(),
            case["nullable"].as_bool().unwrap_or(false)
        );
        let data = format!(
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    value: {}\n",
            case["source"].as_str().unwrap()
        );
        let documents = ProjectDocuments {
            files: vec![
                parse_yaml_document("schema.yaml".into(), &schema).unwrap(),
                parse_yaml_document("data.yaml".into(), &data).unwrap(),
            ],
        };
        assert_eq!(
            masterdata_core::validate_documents(&documents).valid,
            case["valid"].as_bool().unwrap(),
            "{case}"
        );
    }
}

#[test]
fn portable_structural_multi_source_byte_oracle() {
    use masterdata_core::{
        AddFieldCommand, DropFieldCommand, FieldDefinition, MigrationCommand, RenameFieldCommand,
        dry_run_migration,
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rewrite-oracle/v1");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for id in manifest["structuralScenarios"].as_array().unwrap() {
        let case = root.join(id.as_str().unwrap());
        let scenario: Value =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let operation = &scenario["operation"];
        let documents = ProjectDocuments {
            files: fs::read_dir(case.join("input"))
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    parse_yaml_document(
                        entry.file_name().into(),
                        &fs::read_to_string(entry.path()).unwrap(),
                    )
                    .unwrap()
                })
                .collect(),
        };
        let table = operation["table"].as_str().unwrap().into();
        let field = operation["field"].as_str().unwrap().into();
        let command = match operation["intent"].as_str().unwrap() {
            "rename-field" => MigrationCommand::RenameField(RenameFieldCommand {
                table,
                field,
                new_name: operation["newName"].as_str().unwrap().into(),
            }),
            "drop-field" => MigrationCommand::DropField(DropFieldCommand { table, field }),
            "add-field" => MigrationCommand::AddField(AddFieldCommand {
                table,
                field: FieldDefinition {
                    key: operation["key"].as_u64().unwrap() as u32,
                    name: field,
                    type_name: operation["type"].as_str().unwrap().into(),
                    nullable: false,
                    array: false,
                },
                initializer: Some(
                    serde_yaml::from_str(operation["initializer"]["decimal"].as_str().unwrap())
                        .unwrap(),
                ),
                position: None,
            }),
            intent => panic!("unsupported structural intent {intent}"),
        };
        let original = documents.clone();
        let result = dry_run_migration(&documents, &command).unwrap();
        assert_eq!(documents, original, "Plan must not mutate input");
        assert_eq!(
            result.plan.destructive,
            scenario["expected"]["destructive"].as_bool().unwrap()
        );
        assert_eq!(
            result.plan.affected_record_count as u64,
            scenario["expected"]["affectedRecordCount"]
                .as_u64()
                .unwrap()
        );
        for file in result.transformed_documents.files {
            assert_eq!(
                file.source.as_bytes(),
                fs::read(case.join("expected").join(&file.path)).unwrap(),
                "{id}: {:?}",
                file.path
            );
        }
    }
}
