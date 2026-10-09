use masterdata_engine::{
    delivery::{BuildPlan, SavedConfig},
    native::{
        Outcome,
        artifact::{self, ArtifactFault, ArtifactSet, Guard, Receipt},
    },
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn project() -> tempfile::TempDir {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/rewrite-oracle/v1/consumer/minimal");
    let directory = tempfile::tempdir().unwrap();
    fs::copy(
        fixture.join("masterdata.toml"),
        directory.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::create_dir(directory.path().join("sources")).unwrap();
    for entry in fs::read_dir(fixture.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            directory.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    directory
}
fn candidate() -> (BTreeMap<String, String>, Vec<u8>) {
    (
        BTreeMap::from([
            ("Item.g.cs".into(), "// candidate\n".into()),
            ("Removed.g.cs".into(), "// old\n".into()),
        ]),
        b"opaque native output for publication boundary test".to_vec(),
    )
}
fn publish(
    root: &Path,
    files: &BTreeMap<String, String>,
    binary: &[u8],
    fault: ArtifactFault,
) -> artifact::CommitResult {
    let plan = BuildPlan::capture(root, None).unwrap();
    let guard = Guard::prepare(&plan).unwrap();
    let receipt = artifact::receipt(&plan.project().config.project.id, files, binary);
    guard.commit(files, binary, &receipt, fault).unwrap()
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &entry.path(), out)
            } else {
                out.insert(
                    entry.path().strip_prefix(base).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut out)
    }
    out
}
#[test]
fn complete_root_replaces_previous_set_and_prunes_stale_artifacts() {
    let project = project();
    let (mut files, binary) = candidate();
    let sources = tree(&project.path().join("sources"));
    let plan = BuildPlan::capture(project.path(), None).unwrap();
    let guard = Guard::prepare(&plan).unwrap();
    assert!(
        !project.path().join(".masterdata").exists(),
        "read-only prepare created output parent"
    );
    let receipt = artifact::receipt(&plan.project().config.project.id, &files, &binary);
    assert_eq!(
        guard
            .commit(&files, &binary, &receipt, ArtifactFault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    let root = guard.target();
    assert_eq!(fs::read(root.join("masterdata.bytes")).unwrap(), binary);
    assert_eq!(
        serde_json::from_slice::<Receipt>(&fs::read(root.join(artifact::RECEIPT)).unwrap())
            .unwrap(),
        receipt
    );
    files.remove("Removed.g.cs");
    files.insert("Item.g.cs".into(), "// new\n".into());
    assert_eq!(
        publish(project.path(), &files, b"new", ArtifactFault::None).outcome,
        Outcome::Success
    );
    assert!(!root.join("csharp/Removed.g.cs").exists());
    let eligible = ArtifactSet::load(&SavedConfig::load(project.path()).unwrap()).unwrap();
    assert_eq!(eligible.csharp.len(), 1);
    assert_eq!(eligible.binary, b"new");
    assert_eq!(tree(&project.path().join("sources")), sources);
}
#[test]
#[cfg(feature = "oracle-faults")]
fn every_normal_switch_failure_retains_the_complete_previous_root() {
    for fault in [
        ArtifactFault::BeforeSwitch,
        ArtifactFault::AfterOldMoved,
        ArtifactFault::AfterNewMoved,
    ] {
        let project = project();
        let (files, binary) = candidate();
        publish(project.path(), &files, &binary, ArtifactFault::None);
        let root = project.path().join(".masterdata/output");
        let old = tree(&root);
        let new = BTreeMap::from([("New.g.cs".into(), "// new".into())]);
        let outcome = publish(project.path(), &new, b"different", fault);
        assert_eq!(outcome.outcome, Outcome::Failure);
        assert_eq!(tree(&root), old);
        assert!(outcome.retained_backup.is_none());
        ArtifactSet::load(&SavedConfig::load(project.path()).unwrap()).unwrap();
    }
}
#[test]
#[cfg(feature = "oracle-faults")]
fn uncertain_rollback_exposes_recovery_and_retains_owned_previous_bytes() {
    let project = project();
    let (files, binary) = candidate();
    publish(project.path(), &files, &binary, ArtifactFault::None);
    let old = tree(&project.path().join(".masterdata/output"));
    let report = publish(
        project.path(),
        &files,
        b"different",
        ArtifactFault::RollbackFailure,
    );
    assert_eq!(report.outcome, Outcome::RecoveryRequired);
    assert_eq!(tree(report.retained_backup.as_ref().unwrap()), old);
}
#[test]
fn changed_config_object_or_content_never_authorizes_artifact_replacement() {
    for replace in [false, true] {
        let project = project();
        let (files, binary) = candidate();
        publish(project.path(), &files, &binary, ArtifactFault::None);
        let plan = BuildPlan::capture(project.path(), None).unwrap();
        let guard = Guard::prepare(&plan).unwrap();
        let before = tree(guard.target());
        let config = project.path().join("masterdata.toml");
        let bytes = fs::read(&config).unwrap();
        if replace {
            fs::rename(&config, config.with_extension("old")).unwrap();
            fs::write(&config, bytes).unwrap();
        } else {
            let mut bytes = bytes;
            bytes.extend_from_slice(b"\n# changed\n");
            fs::write(&config, bytes).unwrap();
        }
        let receipt = artifact::receipt(&plan.project().config.project.id, &files, &binary);
        assert!(
            guard
                .commit(&files, &binary, &receipt, ArtifactFault::None)
                .is_err()
        );
        assert_eq!(tree(guard.target()), before);
    }
}
#[test]
fn changed_artifact_or_parent_is_a_conflict_and_unrelated_entries_are_preserved() {
    for parent in [false, true] {
        let project = project();
        let (files, binary) = candidate();
        publish(project.path(), &files, &binary, ArtifactFault::None);
        let plan = BuildPlan::capture(project.path(), None).unwrap();
        let guard = Guard::prepare(&plan).unwrap();
        if parent {
            let path = project.path().join(".masterdata");
            fs::rename(&path, project.path().join(".old-output-parent")).unwrap();
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(guard.target().join("masterdata.bytes"), b"external").unwrap();
        }
        let before = tree(guard.target());
        let receipt = artifact::receipt(&plan.project().config.project.id, &files, &binary);
        assert!(
            guard
                .commit(&files, &binary, &receipt, ArtifactFault::None)
                .is_err()
        );
        assert_eq!(tree(guard.target()), before);
    }
    let project = project();
    let (files, binary) = candidate();
    publish(project.path(), &files, &binary, ArtifactFault::None);
    fs::write(
        project.path().join(".masterdata/output/UserNotes.txt"),
        b"unmanaged",
    )
    .unwrap();
    assert!(Guard::prepare(&BuildPlan::capture(project.path(), None).unwrap()).is_err());
    assert_eq!(
        fs::read(project.path().join(".masterdata/output/UserNotes.txt")).unwrap(),
        b"unmanaged"
    );
}
#[test]
fn receipt_eligibility_ignores_invalid_or_deleted_yaml_but_checks_every_artifact_byte() {
    let project = project();
    let (files, binary) = candidate();
    publish(project.path(), &files, &binary, ArtifactFault::None);
    fs::write(project.path().join("sources/data.yaml"), "not: [valid yaml").unwrap();
    ArtifactSet::load(&SavedConfig::load(project.path()).unwrap()).unwrap();
    fs::remove_dir_all(project.path().join("sources")).unwrap();
    ArtifactSet::load(&SavedConfig::load(project.path()).unwrap()).unwrap();
    fs::write(
        project.path().join(".masterdata/output/csharp/Item.g.cs"),
        "tampered",
    )
    .unwrap();
    let error = ArtifactSet::load(&SavedConfig::load(project.path()).unwrap())
        .err()
        .unwrap();
    assert_eq!(error.code, "E-ARTIFACT-RECEIPT");
    assert!(error.message.contains("ARTIFACT-SET-004"));
}
#[test]
fn malformed_missing_extra_and_project_mismatched_receipts_fail_without_repair() {
    for kind in [
        "missing",
        "malformed",
        "extra",
        "identity",
        "version",
        "hash",
        "path",
    ] {
        let project = project();
        let (files, binary) = candidate();
        publish(project.path(), &files, &binary, ArtifactFault::None);
        let root = project.path().join(".masterdata/output");
        let receipt = root.join(artifact::RECEIPT);
        match kind {
            "missing" => fs::remove_file(&receipt).unwrap(),
            "malformed" => fs::write(&receipt, "{}").unwrap(),
            "extra" => fs::write(root.join("csharp/Extra.g.cs"), "extra").unwrap(),
            _ => {
                let mut value: Receipt =
                    serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
                match kind {
                    "identity" => value.project_id = "another".into(),
                    "version" => value.version = 2,
                    "hash" => value.binary.hash = "ABC".into(),
                    "path" => value.csharp[0].path = "../escape.cs".into(),
                    _ => unreachable!(),
                };
                fs::write(&receipt, serde_json::to_vec(&value).unwrap()).unwrap();
            }
        }
        let before = tree(&root);
        assert!(ArtifactSet::load(&SavedConfig::load(project.path()).unwrap()).is_err());
        assert_eq!(tree(&root), before);
    }
}
#[test]
fn protected_missing_cache_case_alias_is_checked_in_the_actual_filesystem_namespace() {
    let project = project();
    let probe = project.path().join("namespace-probe");
    fs::write(&probe, b"probe").unwrap();
    let insensitive = project.path().join("NAMESPACE-PROBE").exists();
    fs::remove_file(probe).unwrap();
    let path = project.path().join("masterdata.toml");
    let config = fs::read_to_string(&path)
        .unwrap()
        .replace(".masterdata/output", ".MASTERDATA/CACHE/sub");
    fs::write(&path, config).unwrap();
    let prepared = Guard::prepare(&BuildPlan::capture(project.path(), None).unwrap());
    assert_eq!(prepared.is_err(), insensitive);
    assert!(!project.path().join(".MASTERDATA").exists());
}
#[test]
#[cfg(unix)]
fn artifact_symlink_and_parent_alias_are_rejected_without_following() {
    use std::os::unix::fs::symlink;
    for parent in [false, true] {
        let project = project();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("keep"), b"unrelated").unwrap();
        if parent {
            symlink(outside.path(), project.path().join(".masterdata")).unwrap();
        } else {
            fs::create_dir(project.path().join(".masterdata")).unwrap();
            symlink(outside.path(), project.path().join(".masterdata/output")).unwrap();
        }
        assert!(
            BuildPlan::capture(project.path(), None)
                .and_then(|plan| Guard::prepare(&plan))
                .is_err()
        );
        assert_eq!(fs::read(outside.path().join("keep")).unwrap(), b"unrelated");
    }
}
