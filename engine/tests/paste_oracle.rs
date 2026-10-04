use masterdata_engine::{clipboard, workspace::Workspace};
use serde_json::Value as Json;
use std::{fs, path::Path};
fn copy_files(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_files(&entry.path(), &target.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
fn independent_paste_bytes_are_lossless_and_failure_is_all_or_none() {
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1");
    let manifest: Json =
        serde_json::from_slice(&fs::read(oracle.join("manifest.json")).unwrap()).unwrap();
    for id in manifest["pasteScenarios"].as_array().unwrap() {
        let case = oracle.join(id.as_str().unwrap());
        let temp = tempfile::tempdir().unwrap();
        copy_files(&case.join("input"), temp.path());
        let scenario: Json =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let op = &scenario["operation"];
        let path = op["source"].as_str().unwrap();
        let before = fs::read(temp.path().join(path)).unwrap();
        let mut w = Workspace::open(temp.path()).unwrap();
        let view = w.select(path, 0, 64).unwrap();
        let rows = op["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| view.rows[n.as_u64().unwrap() as usize - 1].id.clone())
            .collect::<Vec<_>>();
        let fields = op["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().into())
            .collect::<Vec<_>>();
        let text = fs::read_to_string(case.join(op["clipboard"].as_str().unwrap())).unwrap();
        let result = w.paste(path, view.revision, view.generation, &rows, &fields, &text);
        assert_eq!(
            result.is_ok(),
            scenario["expected"]["outcome"] == "candidate",
            "{id}: {result:?}"
        );
        assert_eq!(
            w.current_doc(path).unwrap().bytes.as_bytes(),
            fs::read(case.join("expected/data.yaml")).unwrap(),
            "{id} exact candidate bytes"
        );
        assert_eq!(
            fs::read(temp.path().join(path)).unwrap(),
            before,
            "paste must not Save"
        );
        if result.is_ok() {
            let (problems, _) = w.validation_snapshot().validate(None).unwrap();
            assert_eq!(
                problems.is_empty(),
                scenario["expected"]["semanticValid"].as_bool().unwrap(),
                "{id}: {problems:?}"
            );
            assert!(
                w.undo(path, false).unwrap(),
                "accepted paste is one history unit"
            );
            assert_eq!(w.current_doc(path).unwrap().bytes.as_bytes(), before);
            assert!(!w.undo(path, false).unwrap(), "no per-cell history");
        } else {
            assert!(!w.drafts[path].can_undo());
        }
    }
}

#[test]
fn clipboard_codec_keeps_embedded_delimiters_and_exact_ending_fields() {
    let rows = vec![
        vec!["".into(), "a\tb\nc\r\nd\re\"f".into()],
        vec!["null".into(), "=1+1".into()],
    ];
    assert_eq!(
        clipboard::decode(&clipboard::encode(&rows).unwrap()).unwrap(),
        rows
    );
    assert_eq!(clipboard::decode("").unwrap(), vec![vec![""]]);
    assert_eq!(clipboard::decode("a\t\r\n").unwrap(), vec![vec!["a", ""]]);
    assert_eq!(
        clipboard::decode("a\n\n").unwrap(),
        vec![vec!["a"], vec![""]]
    );
    for unsafe_text in ["a\rb", "a\"b", "\"a\"b", "\"a", "a\tb\nc"] {
        assert!(clipboard::decode(unsafe_text).is_err(), "{unsafe_text:?}");
    }
}

#[test]
fn stale_semantics_rejects_paste_without_mutation_and_copy_keeps_64_bits() {
    let oracle =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1/paste-lossless");
    let temp = tempfile::tempdir().unwrap();
    copy_files(&oracle.join("input"), temp.path());
    let path = "sources/data.yaml";
    let mut w = Workspace::open(temp.path()).unwrap();
    let view = w.select(path, 0, 32).unwrap();
    let row = view.rows[0].id.clone();
    w.paste_at(
        path,
        view.revision,
        view.generation,
        &row,
        "signed",
        "9223372036854775807\t18446744073709551615\tnull",
    )
    .unwrap();
    let view = w.select(path, 0, 32).unwrap();
    let text = w
        .copy(
            path,
            view.revision,
            view.generation,
            std::slice::from_ref(&row),
            &["signed".into(), "unsigned".into(), "note".into()],
        )
        .unwrap();
    assert_eq!(
        text,
        "\"9223372036854775807\"\t\"18446744073709551615\"\t\"null\""
    );
    let bytes = w.current_doc(path).unwrap().bytes.clone();
    w.schema_modifier(
        "sources/schema.yaml",
        view.schema_revision,
        "signed",
        true,
        false,
        None,
    )
    .unwrap();
    assert!(
        w.paste_at(path, view.revision, view.generation, &row, "signed", "1")
            .is_err()
    );
    assert_eq!(w.current_doc(path).unwrap().bytes, bytes);
}
