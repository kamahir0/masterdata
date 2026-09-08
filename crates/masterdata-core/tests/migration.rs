use std::path::{Path, PathBuf};

use masterdata_core::{
    AddFieldCommand, FieldDefinition, MigrationCommand, ProjectDocuments, SourceDocument,
    dry_run_migration, parse_yaml_document,
};
use serde_yaml::{Mapping, Value};

fn documents(sources: &[(&str, &str)]) -> ProjectDocuments {
    ProjectDocuments {
        files: sources
            .iter()
            .map(|(path, source)| {
                parse_yaml_document(PathBuf::from(path), source).expect("YAML source")
            })
            .collect(),
    }
}

fn add_field(
    table: &str,
    key: u32,
    name: &str,
    type_name: &str,
    initializer: Option<Value>,
) -> MigrationCommand {
    MigrationCommand::AddField(AddFieldCommand {
        table: table.to_owned(),
        field: FieldDefinition {
            key,
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            nullable: false,
            array: false,
        },
        initializer,
    })
}

fn string(value: &str) -> Value {
    Value::String(value.to_owned())
}

#[test]
fn migration_add_field_plan_is_deterministic_and_source_preserving() {
    // Covers MIGRATION-004, MIGRATION-006, MIGRATION-009, MIGRATION-014,
    // and MIGRATION-015.
    let schema = r#"kind: schema
table: item

fields:
  - key: 0
    name: id
    type: int
  # Keep this field comment and the presentation order.
  - key: 1
    name: label
    type: string

primaryKey:
  fields: [id]
"#;
    let data = r#"kind: data
table: item
records:
  - id: 2
    label: "Second"
  - id: 1
    label: 'First'
"#;
    let unrelated = r#"kind: schema
table: other
fields:
  - key: 0
    name: id
    type: int
primaryKey:
  fields: [id]
"#;
    let snapshot = documents(&[
        ("sources/item-schema.yaml", schema),
        ("sources/item-data.yaml", data),
        ("sources/other-schema.yaml", unrelated),
    ]);
    let original_snapshot = snapshot.clone();
    let command = add_field("item", 2, "description", "string", Some(string("Potion")));

    let first = dry_run_migration(&snapshot, &command).expect("AddField dry-run");
    let second = dry_run_migration(&snapshot, &command).expect("repeat AddField dry-run");

    assert_eq!(first.plan, second.plan);
    assert_eq!(first.transformed_documents, second.transformed_documents);
    assert_eq!(first.plan.target_table, "item");
    assert_eq!(first.plan.affected_record_count, 2);
    assert!(!first.plan.destructive);
    assert!(first.plan.validation.valid);
    assert_eq!(first.plan.affected_files.len(), 2);
    assert_eq!(snapshot, original_snapshot);

    let transformed_schema = first
        .transformed_documents
        .files
        .iter()
        .find(|loaded| {
            matches!(&loaded.document, SourceDocument::Schema(schema) if schema.table == "item")
        })
        .expect("transformed schema");
    assert_eq!(
        transformed_schema.source,
        r#"kind: schema
table: item

fields:
  - key: 0
    name: id
    type: int
  # Keep this field comment and the presentation order.
  - key: 1
    name: label
    type: string
  - key: 2
    name: description
    type: string

primaryKey:
  fields: [id]
"#
    );

    let transformed_data = first
        .transformed_documents
        .files
        .iter()
        .find(
            |loaded| matches!(&loaded.document, SourceDocument::Data(data) if data.table == "item"),
        )
        .expect("transformed data");
    assert_eq!(
        transformed_data.source,
        r#"kind: data
table: item
records:
  - id: 2
    label: "Second"
    description: "Potion"
  - id: 1
    label: 'First'
    description: "Potion"
"#
    );
    assert_eq!(
        first
            .transformed_documents
            .files
            .iter()
            .find(|loaded| loaded.path == Path::new("sources/other-schema.yaml"))
            .expect("unrelated source")
            .source,
        unrelated
    );
}

#[test]
fn add_field_requires_explicit_initializer_when_records_exist() {
    // Covers MIGRATION-006 initializer semantics.
    let snapshot = documents(&[
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n",
        ),
    ]);
    let error = dry_run_migration(&snapshot, &add_field("item", 1, "label", "string", None))
        .expect_err("missing initializer must fail closed");

    assert_eq!(
        error.diagnostic.code,
        "E-MIGRATION-ADD-FIELD-INITIALIZER-REQUIRED"
    );
    assert!(
        error
            .diagnostic
            .related_requirements
            .contains(&"MIGRATION-006".to_owned())
    );
}

#[test]
fn add_field_allows_omitted_initializer_for_an_empty_table() {
    // Covers MIGRATION-006 and SCHEMA-TABLE-001 empty-table behavior.
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n";
    let snapshot = documents(&[("schema.yaml", schema)]);
    let result = dry_run_migration(&snapshot, &add_field("item", 1, "label", "string", None))
        .expect("empty-table AddField");

    assert_eq!(
        result.transformed_documents.files[0].source,
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: label\n    type: string\nprimaryKey:\n  fields: [id]\n"
    );
    assert_eq!(result.plan.affected_record_count, 0);
}

