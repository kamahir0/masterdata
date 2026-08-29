use std::fs;

use masterdata_codegen_csharp::CSharpGenerator;
use masterdata_core::ProjectService;
use tempfile::tempdir;

#[test]
fn renders_an_immutable_scaffold_from_a_schema() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.test\"\nname = \"Codegen\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(
        directory.path().join("sources").join("schema.yaml"),
        "kind: schema\ntable: item\ncsharpName: ItemMaster\nfields:\n  - id: 0\n    name: id\n    type: int\n  - id: 1\n    name: name\n    type: string\n",
    )
    .expect("schema");
    fs::write(
        directory.path().join("sources").join("data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Potion\n",
    )
    .expect("data");

    let plan = ProjectService::new()
        .prepare_build(Some(directory.path()), directory.path())
        .expect("build plan");
    let generated = CSharpGenerator::default()
        .plan(&plan)
        .expect("generation plan");
    assert_eq!(generated.files.len(), 1);
    assert!(
        generated.files[0]
            .contents
            .contains("public sealed partial class ItemMaster")
    );
    assert!(
        generated.files[0]
            .contents
            .contains("// Source table identity: item")
    );
    assert!(generated.notes.iter().any(|note| note.placeholder));
}

#[test]
fn rejects_reserved_csharp_type_names() {
    let directory = fixture_project("kind: schema\ntable: item\ncsharpName: class\nfields: []\n");
    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default().plan(&plan).expect_err("keyword");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-INVALID-TYPE-NAME");
}

#[test]
fn rejects_normalized_property_name_collisions() {
    let directory = fixture_project(
        "kind: schema\ntable: item\nfields:\n  - id: 0\n    name: foo-bar\n    type: int\n  - id: 1\n    name: foo_bar\n    type: int\n",
    );
    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default()
        .plan(&plan)
        .expect_err("property collision");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-PROPERTY-NAME-COLLISION");
}

#[test]
fn rejects_invalid_namespace_before_rendering() {
    let directory = fixture_project("kind: schema\ntable: item\nfields: []\n");
    let plan = build_plan(directory.path());
    let error = CSharpGenerator::new("Masterdata..Generated")
        .plan(&plan)
        .expect_err("namespace");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-INVALID-NAMESPACE");
}

#[test]
fn rejects_type_name_collisions_across_tables() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.collision\"\nname = \"Codegen Collision\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    for (name, table) in [("first.yaml", "foo-bar"), ("second.yaml", "foo_bar")] {
        fs::write(
            directory.path().join("sources").join(name),
            format!("kind: schema\ntable: {table}\nfields: []\n"),
        )
        .expect("schema");
    }

    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default()
        .plan(&plan)
        .expect_err("type collision");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-TYPE-NAME-COLLISION");
}

#[test]
fn rejects_case_insensitive_generated_filename_collisions() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.filename\"\nname = \"Codegen Filename\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    for (name, csharp_name) in [("first.yaml", "Item"), ("second.yaml", "item")] {
        fs::write(
            directory.path().join("sources").join(name),
            format!("kind: schema\ntable: {name}\ncsharpName: {csharp_name}\nfields: []\n"),
        )
        .expect("schema");
    }

    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default()
        .plan(&plan)
        .expect_err("filename collision");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-FILENAME-COLLISION");
}

fn fixture_project(schema: &str) -> tempfile::TempDir {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.test\"\nname = \"Codegen\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(directory.path().join("sources").join("schema.yaml"), schema).expect("schema");
    directory
}

fn build_plan(root: &std::path::Path) -> masterdata_core::BuildPlan {
    ProjectService::new()
        .prepare_build(Some(root), root)
        .expect("build plan")
}
