use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::symlink;

use masterdata_app::{
    NativeApplicationService, PUBLISH_MANIFEST_FILENAME, write_artifact_set_receipt,
};
use masterdata_core::Result;
use serde_json::json;
use tempfile::{Builder, TempDir};

#[test]
fn publish_path_zero_targets_returns_empty_plan() {
    let project = project_with_targets(&[]);

    let plan = preflight(&project).expect("receipt-valid zero-target project");

    assert!(plan.targets.is_empty());
}

#[test]
fn publish_path_rejects_filesystem_equivalent_unmanaged_collision() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    fs::write(target.join("Item.g.cs"), b"user-owned").expect("unmanaged file");

    let error = preflight(&project).expect_err("unmanaged collision");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-UNMANAGED-COLLISION");
}

#[test]
fn publish_path_rejects_equivalent_binary_targets() {
    let project = project_with_targets(&[
        ("binary", "dist/masterdata.bytes"),
        ("binary", "dist/masterdata.bytes"),
    ]);

    let error = preflight(&project).expect_err("duplicate binary target");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-TARGET-COLLISION");
}

#[test]
fn publish_path_handles_case_sensitive_and_insensitive_volumes() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    fs::write(target.join("item.g.cs"), b"user-owned").expect("case-variant file");
    let case_equivalent = fs::symlink_metadata(target.join("Item.g.cs")).is_ok();

    let result = preflight(&project);

    if case_equivalent {
        assert_eq!(
            result
                .expect_err("case-equivalent collision")
                .diagnostic()
                .code,
            "E-PUBLISH-UNMANAGED-COLLISION"
        );
    } else {
        result.expect("case-sensitive filesystem keeps distinct entries");
    }
}

#[test]
fn publish_path_accepts_safe_missing_target_tail() {
    let project = project_with_targets(&[("csharp", "missing/target")]);

    let plan = preflight(&project).expect("missing target tail is safe");

    assert_eq!(plan.targets.len(), 1);
    assert_eq!(
        plan.targets[0].destination,
        project.path().join("missing/target")
    );
    assert!(!project.path().join("missing").exists());
}

#[test]
fn publish_path_rejects_unresolvable_identity_without_mutation() {
    let project =
        project_with_targets(&[("csharp", "future/Café"), ("csharp", "future/Cafe\u{301}")]);
    let future = project.path().join("future");
    fs::create_dir_all(&future).expect("future parent");
    let before = fs::read_dir(&future)
        .expect("future entries")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("future snapshot")
        .len();

    let error = preflight(&project).expect_err("unresolvable Unicode identity");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-FILESYSTEM-IDENTITY");
    assert_eq!(
        fs::read_dir(&future)
            .expect("future entries")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("future snapshot")
            .len(),
        before
    );
}

#[cfg(unix)]
#[test]
fn publish_path_rejects_csharp_target_symlink() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let actual = project.path().join("actual");
    fs::create_dir_all(&actual).expect("actual target");
    symlink(&actual, project.path().join("dist")).expect("target symlink");

    let error = preflight(&project).expect_err("C# target symlink");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-SYMLINK-ANCESTOR");
}

#[cfg(unix)]
#[test]
fn publish_path_rejects_csharp_ancestor_symlink() {
    let project = project_with_targets(&[("csharp", "link/dist")]);
    let actual = project.path().join("actual");
    fs::create_dir_all(&actual).expect("actual parent");
    symlink(&actual, project.path().join("link")).expect("ancestor symlink");

    let error = preflight(&project).expect_err("C# ancestor symlink");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-SYMLINK-ANCESTOR");
}

#[cfg(unix)]
#[test]
fn publish_path_never_follows_unmanaged_symlink() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    let outside = project.path().join("outside.g.cs");
    fs::create_dir_all(&target).expect("target");
    fs::write(&outside, b"outside").expect("outside file");
    symlink(&outside, target.join("Item.g.cs")).expect("unmanaged symlink");

    let error = preflight(&project).expect_err("symlink collision");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-SYMLINK-ANCESTOR");
    assert_eq!(fs::read(&outside).expect("outside file"), b"outside");
}

