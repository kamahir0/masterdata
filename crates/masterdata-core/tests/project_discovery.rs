use std::fs;

#[cfg(unix)]
use std::os::unix::fs::symlink;

use masterdata_core::{InitOptions, PROJECT_CONFIG_FILENAME, Project, ProjectService};
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
            "output",
            "[build]\noutput = \"\"\n",
            "E-PROJECT-CONFIG-EMPTY-BUILD-PATH",
        ),
        (
            "cache",
            "[build]\ncache = \"\"\n",
            "E-PROJECT-CONFIG-EMPTY-BUILD-PATH",
        ),
        (
            "binary",
            "[build]\nbinary_output = \"\"\n",
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
        "[project]\nid = \"paths\"\nname = \"Paths\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"source-data\"]\n\n[build]\noutput = \"generated\"\ncache = \"cache\"\nbinary_output = \"build/masterdata.bytes\"\n",
    )
    .expect("config");

    let info = Project::discover(Some(directory.path()), directory.path())
        .expect("project")
        .info();
    assert_eq!(
        info.source_roots,
        vec![directory.path().join("source-data")]
    );
    assert_eq!(info.build_output, directory.path().join("generated"));
    assert_eq!(
        info.build_binary_output,
        Some(directory.path().join("build/masterdata.bytes"))
    );
    assert_eq!(info.build_cache, directory.path().join("cache"));
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
