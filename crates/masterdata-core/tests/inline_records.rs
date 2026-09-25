use std::path::{Path, PathBuf};

use masterdata_core::{
    AddFieldCommand, AddedRecordDraft, AddedRecordField, AuthoringValue, BuildSelection,
    DropFieldCommand, FieldDefinition, MigrationCommand, ProjectDocuments, RecordTagEdit,
    RecordValueEdit, RenameFieldCommand, SourceRecordMutation, TypeMigrationCommand,
    TypeMigrationOperation, build_type_system, data_file_snapshot, dry_run_migration,
    dry_run_source_edit, dry_run_source_record_mutation, dry_run_type_migration,
    parse_yaml_document, resolve_tables,
};
use serde_yaml::Value;

const INLINE: &str = "kind: schema\ntable: item\n# keep schema text\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n    note: 'First' # keep style\n";
const SPLIT: &str = "kind: data\ntable: item\nrecords:\n  - id: 2\n    note: Second\n";

fn documents(sources: &[(&str, &str)]) -> ProjectDocuments {
    ProjectDocuments {
        files: sources
            .iter()
            .map(|(path, source)| parse_yaml_document(PathBuf::from(path), source).unwrap())
            .collect(),
    }
}

#[test]
fn inline_records_requires_sequence_when_present() {
    let source = INLINE.replace(
        "records:\n  - id: 1\n    note: 'First' # keep style\n",
        "records: null\n",
    );
    assert!(parse_yaml_document(PathBuf::from("item.yaml"), &source).is_err());
}

#[test]
fn inline_and_split_records_resolve_with_physical_provenance() {
    let docs = documents(&[("item.yaml", INLINE), ("more.yaml", SPLIT)]);
    let types = build_type_system(&docs).model.unwrap();
    let build = resolve_tables(&docs, &types, &BuildSelection::unfiltered());
    assert!(build.diagnostics.is_empty(), "{:#?}", build.diagnostics);
    let records = &build.model.unwrap()[0].records;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].source, PathBuf::from("item.yaml"));
    assert_eq!(records[1].source, PathBuf::from("more.yaml"));
    assert_eq!(records[0].record_index, 0);
    assert_eq!(records[1].record_index, 0);
    let snapshot =
        data_file_snapshot(Path::new("."), &[], &docs, vec![], Path::new("item.yaml")).unwrap();
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].cells[1].text, "First");
}