#[test]
fn publish_path_rejects_case_or_unicode_equivalent_unmanaged_csharp_entry() {
    let project = project_with_targets(&[("csharp", "dist")]);
    replace_csharp_artifact(&project, "Café.g.cs");
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    fs::write(target.join("Cafe\u{301}.g.cs"), b"user-owned").expect("Unicode alias");
    let unicode_equivalent = fs::symlink_metadata(target.join("Café.g.cs")).is_ok();

    let result = preflight(&project);

    if unicode_equivalent {
        assert_eq!(
            result
                .expect_err("Unicode-equivalent collision")
                .diagnostic()
                .code,
            "E-PUBLISH-UNMANAGED-COLLISION"
        );
    } else {
        result.expect("distinct Unicode entries are unmanaged siblings");
    }
}

#[test]
fn publish_path_rejects_managed_path_type_change() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(target.join("Item.g.cs")).expect("replacement directory");
    write_manifest(&target, &["Item.g.cs"]);

    let error = preflight(&project).expect_err("managed path type change");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-MANAGED-OWNERSHIP");
}

#[test]
fn publish_path_preserves_meta_and_unmanaged_files() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    fs::write(target.join("Item.g.cs"), b"previous").expect("managed file");
    fs::write(target.join("Item.g.cs.meta"), b"meta").expect("meta");
    fs::write(target.join("UserNotes.txt"), b"notes").expect("unmanaged file");
    write_manifest(&target, &["Item.g.cs"]);
    let manifest_before = fs::read(target.join(PUBLISH_MANIFEST_FILENAME)).expect("manifest");

    let plan = preflight(&project).expect("preserve unmanaged content");

    assert_eq!(
        plan.targets[0]
            .csharp
            .as_ref()
            .unwrap()
            .previous_managed_paths,
        ["Item.g.cs"]
    );
    assert_eq!(
        fs::read(target.join("Item.g.cs.meta")).expect("meta"),
        b"meta"
    );
    assert_eq!(
        fs::read(target.join("UserNotes.txt")).expect("notes"),
        b"notes"
    );
    assert_eq!(
        fs::read(target.join(PUBLISH_MANIFEST_FILENAME)).expect("manifest"),
        manifest_before
    );
}

#[test]
fn publish_path_rejects_malformed_manifest_without_mutation() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    let malformed = b"{ this is not JSON }";
    fs::write(target.join(PUBLISH_MANIFEST_FILENAME), malformed).expect("manifest");
    fs::write(target.join("UserNotes.txt"), b"notes").expect("unmanaged file");

    let error = preflight(&project).expect_err("malformed manifest");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-MANIFEST-MALFORMED");
    assert_eq!(
        fs::read(target.join(PUBLISH_MANIFEST_FILENAME)).expect("manifest"),
        malformed
    );
    assert_eq!(
        fs::read(target.join("UserNotes.txt")).expect("notes"),
        b"notes"
    );
}

#[test]
fn publish_path_rejects_manifest_alias_duplicates() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    write_manifest(&target, &["Item.g.cs", "Item.g.cs"]);

    let error = preflight(&project).expect_err("manifest duplicate");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-MANIFEST-ALIAS");
}

#[test]
fn publish_path_rejects_reserved_manifest_alias_collision() {
    let project = project_with_targets(&[("csharp", "dist")]);
    replace_csharp_artifact(&project, PUBLISH_MANIFEST_FILENAME);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");

    let error = preflight(&project).expect_err("reserved manifest collision");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-MANIFEST-COLLISION");
}

