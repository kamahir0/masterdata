use masterdata_core::{
    ProjectDocuments, RecordValueEdit, dry_run_source_edit, parse_yaml_document,
};
use std::path::{Path, PathBuf};
#[test]
fn source_edit_preserves_literal_separator_blank_lines() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys: []\n";
    let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: |\n      old\n\n  # next record\n  - id: 2\n    note: other\n";
    let docs = ProjectDocuments {
        files: vec![
            parse_yaml_document(PathBuf::from("schema.yaml"), schema).unwrap(),
            parse_yaml_document(PathBuf::from("data.yaml"), data).unwrap(),
        ],
    };
    let result = dry_run_source_edit(
        &docs,
        Path::new("data.yaml"),
        &[RecordValueEdit {
            record_index: 0,
            field: "note".into(),
            value: "new\n".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        result.plan.candidate_source,
        data.replace("      old", "      new")
    );
    assert!(
        result
            .plan
            .candidate_source
            .contains("      new\n\n  # next record"),
        "unrelated separator blank line was deleted"
    );
}