#[test]
fn inline_record_save_preserves_schema_and_unrelated_record_text() {
    let docs = documents(&[("item.yaml", INLINE)]);
    let result = dry_run_source_edit(
        &docs,
        Path::new("item.yaml"),
        &[RecordValueEdit {
            record_index: 0,
            field: "note".into(),
            value: AuthoringValue::String {
                value: "Updated".into(),
            },
        }],
    )
    .unwrap();
    assert_eq!(
        result.plan.candidate_source,
        INLINE.replace("'First'", "'Updated'")
    );
    let added = dry_run_source_record_mutation(
        &docs,
        Path::new("item.yaml"),
        &SourceRecordMutation {
            additions: vec![AddedRecordDraft {
                fields: vec![
                    AddedRecordField {
                        field: "id".into(),
                        value: AuthoringValue::Number { value: "3".into() },
                    },
                    AddedRecordField {
                        field: "note".into(),
                        value: AuthoringValue::String {
                            value: "Third".into(),
                        },
                    },
                ],
                tags: vec![],
            }],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(added.plan.candidate_source.contains("# keep schema text"));
    assert!(
        added
            .plan
            .candidate_source
            .contains("note: 'First' # keep style")
    );
    assert!(added.plan.candidate_source.contains("id: 3"));
}

#[test]
fn first_record_can_be_added_to_empty_inline_table() {
    let empty = INLINE.replace(
        "records:\n  - id: 1\n    note: 'First' # keep style\n",
        "records: []\n",
    );
    let docs = documents(&[("item.yaml", &empty)]);
    let result = dry_run_source_record_mutation(
        &docs,
        Path::new("item.yaml"),
        &SourceRecordMutation {
            additions: vec![AddedRecordDraft {
                fields: vec![
                    AddedRecordField {
                        field: "id".into(),
                        value: AuthoringValue::Number { value: "1".into() },
                    },
                    AddedRecordField {
                        field: "note".into(),
                        value: AuthoringValue::String {
                            value: "First".into(),
                        },
                    },
                ],
                tags: vec![],
            }],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.plan.candidate_source.contains("records:\n  - id: 1"));
    assert!(result.plan.candidate_source.contains("# keep schema text"));
}

#[test]
fn inline_record_tags_and_deletion_keep_schema_text() {
    let source = INLINE.replace(
        "    note: 'First' # keep style\n",
        "    note: 'First' # keep style\n    $tags: [release]\n  - id: 2\n    note: Second\n",
    );
    let docs = documents(&[("item.yaml", &source)]);
    let selected = resolve_tables(
        &docs,
        &build_type_system(&docs).model.unwrap(),
        &BuildSelection::new(["release"], Vec::<String>::new()).unwrap(),
    );
    assert!(
        selected.diagnostics.is_empty(),
        "{:#?}",
        selected.diagnostics
    );
    assert_eq!(selected.model.unwrap()[0].records.len(), 1);

    let result = dry_run_source_record_mutation(
        &docs,
        Path::new("item.yaml"),
        &SourceRecordMutation {
            deletions: vec![1],
            tag_edits: vec![RecordTagEdit {
                record_index: 0,
                tags: vec!["preview".into()],
            }],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.plan.candidate_source.contains("# keep schema text"));
    assert!(result.plan.candidate_source.contains("$tags: [preview]"));
    assert!(!result.plan.candidate_source.contains("note: Second"));
}

#[test]
fn add_field_migrates_inline_and_split_records_without_duplicate_file_plan() {
    let docs = documents(&[("item.yaml", INLINE), ("more.yaml", SPLIT)]);
    let result = dry_run_migration(
        &docs,
        &MigrationCommand::AddField(AddFieldCommand {
            table: "item".into(),
            field: FieldDefinition {
                key: 2,
                name: "price".into(),
                type_name: "int".into(),
                nullable: false,
                array: false,
            },
            initializer: Some(Value::Number(5.into())),
        }),
    )
    .unwrap();
    assert_eq!(result.plan.affected_record_count, 2);
    assert_eq!(result.plan.affected_files.len(), 2);
    let inline = result
        .transformed_documents
        .files
        .iter()
        .find(|file| file.path == Path::new("item.yaml"))
        .unwrap();
    assert!(inline.source.contains("name: price"));
    assert!(inline.source.contains("price: 5"));
    assert!(inline.source.contains("# keep schema text"));
    assert!(inline.source.contains("note: 'First' # keep style"));
}

#[test]
fn rename_and_drop_field_update_inline_records_in_one_file() {
    let docs = documents(&[("item.yaml", INLINE), ("more.yaml", SPLIT)]);
    let rename = dry_run_migration(
        &docs,
        &MigrationCommand::RenameField(RenameFieldCommand {
            table: "item".into(),
            field: "note".into(),
            new_name: "description".into(),
        }),
    )
    .unwrap();
    assert_eq!(rename.plan.affected_files.len(), 2);
    let inline = &rename.transformed_documents.files[0].source;
    assert!(inline.contains("name: description"));
    assert!(inline.contains("description: 'First' # keep style"));
    let dropped = dry_run_migration(
        &docs,
        &MigrationCommand::DropField(DropFieldCommand {
            table: "item".into(),
            field: "note".into(),
        }),
    )
    .unwrap();
    assert_eq!(dropped.plan.affected_files.len(), 2);
    assert!(
        !dropped.transformed_documents.files[0]
            .source
            .contains("note:")
    );
    assert!(
        dropped.transformed_documents.files[0]
            .source
            .contains("# keep schema text")
    );
}

#[test]
fn type_migration_updates_inline_symbolic_record_without_rewriting_schema() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: rarity\n    type: Rarity\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n    rarity: 'Rare' # keep\n";
    let rarity = "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Rare\n      value: 1\n";
    let docs = documents(&[("item.yaml", schema), ("rarity.yaml", rarity)]);
    let result = dry_run_type_migration(
        &docs,
        &TypeMigrationCommand {
            target: "Rarity".into(),
            operation: TypeMigrationOperation::RenameEnumMember {
                member: "Rare".into(),
                new_name: "Epic".into(),
            },
        },
    )
    .unwrap();
    assert_eq!(result.affected_occurrence_count, 1);
    let inline = &result.candidate.transformed_documents.files[0].source;
    assert!(inline.contains("rarity: 'Epic' # keep"));
    assert!(inline.contains("name: rarity"));
}
