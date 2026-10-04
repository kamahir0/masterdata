use masterdata_engine::source::{Document, Value};

#[test]
fn sequence_delete_and_reorder_keep_independent_comments_blank_lines_and_crlf() {
    let bytes = "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 1 # first\r\n    note: 'one'\r\n\r\n  # standalone separator\r\n  - id: 1 # duplicate PK occurrence\r\n    note: \"two\"\r\n\r\n# unrelated tail\r\n";
    let d = Document::parse(bytes).unwrap();
    let r = d.root.required("records").unwrap();
    let removed = d.remove_sequence(r, 0).unwrap();
    assert_eq!(
        removed.bytes.as_ref(),
        bytes.replace("  - id: 1 # first\r\n    note: 'one'\r\n", "")
    );
    let reordered = d.reorder_sequence(r, &[1, 0]).unwrap();
    let expected = "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 1 # duplicate PK occurrence\r\n    note: \"two\"\r\n\r\n  # standalone separator\r\n  - id: 1 # first\r\n    note: 'one'\r\n\r\n# unrelated tail\r\n";
    assert_eq!(reordered.bytes.as_ref(), expected);
    assert!(d.reorder_sequence(r, &[0, 0]).is_err());
}

#[test]
fn empty_source_adds_block_mapping_without_moving_header_comment() {
    let d = Document::parse("kind: data\ntable: item\nrecords: [] # records header\n# unrelated\n")
        .unwrap();
    let value = Value::Mapping(vec![
        ("id".into(), Value::Null),
        ("note".into(), Value::Text("null".into())),
    ]);
    let added = d
        .insert_sequence(d.root.required("records").unwrap(), 0, &value)
        .unwrap();
    assert_eq!(
        added.bytes.as_ref(),
        "kind: data\ntable: item\nrecords: # records header\n  - id: null\n    note: \"null\"\n# unrelated\n"
    );
    assert_eq!(added.records().unwrap().len(), 1);
}

#[test]
fn flow_array_operations_keep_sibling_lexical_bytes_and_exact_occurrence() {
    let d = Document::parse("value: ['same', \"same\", 'last'] # header\nother: 001\n").unwrap();
    let sequence = d.root.required("value").unwrap();
    assert_eq!(
        d.reorder_sequence(sequence, &[1, 0, 2])
            .unwrap()
            .bytes
            .as_ref(),
        "value: [\"same\", 'same', 'last'] # header\nother: 001\n"
    );
    assert_eq!(
        d.remove_sequence(sequence, 1).unwrap().bytes.as_ref(),
        "value: ['same',  'last'] # header\nother: 001\n"
    );
    assert_eq!(
        d.insert_sequence(sequence, 3, &Value::Text("null".into()))
            .unwrap()
            .bytes
            .as_ref(),
        "value: ['same', \"same\", 'last', \"null\"] # header\nother: 001\n"
    );
}
