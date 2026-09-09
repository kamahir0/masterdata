use std::path::PathBuf;

use masterdata_core::{
    AddFieldCommand, FieldDefinition, MigrationCommand, ProjectDocuments, SourceDocument,
    dry_run_migration, parse_yaml_document,
};
use serde_yaml::Value;

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

fn string(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn add_label() -> MigrationCommand {
    MigrationCommand::AddField(AddFieldCommand {
        table: "item".to_owned(),
        field: FieldDefinition {
            key: 2,
            name: "label".to_owned(),
            type_name: "string".to_owned(),
            nullable: false,
            array: false,
        },
        initializer: Some(string("ok")),
    })
}

fn schema() -> &'static str {
    r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: int
  - key: 1
    name: notes
    type: string
    array: true

primaryKey:
  fields: [id]
"#
}

#[test]
fn add_field_appends_after_sequence_literal_block_scalar_hash_content() {
    // Covers YAML-SUBSET-007/YAML-SUBSET-015 and MIGRATION-006/014/015:
    // a bare literal scalar used directly as a block-sequence item owns its
    // indented `#...` lines; they are not outer comments or append boundaries.
    let data = r#"kind: data
table: item
records:
  - id: 1
    notes:
      - | # preserve this header comment
        # literal sequence content
        second line
"#;
    let snapshot = documents(&[("schema.yaml", schema()), ("data.yaml", data)]);

    let result = dry_run_migration(&snapshot, &add_label())
        .expect("AddField must preserve a sequence literal block scalar region");
    let transformed_data = result
        .transformed_documents
        .files
        .iter()
        .find(|loaded| matches!(&loaded.document, SourceDocument::Data(_)))
        .expect("transformed data");

    assert_eq!(
        transformed_data.source,
        r#"kind: data
table: item
records:
  - id: 1
    notes:
      - | # preserve this header comment
        # literal sequence content
        second line
    label: "ok"
"#
    );

    let SourceDocument::Data(data) = &transformed_data.document else {
        panic!("expected transformed data document");
    };
    assert_eq!(
        data.records[0].get("notes"),
        Some(&Value::Sequence(vec![string(
            "# literal sequence content\nsecond line\n"
        )]))
    );
    assert_eq!(data.records[0].get("label"), Some(&string("ok")));
}

#[test]
fn add_field_keeps_shallower_comment_outside_sequence_literal_scalar() {
    // The first non-empty scalar line fixes the content indentation. A later
    // comment that is shallower than that content indentation is outer source
    // presentation and must remain after the newly appended record member.
    let data = r#"kind: data
table: item
records:
  - id: 1
    notes:
      - |
        scalar content
       # trailing record comment, not scalar data
"#;
    let snapshot = documents(&[("schema.yaml", schema()), ("data.yaml", data)]);

    let result = dry_run_migration(&snapshot, &add_label())
        .expect("AddField must distinguish scalar content from shallower comments");
    let transformed_data = result
        .transformed_documents
        .files
        .iter()
        .find(|loaded| matches!(&loaded.document, SourceDocument::Data(_)))
        .expect("transformed data");

    assert_eq!(
        transformed_data.source,
        r#"kind: data
table: item
records:
  - id: 1
    notes:
      - |
        scalar content
    label: "ok"
       # trailing record comment, not scalar data
"#
    );

    let SourceDocument::Data(data) = &transformed_data.document else {
        panic!("expected transformed data document");
    };
    assert_eq!(
        data.records[0].get("notes"),
        Some(&Value::Sequence(vec![string("scalar content\n")]))
    );
    assert_eq!(data.records[0].get("label"), Some(&string("ok")));
}
