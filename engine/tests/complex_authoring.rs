use masterdata_engine::{
    native::Outcome,
    workspace::{Workspace, complex::Operation},
};
use std::{fs, path::Path};
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
fn project() -> (tempfile::TempDir, Workspace) {
    let t = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/full"),
        t.path(),
    );
    let p = t.path().join("sources/catalog-data.yaml");
    let bytes = fs::read_to_string(&p)
        .unwrap()
        .replace("numbers: [1, -2]", "numbers: [1, \"1\", -2]");
    fs::write(p, bytes).unwrap();
    let w = Workspace::open(t.path()).unwrap();
    (t, w)
}
const SOURCE: &str = "sources/catalog-data.yaml";
fn operation(w: &mut Workspace, row: &str, path: &[&str], op: Operation) {
    let v = w.select(SOURCE, 0, 32).unwrap();
    w.complex_operation(
        SOURCE,
        v.revision,
        v.generation,
        row,
        &path.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        op,
    )
    .unwrap();
}
#[test]
fn independent_complex_direct_workflow_preserves_source_and_operation_history() {
    let (temp, mut w) = project();
    let v = w.select(SOURCE, 0, 32).unwrap();
    let row = v.rows[1].id.clone();
    let before = w.current_doc(SOURCE).unwrap().bytes.clone();
    let editor = w
        .complex_view(
            SOURCE,
            v.revision,
            v.generation,
            &row,
            &["reward".into()],
            0..64,
        )
        .unwrap();
    assert_eq!(editor.node.children.len(), 4);
    assert!(!w.drafts[SOURCE].dirty());
    assert!(!w.drafts[SOURCE].can_undo());
    operation(
        &mut w,
        &row,
        &["reward", "note"],
        Operation::Text {
            text: "changed".into(),
        },
    );
    let after_text = w.current_doc(SOURCE).unwrap().bytes.clone();
    operation(&mut w, &row, &["reward", "note"], Operation::Null);
    operation(&mut w, &row, &["numbers"], Operation::Add);
    let v = w.select(SOURCE, 0, 32).unwrap();
    let editor = w
        .complex_view(
            SOURCE,
            v.revision,
            v.generation,
            &row,
            &["numbers".into()],
            0..64,
        )
        .unwrap();
    let last = editor
        .node
        .children
        .last()
        .unwrap()
        .path
        .last()
        .unwrap()
        .clone();
    assert_eq!(editor.node.total_children, 4);
    operation(&mut w, &row, &["numbers"], Operation::Remove { item: last });
    // Closing has no operation. Undo restores only the last removal.
    w.undo(SOURCE, false).unwrap();
    assert_eq!(
        w.current_doc(SOURCE).unwrap().records().unwrap()[1]
            .value
            .get("numbers")
            .unwrap()
            .items()
            .unwrap()
            .len(),
        4
    );
    w.undo(SOURCE, false).unwrap();
    w.undo(SOURCE, false).unwrap();
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes, after_text);
    w.undo(SOURCE, false).unwrap();
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes, before);
    assert_eq!(
        fs::read_to_string(temp.path().join(SOURCE)).unwrap(),
        before.as_ref()
    );
}
#[test]
fn equal_array_occurrences_keep_lexical_bytes_identity_and_nested_64_bit_values() {
    let (_temp, mut w) = project();
    let v = w.select(SOURCE, 0, 32).unwrap();
    let row = v.rows[1].id.clone();
    let editor = w
        .complex_view(
            SOURCE,
            v.revision,
            v.generation,
            &row,
            &["numbers".into()],
            0..64,
        )
        .unwrap();
    let ids = editor
        .node
        .children
        .iter()
        .map(|n| n.path.last().unwrap().clone())
        .collect::<Vec<_>>();
    assert_ne!(ids[0], ids[1]);
    operation(
        &mut w,
        &row,
        &["numbers"],
        Operation::Reorder {
            items: vec![ids[1].clone(), ids[0].clone(), ids[2].clone()],
        },
    );
    assert!(
        w.current_doc(SOURCE)
            .unwrap()
            .bytes
            .contains("numbers: [\"1\", 1, -2]")
    );
    let v = w.select(SOURCE, 0, 32).unwrap();
    let editor = w
        .complex_view(
            SOURCE,
            v.revision,
            v.generation,
            &row,
            &["numbers".into()],
            0..64,
        )
        .unwrap();
    assert_eq!(editor.node.children[0].path.last(), Some(&ids[1]));
    w.undo(SOURCE, false).unwrap();
    assert!(
        w.current_doc(SOURCE)
            .unwrap()
            .bytes
            .contains("numbers: [1, \"1\", -2]")
    );
    let v = w.select(SOURCE, 0, 32).unwrap();
    let editor = w
        .complex_view(
            SOURCE,
            v.revision,
            v.generation,
            &row,
            &["reward".into(), "values".into()],
            0..64,
        )
        .unwrap();
    let id = editor.node.children[0].path.last().unwrap();
    operation(
        &mut w,
        &row,
        &["reward", "values", id],
        Operation::Text {
            text: "-9223372036854775808".into(),
        },
    );
    assert!(
        w.current_doc(SOURCE)
            .unwrap()
            .bytes
            .contains("values: [-9223372036854775808]")
    );
    let (problems, _) = w.validation_snapshot().validate(None).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
    let results = w.save_table("item", Some(SOURCE)).unwrap();
    assert_eq!(results[0].outcome, Outcome::Success);
}
#[test]
fn search_is_schema_directed_source_local_read_only_and_paste_never_hits_hidden_rows() {
    let (_temp, mut w) = project();
    w.select(SOURCE, 0, 32).unwrap();
    let before = w.current_doc(SOURCE).unwrap().bytes.clone();
    w.set_search(SOURCE, "Sword").unwrap();
    let v = w.select(SOURCE, 0, 32).unwrap();
    assert_eq!(v.total_rows, 2);
    assert!(!v.dirty);
    w.set_search(SOURCE, "null").unwrap();
    assert_eq!(
        w.select(SOURCE, 0, 32).unwrap().total_rows,
        0,
        "null is not a text match"
    );
    w.set_search(SOURCE, "18446744073709551615").unwrap();
    let v = w.select(SOURCE, 0, 32).unwrap();
    assert_eq!(v.total_rows, 1);
    assert_eq!(v.rows[0].occurrence, 2);
    assert_eq!(v.rows[0].view_index, 0);
    let row = v.rows[0].id.clone();
    assert!(
        w.paste_at(SOURCE, v.revision, v.generation, &row, "name", "a\nb")
            .is_err()
    );
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes, before);
    let other = w.select("sources/catalog-schema.yaml", 0, 32).unwrap();
    assert_eq!(other.view_state.search, "18446744073709551615");
    w.set_search(SOURCE, "").unwrap();
    let v = w.select(SOURCE, 0, 32).unwrap();
    assert_eq!(v.rows[0].occurrence, 1);
    assert_eq!(v.rows[1].occurrence, 2);
    assert!(!w.drafts[SOURCE].can_undo());
}
