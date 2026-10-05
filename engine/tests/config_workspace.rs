use masterdata_engine::{
    config::{ListEdit, Operation},
    delivery::BuildPlan,
    instrument,
    native::{Fault, Outcome},
    project::Project,
    workspace::Workspace,
};
use std::fs;
const DATA: &str = "sources/data.yaml";
fn setup() -> (tempfile::TempDir, Workspace) {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir(t.path().join("sources")).unwrap();
    fs::write(t.path().join("masterdata.toml"),"[project]\nid = 'config.workspace'\nname = 'Settings'\nversion = '0.1.0'\n[sources]\nroots = ['sources']\n[build]\nartifact_dir = '.masterdata/output'\ncache = '.masterdata/cache'\n[build.profiles.production]\nexclude_tags = ['debug']\n").unwrap();
    fs::write(t.path().join("sources/schema.yaml"),"kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: long\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys: []\n").unwrap();
    fs::write(
        t.path().join(DATA),
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: original\n",
    )
    .unwrap();
    let w = Workspace::open(t.path()).unwrap();
    (t, w)
}
fn draft(w: &mut Workspace) {
    let p = w.select(DATA, 0, 32).unwrap();
    w.edit_text(DATA, p.revision, &p.rows[0].id, "note", "draft")
        .unwrap();
    w.set_search(DATA, "draft").unwrap();
}
fn config(w: &mut Workspace, text: &str) {
    w.edit_config(
        w.configuration.revision,
        Operation::Tags {
            profile: "production".into(),
            exclude: true,
            edit: ListEdit::Replace {
                index: 0,
                text: text.into(),
            },
        },
    )
    .unwrap();
}
#[test]
fn config_only_save_keeps_yaml_draft_history_search_and_warm_project_work_zero() {
    let (t, mut w) = setup();
    draft(&mut w);
    let disk = fs::read(t.path().join(DATA)).unwrap();
    let doc = w.current_doc(DATA).unwrap().bytes.clone();
    config(&mut w, "development");
    assert!(w.config_dirty());
    assert_eq!(w.dirty_paths(), [DATA]);
    let (_, m) = instrument::measure(|| {
        w.save_config(w.configuration.revision, Fault::None)
            .unwrap()
    });
    assert_eq!(m.work.project_discovery, 0);
    assert_eq!(m.work.project_enumeration, 0);
    assert_eq!(m.work.project_yaml_parse, 0);
    assert_eq!(m.work.project_validation, 0);
    assert!(!w.config_dirty());
    assert_eq!(w.current_doc(DATA).unwrap().bytes, doc);
    assert_eq!(fs::read(t.path().join(DATA)).unwrap(), disk);
    assert_eq!(
        w.read.config.build.profiles["production"].exclude_tags,
        ["development"]
    );
    let (p, m) = instrument::measure(|| w.select(DATA, 0, 32).unwrap());
    assert!(p.can_undo);
    assert_eq!(p.view_state.search, "draft");
    assert_eq!(m.work.project_yaml_parse, 0);
    w.undo(DATA, false).unwrap();
    assert!(!w.drafts[DATA].dirty());
    assert_eq!(
        w.read.config.build.profiles["production"].exclude_tags,
        ["development"]
    );
}
#[test]
fn dirty_config_is_not_used_by_saved_build_and_domain_invalid_save_retains_repair_route() {
    let (t, mut w) = setup();
    draft(&mut w);
    config(&mut w, "");
    assert!(BuildPlan::capture(t.path(), Some("production")).is_ok());
    assert!(w.config_dirty());
    let result = w
        .save_config(w.configuration.revision, Fault::None)
        .unwrap();
    assert_eq!(result.outcome, Outcome::Success);
    assert!(Project::open(t.path()).is_err());
    assert!(BuildPlan::capture(t.path(), Some("production")).is_err());
    assert!(w.select(DATA, 0, 32).is_err());
    let v = w.config_view(Some("production"), [0; 4]);
    assert!(v.editable && !v.valid);
    assert_eq!(v.detail.unwrap().exclude.entries[0].text, "");
    config(&mut w, "debug");
    assert_eq!(
        w.save_config(w.configuration.revision, Fault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    let p = w.select(DATA, 0, 32).unwrap();
    assert!(p.can_undo && p.dirty);
    assert_eq!(p.rows[0].cells[1].display, "draft");
    assert!(w.environment_error.is_none());
}
#[cfg(feature = "oracle-faults")]
#[test]
fn save_all_advances_successful_config_and_retains_failed_yaml_without_global_rollback() {
    let (t, mut w) = setup();
    draft(&mut w);
    config(&mut w, "development");
    let yaml = fs::read(t.path().join(DATA)).unwrap();
    let r = w
        .save_all_with_config_fault(Fault::None, Fault::BeforeCommit)
        .unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].source, "masterdata.toml");
    assert_eq!(r[0].outcome, Outcome::Success);
    assert_eq!(r[1].outcome, Outcome::Failure);
    assert!(!w.config_dirty());
    assert_eq!(w.dirty_paths(), [DATA]);
    assert_eq!(fs::read(t.path().join(DATA)).unwrap(), yaml);
    assert!(
        fs::read_to_string(t.path().join("masterdata.toml"))
            .unwrap()
            .contains("development")
    );
    assert!(w.select(DATA, 0, 32).unwrap().can_undo);
    assert_eq!(w.save_all().unwrap()[0].outcome, Outcome::Success);
    assert!(!w.protected());
}
#[cfg(feature = "oracle-faults")]
#[test]
fn invalid_failed_or_unknown_config_stops_yaml_save_all_before_any_attempt() {
    for fault in [
        Fault::None,
        Fault::BeforeCommit,
        Fault::AfterCommitObservation,
    ] {
        let (t, mut w) = setup();
        draft(&mut w);
        config(&mut w, "");
        let yaml = fs::read(t.path().join(DATA)).unwrap();
        let r = w.save_all_with_config_fault(fault, Fault::None).unwrap();
        assert_eq!(r[1].source, DATA);
        assert_eq!(r[1].outcome, Outcome::NotAttempted);
        assert_eq!(fs::read(t.path().join(DATA)).unwrap(), yaml);
        assert!(w.drafts[DATA].dirty());
        if w.config_uncertain() {
            let before = fs::read(t.path().join("masterdata.toml")).unwrap();
            assert_eq!(w.save_all().unwrap()[0].outcome, Outcome::OutcomeUnknown);
            assert_eq!(fs::read(t.path().join("masterdata.toml")).unwrap(), before);
            assert_eq!(w.recheck_config().unwrap().outcome, Outcome::Success);
        }
    }
}
#[test]
fn initial_invalid_config_can_open_only_the_safe_repair_context_then_requires_explicit_reload() {
    let (t, _) = setup();
    let bytes = fs::read_to_string(t.path().join("masterdata.toml"))
        .unwrap()
        .replace("['debug']", "['']");
    fs::write(t.path().join("masterdata.toml"), &bytes).unwrap();
    let mut w = Workspace::open(t.path()).unwrap();
    assert!(w.environment_error.is_some());
    assert!(w.read.sources.is_empty());
    let v = w.config_view(Some("production"), [0; 4]);
    assert!(v.editable && !v.valid);
    config(&mut w, "debug");
    assert_eq!(
        w.save_config(w.configuration.revision, Fault::None)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    assert!(w.environment_error.is_some());
    let mut reopened = Workspace::open(t.path()).unwrap();
    assert!(reopened.environment_error.is_none());
    assert_eq!(
        reopened.select(DATA, 0, 32).unwrap().rows[0].cells[1].display,
        "original"
    );
}
#[test]
fn external_invalid_config_and_binding_changes_never_discard_yaml_or_reuse_old_service() {
    let (t, mut w) = setup();
    draft(&mut w);
    let original = fs::read_to_string(t.path().join("masterdata.toml")).unwrap();
    fs::write(
        t.path().join("masterdata.toml"),
        original.replace("['debug']", "['']"),
    )
    .unwrap();
    w.refresh_paths(&[t.path().join("masterdata.toml")]);
    assert!(w.select(DATA, 0, 32).is_err());
    let v = w.config_view(Some("production"), [0; 4]);
    assert!(v.editable && !v.valid);
    config(&mut w, "debug");
    w.save_config(w.configuration.revision, Fault::None)
        .unwrap();
    assert!(w.select(DATA, 0, 32).unwrap().dirty);
    fs::write(
        t.path().join("masterdata.toml"),
        original.replace("config.workspace", "another.project"),
    )
    .unwrap();
    w.config_view(Some("production"), [0; 4]);
    assert_eq!(
        w.environment_error.as_ref().unwrap().code,
        "E-CONFIG-BINDING"
    );
    assert!(w.select(DATA, 0, 32).is_err());
    assert!(w.drafts[DATA].dirty());
    assert_eq!(w.save_all().unwrap()[0].outcome, Outcome::NotAttempted);
}
