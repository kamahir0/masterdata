use std::fs;

#[cfg(unix)]
use std::os::unix::fs::symlink;

use masterdata_core::{
    InitOptions, PROJECT_CONFIG_FILENAME, Project, ProjectService, PublishTargetKind,
};
use tempfile::tempdir;

#[test]
fn project_003_discovers_config_from_parent_directory() {
    let directory = tempdir().expect("temp directory");
    let project_root = directory.path();
    fs::write(
        project_root.join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"test.project\"\nname = \"Test\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n",
    )
    .expect("config");
    fs::create_dir(project_root.join("sources")).expect("sources");
    let nested = project_root.join("a").join("b");
    fs::create_dir_all(&nested).expect("nested");

    let project = Project::discover(None, &nested).expect("project discovery");
    assert_eq!(project.root(), project_root);
    assert_eq!(project.config().project.id, "test.project");
}

#[test]
fn project_002_explicit_path_has_priority_over_parent_search() {
    let directory = tempdir().expect("temp directory");
    let outer = directory.path().join("outer");
    let inner = outer.join("inner");
    fs::create_dir_all(inner.join("sources")).expect("directories");
    fs::create_dir_all(outer.join("sources")).expect("directories");
    fs::write(
        outer.join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"outer\"\nname = \"Outer\"\nversion = \"0.1.0\"\n",
    )
    .expect("outer config");
    fs::write(
        inner.join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"inner\"\nname = \"Inner\"\nversion = \"0.1.0\"\n",
    )
    .expect("inner config");

    let project = Project::discover(Some(&inner), &outer).expect("explicit project");
    assert_eq!(project.config().project.id, "inner");

    let project_from_file = Project::discover(Some(&inner.join(PROJECT_CONFIG_FILENAME)), &outer)
        .expect("explicit config file");
    assert_eq!(project_from_file.config().project.id, "inner");
}

#[test]
fn project_info_exposes_serializable_discovered_project() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"serializable\"\nname = \"Serializable\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    let info = ProjectService::new()
        .project_info(Some(directory.path()), directory.path())
        .expect("info");
    let json = serde_json::to_string(&info).expect("json");
    assert!(json.contains("serializable"));
    assert_eq!(
        info.artifact_root,
        directory.path().join(".masterdata/output")
    );
    assert_eq!(
        info.csharp_output,
        directory.path().join(".masterdata/output/csharp")
    );
    assert_eq!(
        info.binary_output,
        directory.path().join(".masterdata/output/masterdata.bytes")
    );
    assert_eq!(info.cache, directory.path().join(".masterdata/cache"));
    assert!(info.publish_targets.is_empty());
}

#[test]
fn project_config_001_requires_non_empty_metadata() {
    let directory = tempdir().expect("temp directory");
    let valid_root = directory.path().join("valid");
    fs::create_dir_all(valid_root.join("sources")).expect("sources");
    fs::write(
        valid_root.join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"test.project\"\nname = \"Test\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    Project::discover(Some(&valid_root), valid_root.as_path()).expect("valid metadata");

    for field in ["id", "name", "version"] {
        let root = directory.path().join(field);
        fs::create_dir_all(root.join("sources")).expect("sources");
        fs::write(
            root.join(PROJECT_CONFIG_FILENAME),
            format!(
                "[project]\nid = \"{}\"\nname = \"{}\"\nversion = \"{}\"\n",
                if field == "id" { "" } else { "test.project" },
                if field == "name" { "" } else { "Test" },
                if field == "version" { "" } else { "0.1.0" },
            ),
        )
        .expect("config");

        let error = Project::discover(Some(&root), root.as_path()).expect_err("invalid metadata");
        assert_eq!(error.diagnostic().code, "E-PROJECT-CONFIG-EMPTY");
    }
}

#[test]
fn project_config_002_requires_a_source_root() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"test.project\"\nname = \"Test\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n",
    )
    .expect("config");
    Project::discover(Some(directory.path()), directory.path()).expect("valid source root");

    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"test.project\"\nname = \"Test\"\nversion = \"0.1.0\"\n\n[sources]\nroots = []\n",
    )
    .expect("config");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("missing source root");
    assert_eq!(error.diagnostic().code, "E-PROJECT-CONFIG-NO-SOURCE-ROOT");
}

