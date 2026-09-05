use std::path::PathBuf;

use masterdata_core::{
    BuildSelection, ProjectDocuments, parse_yaml_document, prepare_semantic_build,
};

const SCHEMA_YAML: &str = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n";
const DATA_YAML: &str = "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n";

// WHY: This compile consumer is the concrete Standalone Web semantic boundary;
// it must not gain native filesystem, process, or transport dependencies.
// IF REMOVED: a native-only dependency could enter the public semantic path
// without the WASM gate detecting it.
// EVIDENCE: docs/specs/runtime-hosts.md; docs/adr/0006-host-capability-composition.md
// Regression: semantic_build_from_in_memory_sources.
fn main() {
    let schema =
        parse_yaml_document(PathBuf::from("schemas/item.yaml"), SCHEMA_YAML).expect("schema");
    let data = parse_yaml_document(PathBuf::from("data/item.yaml"), DATA_YAML).expect("data");
    let documents = ProjectDocuments {
        files: vec![schema, data],
    };

    let preparation = prepare_semantic_build(documents, &BuildSelection::unfiltered())
        .expect("semantic preparation");

    assert!(preparation.validation.valid);
    assert_eq!(preparation.tables.len(), 1);
    assert_eq!(preparation.tables[0].primary_key.fields, vec!["id"]);
    assert_eq!(preparation.tables[0].records.len(), 1);
}
