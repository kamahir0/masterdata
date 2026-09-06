use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use masterdata_app::{
    NativeApplicationService, PUBLISH_MANIFEST_FILENAME, PublishExecutionFailure,
    PublishExecutionReport, PublishFailureInjection, PublishFailurePoint, PublishTargetStatus,
    write_artifact_set_receipt,
};
use serde_json::json;
use tempfile::{Builder, TempDir};

#[test]
fn execution_failure_continues_to_later_targets() {
    let project = project_with_targets(&[("csharp", "first"), ("csharp", "second")]);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::CSharpWhileStagingNew,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert_status(&failure.report, 1, PublishTargetStatus::Succeeded);
    assert!(project.path().join("second/Item.g.cs").is_file());
    assert!(!project.path().join("first/Item.g.cs").exists());
}

#[test]
fn toctou_failure_on_one_target_does_not_skip_others() {
    let project = project_with_targets(&[("csharp", "first"), ("binary", "second.bytes")]);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::ToctouBeforeTarget,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert_status(&failure.report, 1, PublishTargetStatus::Succeeded);
    assert_eq!(
        fs::read(project.path().join("second.bytes")).expect("binary target"),
        b"binary"
    );
}

#[test]
fn protected_region_revalidation_failure_is_target_local_and_continues() {
    let project = project_with_artifacts_and_source_root(
        &[
            ("csharp", "first"),
            ("binary", "second.bytes"),
            ("csharp", "third"),
        ],
        &[("Item.g.cs", b"generated")],
        b"binary",
        "sources/root",
    );

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 1,
            point: PublishFailurePoint::MutateProtectedRegionBeforeTarget,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Succeeded);
    assert_status(&failure.report, 1, PublishTargetStatus::Failed);
    assert_status(&failure.report, 2, PublishTargetStatus::Succeeded);
    let failure_code = &failure.report.targets[1]
        .failure
        .as_ref()
        .expect("protected-region failure")
        .code;
    assert!(
        matches!(
            failure_code.as_str(),
            "E-PUBLISH-FILESYSTEM-INSPECTION" | "E-PUBLISH-TARGET-PATH-UNSAFE"
        ),
        "unexpected protected-region revalidation diagnostic: {failure_code}"
    );
    assert_eq!(
        fs::read(project.path().join("first/Item.g.cs")).expect("first target"),
        b"generated"
    );
    assert!(!project.path().join("second.bytes").exists());
    assert_eq!(
        fs::read(project.path().join("third/Item.g.cs")).expect("third target"),
        b"generated"
    );
    assert!(project.path().join("sources/root").is_dir());
}

#[cfg(unix)]
#[test]
fn protected_region_overlap_after_phase2_is_target_local() {
    let project = project_with_artifacts(
        &[
            ("csharp", "first"),
            ("csharp", "second"),
            ("csharp", "third"),
        ],
        &[("Item.g.cs", b"generated")],
        b"binary",
    );
    fs::create_dir(project.path().join("second")).expect("second target");

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 1,
            point: PublishFailurePoint::AliasProtectedRegionBeforeTarget,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Succeeded);
    assert_status(&failure.report, 1, PublishTargetStatus::Failed);
    assert_status(&failure.report, 2, PublishTargetStatus::Succeeded);
    assert_eq!(
        failure.report.targets[1]
            .failure
            .as_ref()
            .expect("protected-path failure")
            .code,
        "E-PUBLISH-PROTECTED-PATH"
    );
    assert_eq!(
        fs::read(project.path().join("first/Item.g.cs")).expect("first target"),
        b"generated"
    );
    assert!(!project.path().join("second/Item.g.cs").exists());
    assert_eq!(
        fs::read(project.path().join("third/Item.g.cs")).expect("third target"),
        b"generated"
    );
    assert!(project.path().join("sources").is_dir());
    assert!(!project.path().join("sources").is_symlink());
}

#[test]
fn execution_failure_rolls_back_only_failed_target() {
    let project = project_with_targets(&[("csharp", "first"), ("csharp", "second")]);
    let first = project.path().join("first");
    let second = project.path().join("second");
    prepare_managed_target(&first, &[("Item.g.cs", b"old")], &["Item.g.cs"]);
    let before = snapshot(&first);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::CSharpAfterManagedReplacement,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert_status(&failure.report, 1, PublishTargetStatus::Succeeded);
    assert_eq!(snapshot(&first), before);
    assert_eq!(
        fs::read(second.join("Item.g.cs")).expect("later target"),
        b"generated"
    );
}

