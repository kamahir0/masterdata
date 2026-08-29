use std::fs;

use masterdata_codegen_csharp::CSharpGenerator;
use masterdata_core::ProjectService;
use tempfile::tempdir;

#[test]
fn codegen_001_renders_an_immutable_scaffold_from_a_schema() {
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
    assert!(generated.notes.iter().any(|note| note.placeholder));
}
