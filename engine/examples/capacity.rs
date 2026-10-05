//! Fresh adapter for the fixed portable capacity oracle. Measurement is native
//! engine evidence; it does not substitute for native Desktop rendering/input.
use masterdata_engine::{instrument, workspace::Workspace};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};
fn measured<T>(stages: &mut BTreeMap<String, Value>, name: &str, action: impl FnOnce() -> T) -> T {
    let (value, measurement) = instrument::measure(action);
    stages.insert(name.into(), serde_json::to_value(measurement).unwrap());
    eprintln!("completed {name}");
    value
}
fn verify(workspace: &Workspace, spec: &Value, pasted: bool) -> usize {
    let files = spec["input"]["dataFiles"].as_u64().unwrap() as usize;
    let rows = spec["input"]["recordsPerFile"].as_u64().unwrap() as usize;
    let columns = spec["input"]["columns"].as_u64().unwrap() as usize;
    let paste_rows = spec["operation"]["pasteRows"].as_u64().unwrap() as usize;
    let selected = spec["operation"]["selectedFileIndex"].as_u64().unwrap() as usize;
    let mut checked = 0;
    for file in 0..files {
        let path = format!("sources/data{file:02}.yaml");
        let doc = workspace
            .drafts
            .get(&path)
            .map(|draft| &draft.document)
            .unwrap_or_else(|| workspace.read.sources[&path].document.as_ref().unwrap());
        let records = doc.records().unwrap();
        assert_eq!(records.len(), rows);
        for (row, record) in records.iter().enumerate() {
            for column in 0..columns {
                let expected =
                    if pasted && file == selected && row < paste_rows && (1..=10).contains(&column)
                    {
                        1_000_000 + row * 10 + column - 1
                    } else {
                        file * rows + row + column
                    };
                assert_eq!(
                    record
                        .value
                        .get(&format!("field{column:02}"))
                        .unwrap()
                        .text()
                        .unwrap(),
                    expected.to_string(),
                    "file {file} row {row} column {column}"
                );
                checked += 1;
            }
        }
    }
    checked
}
fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("capacity project path required");
    let spec: Value = serde_json::from_str(include_str!(
        "../../fixtures/rewrite-oracle/v1/capacity.json"
    ))
    .unwrap();
    assert_eq!(spec["operation"]["pasteColumns"], "field01..field10");
    assert_eq!(
        spec["operation"]["pasteValue"],
        "1000000 + rowIndex * 10 + zeroBasedPasteColumn"
    );
    let started = Instant::now();
    let mut stages = BTreeMap::new();
    let mut workspace = measured(&mut stages, "coldOpen", || Workspace::open(&root).unwrap());
    let source = "sources/data00.yaml";
    let original = workspace.read.sources[source].bytes.clone();
    let identities = workspace
        .read
        .sources
        .iter()
        .map(|(path, source)| (path.clone(), source.identity.clone()))
        .collect::<BTreeMap<_, _>>();
    let checked_before = measured(&mut stages, "oracleValuesBefore", || {
        verify(&workspace, &spec, false)
    });
    let first = measured(&mut stages, "firstSource", || {
        workspace.select(source, 0, 64).unwrap()
    });
    assert_eq!(first.total_rows, 10000);
    assert!(first.rows.len() <= 64);
    let mut samples = vec![];
    for n in 0..110 {
        let path = format!("sources/data{:02}.yaml", n % 10);
        let (view, measurement) = instrument::measure(|| workspace.select(&path, 0, 64).unwrap());
        assert!(view.rows.len() <= 64);
        assert_eq!(measurement.work.project_discovery, 0);
        assert_eq!(measurement.work.project_enumeration, 0);
        assert_eq!(measurement.work.project_yaml_parse, 0);
        assert_eq!(measurement.work.project_validation, 0);
        let serialization = Instant::now();
        let bytes = serde_json::to_vec(&view).unwrap();
        samples.push(json!({"case":if n<10 {"firstSource"}else{"warm"},"measurement":measurement,"serializationMs":serialization.elapsed().as_secs_f64()*1000.0,"bytes":bytes.len()}));
    }
    let found = measured(&mut stages, "find", || {
        workspace
            .set_search(source, spec["operation"]["findText"].as_str().unwrap())
            .unwrap();
        workspace.select(source, 0, 64).unwrap()
    });
    assert_eq!(found.total_rows, 0);
    assert!(workspace.dirty_paths().is_empty());
    workspace.set_search(source, "").unwrap();
    let validation = measured(&mut stages, "savedValidation", || {
        workspace.validation_snapshot().validate(None).unwrap()
    });
    assert!(validation.0.is_empty());
    drop(validation);
    let p = workspace.select(source, 0, 64).unwrap();
    let paste = (0..1000)
        .map(|row| {
            (0..10)
                .map(|column| (1_000_000 + row * 10 + column).to_string())
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(measured(&mut stages, "paste10000Cells", || workspace
        .paste_at(
            source,
            p.revision,
            p.generation,
            &p.rows[0].id,
            "field01",
            &paste
        )
        .unwrap()));
    assert_eq!(workspace.dirty_paths(), vec![source]);
    let checked_after = measured(&mut stages, "oracleValuesAfter", || {
        verify(&workspace, &spec, true)
    });
    let candidate = workspace.drafts[source].document.bytes.clone();
    let mut expected = String::from("kind: data\ntable: capacity\nrecords:\n");
    for row in 0..10000 {
        for column in 0..20 {
            let number = if row < 1000 && (1..=10).contains(&column) {
                1_000_000 + row * 10 + column - 1
            } else {
                row + column
            };
            expected.push_str(&format!(
                "{}field{column:02}: {number}\n",
                if column == 0 { "  - " } else { "    " }
            ));
        }
    }
    assert_eq!(
        candidate.as_ref(),
        expected,
        "paste changed non-target bytes"
    );
    let validation = measured(&mut stages, "candidateValidation", || {
        workspace.validation_snapshot().validate(None).unwrap()
    });
    assert!(validation.0.is_empty());
    drop(validation);
    let p = workspace.select(source, 0, 64).unwrap();
    assert_eq!(p.rows[0].cells[1].display, "1000000");
    assert!(p.can_undo);
    assert!(measured(&mut stages, "undo", || workspace
        .undo(source, false)
        .unwrap()));
    assert_eq!(workspace.drafts[source].document.bytes, original);
    assert!(workspace.dirty_paths().is_empty());
    assert!(measured(&mut stages, "redo", || workspace
        .undo(source, true)
        .unwrap()));
    assert_eq!(workspace.drafts[source].document.bytes, candidate);
    for (path, identity) in identities {
        assert_eq!(
            masterdata_engine::source::content_identity(&std::fs::read(root.join(&path)).unwrap()),
            identity,
            "implicit Save: {path}"
        );
    }
    assert!(!root.join(".masterdata/output").exists());
    println!(
        "{}",
        json!({"kind":"nativeCapacity","oracle":"fixtures/rewrite-oracle/v1/capacity.json","records":100000,"columns":20,"dataFiles":10,"checkedCellsBefore":checked_before,"checkedCellsAfter":checked_after,"pasteChangedCells":10000,"exactCandidateBytes":true,"diskUnchanged":true,"diagnostics":[],"samples":samples,"stages":stages,"totalMs":started.elapsed().as_secs_f64()*1000.0})
    );
}