#[test]
fn binary_failure_preserves_previous_file() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let destination = project.path().join("dist/masterdata.bytes");
    fs::create_dir_all(destination.parent().expect("binary parent")).expect("binary parent");
    fs::write(&destination, b"old").expect("old binary");

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::BinaryAfterPublication,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert_eq!(fs::read(destination).expect("restored binary"), b"old");
}

#[test]
fn binary_failure_after_previous_file_is_secured_restores_previous_file() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let destination = project.path().join("dist/masterdata.bytes");
    fs::create_dir_all(destination.parent().expect("binary parent")).expect("binary parent");
    fs::write(&destination, b"old").expect("old binary");

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::BinaryAfterPreviousSecured,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert_eq!(fs::read(destination).expect("restored binary"), b"old");
}

#[test]
fn initial_binary_failure_does_not_publish_partial_file() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let destination = project.path().join("dist/masterdata.bytes");

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::BinaryWhilePublishingNew,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert!(!destination.exists());
}

#[test]
fn csharp_failure_preserves_previous_managed_set() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    prepare_managed_target(
        &target,
        &[("Item.g.cs", b"old"), ("Stale.g.cs", b"stale")],
        &["Item.g.cs", "Stale.g.cs"],
    );
    let before = snapshot(&target);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::CSharpWhileRetiringStale,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Failed);
    assert_eq!(snapshot(&target), before);
}

#[test]
fn successful_target_is_not_rolled_back_by_later_failure() {
    let project = project_with_targets(&[("csharp", "first"), ("binary", "second.bytes")]);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 1,
            point: PublishFailurePoint::BinaryAfterPublication,
        }],
    );

    assert_status(&failure.report, 0, PublishTargetStatus::Succeeded);
    assert_status(&failure.report, 1, PublishTargetStatus::Failed);
    assert_eq!(
        fs::read(project.path().join("first/Item.g.cs")).expect("successful target"),
        b"generated"
    );
    assert!(!project.path().join("second.bytes").exists());
}

#[test]
fn publish_returns_error_after_partial_success() {
    let project = project_with_targets(&[("csharp", "first"), ("binary", "second.bytes")]);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 1,
            point: PublishFailurePoint::BinaryBeforePublication,
        }],
    );

    assert_eq!(
        failure.error.diagnostic().code,
        "E-PUBLISH-EXECUTION-FAILED"
    );
    assert_status(&failure.report, 0, PublishTargetStatus::Succeeded);
    assert_status(&failure.report, 1, PublishTargetStatus::Failed);
}

#[test]
fn publish_reports_per_target_status() {
    let project = project_with_targets(&[("csharp", "dist"), ("binary", "masterdata.bytes")]);

    let report = publish_success(&project);

    assert_eq!(report.targets.len(), 2);
    assert_eq!(report.targets[0].index, 0);
    assert_eq!(report.targets[0].configured_path, "dist");
    assert_eq!(report.targets[0].status, PublishTargetStatus::Succeeded);
    assert_eq!(report.targets[1].index, 1);
    assert_eq!(report.targets[1].configured_path, "masterdata.bytes");
    assert_eq!(report.targets[1].status, PublishTargetStatus::Succeeded);
}

#[test]
fn retry_after_partial_success_converges() {
    let project = project_with_targets(&[("csharp", "first"), ("binary", "second.bytes")]);

    let first_failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 1,
            point: PublishFailurePoint::BinaryAfterPublication,
        }],
    );
    assert_status(&first_failure.report, 0, PublishTargetStatus::Succeeded);
    assert!(!project.path().join("second.bytes").exists());

    let report = publish_success(&project);
    assert!(
        report
            .targets
            .iter()
            .all(|target| target.status == PublishTargetStatus::Succeeded)
    );
    assert_eq!(
        fs::read(project.path().join("second.bytes")).expect("retry binary"),
        b"binary"
    );
}