#[test]
fn project_config_003_rejects_empty_source_or_build_paths() {
    let directory = tempdir().expect("temp directory");
    let valid_root = directory.path().join("valid");
    fs::create_dir_all(valid_root.join("sources")).expect("sources");
    fs::write(
        valid_root.join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"test.project\"\nname = \"Test\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    Project::discover(Some(&valid_root), valid_root.as_path()).expect("valid paths");

    let cases = [
        (
            "source",
            "[sources]\nroots = [\"\"]\n",
            "E-PROJECT-CONFIG-EMPTY-SOURCE-ROOT",
        ),
        (
            "artifact",
            "[build]\nartifact_dir = \"\"\n",
            "E-PROJECT-CONFIG-EMPTY-BUILD-PATH",
        ),
        (
            "cache",
            "[build]\ncache = \"\"\n",
            "E-PROJECT-CONFIG-EMPTY-BUILD-PATH",
        ),
    ];
    for (name, extra, code) in cases {
        let root = directory.path().join(name);
        fs::create_dir_all(root.join("sources")).expect("sources");
        fs::write(
            root.join(PROJECT_CONFIG_FILENAME),
            format!(
                "[project]\nid = \"test.project\"\nname = \"Test\"\nversion = \"0.1.0\"\n\n{extra}"
            ),
        )
        .expect("config");

        let error = Project::discover(Some(&root), root.as_path()).expect_err("invalid path");
        assert_eq!(error.diagnostic().code, code);
    }
}

#[test]
fn project_path_001_resolves_relative_paths_against_project_root() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"paths\"\nname = \"Paths\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"source-data\"]\n\n[build]\nartifact_dir = \"generated\"\ncache = \"cache\"\n\n[[publish.targets]]\nkind = \"csharp\"\npath = \"../unity/Generated\"\n\n[[publish.targets]]\nkind = \"binary\"\npath = \"../server/data/masterdata.bytes\"\n",
    )
    .expect("config");

    let info = Project::discover(Some(directory.path()), directory.path())
        .expect("project")
        .info();
    assert_eq!(
        info.source_roots,
        vec![directory.path().join("source-data")]
    );
    assert_eq!(info.artifact_root, directory.path().join("generated"));
    assert_eq!(
        info.csharp_output,
        directory.path().join("generated/csharp")
    );
    assert_eq!(
        info.binary_output,
        directory.path().join("generated/masterdata.bytes")
    );
    assert_eq!(info.cache, directory.path().join("cache"));
    assert_eq!(info.publish_targets.len(), 2);
    assert_eq!(info.publish_targets[0].kind, PublishTargetKind::CSharp);
    assert_eq!(
        info.publish_targets[0].resolved_path,
        directory
            .path()
            .parent()
            .expect("temporary directory parent")
            .join("unity/Generated")
    );
    assert_eq!(info.publish_targets[1].kind, PublishTargetKind::Binary);
    assert_eq!(
        info.publish_targets[1].resolved_path,
        directory
            .path()
            .parent()
            .expect("temporary directory parent")
            .join("server/data/masterdata.bytes")
    );
}

#[test]
fn project_config_004_rejects_legacy_build_paths_with_migration_diagnostics() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");

    for (field, code, guidance) in [
        (
            "output",
            "E-CONFIG-LEGACY-BUILD-OUTPUT",
            "build.artifact_dir",
        ),
        (
            "binary_output",
            "E-CONFIG-LEGACY-BINARY-OUTPUT",
            "masterdata.bytes",
        ),
    ] {
        let content = format!(
            "[project]\nid = \"legacy\"\nname = \"Legacy\"\nversion = \"0.1.0\"\n\n[build]\n{field} = \"old/path\"\n"
        );
        fs::write(directory.path().join(PROJECT_CONFIG_FILENAME), content).expect("config");

        let error = Project::discover(Some(directory.path()), directory.path())
            .expect_err("legacy path must be rejected");
        assert_eq!(error.diagnostic().code, code);
        assert_eq!(
            error.diagnostic().source.as_deref(),
            Some(directory.path().join(PROJECT_CONFIG_FILENAME).as_path())
        );
        assert!(
            error
                .diagnostic()
                .suggestion
                .as_deref()
                .unwrap()
                .contains(guidance)
        );
        assert!(
            error
                .diagnostic()
                .related_requirements
                .contains(&"PROJECT-CONFIG-004".to_owned())
        );
    }
}

