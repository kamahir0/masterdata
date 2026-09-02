use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use masterdata_app::ApplicationService;
use masterdata_core::{BuildSelection, PROJECT_CONFIG_FILENAME};
use tempfile::{Builder, TempDir};

// WHY: The standard test harness runs these real .NET integration tests in
// parallel, but the .NET CLI first-use/NuGet migration state is process-shared.
// IF REMOVED: first-time setup can race and fail before the production builder runs.
// EVIDENCE: .github/workflows/ci.yml; Regression: production_build_handoffs_full_selected_model_through_real_builder.
static DOTNET_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn dotnet_test_guard() -> std::sync::MutexGuard<'static, ()> {
    DOTNET_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("production .NET test lock is not poisoned")
}

#[test]
fn production_build_handoffs_full_selected_model_through_real_builder() {
    let _dotnet_test_guard = dotnet_test_guard();
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping production builder integration test");
        return;
    }

    let project = copy_full_fixture("production project with spaces");
    let execution = ApplicationService::new()
        .build(Some(project.path()), project.path(), false)
        .expect("production build");

    let binary = execution.binary.expect("binary report");
    assert_eq!(binary.status, "ok");
    assert_eq!(binary.table_count, 1);
    assert_eq!(binary.record_count, 2);
    assert!(binary.binary_path.is_file());
    assert!(binary.binary_size > 0);
    assert_eq!(
        binary.binary_path,
        project.path().join(".masterdata/output/masterdata.bytes")
    );

    let table = &execution.plan.tables[0];
    let ids = table
        .records
        .iter()
        .map(|record| record.fields["id"].as_i64().expect("integer id"))
        .collect::<Vec<_>>();
    assert_eq!(ids, [1001, 1002], "records are handed off in PK order");
    assert_eq!(table.secondary_keys[0].index_no, 0);
    assert_eq!(table.secondary_keys[1].index_no, 1);
    assert_eq!(table.secondary_keys[2].index_no, 2);

    let generated = project.path().join(".masterdata/generated/ItemMaster.g.cs");
    let generated_source = fs::read_to_string(&generated).expect("published generated C#");
    assert!(generated_source.contains("MessagePack.MessagePackObject"));
    assert!(generated_source.contains("MasterMemory.PrimaryKey(keyOrder: 0)"));
    assert!(
        generated_source
            .contains("MasterMemory.SecondaryKey(0, keyOrder: 0), MasterMemory.NonUnique")
    );
    assert!(generated_source.contains("MasterMemory.SecondaryKey(1, keyOrder: 0)"));
    assert!(generated_source.contains("MasterMemory.SecondaryKey(2, keyOrder: 0)"));
}

#[test]
fn production_build_accepts_empty_selected_table() {
    let _dotnet_test_guard = dotnet_test_guard();
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping empty-table builder integration test");
        return;
    }

    let project = copy_full_fixture("empty production project with spaces");
    fs::remove_file(project.path().join("sources/catalog-data.yaml")).expect("remove data fixture");

    let execution = ApplicationService::new()
        .build(Some(project.path()), project.path(), false)
        .expect("empty table production build");
    let binary = execution.binary.expect("binary report");
    assert_eq!(binary.table_count, 1);
    assert_eq!(binary.record_count, 0);
    assert!(binary.binary_path.is_file());
    assert!(binary.binary_size > 0);
}

#[test]
fn production_build_uses_resolved_build_selection() {
    let _dotnet_test_guard = dotnet_test_guard();
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping selection builder integration test");
        return;
    }

    let project = copy_full_fixture("selected production project with spaces");
    let selection =
        BuildSelection::new(["production"], std::iter::empty::<&str>()).expect("selection");
    let execution = ApplicationService::new()
        .build_with_selection(Some(project.path()), project.path(), &selection, false)
        .expect("selected production build");
    let binary = execution.binary.expect("binary report");
    assert_eq!(binary.record_count, 1);
    let ids = execution.plan.tables[0]
        .records
        .iter()
        .map(|record| record.fields["id"].as_i64().expect("integer id"))
        .collect::<Vec<_>>();
    assert_eq!(ids, [1001]);
}

#[test]
fn production_build_removes_stale_generated_file_after_source_removal() {
    let _dotnet_test_guard = dotnet_test_guard();
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping stale generated output test");
        return;
    }

    let project = copy_full_fixture("stale generated project with spaces");
    let service = ApplicationService::new();
    service
        .build(Some(project.path()), project.path(), false)
        .expect("initial production build");
    let generated = project.path().join(".masterdata/generated");
    let stale = generated.join("LongFeatures.g.cs");
    assert!(stale.is_file(), "initial build should publish LongFeatures");

    fs::remove_file(project.path().join("sources/long-features.yaml"))
        .expect("remove deleted type source");
    service
        .build(Some(project.path()), project.path(), false)
        .expect("production rebuild after source removal");

    assert!(
        !stale.exists(),
        "removed generated type must not remain stale"
    );
    assert!(generated.join("ItemMaster.g.cs").is_file());
}

#[test]
fn production_build_rolls_back_generated_output_when_binary_publication_fails() {
    let _dotnet_test_guard = dotnet_test_guard();
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping publication rollback test");
        return;
    }

    let project = copy_full_fixture("publication rollback project with spaces");
    let service = ApplicationService::new();
    service
        .build(Some(project.path()), project.path(), false)
        .expect("initial production build");

    let generated = project.path().join(".masterdata/generated");
    let before = snapshot_files(&generated);
    let binary = project.path().join(".masterdata/output/masterdata.bytes");
    fs::remove_file(&binary).expect("remove binary for deterministic failure");
    fs::create_dir(&binary).expect("binary path conflict");
    fs::write(
        project.path().join("sources/new-type.yaml"),
        "kind: type\nname: NewGeneratedType\nvalueObject:\n  underlying: int\n",
    )
    .expect("new type source");

    let error = service
        .build(Some(project.path()), project.path(), false)
        .expect_err("binary publication must fail");

    assert_eq!(error.diagnostic().code, "E-BUILD-BINARY-PUBLISH");
    assert_eq!(snapshot_files(&generated), before);
    assert!(!generated.join("NewGeneratedType.g.cs").exists());
    assert!(
        binary.is_dir(),
        "the injected binary conflict remains untouched"
    );
}

fn copy_full_fixture(label: &str) -> TempDir {
    let directory = Builder::new()
        .prefix(&format!("{label}-"))
        .tempdir()
        .expect("temporary directory");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("full");
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

fn dotnet_available() -> bool {
    let executable = std::env::var_os("MASTERDATA_DOTNET").unwrap_or_else(|| "dotnet".into());
    Command::new(executable)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn snapshot_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut snapshot = BTreeMap::new();
    snapshot_files_recursive(root, root, &mut snapshot);
    snapshot
}

fn snapshot_files_recursive(
    root: &Path,
    directory: &Path,
    snapshot: &mut BTreeMap<PathBuf, Vec<u8>>,
) {
    for entry in fs::read_dir(directory).expect("snapshot directory") {
        let entry = entry.expect("snapshot entry");
        let path = entry.path();
        if path.is_dir() {
            snapshot_files_recursive(root, &path, snapshot);
        } else {
            let relative = path.strip_prefix(root).expect("snapshot relative path");
            snapshot.insert(
                relative.to_path_buf(),
                fs::read(path).expect("snapshot file"),
            );
        }
    }
}
