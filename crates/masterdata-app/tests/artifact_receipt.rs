use std::fs;
use std::path::{Path, PathBuf};

use masterdata_app::{NativeApplicationService, write_artifact_set_receipt};
use masterdata_core::PROJECT_CONFIG_FILENAME;
use tempfile::{Builder, TempDir};

#[test]
fn receipt_validation_accepts_last_set_after_source_change_without_parsing_yaml() {
    let project = copy_minimal_fixture("receipt validation project");
    let artifact_root = project.path().join(".masterdata/output");
    fs::create_dir_all(artifact_root.join("csharp")).expect("C# directory");
    fs::write(artifact_root.join("csharp/Item.g.cs"), b"generated").expect("C# artifact");
    fs::write(artifact_root.join("masterdata.bytes"), b"binary").expect("binary artifact");
    write_artifact_set_receipt(&artifact_root, "fixture.minimal").expect("receipt");

    fs::write(
        project.path().join("sources/item-schema.yaml"),
        b"this is intentionally not valid masterdata YAML",
    )
    .expect("mutate source");

    let validated = NativeApplicationService::with_dotnet(
        masterdata_dotnet::DotnetBridge::with_executable("/definitely/missing/masterdata-dotnet"),
    )
    .validate_artifact_set(Some(project.path()), project.path())
    .expect("receipt validation must not parse source YAML or invoke .NET");

    assert_eq!(validated.receipt.project_id, "fixture.minimal");
    assert_eq!(validated.csharp.len(), 1);
}

#[test]
fn receipt_validation_rejects_legacy_pre_receipt_set_without_adoption() {
    let project = copy_minimal_fixture("legacy receipt validation project");
    let artifact_root = project.path().join(".masterdata/output");
    fs::create_dir_all(artifact_root.join("csharp")).expect("C# directory");
    fs::write(artifact_root.join("csharp/Item.g.cs"), b"generated").expect("C# artifact");
    fs::write(artifact_root.join("masterdata.bytes"), b"binary").expect("binary artifact");

    let error = NativeApplicationService::new()
        .validate_artifact_set(Some(project.path()), project.path())
        .expect_err("pre-receipt set must not be adopted");

    assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-RECEIPT-MISSING");
    assert!(
        !artifact_root.join(".masterdata-artifact-set.json").exists(),
        "validation must not repair the legacy set"
    );
}

fn copy_minimal_fixture(label: &str) -> TempDir {
    let directory = Builder::new()
        .prefix(&format!("{label}-"))
        .tempdir()
        .expect("temporary directory");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("minimal");
    copy_directory(&fixture, directory.path());
    assert!(directory.path().join(PROJECT_CONFIG_FILENAME).is_file());
    directory
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination");
    for entry in fs::read_dir(source).expect("fixture directory") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("fixture file");
        }
    }
}