#[test]
fn project_config_004_reports_legacy_output_before_binary_output() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"legacy\"\nname = \"Legacy\"\nversion = \"0.1.0\"\n\n[build]\noutput = \"old/csharp\"\nbinary_output = \"old/data.bytes\"\n",
    )
    .expect("config");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("legacy paths must be rejected");
    assert_eq!(error.diagnostic().code, "E-CONFIG-LEGACY-BUILD-OUTPUT");
}

#[test]
fn project_config_005_rejects_legacy_config_without_artifact_mutation() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    let legacy_generated = directory.path().join(".masterdata/generated/legacy.g.cs");
    let canonical_sentinel = directory.path().join(".masterdata/output/sentinel");
    let external_sentinel = directory.path().join("Generated/external.g.cs");
    let binary_sentinel = directory.path().join("old/masterdata.bytes");
    for path in [
        &legacy_generated,
        &canonical_sentinel,
        &external_sentinel,
        &binary_sentinel,
    ] {
        fs::create_dir_all(path.parent().expect("sentinel parent")).expect("sentinel parent");
        fs::write(path, b"KEEP").expect("sentinel");
    }
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"legacy\"\nname = \"Legacy\"\nversion = \"0.1.0\"\n\n[build]\noutput = \"Generated\"\nbinary_output = \"old/masterdata.bytes\"\n",
    )
    .expect("config");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("legacy configuration must stop before build work");

    assert_eq!(error.diagnostic().code, "E-CONFIG-LEGACY-BUILD-OUTPUT");
    for path in [
        &legacy_generated,
        &canonical_sentinel,
        &external_sentinel,
        &binary_sentinel,
    ] {
        assert_eq!(fs::read(path).expect("sentinel remains"), b"KEEP");
    }
}

#[test]
fn publish_config_rejects_unknown_target_kind() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"publish\"\nname = \"Publish\"\nversion = \"0.1.0\"\n\n[[publish.targets]]\nkind = \"package\"\npath = \"out/package\"\n",
    )
    .expect("config");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("unknown publish target kind");
    assert_eq!(error.diagnostic().code, "E-PROJECT-CONFIG-PARSE");
}

#[test]
fn project_path_001_rejects_unsafe_canonical_artifact_paths() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    for (name, artifact_dir) in [
        ("absolute", directory.path().to_string_lossy().to_string()),
        ("escape", "../outside".to_owned()),
        ("root", ".".to_owned()),
    ] {
        let root = directory.path().join(name);
        fs::create_dir_all(root.join("sources")).expect("sources");
        fs::write(
            root.join(PROJECT_CONFIG_FILENAME),
            format!(
                "[project]\nid = \"path\"\nname = \"Path\"\nversion = \"0.1.0\"\n\n[build]\nartifact_dir = '{artifact_dir}'\n"
            ),
        )
        .expect("config");
        let error = Project::discover(Some(&root), &root).expect_err("unsafe artifact path");
        assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    }
}

#[test]
fn project_path_001_rejects_artifact_source_and_cache_overlap() {
    let cases = [
        ("source-overlap", "sources", "sources"),
        (
            "cache-overlap",
            ".masterdata/output",
            ".masterdata/output/cache",
        ),
    ];
    for (name, artifact_dir, cache) in cases {
        let directory = tempdir().expect("temp directory");
        fs::create_dir_all(directory.path().join("sources")).expect("sources");
        fs::write(
            directory.path().join(PROJECT_CONFIG_FILENAME),
            format!(
                "[project]\nid = \"path\"\nname = \"Path\"\nversion = \"0.1.0\"\n\n[build]\nartifact_dir = \"{artifact_dir}\"\ncache = \"{cache}\"\n"
            ),
        )
        .expect("config");
        let error = Project::discover(Some(directory.path()), directory.path()).expect_err(name);
        assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    }
}

