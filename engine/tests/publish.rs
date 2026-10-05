use masterdata_engine::{
    delivery::{BuildPlan, SavedConfig},
    native::{
        Outcome,
        artifact::{self, ArtifactFault, ArtifactSet, Guard},
        publish::{MANIFEST, PublishFault, PublishPlan},
    },
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn project(targets: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/rewrite-oracle/v1/consumer/minimal");
    let directory = tempfile::tempdir().unwrap();
    let mut config = fs::read_to_string(fixture.join("masterdata.toml")).unwrap();
    for (kind, path) in targets {
        config.push_str(&format!(
            "\n[[publish.targets]]\nkind = {kind:?}\npath = {path:?}\n"
        ));
    }
    fs::write(directory.path().join("masterdata.toml"), config).unwrap();
    fs::create_dir(directory.path().join("sources")).unwrap();
    for entry in fs::read_dir(fixture.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            directory.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    artifacts(
        directory.path(),
        &[("Item.g.cs", "// original\n"), ("Stale.g.cs", "// stale\n")],
        b"native-binary-original",
    );
    directory
}
fn artifacts(root: &Path, files: &[(&str, &str)], binary: &[u8]) {
    let plan = BuildPlan::capture(root, None).unwrap();
    let files = files
        .iter()
        .map(|(path, bytes)| (path.to_string(), bytes.to_string()))
        .collect();
    let receipt = artifact::receipt(&plan.project().config.project.id, &files, binary);
    assert_eq!(
        Guard::prepare(&plan)
            .unwrap()
            .commit(&files, binary, &receipt, ArtifactFault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &entry.path(), out)
            } else if entry.file_type().unwrap().is_symlink() {
                out.insert(
                    entry.path().strip_prefix(base).unwrap().to_path_buf(),
                    fs::read_link(entry.path())
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                );
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
fn first_publish_is_read_only_until_confirmation_and_supports_safe_missing_shared_parents() {
    let project = project(&[
        ("csharp", "dist/generated"),
        ("binary", "dist/data/masterdata.bytes"),
    ]);
    let before = tree(project.path());
    let plan = PublishPlan::prepare(project.path()).unwrap();
    assert_eq!(tree(project.path()), before);
    assert!(!project.path().join("dist").exists());
    assert_eq!(
        plan.preview().targets[0].additions,
        ["Item.g.cs", "Stale.g.cs"]
    );
    assert_eq!(plan.preview().source_freshness, "not_checked");
    let report = plan.execute(PublishFault::None).unwrap();
    assert_eq!(report.outcome, Outcome::Success);
    assert!(
        report
            .targets
            .iter()
            .all(|target| target.status == "succeeded")
    );
    assert_eq!(report.unity_verification, "not_observed");
    assert_eq!(
        fs::read(project.path().join("dist/generated/Item.g.cs")).unwrap(),
        b"// original\n"
    );
    assert_eq!(
        fs::read(project.path().join("dist/data/masterdata.bytes")).unwrap(),
        b"native-binary-original"
    );
    assert!(
        !project
            .path()
            .join("dist/generated/.masterdata-artifact-set.json")
            .exists()
    );
    assert!(!project.path().join("dist/data").join(MANIFEST).exists());
}
#[test]
fn unmanaged_collision_in_later_target_prevents_every_write_and_parent_creation() {
    let project = project(&[
        ("binary", "dist/first/db.bytes"),
        ("csharp", "dist/generated"),
    ]);
    fs::create_dir_all(project.path().join("dist/generated")).unwrap();
    fs::write(
        project.path().join("dist/generated/Item.g.cs"),
        b"unmanaged",
    )
    .unwrap();
    let before = tree(project.path());
    assert!(PublishPlan::prepare(project.path()).is_err());
    assert_eq!(tree(project.path()), before);
    assert!(!project.path().join("dist/first").exists());
}
#[test]
fn managed_updates_and_stale_retirement_preserve_unmanaged_content_and_unity_meta() {
    let project = project(&[("csharp", "dist/generated")]);
    PublishPlan::prepare(project.path())
        .unwrap()
        .execute(PublishFault::None)
        .unwrap();
    let root = project.path().join("dist/generated");
    for (name, bytes) in [
        ("UserNotes.txt", b"notes".as_slice()),
        ("Item.g.cs.meta", b"item-guid"),
        ("Stale.g.cs.meta", b"stale-guid"),
        ("EditorUtility.cs", b"user-code"),
    ] {
        fs::write(root.join(name), bytes).unwrap();
    }
    artifacts(
        project.path(),
        &[("Item.g.cs", "// updated\n"), ("Added.g.cs", "// added\n")],
        b"updated-native",
    );
    let plan = PublishPlan::prepare(project.path()).unwrap();
    let preview = &plan.preview().targets[0];
    assert_eq!(preview.updates, ["Item.g.cs"]);
    assert_eq!(preview.removals, ["Stale.g.cs"]);
    assert_eq!(preview.additions, ["Added.g.cs"]);
    assert_eq!(
        plan.execute(PublishFault::None).unwrap().outcome,
        Outcome::Success
    );
    assert!(!root.join("Stale.g.cs").exists());
    assert_eq!(
        fs::read(root.join("Stale.g.cs.meta")).unwrap(),
        b"stale-guid"
    );
    assert_eq!(fs::read(root.join("Item.g.cs.meta")).unwrap(), b"item-guid");
    assert_eq!(
        fs::read(root.join("EditorUtility.cs")).unwrap(),
        b"user-code"
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(
        manifest["files"],
        serde_json::json!(["Added.g.cs", "Item.g.cs"])
    );
}
#[test]
fn invalid_deleted_sources_do_not_change_last_successful_publish_eligibility() {
    let project = project(&[("binary", "dist/masterdata.bytes")]);
    fs::write(project.path().join("sources/data.yaml"), "invalid: [").unwrap();
    PublishPlan::prepare(project.path())
        .unwrap()
        .execute(PublishFault::None)
        .unwrap();
    fs::remove_dir_all(project.path().join("sources")).unwrap();
    assert_eq!(
        PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
}
#[test]
fn invalid_receipt_prevents_external_inspection_repair_or_parent_creation() {
    let project = project(&[("binary", "dist/masterdata.bytes")]);
    let receipt = project
        .path()
        .join(".masterdata/output")
        .join(artifact::RECEIPT);
    fs::remove_file(&receipt).unwrap();
    let before = tree(project.path());
    let error = PublishPlan::prepare(project.path()).err().unwrap();
    assert_eq!(error.code, "E-ARTIFACT-RECEIPT");
    assert_eq!(tree(project.path()), before);
    assert!(!project.path().join("dist").exists());
}
#[test]
fn malformed_or_ambiguous_manifest_and_owned_type_changes_never_repair_or_delete() {
    for value in [
        "{}",
        r#"{"version":2,"files":[]}"#,
        r#"{"version":1,"files":["../escape.cs"]}"#,
        r#"{"version":1,"files":["Item.g.cs","Item.g.cs"]}"#,
        r#"{"version":1,"files":[".masterdata-publish-manifest.json"]}"#,
    ] {
        let project = project(&[("csharp", "dist/generated")]);
        let root = project.path().join("dist/generated");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(MANIFEST), value).unwrap();
        fs::write(root.join("keep.txt"), b"keep").unwrap();
        let before = tree(&root);
        assert!(PublishPlan::prepare(project.path()).is_err());
        assert_eq!(tree(&root), before);
    }
    let project = project(&[("csharp", "dist/generated")]);
    PublishPlan::prepare(project.path())
        .unwrap()
        .execute(PublishFault::None)
        .unwrap();
    let path = project.path().join("dist/generated/Item.g.cs");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("user"), b"preserve").unwrap();
    assert!(PublishPlan::prepare(project.path()).is_err());
    assert_eq!(fs::read(path.join("user")).unwrap(), b"preserve");
}
#[test]
fn all_target_protected_regions_and_ownership_overlaps_are_rejected_before_mutation() {
    for targets in [
        vec![("csharp", "dist"), ("binary", "dist/db.bytes")],
        vec![("binary", "dist/db.bytes"), ("binary", "dist/db.bytes")],
        vec![("csharp", "sources/generated")],
        vec![("csharp", ".masterdata")],
        vec![("binary", "masterdata.toml")],
        vec![("csharp", ".")],
        vec![("binary", ".masterdata/cache/bytes")],
    ] {
        let project = project(&targets);
        let before = tree(project.path());
        assert!(
            PublishPlan::prepare(project.path()).is_err(),
            "accepted {targets:?}"
        );
        assert_eq!(tree(project.path()), before);
        assert!(!project.path().join("dist").exists());
    }
}
#[test]
fn confirm_rejects_changed_artifact_config_manifest_and_managed_content_without_mutation() {
    for kind in ["artifact", "config", "manifest", "managed"] {
        let project = project(&[("csharp", "dist/generated")]);
        PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::None)
            .unwrap();
        let plan = PublishPlan::prepare(project.path()).unwrap();
        let path = match kind {
            "artifact" => project.path().join(".masterdata/output/masterdata.bytes"),
            "config" => project.path().join("masterdata.toml"),
            "manifest" => project.path().join("dist/generated").join(MANIFEST),
            "managed" => project.path().join("dist/generated/Item.g.cs"),
            _ => unreachable!(),
        };
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(b"\n ");
        fs::write(&path, bytes).unwrap();
        let before = tree(project.path());
        assert!(
            plan.execute(PublishFault::None).is_err(),
            "accepted stale {kind}"
        );
        assert_eq!(tree(project.path()), before);
    }
}
#[test]
#[cfg(feature = "oracle-faults")]
fn every_independent_target_is_attempted_and_successes_are_retained_after_other_failure() {
    let project = project(&[
        ("binary", "dist/a.bytes"),
        ("binary", "dist/b.bytes"),
        ("csharp", "dist/generated"),
    ]);
    let result = PublishPlan::prepare(project.path())
        .unwrap()
        .execute(PublishFault::BeforeTarget(1))
        .unwrap();
    assert_eq!(result.outcome, Outcome::Failure);
    assert_eq!(
        result.targets.iter().map(|r| r.status).collect::<Vec<_>>(),
        ["succeeded", "failed", "succeeded"]
    );
    assert_eq!(
        fs::read(project.path().join("dist/a.bytes")).unwrap(),
        b"native-binary-original"
    );
    assert!(!project.path().join("dist/b.bytes").exists());
    assert!(project.path().join("dist/generated/Item.g.cs").exists());
    assert_eq!(
        PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
}
#[test]
#[cfg(feature = "oracle-faults")]
fn every_csharp_commit_boundary_rolls_back_only_its_managed_files() {
    for file in 0..4 {
        let project = project(&[("csharp", "dist/generated"), ("binary", "dist/b.bytes")]);
        PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::None)
            .unwrap();
        let root = project.path().join("dist/generated");
        fs::write(root.join("Stale.g.cs.meta"), b"guid").unwrap();
        let old = tree(&root);
        artifacts(
            project.path(),
            &[("Added.g.cs", "// added"), ("Item.g.cs", "// updated")],
            b"native-new",
        );
        let result = PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::AfterFile {
                target: 0,
                file,
                rollback_failure: false,
            })
            .unwrap();
        assert_eq!(result.outcome, Outcome::Failure);
        assert_eq!(tree(&root), old);
        assert_eq!(result.targets[1].outcome, Outcome::Success);
        assert_eq!(
            fs::read(project.path().join("dist/b.bytes")).unwrap(),
            b"native-new"
        );
    }
}
#[test]
#[cfg(feature = "oracle-faults")]
fn rollback_failure_and_unknown_outcome_are_explicit_and_never_implicitly_retried() {
    for fault in [
        PublishFault::AfterFile {
            target: 0,
            file: 0,
            rollback_failure: true,
        },
        PublishFault::UnknownAfterCommit(0),
    ] {
        let project = project(&[("csharp", "dist/generated"), ("binary", "dist/b.bytes")]);
        PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::None)
            .unwrap();
        artifacts(project.path(), &[("Item.g.cs", "// new")], b"native-new");
        let result = PublishPlan::prepare(project.path())
            .unwrap()
            .execute(fault)
            .unwrap();
        assert!(matches!(
            result.outcome,
            Outcome::RecoveryRequired | Outcome::OutcomeUnknown
        ));
        assert!(
            result.targets[0]
                .recovery_directory
                .as_ref()
                .unwrap()
                .is_dir()
        );
        assert_eq!(result.targets[1].outcome, Outcome::Success);
    }
}
#[test]
fn zero_targets_is_a_receipt_valid_no_op_and_directory_relocation_keeps_project_identity() {
    let project = project(&[]);
    let before = tree(project.path());
    let report = PublishPlan::prepare(project.path())
        .unwrap()
        .execute(PublishFault::None)
        .unwrap();
    assert_eq!(report.outcome, Outcome::Success);
    assert!(report.no_op);
    assert_eq!(tree(project.path()), before);
    let moved = tempfile::tempdir().unwrap();
    let root = moved.path().join("renamed-project");
    fs::rename(project.path(), &root).unwrap();
    ArtifactSet::load(&SavedConfig::load(&root).unwrap()).unwrap();
}
#[test]
fn case_aliases_use_actual_destination_namespace_without_universal_case_folding() {
    let project = project(&[
        ("binary", "dist/Database.bytes"),
        ("binary", "dist/database.bytes"),
    ]);
    let probe = project.path().join("CaseProbe");
    fs::write(&probe, b"probe").unwrap();
    let insensitive = project.path().join("caseprobe").exists();
    fs::remove_file(probe).unwrap();
    assert_eq!(PublishPlan::prepare(project.path()).is_err(), insensitive);
    assert!(!project.path().join("dist").exists());
}
#[test]
#[cfg(unix)]
fn unmanaged_symlinks_and_meta_survive_but_colliding_or_ancestor_links_are_rejected() {
    use std::os::unix::fs::symlink;
    let project = project(&[("csharp", "dist/generated")]);
    let root = project.path().join("dist/generated");
    fs::create_dir_all(&root).unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("keep"), b"outside").unwrap();
    symlink(outside.path(), root.join("user-link")).unwrap();
    assert_eq!(
        PublishPlan::prepare(project.path())
            .unwrap()
            .execute(PublishFault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    assert!(
        fs::symlink_metadata(root.join("user-link"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(outside.path().join("keep")).unwrap(), b"outside");
    fs::remove_file(root.join("Item.g.cs")).unwrap();
    symlink(outside.path().join("keep"), root.join("Item.g.cs")).unwrap();
    assert!(PublishPlan::prepare(project.path()).is_err());
    assert_eq!(fs::read(outside.path().join("keep")).unwrap(), b"outside");
}

#[test]
fn protected_source_and_artifact_hard_links_are_not_claimed_as_binary_destinations() {
    for source in ["sources/data.yaml", ".masterdata/output/masterdata.bytes"] {
        let project = project(&[("binary", "dist/linked.bytes")]);
        fs::create_dir(project.path().join("dist")).unwrap();
        fs::hard_link(
            project.path().join(source),
            project.path().join("dist/linked.bytes"),
        )
        .unwrap();
        let before = tree(project.path());
        assert!(PublishPlan::prepare(project.path()).is_err());
        assert_eq!(tree(project.path()), before);
    }
}