#[test]
fn zero_targets_is_successful_noop() {
    let project = project_with_targets(&[]);

    let report = publish_success(&project);

    assert!(report.targets.is_empty());
}

#[test]
fn csharp_publish_adds_updates_retires_and_writes_current_manifest() {
    let project = project_with_artifacts(
        &[("csharp", "dist")],
        &[("Item.g.cs", b"new"), ("nested/New.g.cs", b"new nested")],
        b"binary",
    );
    let target = project.path().join("dist");
    prepare_managed_target(
        &target,
        &[("Item.g.cs", b"old"), ("Stale.g.cs", b"stale")],
        &["Item.g.cs", "Stale.g.cs"],
    );

    publish_success(&project);

    assert_eq!(fs::read(target.join("Item.g.cs")).unwrap(), b"new");
    assert_eq!(
        fs::read(target.join("nested/New.g.cs")).unwrap(),
        b"new nested"
    );
    assert!(!target.join("Stale.g.cs").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join(PUBLISH_MANIFEST_FILENAME)).unwrap()).unwrap();
    assert_eq!(
        manifest,
        json!({
            "version": 1,
            "files": ["Item.g.cs", "nested/New.g.cs"]
        })
    );
}

#[test]
fn csharp_publish_preserves_unmanaged_files_and_meta() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    prepare_managed_target(&target, &[("Item.g.cs", b"old")], &["Item.g.cs"]);
    fs::write(target.join("Item.g.cs.meta"), b"meta").expect("meta");
    fs::write(target.join("UserNotes.txt"), b"user").expect("unmanaged");
    fs::create_dir_all(target.join("UserDirectory")).expect("user directory");
    fs::write(target.join("UserDirectory/keep.txt"), b"keep").expect("user file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        fs::write(target.join("outside.txt"), b"outside").expect("symlink target");
        symlink(target.join("outside.txt"), target.join("UserLink.txt")).expect("user symlink");
    }
    let unmanaged_before = snapshot_subset(
        &target,
        &["Item.g.cs.meta", "UserNotes.txt", "UserDirectory/keep.txt"],
    );

    publish_success(&project);

    assert_eq!(fs::read(target.join("Item.g.cs.meta")).unwrap(), b"meta");
    assert_eq!(fs::read(target.join("UserNotes.txt")).unwrap(), b"user");
    assert_eq!(
        snapshot_subset(
            &target,
            &["Item.g.cs.meta", "UserNotes.txt", "UserDirectory/keep.txt",]
        ),
        unmanaged_before
    );
    #[cfg(unix)]
    assert_eq!(
        fs::read_link(target.join("UserLink.txt")).expect("preserved user symlink"),
        target.join("outside.txt")
    );
}

#[test]
fn csharp_publish_handles_nested_generated_paths() {
    let project = project_with_artifacts(
        &[("csharp", "dist")],
        &[("a/b/Enemy.g.cs", b"nested")],
        b"binary",
    );

    publish_success(&project);

    assert_eq!(
        fs::read(project.path().join("dist/a/b/Enemy.g.cs")).unwrap(),
        b"nested"
    );
}

#[test]
fn csharp_manifest_failure_rolls_back_managed_set() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    prepare_managed_target(&target, &[("Item.g.cs", b"old")], &["Item.g.cs"]);
    let before = snapshot(&target);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::CSharpWhilePublishingManifest,
        }],
    );

    assert_eq!(snapshot(&target), before);
    assert_eq!(
        failure.report.targets[0]
            .failure
            .as_ref()
            .expect("target diagnostic")
            .code,
        "E-PUBLISH-INJECTED-FAILURE"
    );
}

#[test]
fn csharp_failure_before_manifest_rolls_back_managed_set() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    prepare_managed_target(&target, &[("Item.g.cs", b"old")], &["Item.g.cs"]);
    let before = snapshot(&target);

    let failure = publish_failure(
        &project,
        &[PublishFailureInjection {
            target_index: 0,
            point: PublishFailurePoint::CSharpBeforeManifest,
        }],
    );

    assert_eq!(snapshot(&target), before);
    assert_eq!(
        failure.report.targets[0]
            .failure
            .as_ref()
            .expect("target diagnostic")
            .code,
        "E-PUBLISH-INJECTED-FAILURE"
    );
}
#[test]
fn binary_publish_creates_initial_file() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);

    publish_success(&project);

    assert_eq!(
        fs::read(project.path().join("dist/masterdata.bytes")).unwrap(),
        b"binary"
    );
}

