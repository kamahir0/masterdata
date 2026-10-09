use masterdata_engine::{
    project::Project,
    semantic::{Field, Types, interpret},
    source::{Document, Raw},
};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn independent_schema_directed_interpretation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1");
    let cases: Value =
        serde_json::from_slice(&fs::read(root.join("interpretation.json")).unwrap()).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let doc =
            Document::parse(format!("value: {}\n", case["source"].as_str().unwrap())).unwrap();
        let field = Field {
            key: 0,
            name: "value".into(),
            type_name: case["target"].as_str().unwrap().into(),
            nullable: case["nullable"].as_bool().unwrap_or(false),
            array: false,
        };
        let raw = doc.root.required("value").unwrap();
        let result = interpret(&field, raw, &Types::new());
        assert_eq!(result.is_ok(), case["valid"].as_bool().unwrap(), "{case}");
        if let Some(s) = case["expectedText"].as_str() {
            assert_eq!(raw.text().unwrap(), s);
        }
        if let Some(is_null) = case["isNull"].as_bool() {
            assert_eq!(matches!(raw.raw, Raw::Null), is_null);
        }
        println!("PASS interpretation {case}");
    }
}

#[test]
fn full_project_lowering_keeps_nested_and_64_bit_values() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/full");
    let p = Project::open(&root).unwrap();
    let (problems, rows) = p.validate(None).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
    let rows = &rows["item"];
    assert_eq!(rows.len(), 2);
    let json = serde_json::to_value(rows).unwrap();
    assert!(json.to_string().contains("18446744073709551615"));
    assert!(json.to_string().contains("-9223372036854775808"));
    assert!(json.to_string().contains("2001"));
    assert_eq!(
        rows[0].occurrence, 2,
        "Build order is PK ascending, independent of source order"
    );
}

#[test]
fn empty_table_has_no_implicit_source_and_minimal_is_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
    let p = Project::open(&root.join("rewrite-oracle/v1/empty-table/input")).unwrap();
    let name = p.tables.keys().next().unwrap();
    assert!(p.record_sources(name).is_empty());
    assert!(p.validate(None).unwrap().0.is_empty());
    assert!(
        Project::open(&root.join("minimal"))
            .unwrap()
            .validate(None)
            .unwrap()
            .0
            .is_empty()
    );
}

#[test]
fn schema_keys_and_generated_identifiers_are_validated_without_repair() {
    for key in ["\"0\"", "+0", "01", "-1", "2147483648"] {
        let doc = Document::parse(format!(
            "fields:\n  - key: {key}\n    name: id\n    type: int\n"
        ))
        .unwrap();
        assert!(
            masterdata_engine::semantic::fields(doc.root.required("fields").unwrap()).is_err(),
            "{key}"
        );
    }
    let doc = Document::parse("fields:\n  - key: 0\n    name: class\n    type: int\n").unwrap();
    assert!(masterdata_engine::semantic::fields(doc.root.required("fields").unwrap()).is_err());
}

#[test]
fn invalid_key_capability_returns_diagnostics_instead_of_aborting_validation() {
    let temp = tempfile::tempdir().unwrap();
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/minimal/masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    fs::write(temp.path().join("sources/item.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: ratio\n    type: float\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [ratio]\nrecords:\n  - id: 1\n    ratio: 1.5\n").unwrap();
    let problems = Project::open(temp.path())
        .unwrap()
        .validate(None)
        .unwrap()
        .0;
    assert!(
        problems.iter().any(|d| d.code == "E-KEY-CAPABILITY"),
        "{problems:#?}"
    );
}