#[test]
fn init_creates_a_project_marker_and_source_root() {
    let directory = tempdir().expect("temp directory");
    let target = directory.path().join("new-project");
    let info = ProjectService::new()
        .init(
            &target,
            &InitOptions {
                project_id: "new.project".to_owned(),
                name: "New Project".to_owned(),
                version: "0.1.0".to_owned(),
            },
        )
        .expect("initialize project");
    assert_eq!(info.project_id, "new.project");
    assert!(target.join(PROJECT_CONFIG_FILENAME).is_file());
    assert!(target.join("sources").is_dir());
    let config = fs::read_to_string(target.join(PROJECT_CONFIG_FILENAME)).expect("config");
    assert!(config.contains("artifact_dir = \".masterdata/output\""));
    assert!(config.contains("cache = \".masterdata/cache\""));
    assert!(!config.contains("output = "));
    assert!(!config.contains("binary_output = "));
    assert!(!config.contains("publish.targets"));
}

#[test]
fn project_001_explicit_directory_uses_masterdata_marker() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"marker\"\nname = \"Marker\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    assert_eq!(
        project.config_path(),
        &directory.path().join(PROJECT_CONFIG_FILENAME)
    );
}

#[test]
fn project_004_returns_structured_not_found_diagnostic() {
    let directory = tempdir().expect("temp directory");
    let error = Project::discover(None, directory.path()).expect_err("missing project");
    assert_eq!(error.diagnostic().code, "E-PROJECT-NOT-FOUND");
    assert_eq!(
        error.diagnostic().kind,
        masterdata_core::ErrorKind::ProjectNotFound
    );
    assert!(error.diagnostic().source.is_none());
    assert!(
        error
            .diagnostic()
            .related_requirements
            .contains(&"PROJECT-004".to_owned())
    );
}

#[test]
fn project_005_unity_folders_do_not_define_identity() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("Assets")).expect("Assets");
    fs::create_dir(directory.path().join("ProjectSettings")).expect("ProjectSettings");

    let error = Project::discover(None, directory.path()).expect_err("missing marker");
    assert_eq!(error.diagnostic().code, "E-PROJECT-NOT-FOUND");
}

#[cfg(unix)]
#[test]
fn source_discovery_does_not_follow_symlink_entries_guard() {
    let directory = tempdir().expect("temp directory");
    write_project_config(directory.path());
    fs::create_dir_all(directory.path().join("sources").join("nested")).expect("source directory");
    fs::write(
        directory
            .path()
            .join("sources")
            .join("nested")
            .join("item.yaml"),
        "kind: data\ntable: item\nrecords: []\n",
    )
    .expect("source file");
    symlink(
        directory.path().join("sources"),
        directory.path().join("sources").join("nested").join("loop"),
    )
    .expect("symlink");

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let files = project.source_files().expect("source files");
    assert_eq!(files.len(), 1);
}

#[cfg(unix)]
#[test]
fn project_path_001_rejects_source_symlink_alias() {
    let directory = tempdir().expect("temporary directory");
    let actual_sources = directory.path().join("actual-sources");
    fs::create_dir(&actual_sources).expect("actual sources");
    let sentinel = actual_sources.join("item.yaml");
    fs::write(&sentinel, "kind: data\ntable: item\nrecords: []\n").expect("source");
    symlink(&actual_sources, directory.path().join("sources-link")).expect("source symlink");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"alias\"\nname = \"Alias\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources-link\"]\n\n[build]\nartifact_dir = \"actual-sources\"\ncache = \".masterdata/cache\"\n",
    )
    .expect("config");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("source alias must be rejected");
    assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    assert_eq!(
        fs::read(&sentinel).expect("source remains"),
        b"kind: data\ntable: item\nrecords: []\n"
    );
}

#[cfg(unix)]
#[test]
fn project_path_001_allows_nonoverlapping_source_symlink() {
    let directory = tempdir().expect("temporary directory");
    let actual_sources = directory.path().join("actual-sources");
    fs::create_dir(&actual_sources).expect("actual sources");
    symlink(&actual_sources, directory.path().join("sources-link")).expect("source symlink");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"alias\"\nname = \"Alias\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources-link\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
    )
    .expect("config");

    Project::discover(Some(directory.path()), directory.path())
        .expect("unrelated source symlink remains allowed");
}