#[test]
fn binary_publish_replaces_existing_file() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let destination = project.path().join("dist/masterdata.bytes");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"old").unwrap();

    publish_success(&project);

    assert_eq!(fs::read(destination).unwrap(), b"binary");
}

#[test]
fn binary_publish_preserves_siblings() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let destination = project.path().join("dist/masterdata.bytes");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"old").unwrap();
    fs::write(project.path().join("dist/keep.meta"), b"meta").unwrap();
    fs::write(project.path().join("dist/keep.txt"), b"user").unwrap();

    publish_success(&project);

    assert_eq!(fs::read(destination).unwrap(), b"binary");
    assert_eq!(
        fs::read(project.path().join("dist/keep.meta")).unwrap(),
        b"meta"
    );
    assert_eq!(
        fs::read(project.path().join("dist/keep.txt")).unwrap(),
        b"user"
    );
}

#[test]
fn publish_uses_receipt_valid_artifacts_without_loading_yaml() {
    let project = project_with_targets(&[("csharp", "dist")]);
    fs::write(project.path().join("sources/invalid.yaml"), b"{{{{").expect("invalid YAML");

    publish_success(&project);

    assert_eq!(
        fs::read(project.path().join("dist/Item.g.cs")).unwrap(),
        b"generated"
    );
}

#[test]
fn receipt_failure_mutates_no_targets() {
    let project = project_with_targets(&[("csharp", "missing/target")]);
    fs::remove_file(
        project
            .path()
            .join(".masterdata/output/.masterdata-artifact-set.json"),
    )
    .unwrap();

    let failure = NativeApplicationService::new()
        .publish(Some(project.path()), project.path())
        .expect_err("receipt failure");

    assert_eq!(
        failure.error.diagnostic().code,
        "E-ARTIFACT-SET-RECEIPT-MISSING"
    );
    assert_eq!(
        failure.report.targets[0].status,
        PublishTargetStatus::NotAttempted
    );
    assert!(!project.path().join("missing").exists());
}

#[test]
fn preflight_failure_mutates_no_targets() {
    let project = project_with_targets(&[("csharp", "dist"), ("csharp", "missing")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join(PUBLISH_MANIFEST_FILENAME), b"malformed").unwrap();

    let failure = NativeApplicationService::new()
        .publish(Some(project.path()), project.path())
        .expect_err("preflight failure");

    assert_eq!(
        failure.error.diagnostic().code,
        "E-PUBLISH-MANIFEST-MALFORMED"
    );
    assert!(
        failure
            .report
            .targets
            .iter()
            .all(|target| target.status == PublishTargetStatus::NotAttempted)
    );
    assert!(!project.path().join("missing").exists());
}

#[test]
fn csharp_rollback_failure_is_structured() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    prepare_managed_target(&target, &[("Item.g.cs", b"old")], &["Item.g.cs"]);

    let failure = publish_failure(
        &project,
        &[
            PublishFailureInjection {
                target_index: 0,
                point: PublishFailurePoint::CSharpAfterManagedReplacement,
            },
            PublishFailureInjection {
                target_index: 0,
                point: PublishFailurePoint::CSharpRollback,
            },
        ],
    );

    assert_eq!(
        failure.error.diagnostic().code,
        "E-PUBLISH-EXECUTION-FAILED"
    );
    assert_eq!(
        failure.report.targets[0]
            .failure
            .as_ref()
            .expect("rollback diagnostic")
            .code,
        "E-PUBLISH-ROLLBACK-FAILED"
    );
}

#[test]
fn binary_rollback_failure_is_structured() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let destination = project.path().join("dist/masterdata.bytes");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"old").unwrap();

    let failure = publish_failure(
        &project,
        &[
            PublishFailureInjection {
                target_index: 0,
                point: PublishFailurePoint::BinaryAfterPublication,
            },
            PublishFailureInjection {
                target_index: 0,
                point: PublishFailurePoint::BinaryRollback,
            },
        ],
    );

    assert_eq!(
        failure.report.targets[0]
            .failure
            .as_ref()
            .expect("rollback diagnostic")
            .code,
        "E-PUBLISH-ROLLBACK-FAILED"
    );
}

