use masterdata_engine::{
    native::{self, Fault, Outcome},
    workspace::{SourceViewState, Workspace},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
const DATA: &str = "sources/data.yaml";
fn copy(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &dst.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
        }
    }
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/rewrite-oracle/v1/save-both/input"),
        temp.path(),
    );
    fs::create_dir(temp.path().join("sources/folder")).unwrap();
    temp
}
#[test]
fn rename_and_move_keep_exact_bytes_identity_search_and_unrelated_draft() {
    let temp = fixture();
    let original = fs::read_to_string(temp.path().join(DATA))
        .unwrap()
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        + "\r\n# preserved tail\r\n";
    fs::write(temp.path().join(DATA), &original).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let unrelated = w.select("sources/inactive.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/inactive.yaml",
        0,
        &unrelated.rows[0].id,
        "note",
        "independent draft",
    )
    .unwrap();
    let draft = w
        .current_doc("sources/inactive.yaml")
        .unwrap()
        .bytes
        .clone();
    w.select(DATA, 0, 32).unwrap();
    w.views.insert(
        DATA.into(),
        SourceViewState {
            search: "local find".into(),
            ..Default::default()
        },
    );
    for (source, destination) in [
        (DATA, "sources/new-name.yml"),
        ("sources/new-name.yml", "sources/folder/storage.yaml"),
    ] {
        let plan = w.prepare_path_move(source, destination).unwrap();
        assert_eq!(
            w.apply_path_move(&plan.token).unwrap().outcome,
            Outcome::Success
        );
        assert!(!temp.path().join(source).exists());
        assert_eq!(
            fs::read(temp.path().join(destination)).unwrap(),
            original.as_bytes()
        );
        assert!(!w.read.sources.contains_key(source));
        assert_eq!(w.views[destination].search, "local find");
        assert_eq!(w.read.sources[destination].binding.as_deref(), Some("item"));
        assert_eq!(w.dirty_paths(), ["sources/inactive.yaml"]);
        assert_eq!(w.current_doc("sources/inactive.yaml").unwrap().bytes, draft);
        assert!(w.drafts["sources/inactive.yaml"].can_undo());
        assert_eq!(
            w.recheck_path_move(&plan.token).unwrap().outcome,
            Outcome::Success
        );
    }
    assert!(!temp.path().join(".masterdata").exists());
}
#[test]
fn path_mutation_resolves_only_target_dirty_and_rejects_a_pre_save_review() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select(DATA, 0, 32).unwrap();
    let plan = w.prepare_path_move(DATA, "sources/renamed.yaml").unwrap();
    w.edit_text(DATA, 0, &p.rows[0].id, "note", "saved value")
        .unwrap();
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap_err().code,
        "E-PATH-DIRTY"
    );
    assert_eq!(
        w.save_paths(vec![DATA.into()], Fault::None).unwrap()[0].outcome,
        Outcome::Success
    );
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap().outcome,
        Outcome::Conflict
    );
    assert!(temp.path().join(DATA).exists());
    assert!(!temp.path().join("sources/renamed.yaml").exists());
    let p = w.select(DATA, 0, 32).unwrap();
    w.edit_text(
        DATA,
        p.revision,
        &p.rows[0].id,
        "note",
        "discard only this draft",
    )
    .unwrap();
    w.discard_source(DATA).unwrap();
    let plan = w.prepare_path_move(DATA, "sources/renamed.yaml").unwrap();
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap().outcome,
        Outcome::Success
    );
    assert!(
        fs::read_to_string(temp.path().join("sources/renamed.yaml"))
            .unwrap()
            .contains("saved value")
    );
}
#[test]
fn stale_source_content_same_mtime_and_new_destination_never_overwrite() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let plan = w.prepare_path_move(DATA, "sources/renamed.yaml").unwrap();
    let path = temp.path().join(DATA);
    let mtime = fs::metadata(&path).unwrap().modified().unwrap();
    let actual = fs::read_to_string(&path).unwrap() + "\n# external fresh bytes\n";
    fs::write(&path, &actual).unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(mtime))
        .unwrap();
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap().outcome,
        Outcome::Conflict
    );
    assert_eq!(fs::read(&path).unwrap(), actual.as_bytes());
    w.refresh_source(DATA).unwrap();
    let plan = w.prepare_path_move(DATA, "sources/renamed.yaml").unwrap();
    fs::write(
        temp.path().join("sources/renamed.yaml"),
        b"external destination",
    )
    .unwrap();
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap().outcome,
        Outcome::Conflict
    );
    assert_eq!(
        fs::read(temp.path().join("sources/renamed.yaml")).unwrap(),
        b"external destination"
    );
    assert_eq!(fs::read(&path).unwrap(), actual.as_bytes());
}
#[test]
fn exclusive_path_commit_rejects_last_moment_destination_and_parent_races() {
    let temp = fixture();
    let p = Workspace::open(temp.path()).unwrap().read;
    let base = native::capture(&p.root, &p.roots, DATA).unwrap();
    let plan =
        native::prepare_move(&p.root, &p.roots, DATA, "sources/folder/new.yaml", &base).unwrap();
    let result = native::commit_move(&plan, Fault::None, || {
        fs::write(
            p.root.join("sources/folder/new.yaml"),
            b"racing destination",
        )
        .unwrap();
        Ok(())
    });
    assert_eq!(result.outcome, Outcome::Conflict);
    assert_eq!(fs::read(p.root.join(DATA)).unwrap(), base.bytes.as_bytes());
    assert_eq!(
        fs::read(p.root.join("sources/folder/new.yaml")).unwrap(),
        b"racing destination"
    );
    fs::remove_file(p.root.join("sources/folder/new.yaml")).unwrap();
    let plan =
        native::prepare_move(&p.root, &p.roots, DATA, "sources/folder/new.yaml", &base).unwrap();
    let result = native::commit_move(&plan, Fault::None, || {
        fs::rename(
            p.root.join("sources/folder"),
            p.root.join("sources/old-folder"),
        )
        .unwrap();
        fs::create_dir(p.root.join("sources/folder")).unwrap();
        Ok(())
    });
    assert_eq!(result.outcome, Outcome::Conflict);
    assert!(!p.root.join("sources/folder/new.yaml").exists());
}
#[test]
fn source_path_scope_and_config_are_fresh_and_invalid_yaml_is_not_rewritten() {
    let temp = fixture();
    fs::create_dir(temp.path().join("other")).unwrap();
    let config = temp.path().join("masterdata.toml");
    let bytes = fs::read_to_string(&config)
        .unwrap()
        .replace("[\"sources\"]", "[\"sources\", \"other\"]");
    fs::write(&config, &bytes).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    assert_eq!(
        w.prepare_path_move(DATA, "other/new.yaml")
            .unwrap_err()
            .code,
        "E-PATH-SCOPE"
    );
    let plan = w.prepare_path_move(DATA, "sources/new.yaml").unwrap();
    fs::write(&config, bytes + "\n# external config change\n").unwrap();
    assert!(w.apply_path_move(&plan.token).is_err());
    assert!(temp.path().join(DATA).exists());
    assert!(!temp.path().join("sources/new.yaml").exists());
    let invalid = b"# retain even invalid YAML\r\nkind: [\r\n";
    fs::write(temp.path().join("sources/invalid.yaml"), invalid).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let plan = w
        .prepare_path_move("sources/invalid.yaml", "sources/folder/invalid.yml")
        .unwrap();
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap().outcome,
        Outcome::Success
    );
    assert_eq!(
        fs::read(temp.path().join("sources/folder/invalid.yml")).unwrap(),
        invalid
    );
    assert!(w.read.sources["sources/folder/invalid.yml"].error.is_some());
}
#[cfg(unix)]
#[test]
fn source_path_rejects_alias_destinations_before_mutation() {
    use std::os::unix::fs::symlink;
    let temp = fixture();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), temp.path().join("sources/alias")).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    assert_eq!(
        w.prepare_path_move(DATA, "sources/alias/new.yaml")
            .unwrap_err()
            .code,
        "E-PATH-ALIAS"
    );
    assert!(temp.path().join(DATA).exists());
    assert!(!outside.path().join("new.yaml").exists());
}
#[test]
fn case_only_rename_uses_requested_spelling_and_a_distinct_hardlink_is_conflict() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let original = fs::read(temp.path().join(DATA)).unwrap();
    let plan = w.prepare_path_move(DATA, "sources/DATA.yaml").unwrap();
    assert_eq!(
        w.apply_path_move(&plan.token).unwrap().outcome,
        Outcome::Success
    );
    let entries = fs::read_dir(temp.path().join("sources"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    assert!(entries.iter().any(|e| e == "DATA.yaml"));
    assert!(!entries.iter().any(|e| e == "data.yaml"));
    assert_eq!(
        fs::read(temp.path().join("sources/DATA.yaml")).unwrap(),
        original
    );
    fs::hard_link(
        temp.path().join("sources/DATA.yaml"),
        temp.path().join("sources/alias.yaml"),
    )
    .unwrap();
    assert_eq!(
        w.prepare_path_move("sources/DATA.yaml", "sources/alias.yaml")
            .unwrap_err()
            .code,
        "E-PATH-CONFLICT"
    );
    for bad in [
        "../escape.yaml",
        "sources/folder/missing/new.yaml",
        "sources/new.txt",
        "/absolute.yaml",
    ] {
        assert!(
            w.prepare_path_move("sources/DATA.yaml", bad).is_err(),
            "{bad}"
        );
    }
    w.recovery_required = true;
    assert_eq!(
        w.prepare_path_move("sources/DATA.yaml", "sources/new.yaml")
            .unwrap_err()
            .code,
        "E-RECOVERY-REQUIRED"
    );
}
#[cfg(feature = "oracle-faults")]
#[test]
fn path_failure_keeps_complete_old_and_unknown_requires_both_path_recheck() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let bytes = fs::read(temp.path().join(DATA)).unwrap();
    let plan = w.prepare_path_move(DATA, "sources/new.yaml").unwrap();
    assert_eq!(
        w.apply_path_move_with_fault(&plan.token, Fault::BeforeCommit)
            .unwrap()
            .outcome,
        Outcome::Failure
    );
    assert_eq!(fs::read(temp.path().join(DATA)).unwrap(), bytes);
    assert!(!temp.path().join("sources/new.yaml").exists());
    let plan = w.prepare_path_move(DATA, "sources/new.yaml").unwrap();
    assert_eq!(
        w.apply_path_move_with_fault(&plan.token, Fault::AfterCommitObservation)
            .unwrap()
            .outcome,
        Outcome::OutcomeUnknown
    );
    assert_eq!(w.uncertain_paths(), [DATA, "sources/new.yaml"]);
    assert_eq!(
        w.discard_source(DATA).unwrap_err().code,
        "E-OUTCOME-UNKNOWN"
    );
    assert_eq!(
        w.reload_source("sources/new.yaml").unwrap_err().code,
        "E-OUTCOME-UNKNOWN"
    );
    assert_eq!(
        w.prepare_path_move(DATA, "sources/another.yaml")
            .unwrap_err()
            .code,
        "E-OUTCOME-UNKNOWN"
    );
    assert!(w.save_all().unwrap().is_empty());
    assert_eq!(w.uncertain_paths().len(), 2);
    assert_eq!(
        w.recheck_path_move(&plan.token).unwrap().outcome,
        Outcome::Success
    );
    assert!(w.uncertain_paths().is_empty());
    assert_eq!(
        fs::read(temp.path().join("sources/new.yaml")).unwrap(),
        bytes
    );
    assert!(w.read.sources.contains_key("sources/new.yaml"));
    assert!(!w.read.sources.contains_key(DATA));
}