#[cfg(unix)]
#[test]
fn project_path_001_rejects_source_symlink_ancestor_alias() {
    let directory = tempdir().expect("temporary directory");
    let actual_sources = directory.path().join("actual-sources");
    fs::create_dir(&actual_sources).expect("actual sources");
    symlink(&actual_sources, directory.path().join("sources-link")).expect("source symlink");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"alias\"\nname = \"Alias\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources-link\"]\n\n[build]\nartifact_dir = \"actual-sources/generated\"\ncache = \".masterdata/cache\"\n",
    )
    .expect("config");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("source ancestor alias must be rejected");
    assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    assert!(!actual_sources.join("generated").exists());
}

#[cfg(unix)]
#[test]
fn project_path_001_rejects_cache_symlink_alias() {
    let directory = tempdir().expect("temporary directory");
    let artifact_root = directory.path().join(".masterdata/output");
    fs::create_dir_all(&artifact_root).expect("artifact root");
    let sentinel = artifact_root.join("sentinel");
    fs::write(&sentinel, b"KEEP").expect("artifact sentinel");
    symlink(
        &artifact_root,
        directory.path().join(".masterdata/cache-link"),
    )
    .expect("cache symlink");
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"cache-alias\"\nname = \"Cache Alias\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache-link\"\n",
    )
    .expect("config");
    fs::create_dir(directory.path().join("sources")).expect("sources");

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect_err("cache alias must be rejected");
    assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    assert_eq!(fs::read(&sentinel).expect("artifact remains"), b"KEEP");
}

#[test]
fn project_path_001_handles_case_alias_using_filesystem_behavior() {
    let directory = tempdir().expect("temporary directory");
    let sources = directory.path().join("Sources");
    fs::create_dir(&sources).expect("sources");
    let case_insensitive = fs::metadata(directory.path().join("sources")).is_ok();
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"case-alias\"\nname = \"Case Alias\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"Sources\"]\n\n[build]\nartifact_dir = \"sources\"\ncache = \".masterdata/cache\"\n",
    )
    .expect("config");

    let result = Project::discover(Some(directory.path()), directory.path());
    if case_insensitive {
        let error = result.expect_err("case alias must be rejected");
        assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    } else {
        result.expect("distinct case-sensitive paths remain valid");
    }
}

#[test]
fn project_path_001_handles_case_alias_with_missing_tails_using_filesystem_behavior() {
    let directory = tempdir().expect("temporary directory");
    let sources = directory.path().join("Sources");
    fs::create_dir(&sources).expect("sources");
    fs::write(sources.join("case-probe.yaml"), "# probe\n").expect("probe entry");
    let case_insensitive = fs::metadata(directory.path().join("sources")).is_ok();
    fs::write(
        directory.path().join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"case-future-alias\"\nname = \"Case Future Alias\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"Sources/source-data\"]\n\n[build]\nartifact_dir = \"sources/Source-Data\"\ncache = \".masterdata/cache\"\n",
    )
    .expect("config");

    let result = Project::discover(Some(directory.path()), directory.path());
    if case_insensitive {
        let error = result.expect_err("future case alias must be rejected");
        assert_eq!(error.diagnostic().code, "E-PROJECT-PATH-UNSAFE");
    } else {
        result.expect("distinct case-sensitive future paths remain valid");
    }
}

#[test]
fn missing_source_root_has_no_unrelated_table_identity_requirement() {
    let directory = tempdir().expect("temp directory");
    write_project_config(directory.path());

    let error = Project::discover(Some(directory.path()), directory.path())
        .expect("project")
        .source_files()
        .expect_err("missing source root");
    assert_eq!(error.diagnostic().code, "E-PROJECT-SOURCE-ROOT-MISSING");
    assert!(error.diagnostic().related_requirements.is_empty());
}

fn write_project_config(root: &std::path::Path) {
    fs::write(
        root.join(PROJECT_CONFIG_FILENAME),
        "[project]\nid = \"symlink.test\"\nname = \"Symlink\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
}
