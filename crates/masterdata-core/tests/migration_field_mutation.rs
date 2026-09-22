use masterdata_core::*;
use std::path::{Path, PathBuf};
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

#[test]
fn rename_updates_reference_source_component_and_drop_remains_fail_closed() {
    let mut documents = docs();
    documents.files.push(
        parse_yaml_document(
            PathBuf::from("category.yaml"),
            "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        )
        .unwrap(),
    );
    documents.files[0] = parse_yaml_document(
        PathBuf::from("schema.yaml"),
        &(documents.files[0].source.clone()
            + "references:\n  - name: category # keep reference\n    csharpName: GetCategory\n    fields: ['id'] # source component\n    target:\n      table: category\n      fields: [id]\n"),
    )
    .unwrap();

    let renamed = dry_run_migration(
        &documents,
        &MigrationCommand::RenameField(RenameFieldCommand {
            table: "item".into(),
            field: "id".into(),
            new_name: "itemId".into(),
        }),
    )
    .expect("Reference source component follows explicit RenameField");
    let item = renamed
        .transformed_documents
        .schemas()
        .find(|(_, schema)| schema.table == "item")
        .unwrap()
        .1;
    assert_eq!(item.references[0].fields, ["itemId"]);
    assert_eq!(item.references[0].target.fields, ["id"]);
    assert_eq!(item.references[0].name, "category");
    assert_eq!(item.references[0].csharp_name.as_deref(), Some("GetCategory"));
    let item_source = renamed
        .transformed_documents
        .files
        .iter()
        .find(|file| file.path == Path::new("schema.yaml"))
        .unwrap()
        .source
        .as_str();
    assert!(item_source.contains("fields: ['itemId'] # source component"));
    assert!(item_source.contains("name: category # keep reference"));

    let error = dry_run_migration(
        &documents,
        &MigrationCommand::DropField(DropFieldCommand {
            table: "category".into(),
            field: "id".into(),
        }),
    )
    .expect_err("DropField must not infer Reference replacement");
    assert_eq!(error.diagnostic().code, "E-MIGRATION-FIELD-PRECONDITION");
}

#[test]
fn rename_updates_inbound_reference_target_component_source_preservingly() {
    let documents = ProjectDocuments {
        files: vec![
            parse_yaml_document(
                PathBuf::from("category.yaml"),
                "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: 'id' # target field\n    type: int\nprimaryKey:\n  fields: ['id'] # target key\n",
            )
            .unwrap(),
            parse_yaml_document(
                PathBuf::from("category-data.yaml"),
                "kind: data\ntable: category\nrecords:\n  - 'id': 1 # target value\n",
            )
            .unwrap(),
            parse_yaml_document(
                PathBuf::from("item.yaml"),
                "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category # preserve\n    csharpName: GetCategoryMaster\n    fields: [categoryId] # source stays\n    target:\n      table: category\n      fields:\n        - 'id' # target follows\n",
            )
            .unwrap(),
        ],
    };

    let renamed = dry_run_migration(
        &documents,
        &MigrationCommand::RenameField(RenameFieldCommand {
            table: "category".into(),
            field: "id".into(),
            new_name: "categoryKey".into(),
        }),
    )
    .expect("inbound Reference target follows explicit RenameField");
    let item = renamed
        .transformed_documents
        .schemas()
        .find(|(_, schema)| schema.table == "item")
        .unwrap()
        .1;
    assert_eq!(item.references[0].fields, ["categoryId"]);
    assert_eq!(item.references[0].target.fields, ["categoryKey"]);
    assert_eq!(
        item.references[0].csharp_name.as_deref(),
        Some("GetCategoryMaster")
    );
    let item_source = renamed
        .transformed_documents
        .files
        .iter()
        .find(|file| file.path == Path::new("item.yaml"))
        .unwrap()
        .source
        .as_str();
    assert!(item_source.contains("fields: [categoryId] # source stays"));
    assert!(item_source.contains("- 'categoryKey' # target follows"));
    assert!(item_source.contains("name: category # preserve"));
    let category_source = renamed
        .transformed_documents
        .files
        .iter()
        .find(|file| file.path == Path::new("category.yaml"))
        .unwrap()
        .source
        .as_str();
    assert!(category_source.contains("name: 'categoryKey' # target field"));
    assert!(category_source.contains("fields: ['categoryKey'] # target key"));
}

fn reference_docs() -> ProjectDocuments {
    let mut documents = docs();
    documents.files.push(
        parse_yaml_document(
            PathBuf::from("category.yaml"),
            "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        )
        .unwrap(),
    );
    documents.files.push(
        parse_yaml_document(
            PathBuf::from("category-data.yaml"),
            "kind: data\ntable: category\nrecords:\n  - id: 1\n  - id: 2\n",
        )
        .unwrap(),
    );
    documents
}

#[test]
fn reference_add_edit_remove_use_reviewed_source_patches() {
    let reference = ReferenceDefinition {
        name: "category".into(),
        csharp_name: None,
        fields: vec!["id".into()],
        target: ReferenceTargetDefinition {
            table: "category".into(),
            fields: vec!["id".into()],
        },
    };
    let add = dry_run_migration(
        &reference_docs(),
        &MigrationCommand::AddReference(AddReferenceCommand {
            table: "item".into(),
            reference: reference.clone(),
        }),
    )
    .unwrap();
    let schema_after_add = &add
        .transformed_documents
        .files
        .iter()
        .find(|file| file.path == Path::new("schema.yaml"))
        .unwrap()
        .source;
    assert!(schema_after_add.contains("references:\n  - name: category"));
    assert!(schema_after_add.contains("# retain"));

    let edited = dry_run_migration(
        &add.transformed_documents,
        &MigrationCommand::EditReference(EditReferenceCommand {
            table: "item".into(),
            name: "category".into(),
            reference: ReferenceDefinition {
                name: "categories".into(),
                csharp_name: Some("GetCategories".into()),
                fields: vec!["id".into()],
                target: ReferenceTargetDefinition {
                    table: "category".into(),
                    fields: vec!["id".into()],
                },
            },
        }),
    )
    .unwrap();
    assert!(
        edited
            .transformed_documents
            .files
            .iter()
            .any(|file| file.source.contains("name: categories"))
    );
    assert!(
        edited
            .transformed_documents
            .files
            .iter()
            .any(|file| file.source.contains("csharpName: GetCategories"))
    );

    let removed = dry_run_migration(
        &edited.transformed_documents,
        &MigrationCommand::RemoveReference(RemoveReferenceCommand {
            table: "item".into(),
            name: "categories".into(),
        }),
    )
    .unwrap();
    let removed_schema = removed
        .transformed_documents
        .files
        .iter()
        .find(|file| file.path == Path::new("schema.yaml"))
        .unwrap();
    assert!(removed_schema.source.contains("references: []"));
    assert!(removed_schema.source.contains("# retain"));
}
