use std::fs;

use masterdata_core::{
    BuildSelection, Project, ProjectDocuments, ProjectService, compute_schema_source_content_hash,
    parse_yaml_document, prepare_build_from_documents, prepare_semantic_build,
};
use tempfile::tempdir;

const IN_MEMORY_SCHEMA: &str = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n";
const IN_MEMORY_DATA: &str = "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n";

fn write_project(root: &std::path::Path, schema: &str, build: &str) {
    fs::create_dir_all(root.join("sources")).expect("sources");
    fs::write(
        root.join("masterdata.toml"),
        format!(
            "[project]\nid = \"pipeline.test\"\nname = \"Pipeline\"\nversion = \"0.1.0\"\n\n[build]\n{build}"
        ),
    )
    .expect("config");
    let schema = if schema.contains("fields: []") && !schema.contains("primaryKey:") {
        schema.replace(
            "fields: []",
            "fields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]",
        )
    } else {
        schema.to_owned()
    };
    fs::write(root.join("sources").join("schema.yaml"), schema).expect("schema");
}

#[test]
fn schema_source_hash_tracks_raw_schema_bytes() {
    let directory = tempdir().expect("temp directory");
    let schema = "kind: schema\ntable: item\nfields: []\n";
    write_project(directory.path(), schema, "");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let first = ProjectService::new()
        .prepare_build(Some(project.root()), project.root())
        .expect("first build plan")
        .schema_source_content_hash;

    fs::write(
        directory.path().join("sources").join("schema.yaml"),
        "# formatting-only source change\nkind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
    )
    .expect("updated schema");
    let second = ProjectService::new()
        .prepare_build(Some(project.root()), project.root())
        .expect("second build plan")
        .schema_source_content_hash;

    assert_ne!(first, second);
}

#[test]
fn schema_source_hash_uses_loaded_source_content() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\nfields: []\n",
        "",
    );

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let documents = project.load_documents().expect("loaded documents");
    let expected = compute_schema_source_content_hash(&documents);

    fs::remove_file(directory.path().join("sources/schema.yaml")).expect("remove source");

    assert_eq!(compute_schema_source_content_hash(&documents), expected);
}

#[test]
fn build_preparation_accepts_loaded_documents() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\nfields: []\n",
        "",
    );

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let documents = project.load_documents().expect("loaded documents");
    fs::remove_file(directory.path().join("sources/schema.yaml")).expect("remove source");

    let semantic = prepare_semantic_build(documents.clone(), &BuildSelection::unfiltered())
        .expect("semantic preparation");
    let plan =
        prepare_build_from_documents(project.info(), documents, &BuildSelection::unfiltered())
            .expect("build plan from snapshot");

    assert_eq!(
        plan.schema_source_content_hash,
        semantic.schema_source_content_hash
    );
    assert!(plan.validation.valid);
}

#[test]
fn semantic_build_from_in_memory_sources() {
    let schema = parse_yaml_document(
        std::path::PathBuf::from("schemas/item.yaml"),
        IN_MEMORY_SCHEMA,
    )
    .expect("schema");
    let data = parse_yaml_document(std::path::PathBuf::from("data/item.yaml"), IN_MEMORY_DATA)
        .expect("data");
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

#[test]
fn type_source_hash_tracks_raw_type_bytes() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\nfields: []\n",
        "",
    );
    fs::write(
        directory.path().join("sources").join("item-id.yaml"),
        "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
    )
    .expect("type");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let first = ProjectService::new()
        .prepare_build(Some(project.root()), project.root())
        .expect("first build plan")
        .schema_source_content_hash;

    fs::write(
        directory.path().join("sources").join("item-id.yaml"),
        "# formatting-only source change\nkind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
    )
    .expect("updated type");
    let second = ProjectService::new()
        .prepare_build(Some(project.root()), project.root())
        .expect("second build plan")
        .schema_source_content_hash;

    assert_ne!(first, second);
}

#[test]
fn build_plan_uses_canonical_artifact_layout_and_cache() {
    let directory = tempdir().expect("temp directory");
    write_project(
        directory.path(),
        "kind: schema\ntable: item\nfields: []\n",
        "artifact_dir = \"generated-artifacts\"\ncache = \"build-cache\"\n",
    );

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let plan = ProjectService::new()
        .prepare_build(Some(project.root()), project.root())
        .expect("build plan");

    assert_eq!(
        plan.artifact_root,
        directory.path().join("generated-artifacts")
    );
    assert_eq!(
        plan.binary_output,
        directory
            .path()
            .join("generated-artifacts/masterdata.bytes")
    );
    assert_eq!(
        plan.csharp_output,
        directory.path().join("generated-artifacts/csharp")
    );
    assert_eq!(plan.cache_directory, directory.path().join("build-cache"));
}
