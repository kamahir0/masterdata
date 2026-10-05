use masterdata_engine::{
    native::Outcome,
    workspace::{Workspace, tags::TagOperation},
};
use std::fs;
const PATH: &str = "sources/data.yaml";
fn project(records: &str) -> (tempfile::TempDir, Workspace) {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    fs::write(temp.path().join("masterdata.toml"), "[project]\nname = \"Tags\"\nid = \"rewrite.tags\"\nversion = \"0.1.0\"\n[sources]\nroots = [\"sources\"]\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n[build.profiles.production]\nexclude_tags = [\"profile-only\"]\n").unwrap();
    fs::write(temp.path().join("sources/schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: long\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys: []\n").unwrap();
    fs::write(temp.path().join(PATH), records).unwrap();
    let w = Workspace::open(temp.path()).unwrap();
    (temp, w)
}
fn edit(w: &mut Workspace, row: &str, op: TagOperation) {
    let v = w.select(PATH, 0, 64).unwrap();
    w.edit_tag(PATH, v.revision, v.generation, row, op).unwrap();
}
#[test]
fn absent_add_remove_is_exact_clean_including_crlf_comments_and_no_final_newline() {
    for bytes in [
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: 'x'",
        "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 1\r\n    note: 'x' # keep\r\n\r\n# outside\r\n",
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: |\n      x\n\n# keep\n  - id: 2\n    note: y\n",
    ] {
        let (temp, mut w) = project(bytes);
        let v = w.select(PATH, 0, 64).unwrap();
        let row = &v.rows[0].id;
        let tags = w.tag_view(PATH, v.revision, v.generation, row, 0).unwrap();
        assert!(tags.editable);
        assert!(!w.drafts[PATH].dirty(), "opening alone is clean");
        assert_eq!(tags.known, ["profile-only"]);
        edit(
            &mut w,
            row,
            TagOperation::Add {
                text: "debug".into(),
            },
        );
        edit(&mut w, row, TagOperation::Remove { index: 0 });
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
fn existing_flow_empty_and_quoted_entries_preserve_style_and_other_bytes() {
    for record in [
        "{id: 1, note: x, $tags: [ ]}",
        "\n    id: 1\n    note: x\n    $tags: [] # keep",
    ] {
        let bytes = format!("kind: data\ntable: item\nrecords:\n  - {record}\n");
        let (_temp, mut w) = project(&bytes);
        let v = w.select(PATH, 0, 64).unwrap();
        let row = &v.rows[0].id;
        edit(
            &mut w,
            row,
            TagOperation::Add {
                text: "debug".into(),
            },
        );
        assert_eq!(
            w.current_doc(PATH).unwrap().bytes.as_ref(),
            bytes.replace("[ ]", "[ debug]").replace("[]", "[debug]")
        );
        edit(&mut w, row, TagOperation::Remove { index: 0 });
        assert_eq!(w.current_doc(PATH).unwrap().bytes.as_ref(), bytes);
    }
    let bytes = "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 1\r\n    note: 'x'\r\n    $tags:\r\n      - 'debug' # entry\r\n      - \"common\"\r\n# outside\r\n";
    let (_temp, mut w) = project(bytes);
    let v = w.select(PATH, 0, 64).unwrap();
    let row = &v.rows[0].id;
    edit(
        &mut w,
        row,
        TagOperation::Replace {
            index: 0,
            text: "release".into(),
        },
    );
    assert_eq!(
        w.current_doc(PATH).unwrap().bytes.as_ref(),
        bytes.replace("'debug'", "'release'")
    );
    edit(&mut w, row, TagOperation::Remove { index: 1 });
    edit(&mut w, row, TagOperation::Remove { index: 0 });
    let doc = w.current_doc(PATH).unwrap();
    assert!(
        doc.occurrence_value(1, &["$tags".into()])
            .unwrap()
            .items()
            .unwrap()
            .is_empty()
    );
    assert!(doc.bytes.contains("# outside\r\n"));
}
#[test]
fn invalid_empty_duplicate_tags_are_retained_saveable_and_strict_build_invalid() {
    let (temp, mut w) = project("kind: data\ntable: item\nrecords:\n  - id: 1\n    note: x\n");
    let v = w.select(PATH, 0, 64).unwrap();
    let row = &v.rows[0].id;
    for text in ["", " Debug ", "debug", "debug"] {
        edit(&mut w, row, TagOperation::Add { text: text.into() });
    }
    let v = w.select(PATH, 0, 64).unwrap();
    let tags = w.tag_view(PATH, v.revision, v.generation, row, 0).unwrap();
    assert_eq!(
        tags.entries
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>(),
        ["", " Debug ", "debug", "debug"]
    );
    assert!(tags.entries.iter().all(|t| !t.valid));
    assert_eq!(
        w.save_table("item", Some(PATH)).unwrap()[0].outcome,
        Outcome::Success
    );
    let saved = masterdata_engine::project::Project::open(temp.path()).unwrap();
    assert!(
        saved
            .validate(None)
            .unwrap()
            .0
            .iter()
            .any(|d| d.code == "E-RECORD-TAGS")
    );
    let v = w.select(PATH, 0, 64).unwrap();
    edit(&mut w, row, TagOperation::Remove { index: 3 });
    edit(&mut w, row, TagOperation::Remove { index: 2 });
    edit(&mut w, row, TagOperation::Remove { index: 1 });
    edit(&mut w, row, TagOperation::Remove { index: 0 });
    assert!(
        w.current_doc(PATH)
            .unwrap()
            .occurrence_value(1, &["$tags".into()])
            .unwrap()
            .items()
            .unwrap()
            .is_empty(),
        "saved Tags property no longer has an absent origin"
    );
    assert!(!v.can_undo, "Save does not provide filesystem Undo");
}
#[test]
fn exact_duplicate_occurrence_composes_with_value_added_delete_and_undo() {
    let (_temp, mut w) = project(
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: 'a'\n  - id: 1\n    note: \"b\"\n",
    );
    let v = w.select(PATH, 0, 64).unwrap();
    let a = &v.rows[0].id;
    let b = &v.rows[1].id;
    edit(
        &mut w,
        b,
        TagOperation::Add {
            text: "debug".into(),
        },
    );
    let v = w.select(PATH, 0, 64).unwrap();
    w.edit_text(PATH, v.revision, b, "note", "edited").unwrap();
    let before_delete = w.current_doc(PATH).unwrap().bytes.clone();
    let v = w.select(PATH, 0, 64).unwrap();
    w.delete_row(PATH, v.revision, v.generation, b).unwrap();
    let v = w.select(PATH, 0, 64).unwrap();
    assert!(
        w.edit_tag(
            PATH,
            v.revision,
            v.generation,
            b,
            TagOperation::Add {
                text: "other".into()
            }
        )
        .is_err()
    );
    w.undo_delete(PATH, v.revision, v.generation, b).unwrap();
    assert_eq!(w.current_doc(PATH).unwrap().bytes, before_delete);
    let v = w.select(PATH, 0, 64).unwrap();
    assert!(
        w.tag_view(PATH, v.revision, v.generation, a, 0)
            .unwrap()
            .entries
            .is_empty()
    );
    let row = w.add_row(PATH, v.revision, v.generation, None).unwrap();
    edit(&mut w, &row, TagOperation::Add { text: "new".into() });
    let v = w.select(PATH, 0, 64).unwrap();
    w.delete_row(PATH, v.revision, v.generation, &row).unwrap();
    assert_eq!(w.current_doc(PATH).unwrap().bytes, before_delete);
}
#[test]
fn unsafe_tag_shape_is_read_only_without_disabling_value_edit_and_stale_request_is_rejected() {
    for tags in ["null", "debug", "[debug, null]", "[debug, {bad: x}]"] {
        let bytes = format!(
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: 'x'\n    $tags: {tags}\n"
        );
        let (_temp, mut w) = project(&bytes);
        let v = w.select(PATH, 0, 64).unwrap();
        let row = &v.rows[0].id;
        let tags = w.tag_view(PATH, v.revision, v.generation, row, 0).unwrap();
        assert!(!tags.editable && tags.reason.is_some());
        assert!(
            w.edit_tag(
                PATH,
                v.revision,
                v.generation,
                row,
                TagOperation::Add {
                    text: "debug".into()
                }
            )
            .is_err()
        );
        assert_eq!(w.current_doc(PATH).unwrap().bytes.as_ref(), bytes);
        w.edit_text(PATH, v.revision, row, "note", "new").unwrap();
        assert_eq!(
            w.current_doc(PATH).unwrap().bytes.as_ref(),
            bytes.replace("'x'", "'new'")
        );
    }
    let (_temp, mut w) = project("kind: data\ntable: item\nrecords:\n  - id: 1\n    note: x\n");
    let old = w.select(PATH, 0, 64).unwrap();
    let row = &old.rows[0].id;
    edit(
        &mut w,
        row,
        TagOperation::Add {
            text: "debug".into(),
        },
    );
    let bytes = w.current_doc(PATH).unwrap().bytes.clone();
    assert!(
        w.edit_tag(
            PATH,
            old.revision,
            old.generation,
            row,
            TagOperation::Remove { index: 0 }
        )
        .is_err()
    );
    assert_eq!(w.current_doc(PATH).unwrap().bytes, bytes);
}

#[test]
fn inline_tag_edit_keeps_schema_bytes_and_paged_metadata_stays_bounded() {
    let (temp, _) = project("kind: data\ntable: item\nrecords: []\n");
    let schema = fs::read_to_string(temp.path().join("sources/schema.yaml")).unwrap();
    let inline = schema.clone() + "records:\n  - id: 1\n    note: x\n";
    fs::write(temp.path().join("sources/schema.yaml"), &inline).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let path = "sources/schema.yaml";
    for i in 0..110 {
        let v = w.select(path, 0, 8).unwrap();
        w.edit_tag(
            path,
            v.revision,
            v.generation,
            &v.rows[0].id,
            TagOperation::Add {
                text: format!("tag-{i}"),
            },
        )
        .unwrap();
    }
    let v = w.select(path, 0, 8).unwrap();
    let tags = w
        .tag_view(path, v.revision, v.generation, &v.rows[0].id, 64)
        .unwrap();
    assert_eq!(tags.total, 110);
    assert_eq!(tags.entries.len(), 46);
    assert_eq!(tags.entries[0].index, 64);
    assert!(tags.partial && tags.known.len() == 100);
    assert!(w.current_doc(path).unwrap().bytes.starts_with(&schema));
    assert_eq!(w.dirty_paths(), [path]);
    assert_eq!(
        w.current_doc(PATH).unwrap().bytes.as_ref(),
        "kind: data\ntable: item\nrecords: []\n"
    );
}

#[test]
fn metadata_insertion_preserves_unterminated_block_scalar_value_and_exact_inverse() {
    let bytes = "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: |\n      x";
    let (_temp, mut w) = project(bytes);
    let v = w.select(PATH, 0, 8).unwrap();
    w.edit_tag(
        PATH,
        v.revision,
        v.generation,
        &v.rows[0].id,
        TagOperation::Add {
            text: "debug".into(),
        },
    )
    .unwrap();
    assert_eq!(
        w.current_doc(PATH)
            .unwrap()
            .occurrence_value(1, &["note".into()])
            .unwrap()
            .text()
            .unwrap(),
        "x\n"
    );
    assert_eq!(
        w.current_doc(PATH).unwrap().bytes.as_ref(),
        format!("{bytes}\n    $tags:\n      - debug")
    );
    edit(&mut w, &v.rows[0].id, TagOperation::Remove { index: 0 });
    assert_eq!(w.current_doc(PATH).unwrap().bytes.as_ref(), bytes);
    assert!(!w.drafts[PATH].dirty());
}