#[test]
fn publish_path_rejects_binary_symlink_or_directory() {
    let directory_project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    fs::create_dir_all(directory_project.path().join("dist/masterdata.bytes"))
        .expect("binary directory");
    assert_eq!(
        preflight(&directory_project)
            .expect_err("binary directory")
            .diagnostic()
            .code,
        "E-PUBLISH-TARGET-TYPE"
    );

    #[cfg(unix)]
    {
        let symlink_project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
        let outside = symlink_project.path().join("outside.bytes");
        fs::write(&outside, b"outside").expect("outside binary");
        fs::create_dir_all(symlink_project.path().join("dist")).expect("parent");
        symlink(
            &outside,
            symlink_project.path().join("dist/masterdata.bytes"),
        )
        .expect("binary symlink");
        assert_eq!(
            preflight(&symlink_project)
                .expect_err("binary symlink")
                .diagnostic()
                .code,
            "E-PUBLISH-SYMLINK-ANCESTOR"
        );
    }
}

#[test]
fn publish_path_preserves_binary_siblings() {
    let project = project_with_targets(&[("binary", "dist/masterdata.bytes")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    fs::write(target.join("masterdata.bytes"), b"previous").expect("binary");
    fs::write(target.join("masterdata.bytes.meta"), b"meta").expect("meta");

    let plan = preflight(&project).expect("binary preflight");

    assert!(
        plan.targets[0]
            .binary
            .as_ref()
            .unwrap()
            .existing_regular_file
    );
    assert_eq!(
        fs::read(target.join("masterdata.bytes")).expect("binary"),
        b"previous"
    );
    assert_eq!(
        fs::read(target.join("masterdata.bytes.meta")).expect("meta"),
        b"meta"
    );
}

#[test]
fn publish_path_rejects_source_canonical_cache_and_config_overlap() {
    for (kind, path) in [
        ("csharp", "sources"),
        ("csharp", ".masterdata/output"),
        ("csharp", ".masterdata/cache"),
        ("binary", "masterdata.toml"),
    ] {
        let project = project_with_targets(&[(kind, path)]);
        let error = preflight(&project).expect_err("protected path overlap");
        assert_eq!(error.diagnostic().code, "E-PUBLISH-PROTECTED-PATH");
    }
}

#[test]
fn publish_path_allows_safe_project_local_dist() {
    let project = project_with_targets(&[("csharp", "dist")]);

    let plan = preflight(&project).expect("safe project-local destination");

    assert_eq!(plan.targets.len(), 1);
    assert!(!project.path().join("dist").exists());
}

#[test]
fn publish_path_rejects_nested_csharp_targets() {
    let project = project_with_targets(&[("csharp", "dist"), ("csharp", "dist/nested")]);

    let error = preflight(&project).expect_err("nested C# targets");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-TARGET-COLLISION");
}

#[test]
fn publish_path_rejects_csharp_binary_overlap() {
    let project = project_with_targets(&[("csharp", "dist"), ("binary", "dist/masterdata.bytes")]);

    let error = preflight(&project).expect_err("C# and binary overlap");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-TARGET-COLLISION");
}

#[test]
fn publish_path_rejects_target_aliases() {
    let project = project_with_targets(&[("csharp", "dist"), ("csharp", "dist")]);

    let error = preflight(&project).expect_err("target aliases");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-TARGET-COLLISION");
}

#[test]
fn publish_path_resolves_relative_target_from_project_root() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let other_cwd = tempdir("other cwd");

    let plan = NativeApplicationService::new()
        .preflight_publish(Some(project.path()), other_cwd.path())
        .expect("relative target");

    assert_eq!(plan.targets[0].destination, project.path().join("dist"));
}

#[test]
fn publish_path_accepts_absolute_target() {
    let project = tempdir("absolute project");
    let destination = project
        .path()
        .parent()
        .expect("temporary parent")
        .join("absolute-publish-target");
    let project = project_with_targets_at(
        project,
        &[("csharp", destination.to_str().expect("UTF-8 path"))],
    );

    let plan = preflight(&project).expect("absolute target");

    assert_eq!(plan.targets[0].destination, destination);
    assert!(!destination.exists());
}

#[test]
fn publish_path_validates_all_targets_before_mutation() {
    let project = project_with_targets(&[("csharp", "missing/first"), ("csharp", "sources")]);

    let error = preflight(&project).expect_err("later target failure");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-PROTECTED-PATH");
    assert!(!project.path().join("missing").exists());
}

#[test]
fn preflight_failure_mutates_no_targets() {
    let project = project_with_targets(&[("csharp", "dist")]);
    let target = project.path().join("dist");
    fs::create_dir_all(&target).expect("target");
    let malformed = b"malformed";
    fs::write(target.join(PUBLISH_MANIFEST_FILENAME), malformed).expect("manifest");
    fs::write(target.join("UserNotes.txt"), b"notes").expect("unmanaged");

    let error = preflight(&project).expect_err("preflight failure");

    assert_eq!(error.diagnostic().code, "E-PUBLISH-MANIFEST-MALFORMED");
    assert_eq!(
        fs::read(target.join(PUBLISH_MANIFEST_FILENAME)).expect("manifest"),
        malformed
    );
    assert_eq!(
        fs::read(target.join("UserNotes.txt")).expect("unmanaged"),
        b"notes"
    );
}

#[test]
fn receipt_failure_prevents_target_inspection_and_mutation() {
    let project = project_with_targets(&[("csharp", "missing/target")]);
    let receipt = project
        .path()
        .join(".masterdata/output/.masterdata-artifact-set.json");
    fs::remove_file(receipt).expect("remove receipt");

    let error = preflight(&project).expect_err("receipt failure");

    assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-RECEIPT-MISSING");
    assert!(!project.path().join("missing").exists());
}

fn preflight(project: &TempDir) -> Result<masterdata_app::PublishPreflightPlan> {
    NativeApplicationService::new().preflight_publish(Some(project.path()), project.path())
}

fn project_with_targets(targets: &[(&str, &str)]) -> TempDir {
    project_with_targets_at(tempdir("publish project"), targets)
}

fn project_with_targets_at(project: TempDir, targets: &[(&str, &str)]) -> TempDir {
    fs::create_dir_all(project.path().join("sources")).expect("sources");
    let mut config = String::from(
        "[project]\nid = \"publish.project\"\nname = \"Publish\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
    );
    for (kind, path) in targets {
        config.push_str(&format!(
            "\n[[publish.targets]]\nkind = {kind:?}\npath = {}\n",
            toml_string(path)
        ));
    }
    fs::write(project.path().join("masterdata.toml"), config).expect("project config");

    let artifact_root = project.path().join(".masterdata/output");
    fs::create_dir_all(artifact_root.join("csharp")).expect("artifact C# directory");
    fs::write(artifact_root.join("csharp/Item.g.cs"), b"generated").expect("C# artifact");
    fs::write(artifact_root.join("masterdata.bytes"), b"binary").expect("binary artifact");
    write_artifact_set_receipt(&artifact_root, "publish.project").expect("receipt");
    project
}

fn replace_csharp_artifact(project: &TempDir, filename: &str) {
    let artifact_root = project.path().join(".masterdata/output");
    let csharp = artifact_root.join("csharp");
    fs::remove_dir_all(&csharp).expect("old C# artifacts");
    fs::create_dir_all(&csharp).expect("C# directory");
    fs::write(csharp.join(filename), b"generated").expect("C# artifact");
    fs::remove_file(artifact_root.join(".masterdata-artifact-set.json")).expect("old receipt");
    write_artifact_set_receipt(&artifact_root, "publish.project").expect("receipt");
}

fn write_manifest(target: &Path, files: &[&str]) {
    fs::create_dir_all(target).expect("target");
    fs::write(
        target.join(PUBLISH_MANIFEST_FILENAME),
        serde_json::to_vec(&json!({ "version": 1, "files": files })).expect("manifest JSON"),
    )
    .expect("manifest");
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("TOML-compatible string")
}

fn tempdir(label: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("{label}-"))
        .tempdir()
        .expect("temporary directory")
}
