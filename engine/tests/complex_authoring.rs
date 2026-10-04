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
fn publish_problems(w: &mut Workspace) {
    let (problems, _) = w.validation_snapshot().validate(None).unwrap();
    assert!(w.accept_diagnostics(w.generation, problems));
}
#[test]
fn problems_resolve_exact_record_and_array_occurrences_without_changing_search_or_bytes() {
    let (_temp, mut w) = project();
    let p = w.select(SOURCE, 0, 32).unwrap();
    let row = p.rows[1].id.clone();
    let numbers = w
        .complex_view(
            SOURCE,
            p.revision,
            p.generation,
            &row,
            &["numbers".into()],
            0..64,
        )
        .unwrap();
    let item = numbers.node.children[1].path.last().unwrap().clone();
    operation(
        &mut w,
        &row,
        &["numbers", &item],
        Operation::Text {
            text: "invalid".into(),
        },
    );
    let p = w.select(SOURCE, 0, 32).unwrap();
    w.delete_row(SOURCE, p.revision, p.generation, &p.rows[0].id)
        .unwrap();
    publish_problems(&mut w);
    let problem = w
        .diagnostics
        .iter()
        .find(|d| d.field_path == ["numbers", "1"])
        .unwrap()
        .clone();
    assert_eq!(problem.occurrence, Some(2));
    let before = w.current_doc(SOURCE).unwrap().bytes.clone();
    let target = w
        .problem_target(
            SOURCE,
            problem.generation,
            problem.occurrence,
            &problem.field_path,
        )
        .unwrap()
        .unwrap();
    assert_eq!(target.row, row);
    assert_eq!(target.view_index, Some(1));
    assert_eq!(target.editor_path, Some(vec!["numbers".into()]));
    assert_eq!(target.focus_path, vec!["numbers".to_owned(), item.clone()]);
    w.set_search(SOURCE, "no matching scalar").unwrap();
    let hidden = w
        .problem_target(
            SOURCE,
            problem.generation,
            problem.occurrence,
            &problem.field_path,
        )
        .unwrap()
        .unwrap();
    assert_eq!(hidden.row, row);
    assert_eq!(hidden.view_index, None);
    assert_eq!(w.views[SOURCE].search, "no matching scalar");
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes, before);
    w.undo(SOURCE, false).unwrap();
    assert!(
        w.problem_target(
            SOURCE,
            problem.generation,
            problem.occurrence,
            &problem.field_path
        )
        .is_err()
    );
}
#[test]
fn problems_open_the_correct_bounded_nested_container_and_preserve_unknown_members() {
    let (temp, _) = project();
    let file = temp.path().join(SOURCE);
    let values = (0..70)
        .map(|i| {
            if i == 69 {
                "invalid".into()
            } else {
                i.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let bytes = fs::read_to_string(&file).unwrap().replace(
        "values: [1]",
        &format!("values: [{values}]\n      unexpected: retained"),
    );
    fs::write(&file, &bytes).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    w.select(SOURCE, 0, 32).unwrap();
    publish_problems(&mut w);
    let unknown = w
        .diagnostics
        .iter()
        .find(|d| d.field_path == ["reward", "unexpected"])
        .unwrap()
        .clone();
    let target = w
        .problem_target(
            SOURCE,
            unknown.generation,
            unknown.occurrence,
            &unknown.field_path,
        )
        .unwrap()
        .unwrap();
    assert_eq!(target.editor_path, Some(vec!["reward".into()]));
    assert_eq!(target.focus_path, vec!["reward", "unexpected"]);
    // A separate validation sees the nested value problem after the explicitly
    // inspected unknown member is absent; no implicit repair by opening occurs.
    assert_eq!(fs::read_to_string(&file).unwrap(), bytes);
    let without_unknown = bytes.replace("      unexpected: retained\n", "");
    fs::write(&file, &without_unknown).unwrap();
    let p = w.select(SOURCE, 0, 32).unwrap();
    publish_problems(&mut w);
    let problem = w
        .diagnostics
        .iter()
        .find(|d| d.field_path == ["reward", "values", "69"])
        .unwrap()
        .clone();
    let target = w
        .problem_target(
            SOURCE,
            problem.generation,
            problem.occurrence,
            &problem.field_path,
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        target.editor_path,
        Some(vec!["reward".into(), "values".into()])
    );
    assert_eq!(target.editor_start, 64);
    let view = w
        .complex_view(
            SOURCE,
            p.revision,
            p.generation,
            &target.row,
            &target.editor_path.unwrap(),
            64..128,
        )
        .unwrap();
    assert_eq!(view.node.children.len(), 6);
    assert_eq!(target.focus_path, view.node.children[5].path);
    assert!(!view.node.children[5].valid);
    assert_eq!(
        w.current_doc(SOURCE).unwrap().bytes.as_ref(),
        without_unknown
    );
    assert!(!w.drafts[SOURCE].dirty());
}
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
fn scalar_input_cannot_cross_an_observed_schema_generation() {
    let (_temp, mut w) = project();
    let before = w.select(SOURCE, 0, 32).unwrap();
    let bytes = w.current_doc(SOURCE).unwrap().bytes.clone();
    w.schema_modifier(
        &before.table.source,
        before.schema_revision,
        "longValue",
        false,
        false,
        Some("string"),
    )
    .unwrap();
    let after = w.select(SOURCE, 0, 32).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_ne!(after.generation, before.generation);
    let stale = w
        .edit_text_at(
            SOURCE,
            before.revision,
            before.generation,
            &before.rows[0].id,
            "longValue",
            "9223372036854775807",
        )
        .unwrap_err();
    assert_eq!(stale.code, "E-DRAFT-STALE");
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes, bytes);
    assert!(!w.drafts[SOURCE].dirty());
    assert!(!w.drafts[SOURCE].can_undo());
    assert!(w.drafts[&before.table.source].dirty());
    assert!(
        w.edit_text_at(
            SOURCE,
            after.revision,
            after.generation,
            &after.rows[0].id,
            "longValue",
            "new text",
        )
        .unwrap()
    );
    w.undo(SOURCE, false).unwrap();
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes, bytes);
    assert!(w.drafts[&before.table.source].dirty());
}
#[test]
fn bounded_array_drop_uses_exact_global_destination_and_one_source_local_undo() {
    let (temp, _) = project();
    let file = temp.path().join(SOURCE);
    let values = (0..70).map(|n| n.to_string()).collect::<Vec<_>>();
    let original = fs::read_to_string(&file).unwrap().replace(
        "numbers: [1, \"1\", -2]",
        &format!("numbers: [{}]", values.join(", ")),
    );
    fs::write(file, &original).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select(SOURCE, 0, 32).unwrap();
    let row = p.rows[1].id.clone();
    let view = w
        .complex_view(
            SOURCE,
            p.revision,
            p.generation,
            &row,
            &["numbers".into()],
            0..64,
        )
        .unwrap();
    assert_eq!(view.node.children.len(), 64);
    let item = view.node.children[0].path.last().unwrap().clone();
    let invalid = w
        .complex_operation(
            SOURCE,
            p.revision,
            p.generation,
            &row,
            &["numbers".into()],
            Operation::Place {
                item: item.clone(),
                index: 70,
            },
        )
        .unwrap_err();
    assert_eq!(invalid.code, "E-REORDER-POSITION");
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes.as_ref(), original);
    assert!(!w.drafts[SOURCE].can_undo());
    assert!(
        w.complex_operation(
            SOURCE,
            p.revision,
            p.generation,
            &row,
            &["numbers".into()],
            Operation::Place {
                item: item.clone(),
                index: 63
            },
        )
        .unwrap()
    );
    let mut expected = values.clone();
    expected.remove(0);
    expected.insert(63, "0".into());
    assert_eq!(
        w.current_doc(SOURCE).unwrap().bytes.as_ref(),
        original.replace(
            &format!("numbers: [{}]", values.join(", ")),
            &format!("numbers: [{}]", expected.join(", ")),
        )
    );
    let next = w.select(SOURCE, 0, 32).unwrap();
    let view = w
        .complex_view(
            SOURCE,
            next.revision,
            next.generation,
            &row,
            &["numbers".into()],
            0..64,
        )
        .unwrap();
    assert_eq!(view.node.children[63].path.last(), Some(&item));
    assert!(w.undo(SOURCE, false).unwrap());
    assert_eq!(w.current_doc(SOURCE).unwrap().bytes.as_ref(), original);
    assert!(!w.drafts[SOURCE].can_undo());
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
