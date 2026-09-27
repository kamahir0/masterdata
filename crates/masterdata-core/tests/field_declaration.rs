use masterdata_core::*;
use std::path::PathBuf;

fn docs(inline: bool) -> ProjectDocuments {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: 'note' # keep name\n    type: string # keep type\nprimaryKey:\n  fields: [id]\n";
    let mut files = vec![
        parse_yaml_document(
            PathBuf::from("schema.yaml"),
            &format!(
                "{schema}{}",
                if inline {
                    "records:\n  - id: 1\n    note: null\n"
                } else {
                    ""
                }
            ),
        )
        .unwrap(),
    ];
    if !inline {
        files.push(
            parse_yaml_document(
                PathBuf::from("data.yaml"),
                "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: null\n",
            )
            .unwrap(),
        );
    }
    ProjectDocuments { files }
}

fn change(nullable: bool, array: bool, type_name: &str) -> MigrationCommand {
    MigrationCommand::ChangeFieldDeclaration(ChangeFieldDeclarationCommand {
        table: "item".into(),
        field: "note".into(),
        declaration: FieldDefinition {
            key: 1,
            name: "note".into(),
            type_name: type_name.into(),
            nullable,
            array,
        },
    })
}

#[test]
fn nullable_change_preserves_source_and_supports_inline_and_separate_records() {
    for inline in [true, false] {
        let before = docs(inline);
        let result = dry_run_migration(&before, &change(true, false, "string")).unwrap();
        assert_eq!(result.plan.affected_files.len(), 1);
        assert_eq!(result.plan.affected_record_count, 1);
        let schema = &result.transformed_documents.files[0].source;
        assert!(schema.contains("type: string # keep type\n    nullable: true\n"));
        assert!(schema.contains("name: 'note' # keep name"));
        if !inline {
            assert_eq!(
                result.transformed_documents.files[1].source,
                before.files[1].source
            );
        }
    }
}

#[test]
fn incompatible_values_and_key_changes_are_rejected_before_mutation() {
    let before = docs(false);
    assert!(dry_run_migration(&before, &change(false, true, "string")).is_err());
    assert!(dry_run_migration(&before, &change(false, false, "int")).is_err());
    let key = MigrationCommand::ChangeFieldDeclaration(ChangeFieldDeclarationCommand {
        table: "item".into(),
        field: "id".into(),
        declaration: FieldDefinition {
            key: 0,
            name: "id".into(),
            type_name: "long".into(),
            nullable: false,
            array: false,
        },
    });
    assert_eq!(
        dry_run_migration(&before, &key)
            .unwrap_err()
            .diagnostic()
            .code,
        "E-FIELD-DECL-KEY-DEPENDENCY"
    );
}

#[test]
fn empty_table_can_change_type_and_array() {
    let before = ProjectDocuments { files: vec![parse_yaml_document(PathBuf::from("schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nrecords: []\n").unwrap()] };
    let result = dry_run_migration(&before, &change(false, true, "int")).unwrap();
    assert!(
        result.transformed_documents.files[0]
            .source
            .contains("type: int\n    array: true\n")
    );
}

#[test]
fn declaration_validates_every_record_source_without_rewriting_values() {
    let mut before = docs(false);
    before.files.push(
        parse_yaml_document(
            PathBuf::from("more-data.yaml"),
            "kind: data\ntable: item\nrecords:\n  - id: 2\n    note: 'retained' # keep value\n",
        )
        .unwrap(),
    );
    let changed = dry_run_migration(&before, &change(true, false, "string")).unwrap();
    assert_eq!(changed.plan.affected_record_count, 2);
    assert_eq!(changed.plan.affected_files.len(), 1);
    assert_eq!(
        changed.transformed_documents.files[2].source,
        before.files[2].source
    );
    assert_eq!(
        dry_run_migration(&before, &change(false, false, "int"))
            .unwrap_err()
            .diagnostic()
            .code,
        "E-FIELD-DECL-INVALID-VALUE"
    );
}

#[test]
fn reference_component_change_is_rejected_with_dependency_name() {
    let mut before = docs(false);
    before.files.push(
        parse_yaml_document(
            PathBuf::from("other-schema.yaml"),
            "kind: schema\ntable: other\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: itemNote\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: byNote\n    fields: [itemNote]\n    target:\n      table: item\n      fields: [note]\n",
        )
        .unwrap(),
    );
    let error = dry_run_migration(&before, &change(true, false, "string")).unwrap_err();
    assert_eq!(error.diagnostic().code, "E-FIELD-DECL-REFERENCE-DEPENDENCY");
    assert!(error.diagnostic().message.contains("other.byNote"));
}
