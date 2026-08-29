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
fn schema_001_and_data_001_load_documents_by_declared_kind() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\ncsharpName: ItemMaster\nfields:\n  - id: 0\n    name: id\n    type: int\n",
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
fn data_primary_001_rejects_duplicate_ids_across_files() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Potion\n",
    );
    fs::write(
        directory.path().join("sources").join("other.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Hi-Potion\n",
    )
    .expect("duplicate");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let report = project.validate().expect("validation report");
    assert!(!report.valid);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "DATA-PRIMARY-001")
    );
}
