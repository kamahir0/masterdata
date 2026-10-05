use masterdata_engine::{
    native::{self, Outcome},
    project::{Metadata, Project},
};
use std::fs;

fn metadata() -> Metadata {
    Metadata {
        id: "game.masterdata".into(),
        name: "Game \"Data\"".into(),
        version: "0.1.0".into(),
    }
}
#[test]
fn gui_create_accepts_empty_or_one_new_directory_and_resolves_only_the_minimum_scaffold() {
    let t = tempfile::tempdir().unwrap();
    let parent = t.path().canonicalize().unwrap();
    for existing in [true, false] {
        let root = parent.join(if existing { "existing" } else { "new" });
        if existing {
            fs::create_dir(&root).unwrap();
        }
        let created = native::create_project(&root, metadata()).unwrap();
        assert_eq!(created.outcome, Outcome::Success);
        assert!(created.unconfirmed.is_empty());
        assert_eq!(created.remaining.len(), 7);
        let p = Project::open(&root).unwrap();
        assert_eq!(p.config.project.name, "Game \"Data\"");
        assert!(p.sources.is_empty());
        assert!(p.config.publish.targets.is_empty());
        assert_eq!(
            fs::read(root.join(".gitignore")).unwrap(),
            b"/.masterdata/\n"
        );
        for directory in ["sources/schemas", "sources/types", "sources/data"] {
            assert!(root.join(directory).is_dir());
        }
        assert!(!root.join(".masterdata").exists());
        assert!(!root.join(".git").exists());
        assert!(!root.join("Assets").exists());
    }
}
#[test]
fn gui_create_rejects_hidden_nonempty_targets_missing_parent_and_invalid_metadata_without_writes() {
    let t = tempfile::tempdir().unwrap();
    let parent = t.path().canonicalize().unwrap();
    for entry in [".hidden", "notes.txt"] {
        let root = parent.join(entry.replace('.', "_"));
        fs::create_dir(&root).unwrap();
        fs::write(root.join(entry), b"user bytes").unwrap();
        assert!(native::create_project(&root, metadata()).is_err());
        assert_eq!(fs::read(root.join(entry)).unwrap(), b"user bytes");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    }
    let missing = parent.join("missing/child");
    assert!(native::create_project(&missing, metadata()).is_err());
    assert!(!parent.join("missing").exists());
    let invalid = parent.join("invalid");
    let mut m = metadata();
    m.name = " \t".into();
    assert!(native::create_project(&invalid, m).is_err());
    assert!(!invalid.exists());
    let file = parent.join("file");
    fs::write(&file, b"original").unwrap();
    assert!(native::create_project(&file, metadata()).is_err());
    assert_eq!(fs::read(file).unwrap(), b"original");
}
#[cfg(unix)]
#[test]
fn gui_create_does_not_canonicalize_away_symlink_traversal_including_cancelled_components() {
    let t = tempfile::tempdir().unwrap();
    let parent = t.path().canonicalize().unwrap();
    let real = parent.join("real");
    fs::create_dir(&real).unwrap();
    let alias = parent.join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    for target in [alias.clone(), alias.join("new"), alias.join("../cancelled")] {
        assert!(native::create_project(&target, metadata()).is_err());
    }
    assert_eq!(fs::read_dir(&real).unwrap().count(), 0);
    assert!(!parent.join("cancelled").exists());
}
#[cfg(feature = "oracle-faults")]
#[test]
fn interrupted_creation_reports_actual_leftovers_and_does_not_cleanup_or_blindly_retry() {
    let t = tempfile::tempdir().unwrap();
    let parent = t.path().canonicalize().unwrap();
    for (index, fault) in [
        native::InitFault::BeforeConfig,
        native::InitFault::AfterConfig,
    ]
    .into_iter()
    .enumerate()
    {
        let root = parent.join(format!("partial-{index}"));
        let result = native::create_project_with_fault(&root, metadata(), fault).unwrap();
        assert_eq!(result.outcome, Outcome::Failure);
        assert!(result.remaining.contains(&"sources/data".into()));
        assert_eq!(root.join("masterdata.toml").exists(), index == 1);
        assert!(!root.join(".gitignore").exists());
        assert_eq!(Project::open(&root).is_ok(), index == 1);
        let config = fs::read(root.join("masterdata.toml")).ok();
        assert!(native::create_project(&root, metadata()).is_err());
        assert_eq!(fs::read(root.join("masterdata.toml")).ok(), config);
    }
}
#[cfg(feature = "oracle-faults")]
#[test]
fn a_config_appearing_during_scaffold_creation_is_never_overwritten() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().canonicalize().unwrap().join("race");
    let result =
        native::create_project_with_fault(&root, metadata(), native::InitFault::ConfigRace)
            .unwrap();
    assert_eq!(result.outcome, Outcome::Failure);
    assert_eq!(
        fs::read(root.join("masterdata.toml")).unwrap(),
        b"concurrently created config\n"
    );
    assert!(!root.join(".gitignore").exists());
    assert!(result.remaining.contains(&"masterdata.toml".into()));
}
#[test]
fn desktop_creation_does_not_revoke_cli_init_in_an_existing_user_directory() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().canonicalize().unwrap();
    fs::write(root.join("notes.txt"), b"user notes").unwrap();
    fs::write(root.join(".gitignore"), b"# user policy\r\n").unwrap();
    assert!(native::create_project(&root, metadata()).is_err());
    let result = native::initialize_project(&root, metadata()).unwrap();
    assert_eq!(result.outcome, Outcome::Success);
    assert!(result.kept_gitignore);
    assert_eq!(fs::read(root.join("notes.txt")).unwrap(), b"user notes");
    assert_eq!(
        fs::read(root.join(".gitignore")).unwrap(),
        b"# user policy\r\n"
    );
}
