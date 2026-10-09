use masterdata_engine::{native::Outcome, workspace::Workspace};
use std::fs;

fn project(records: &str) -> (tempfile::TempDir, Workspace) {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    fs::write(temp.path().join("masterdata.toml"), "[project]\nname = \"Records\"\nid = \"rewrite.records\"\nversion = \"0.1.0\"\n[sources]\nroots = [\"sources\"]\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n").unwrap();
    fs::write(temp.path().join("sources/schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: long\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys: []\n").unwrap();
    fs::write(temp.path().join("sources/data.yaml"), records).unwrap();
    let w = Workspace::open(temp.path()).unwrap();
    (temp, w)
}
const PATH: &str = "sources/data.yaml";
#[test]
fn cancelling_addition_restores_exact_bytes_including_empty_style_and_no_final_newline() {
    for bytes in [
        "kind: data\ntable: item\nrecords: [] # kept\n",
        "kind: data\ntable: item\nrecords: [ ]",
        "kind: data\ntable: item\nrecords:\n  []\n# outside\n",
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: 'x'",
    ] {
        let (temp, mut w) = project(bytes);
        let v = w.select(PATH, 0, 32).unwrap();
        let added = w.add_row(PATH, v.revision, v.generation, None).unwrap();
        let v = w.select(PATH, 0, 32).unwrap();
        let row = v.rows.iter().find(|r| r.id == added).unwrap();
        assert!(row.added);
        assert!(row.cells.iter().all(|c| c.display == "null" && !c.valid));
        w.delete_row(PATH, v.revision, v.generation, &added)
            .unwrap();
        assert_eq!(w.current_doc(PATH).unwrap().bytes.as_ref(), bytes);
        assert!(!w.drafts[PATH].dirty());
        assert_eq!(fs::read_to_string(temp.path().join(PATH)).unwrap(), bytes);
        w.undo(PATH, false).unwrap();
        assert!(w.drafts[PATH].dirty());
        w.undo(PATH, true).unwrap();
        assert!(!w.drafts[PATH].dirty());
    }
}
#[test]
fn edited_duplicate_occurrence_survives_pending_delete_reorder_undo_and_save() {
    let bytes = "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 1\r\n    note: 'a' # item\r\n\r\n# separator\r\n  - id: 1\r\n    note: \"b\"\r\n";
    let (temp, mut w) = project(bytes);
    let v = w.select(PATH, 0, 32).unwrap();
    let a = v.rows[0].id.clone();
    let b = v.rows[1].id.clone();
    w.edit_text(PATH, v.revision, &b, "note", "edited").unwrap();
    let edited = w.current_doc(PATH).unwrap().bytes.clone();
    let v = w.select(PATH, 0, 32).unwrap();
    w.delete_row(PATH, v.revision, v.generation, &b).unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    assert_eq!(v.rows.len(), 2);
    assert!(v.rows[1].pending_delete);
    assert!(v.rows[1].cells.iter().all(|c| !c.editable));
    assert!(
        w.current_doc(PATH)
            .unwrap()
            .bytes
            .contains("# separator\r\n")
    );
    assert!(!w.current_doc(PATH).unwrap().bytes.contains("edited"));
    w.undo_delete(PATH, v.revision, v.generation, &b).unwrap();
    assert_eq!(w.current_doc(PATH).unwrap().bytes, edited);
    let v = w.select(PATH, 0, 32).unwrap();
    w.reorder_rows(PATH, v.revision, v.generation, &[b.clone(), a.clone()])
        .unwrap();
    let expected = "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 1\r\n    note: \"edited\"\r\n\r\n# separator\r\n  - id: 1\r\n    note: 'a' # item\r\n";
    assert_eq!(w.current_doc(PATH).unwrap().bytes.as_ref(), expected);
    let v = w.select(PATH, 0, 32).unwrap();
    assert_eq!(v.rows[0].id, b);
    w.delete_row(PATH, v.revision, v.generation, &a).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join(PATH)).unwrap(),
        bytes,
        "all authoring remains local"
    );
    let results = w.save_table("item", Some(PATH)).unwrap();
    assert_eq!(results[0].outcome, Outcome::Success);
    let v = w.select(PATH, 0, 32).unwrap();
    assert_eq!(v.rows.len(), 1);
    assert_eq!(v.rows[0].id, b);
    assert!(!v.rows[0].added);
    assert!(!v.can_undo);
}
#[test]
fn added_key_is_editable_and_paste_is_one_undo_but_existing_key_is_rejected() {
    let (_temp, mut w) = project("kind: data\ntable: item\nrecords: []\n");
    let v = w.select(PATH, 0, 32).unwrap();
    let row = w.add_row(PATH, v.revision, v.generation, None).unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    w.paste_at(
        PATH,
        v.revision,
        v.generation,
        &row,
        "id",
        "9223372036854775807\tnull",
    )
    .unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    assert_eq!(v.rows[0].cells[0].display, "9223372036854775807");
    assert!(v.rows[0].cells.iter().all(|c| c.valid));
    w.undo(PATH, false).unwrap();
    assert!(w.select(PATH, 0, 32).unwrap().rows[0].added);
    w.undo(PATH, true).unwrap();
    w.save_table("item", Some(PATH)).unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    assert!(!v.rows[0].added);
    assert!(
        w.paste_at(PATH, v.revision, v.generation, &row, "id", "2")
            .is_err()
    );
}
#[test]
fn column_reorder_preserves_field_bytes_and_does_not_dirty_separate_data() {
    let (_temp, mut w) = project("kind: data\ntable: item\nrecords:\n  - id: 1\n    note: text\n");
    let v = w.select(PATH, 0, 32).unwrap();
    let before = w.current_doc(PATH).unwrap().bytes.clone();
    w.reorder_columns(
        "sources/schema.yaml",
        v.schema_revision,
        &["note".into(), "id".into()],
    )
    .unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    assert_eq!(v.columns[0].field.name, "note");
    assert!(v.schema_dirty);
    assert!(!v.dirty);
    assert_eq!(w.current_doc(PATH).unwrap().bytes, before);
    w.undo("sources/schema.yaml", false).unwrap();
    assert_eq!(w.select(PATH, 0, 32).unwrap().columns[0].field.name, "id");
}

#[test]
fn pending_delete_anchors_survive_scalar_length_changes_and_multiple_deletes() {
    let bytes = "kind: data\ntable: item\nrecords:\n# before\n  - id: 1\n    note: a\n\n# middle\n  - id: 2\n    note: b\n\n# after\n  - id: 3\n    note: c\n# eof\n";
    let (_temp, mut w) = project(bytes);
    let v = w.select(PATH, 0, 32).unwrap();
    let ids = v.rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    w.delete_row(PATH, v.revision, v.generation, &ids[1])
        .unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    w.delete_row(PATH, v.revision, v.generation, &ids[0])
        .unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    w.edit_text(PATH, v.revision, &ids[2], "note", "longer text")
        .unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    w.undo_delete(PATH, v.revision, v.generation, &ids[0])
        .unwrap();
    let v = w.select(PATH, 0, 32).unwrap();
    w.undo_delete(PATH, v.revision, v.generation, &ids[1])
        .unwrap();
    assert_eq!(
        w.current_doc(PATH).unwrap().bytes.as_ref(),
        bytes.replace("note: c", "note: longer text")
    );
    w.undo(PATH, false).unwrap();
    assert!(w.select(PATH, 0, 32).unwrap().rows[1].pending_delete);
}
