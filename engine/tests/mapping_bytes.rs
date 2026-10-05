use masterdata_engine::source::{Document, Value};

#[test]
fn member_removal_keeps_cst_comments_crlf_blank_lines_and_record_identity() {
    let input = "records:\r\n  - 'note': | # field comment\r\n      # value text\r\n      text\r\n\r\n    id: 7 # identity\r\n    keep: \"null\"\r\n# tail\r\n";
    let doc = Document::parse(input).unwrap();
    let record = &doc.records().unwrap()[0].value;
    let mut patches = Vec::new();
    doc.derive_member_drop(record, "note", &mut patches)
        .unwrap();
    let after = doc.patched(patches).unwrap();
    assert_eq!(
        after.bytes.as_ref(),
        "records:\r\n    # field comment\r\n\r\n  - id: 7 # identity\r\n    keep: \"null\"\r\n# tail\r\n"
    );
    assert_eq!(after.records().unwrap().len(), 1);
    assert_eq!(
        after.records().unwrap()[0]
            .value
            .required("id")
            .unwrap()
            .text()
            .unwrap(),
        "7"
    );
}

#[test]
fn mapping_rename_add_and_drop_are_local_and_preserve_existing_flow() {
    let input = "records:\n  - {'id': 1, \"note\": 'two', keep: 0007,} # untouched\n# tail";
    let doc = Document::parse(input).unwrap();
    let record = &doc.records().unwrap()[0].value;
    let mut patches = Vec::new();
    doc.derive_member_rename(record, "note", "memo", &mut patches)
        .unwrap();
    doc.derive_member_drop(record, "id", &mut patches).unwrap();
    doc.derive_member_add(record, "rank", &Value::Literal("7".into()), &mut patches)
        .unwrap();
    let after = doc.patched(patches).unwrap();
    assert_eq!(
        after.bytes.as_ref(),
        "records:\n  - { \"memo\": 'two', keep: 0007,rank: 7} # untouched\n# tail"
    );
    let mut patches = Vec::new();
    assert!(
        doc.derive_member_add(
            record,
            "new",
            &Value::Mapping(vec![("x".into(), Value::Literal("1".into()))]),
            &mut patches
        )
        .is_err()
    );
}

#[test]
fn a_new_mapping_is_block_style_without_serializing_existing_members() {
    let doc = Document::parse("records:\n  - id: 1 # keep\n    note: 'text'\n# tail").unwrap();
    let mut patches = Vec::new();
    doc.derive_member_add(
        &doc.records().unwrap()[0].value,
        "reward",
        &Value::Mapping(vec![("itemId".into(), Value::Literal("2001".into()))]),
        &mut patches,
    )
    .unwrap();
    assert_eq!(
        doc.patched(patches).unwrap().bytes.as_ref(),
        "records:\n  - id: 1 # keep\n    note: 'text'\n    reward:\n      itemId: 2001\n# tail"
    );
}
