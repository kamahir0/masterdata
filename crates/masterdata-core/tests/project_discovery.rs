use std::fs;

use masterdata_core::{InitOptions, PROJECT_CONFIG_FILENAME, Project, ProjectService};
use tempfile::tempdir;

#[test]
fn project_001_discovers_config_from_a_nested_directory() {
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
}

#[test]
fn project_003_service_returns_serializable_project_info() {
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
fn project_004_init_creates_a_project_marker_and_source_root() {
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
