use masterdata_engine::source::{Document, Value};
use serde_json::Value as Json;
use std::{fs, path::PathBuf};

fn value(v: &Json) -> Value {
    match v["meaning"].as_str().unwrap() {
        "text" => Value::Text(v["text"].as_str().unwrap().into()),
        "integer" => Value::Literal(v["decimal"].as_str().unwrap().into()),
        "null" => Value::Null,
        "mapping" => Value::Mapping(
            v["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| (m["name"].as_str().unwrap().into(), value(&m["value"])))
                .collect(),
        ),
        "sequence" => Value::Sequence(v["items"].as_array().unwrap().iter().map(value).collect()),
        s => panic!("oracle meaning {s}"),
    }
}

#[test]
fn independent_source_byte_scenarios() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1");
    let manifest: Json =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for case in manifest["byteScenarios"].as_array().unwrap() {
        let id = case.as_str().unwrap();
        let dir = root.join(id);
        let scenario: Json =
            serde_json::from_slice(&fs::read(dir.join("scenario.json")).unwrap()).unwrap();
        let operation = &scenario["operation"];
        let source = operation["source"].as_str().unwrap();
        let input = fs::read_to_string(dir.join("input").join(source)).unwrap();
        let document = Document::parse(input.clone()).unwrap_or_else(|e| panic!("{id}: {e}"));
        let candidate = if operation["intent"].as_str() == Some("read-source") {
            Ok(document.clone())
        } else {
            document.edit_occurrence(
                operation["occurrence"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{id}: missing occurrence")) as usize,
                &[operation["field"].as_str().unwrap().into()],
                &value(&operation["value"]),
            )
        };
        let actual = match scenario["expected"]["outcome"].as_str().unwrap() {
            "candidate" | "read" => candidate
                .unwrap_or_else(|e| panic!("{id}: {e}"))
                .bytes
                .to_string(),
            "unsafe-localization" => {
                assert!(candidate.is_err(), "{id} must fail closed");
                input.clone()
            }
            s => panic!("unknown outcome {s}"),
        };
        let expected =
            fs::read_to_string(dir.join(scenario["expected"]["source"].as_str().unwrap())).unwrap();
        assert_eq!(
            actual.as_bytes(),
            expected.as_bytes(),
            "exact bytes: {id}\nactual={actual:?}\nexpected={expected:?}"
        );
        assert_eq!(document.bytes.as_ref(), input, "oracle input retained");
        println!("PASS {id}");
    }
}

#[test]
fn no_op_keeps_exact_bytes_and_decoded_mapping_keys_reject_duplicates() {
    let input = "kind: data\r\ntable: item\r\nrecords:\r\n  - id: '1' # quote\r\n";
    let doc = Document::parse(input).unwrap();
    let candidate = doc
        .edit_occurrence(1, &["id".into()], &Value::Literal("1".into()))
        .unwrap();
    assert_eq!(candidate.bytes.as_ref(), input);
    assert!(Document::parse("name: 1\n\"name\": 2\n").is_err());
    assert!(Document::parse("name:\n").is_err());
    assert!(Document::parse("name: &secret foo\n").is_err());
}

#[test]
fn typed_literal_cannot_inject_members_and_quoted_unicode_is_lossless() {
    let input = "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: 'old'\n";
    let doc = Document::parse(input).unwrap();
    assert!(
        doc.edit_occurrence(
            1,
            &["id".into()],
            &Value::Literal("2\n    injected: true".into())
        )
        .is_err()
    );
    assert_eq!(doc.bytes.as_ref(), input);
    let text = "\u{65e5}\u{672c}\u{8a9e}\n\t\"\\'";
    let new = doc
        .edit_occurrence(1, &["note".into()], &Value::Text(text.into()))
        .unwrap();
    assert_eq!(
        new.records().unwrap()[0]
            .value
            .get("note")
            .unwrap()
            .text()
            .unwrap(),
        text
    );
}

#[test]
fn large_source_crosses_scanner_line_boundary() {
    let mut source = String::from("kind: data\ntable: item\nrecords:\n");
    for row in 0..2000 {
        source.push_str(&format!("  - id: {row}\n"));
        for field in 1..20 {
            source.push_str(&format!("    field{field:02}: {}\n", row + field));
        }
    }
    let doc = Document::parse(source.clone()).unwrap();
    assert_eq!(doc.records().unwrap().len(), 2000);
    let edited = doc
        .edit_occurrence(
            2000,
            &["field19".into()],
            &Value::Literal("2147483647".into()),
        )
        .unwrap();
    assert_eq!(
        edited.bytes.as_ref(),
        source.replacen("field19: 2018", "field19: 2147483647", 1)
    );
    assert_eq!(
        edited.records().unwrap()[1999]
            .value
            .required("field19")
            .unwrap()
            .text()
            .unwrap(),
        "2147483647"
    );
}
