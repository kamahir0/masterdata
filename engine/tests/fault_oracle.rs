#[cfg(feature = "oracle-faults")]
use masterdata_engine::{
    native::{Fault, Outcome},
    workspace::Workspace,
};
#[cfg(feature = "oracle-faults")]
use std::{fs, path::Path};
#[cfg(feature = "oracle-faults")]
fn copy(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &dst.join(e.file_name()));
        } else {
            fs::copy(e.path(), dst.join(e.file_name())).unwrap();
        }
    }
}
#[cfg(feature = "oracle-faults")]
#[test]
fn precommit_failure_and_unknown_keep_draft_history_and_never_retry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1");
    for fault in [Fault::BeforeCommit, Fault::AfterCommitObservation] {
        let temp = tempfile::tempdir().unwrap();
        copy(&root.join("save-record/input"), temp.path());
        let path = temp.path().join("sources/data.yaml");
        let old = fs::read(&path).unwrap();
        let mut w = Workspace::open(temp.path()).unwrap();
        let view = w.select("sources/data.yaml", 0, 16).unwrap();
        w.edit_text(
            "sources/data.yaml",
            view.revision,
            &view.rows[0].id,
            "note",
            "changed",
        )
        .unwrap();
        let r = w
            .save_paths(vec!["sources/data.yaml".into()], fault)
            .unwrap();
        assert!(w.drafts["sources/data.yaml"].dirty());
        assert!(w.drafts["sources/data.yaml"].can_undo());
        if matches!(fault, Fault::BeforeCommit) {
            assert_eq!(r[0].outcome, Outcome::Failure);
            assert_eq!(fs::read(path).unwrap(), old);
        } else {
            assert_eq!(
                r[0].outcome,
                Outcome::OutcomeUnknown,
                "native write result: {r:#?}"
            );
            let candidate = fs::read(root.join("save-record/expected/sources/data.yaml")).unwrap();
            assert_eq!(fs::read(&path).unwrap(), candidate);
            assert!(w.save_all().is_err());
            assert_eq!(fs::read(&path).unwrap(), candidate);
            let uncertain = w.select("sources/data.yaml", 0, 16).unwrap();
            assert!(
                !uncertain.conflict,
                "Unknown must not be presented as Conflict"
            );
            assert!(!uncertain.can_add);
            assert!(
                uncertain
                    .rows
                    .iter()
                    .flat_map(|r| &r.cells)
                    .all(|c| !c.editable)
            );
            assert_eq!(w.uncertain_paths(), ["sources/data.yaml"]);
            assert!(
                w.undo("sources/data.yaml", false).is_err(),
                "history remains protected until observation"
            );
            assert_eq!(
                w.drafts["sources/data.yaml"].outcome,
                Some(Outcome::OutcomeUnknown)
            );
            assert!(
                w.save_all().is_err(),
                "navigation cannot authorize automatic retry"
            );
            let (_, observed, _) = w.compare("sources/data.yaml").unwrap();
            assert_eq!(observed.as_bytes(), candidate);
        }
    }
}