fn publish_success(project: &TempDir) -> PublishExecutionReport {
    NativeApplicationService::new()
        .publish(Some(project.path()), project.path())
        .expect("publish success")
}

fn publish_failure(
    project: &TempDir,
    injections: &[PublishFailureInjection],
) -> PublishExecutionFailure {
    NativeApplicationService::new()
        .publish_with_failures(Some(project.path()), project.path(), injections)
        .expect_err("publish failure")
}

fn assert_status(report: &PublishExecutionReport, index: usize, status: PublishTargetStatus) {
    assert_eq!(report.targets[index].status, status);
}

fn project_with_targets(targets: &[(&str, &str)]) -> TempDir {
    project_with_artifacts(targets, &[("Item.g.cs", b"generated")], b"binary")
}

fn project_with_artifacts(
    targets: &[(&str, &str)],
    csharp: &[(&str, &[u8])],
    binary: &[u8],
) -> TempDir {
    project_with_artifacts_and_source_root(targets, csharp, binary, "sources")
}

fn project_with_artifacts_and_source_root(
    targets: &[(&str, &str)],
    csharp: &[(&str, &[u8])],
    binary: &[u8],
    source_root: &str,
) -> TempDir {
    let project = Builder::new()
        .prefix("publish-execution-")
        .tempdir()
        .expect("project directory");
    fs::create_dir_all(project.path().join(source_root)).expect("sources");
    let mut config = String::from(
        "[project]\nid = \"publish.project\"\nname = \"Publish\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [",
    );
    config.push_str(&toml_string(source_root));
    config.push_str(
        "]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
    );
    for (kind, path) in targets {
        config.push_str(&format!(
            "\n[[publish.targets]]\nkind = {kind:?}\npath = {}\n",
            toml_string(path)
        ));
    }
    fs::write(project.path().join("masterdata.toml"), config).expect("project config");

    let artifact_root = project.path().join(".masterdata/output");
    let csharp_root = artifact_root.join("csharp");
    for (relative, bytes) in csharp {
        let path = csharp_root.join(relative);
        fs::create_dir_all(path.parent().expect("artifact parent")).expect("artifact parent");
        fs::write(path, bytes).expect("C# artifact");
    }
    fs::create_dir_all(&csharp_root).expect("C# root");
    fs::write(artifact_root.join("masterdata.bytes"), binary).expect("binary artifact");
    write_artifact_set_receipt(&artifact_root, "publish.project").expect("receipt");
    project
}

fn prepare_managed_target(target: &Path, files: &[(&str, &[u8])], manifest: &[&str]) {
    fs::create_dir_all(target).expect("target");
    for (relative, bytes) in files {
        let path = target.join(relative);
        fs::create_dir_all(path.parent().expect("managed parent")).expect("managed parent");
        fs::write(path, bytes).expect("managed file");
    }
    fs::write(
        target.join(PUBLISH_MANIFEST_FILENAME),
        serde_json::to_vec(&json!({ "version": 1, "files": manifest })).unwrap(),
    )
    .expect("publish manifest");
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut result = BTreeMap::new();
    if fs::symlink_metadata(root).is_err() {
        return result;
    }
    snapshot_recursive(root, root, &mut result);
    result
}

fn snapshot_recursive(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in fs::read_dir(directory).expect("snapshot directory") {
        let path = entry.expect("snapshot entry").path();
        let metadata = fs::symlink_metadata(&path).expect("snapshot metadata");
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            snapshot_recursive(root, &path, result);
        } else {
            result.insert(
                path.strip_prefix(root)
                    .expect("snapshot relative")
                    .to_path_buf(),
                fs::read(path).expect("snapshot bytes"),
            );
        }
    }
}

fn snapshot_subset(root: &Path, paths: &[&str]) -> BTreeMap<PathBuf, Vec<u8>> {
    paths
        .iter()
        .map(|path| {
            (
                PathBuf::from(path),
                fs::read(root.join(path)).expect("snapshot subset file"),
            )
        })
        .collect()
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("TOML string")
}
