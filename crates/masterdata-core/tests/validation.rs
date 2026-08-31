use std::fs;

use masterdata_core::{Project, SourceDocument};
use tempfile::tempdir;

fn write_project(root: &std::path::Path, data: &str) {
    fs::create_dir_all(root.join("sources")).expect("sources");
    fs::write(
        root.join("masterdata.toml"),
        "[project]\nid = \"validation.test\"\nname = \"Validation\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(root.join("sources").join("source.yaml"), data).expect("yaml");
}

#[test]
fn declared_kind_loads_schema_and_data_documents() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\ncsharpName: ItemMaster\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n",
    );
    fs::write(
        directory.path().join("sources").join("records.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Potion\n",
    )
    .expect("records");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let documents = project.load_documents().expect("documents");
    assert_eq!(documents.schemas().count(), 1);
    assert_eq!(documents.data().count(), 1);
    assert!(matches!(
        &documents.files[0].document,
        SourceDocument::Data(_) | SourceDocument::Schema(_)
    ));
    let report = project.validate().expect("validation");
    assert!(report.valid, "{report:?}");
}

#[test]
fn project_006_source_directory_does_not_define_table_identity() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Potion\n",
    );
    fs::write(
        directory.path().join("sources").join("other.yaml"),
        "kind: data\ntable: other\nrecords:\n  - id: 1001\n    name: Hi-Potion\n",
    )
    .expect("duplicate");
    fs::write(
        directory.path().join("sources").join("item-schema.yaml"),
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n",
    )
    .expect("item schema");
    fs::write(
        directory.path().join("sources").join("other-schema.yaml"),
        "kind: schema\ntable: other\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n",
    )
    .expect("other schema");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let report = project.validate().expect("validation report");
    assert!(report.valid, "{report:?}");
    assert_eq!(report.tables, ["item", "other"]);
}

#[test]
fn id_field_is_not_an_implicit_primary_key() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\nprimaryKey:\n  fields: [code]\n",
    );
    fs::write(
        directory.path().join("sources").join("other.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    code: potion\n",
    )
    .expect("ordinary field value");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let report = project.validate().expect("validation report");
    assert!(report.valid, "{report:?}");
}
