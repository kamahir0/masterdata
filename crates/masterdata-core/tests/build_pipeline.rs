use std::fs;

use masterdata_core::{Project, ProjectService};
use tempfile::tempdir;

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