#[test]
fn add_field_canonicalizes_nested_custom_initializer_and_array_value() {
    // Covers MIGRATION-006 canonical constant representations and
    // MIGRATION-015 semantic round-trip for Custom Type and Array values.
    let snapshot = documents(&[
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n",
        ),
        (
            "reward.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: itemId\n      type: int\n    - key: 1\n      name: amount\n      type: int\n",
        ),
    ]);
    let mut initializer_mapping = Mapping::new();
    initializer_mapping.insert(string("amount"), Value::Number(3.into()));
    initializer_mapping.insert(string("itemId"), Value::Number(1001.into()));
    let initializer = Value::Mapping(initializer_mapping);
    let command = MigrationCommand::AddField(AddFieldCommand {
        table: "item".to_owned(),
        field: FieldDefinition {
            key: 1,
            name: "rewards".to_owned(),
            type_name: "Reward".to_owned(),
            nullable: false,
            array: true,
        },
        initializer: Some(Value::Sequence(vec![initializer])),
    });

    let result = dry_run_migration(&snapshot, &command).expect("nested initializer");
    let data = result
        .transformed_documents
        .files
        .iter()
        .find(|loaded| matches!(&loaded.document, SourceDocument::Data(_)))
        .expect("data source");
    assert!(
        data.source
            .contains("    rewards:\n      -\n        itemId: 1001\n        amount: 3\n")
    );
}

#[test]
fn unrelated_invalid_type_does_not_block_add_field_resolution() {
    // Covers MIGRATION-005 and MIGRATION-017 closure classification.
    let snapshot = documents(&[
        (
            "item-schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "item-data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n",
        ),
        (
            "unrelated-invalid-type.yaml",
            "kind: type\nname: Broken\ncustom:\n  fields: []\n",
        ),
    ]);

    dry_run_migration(
        &snapshot,
        &add_field("item", 1, "label", "string", Some(string("ok"))),
    )
    .expect("unrelated type diagnostics are outside the migration closure");
}

#[test]
fn target_table_duplicate_primary_key_does_not_block_add_field_resolution() {
    // Covers MIGRATION-005 and MIGRATION-017: Build Selection constraints are
    // not Migration success gates when the AddField transformation is resolvable.
    let snapshot = documents(&[
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n  - id: 1\n",
        ),
    ]);

    let result = dry_run_migration(
        &snapshot,
        &add_field("item", 1, "label", "string", Some(string("ok"))),
    )
    .expect("duplicate PK values are unrelated to AddField planning");

    assert_eq!(result.plan.affected_record_count, 2);
    let data = result
        .transformed_documents
        .files
        .iter()
        .find(|loaded| matches!(&loaded.document, SourceDocument::Data(_)))
        .expect("transformed data");
    assert_eq!(data.source.matches("    label: \"ok\"\n").count(), 2);
}

#[test]
fn target_table_unrelated_record_diagnostic_does_not_block_add_field_resolution() {
    // Covers MIGRATION-005 and MIGRATION-017: an existing value diagnostic on
    // another field remains outside the AddField success gate when the source
    // can still be parsed, located, patched, and postcondition-checked safely.
    let snapshot = documents(&[
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: amount\n    type: int\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    amount: \"not-an-int\"\n",
        ),
    ]);

    let result = dry_run_migration(
        &snapshot,
        &add_field("item", 2, "label", "string", Some(string("ok"))),
    )
    .expect("unrelated target record value diagnostics must not block AddField");

    let data = result
        .transformed_documents
        .files
        .iter()
        .find(|loaded| matches!(&loaded.document, SourceDocument::Data(_)))
        .expect("transformed data");
    assert!(data.source.contains("    amount: \"not-an-int\"\n"));
    assert!(data.source.contains("    label: \"ok\"\n"));
}

#[test]
fn target_table_schema_resolution_error_blocks_add_field() {
    // Covers MIGRATION-005 and MIGRATION-017: schema/type closure semantics are
    // still blocking even though record-level Build Selection constraints are not.
    let snapshot = documents(&[
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: broken\n    type: MissingType\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    broken: value\n",
        ),
    ]);

    let error = dry_run_migration(
        &snapshot,
        &add_field("item", 2, "label", "string", Some(string("ok"))),
    )
    .expect_err("unresolved target schema type must remain blocking");

    assert_eq!(error.diagnostic.code, "E-TYPE-UNKNOWN-REFERENCE");
}

#[test]
fn add_field_rejects_existing_record_member_with_target_name() {
    // Covers MIGRATION-006 and MIGRATION-017: a record member that directly
    // collides with the new field is operation-relevant and must fail closed.
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n";
    let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    label: legacy\n";
    let snapshot = documents(&[("schema.yaml", schema), ("data.yaml", data)]);
    let original_snapshot = snapshot.clone();

    let error = dry_run_migration(
        &snapshot,
        &add_field("item", 1, "label", "string", Some(string("ok"))),
    )
    .expect_err("existing member with the target name must fail closed");

    assert_eq!(error.diagnostic.code, "E-TABLE-UNKNOWN-RECORD-FIELD");
    assert_eq!(snapshot, original_snapshot);
}

#[test]
fn add_field_rejects_key_collision_without_source_change() {
    // Covers MIGRATION-006 and SCHEMA-KEY-001.
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n";
    let snapshot = documents(&[("schema.yaml", schema)]);
    let error = dry_run_migration(&snapshot, &add_field("item", 0, "label", "string", None))
        .expect_err("MessagePack key collision must fail");

    assert_eq!(error.diagnostic.code, "E-TABLE-DUPLICATE-FIELD-KEY");
    assert_eq!(snapshot.files[0].source, schema);
}
