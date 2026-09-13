use masterdata_core::*;
use std::path::PathBuf;
fn docs() -> ProjectDocuments {
    ProjectDocuments { files: vec![
        parse_yaml_document(PathBuf::from("schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: 'id' # name\n    type: int\n  # retain\n  - key: 2\n    name: note\n    type: string\nprimaryKey:\n  fields: [\n    'id', # key\n  ]\n").unwrap(),
        parse_yaml_document(PathBuf::from("data.yaml"), "kind: data\ntable: item\nrecords:\n  - 'id': 1 # keep\n    note: |\n      hello\n      # scalar data\n  # between\n  - id: 2\n    note: \"world\"\n").unwrap(),
    ] }
}
#[test]
fn rename_preserves_key_comments_quotes_and_updates_only_resolved_references() {
    let mut docs = docs();
    docs.files[0] = parse_yaml_document(
        PathBuf::from("schema.yaml"),
        &(docs.files[0].source.clone()
            + "secondaryKeys:\n  - fields:\n      - id\n      - note\n    nonUnique: true\n"),
    )
    .unwrap();
    let command = MigrationCommand::RenameField(RenameFieldCommand {
        table: "item".into(),
        field: "id".into(),
        new_name: "itemId".into(),
    });
    let result = dry_run_migration(&docs, &command).unwrap();
    assert_eq!(result, dry_run_migration(&docs, &command).unwrap());
    assert!(
        result.transformed_documents.files[0]
            .source
            .contains("name: 'itemId' # name")
    );
    assert!(
        result.transformed_documents.files[0]
            .source
            .contains("'itemId', # key")
    );
    assert!(
        result.transformed_documents.files[0]
            .source
            .contains("- itemId\n")
    );
    assert!(
        result.transformed_documents.files[1]
            .source
            .contains("'itemId': 1 # keep")
    );
    assert!(
        result.transformed_documents.files[1]
            .source
            .contains("# scalar data")
    );
    assert!(!result.plan.destructive);
    assert_eq!(result.plan.field.key, 0);
}
#[test]
fn drop_removes_values_including_literal_content_but_preserves_unrelated_comments() {
    let result = dry_run_migration(
        &docs(),
        &MigrationCommand::DropField(DropFieldCommand {
            table: "item".into(),
            field: "note".into(),
        }),
    )
    .unwrap();
    assert!(result.plan.destructive);
    assert_eq!(result.plan.affected_record_count, 2);
    let data = &result.transformed_documents.files[1].source;
    assert!(data.contains("# between"));
    assert!(data.contains("# keep"));
    assert!(!data.contains("scalar data"));
    assert!(!data.contains("note:"));
    assert!(
        result.transformed_documents.files[0]
            .source
            .contains("# retain")
    );
}
#[test]
fn drop_key_dependency_and_rename_collision_fail_before_mutation() {
    assert!(
        dry_run_migration(
            &docs(),
            &MigrationCommand::DropField(DropFieldCommand {
                table: "item".into(),
                field: "id".into()
            })
        )
        .is_err()
    );
    assert!(
        dry_run_migration(
            &docs(),
            &MigrationCommand::RenameField(RenameFieldCommand {
                table: "item".into(),
                field: "id".into(),
                new_name: "note".into()
            })
        )
        .is_err()
    );
}
#[test]
fn drop_first_compact_record_member_retains_sequence_and_next_member() {
    let mut docs = docs();
    docs.files[1] = parse_yaml_document(
        PathBuf::from("data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - note: hi # keep comment\n    id: 1\n",
    )
    .unwrap();
    let result = dry_run_migration(
        &docs,
        &MigrationCommand::DropField(DropFieldCommand {
            table: "item".into(),
            field: "note".into(),
        }),
    )
    .unwrap();
    assert!(
        result.transformed_documents.files[1]
            .source
            .contains("  - id: 1")
    );
    assert!(
        result.transformed_documents.files[1]
            .source
            .contains("# keep comment")
    );
}
