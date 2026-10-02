//! Fixed-input Desktop制作v1 performance evidence.
//!
//! This is intentionally a reproducible measurement harness, not a product
//! limit or latency promise. Run it with `/usr/bin/time -l` to capture peak
//! resident memory on macOS.

use std::fmt::Write as _;
use std::fs;
use std::time::Instant;

use masterdata_app::{
    AuthoringBatchRequest, AuthoringRecordMutation, BatchTarget, DataFileQueryRequest,
    NativeApplicationService,
};
use masterdata_core::{AuthoringQuery, encode_clipboard_tsv};

const RECORD_COUNT: usize = 100_000;
const FILE_COUNT: usize = 10;
const COLUMNS: usize = 20;
const RECORDS_PER_FILE: usize = RECORD_COUNT / FILE_COUNT;
const PASTE_CELLS: usize = 10_000;
const PASTE_COLUMNS: usize = 10;
const PASTE_ROWS: usize = PASTE_CELLS / PASTE_COLUMNS;

fn main() {
    let project = tempfile::tempdir().expect("temporary performance project");
    write_project(project.path());
    let service = NativeApplicationService::new();
    let current_dir = project.path();
    let relative_path = "sources/data/items-00.yaml";
    let base_source = fs::read_to_string(project.path().join(relative_path)).expect("base source");

    let started = Instant::now();
    let snapshot = service
        .open_data_file(Some(project.path()), current_dir, relative_path)
        .expect("load snapshot");
    let load_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(snapshot.rows.len(), RECORDS_PER_FILE);
    assert_eq!(snapshot.columns.len(), COLUMNS);
    assert!(snapshot.validation.valid);

    let started = Instant::now();
    let query = service
        .query_data_file(
            Some(project.path()),
            current_dir,
            &DataFileQueryRequest {
                relative_path: relative_path.to_owned(),
                base_source: base_source.clone(),
                mutation: AuthoringRecordMutation::default(),
                query: AuthoringQuery {
                    search: "99999".to_owned(),
                    ..AuthoringQuery::default()
                },
            },
        )
        .expect("query snapshot");
    let query_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(query.total_count, RECORDS_PER_FILE);
    assert_eq!(query.displayed_count, 0);
    assert!(query.ordered_record_indices.is_empty());

    let targets = (0..PASTE_ROWS)
        .flat_map(|record_index| {
            (0..PASTE_COLUMNS).map(move |column_index| BatchTarget {
                record_index: Some(record_index),
                added_record_index: None,
                field: field_name(column_index + 1),
            })
        })
        .collect::<Vec<_>>();
    let clipboard_rows = (0..PASTE_ROWS)
        .map(|row| {
            (0..PASTE_COLUMNS)
                .map(|column| (1_000_000 + row * PASTE_COLUMNS + column).to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let clipboard_text = encode_clipboard_tsv(&clipboard_rows).expect("clipboard");
    let started = Instant::now();
    let preview = service
        .preview_data_file_batch(
            Some(project.path()),
            current_dir,
            relative_path,
            &base_source,
            &AuthoringRecordMutation::default(),
            &AuthoringBatchRequest {
                targets,
                clipboard_text,
                fill: false,
            },
        )
        .expect("batch preview");
    let preview_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(preview.target_count, PASTE_CELLS);
    assert_eq!(preview.changed_cell_count, PASTE_CELLS);
    assert!(preview.source.validation.valid);
    // Portable capacity oracle: inspect every candidate cell, including untargeted
    // rows/columns. Success alone cannot prove paste landed on the right targets.
    let candidate: serde_yaml::Value =
        serde_yaml::from_str(&preview.source.candidate_source).unwrap();
    let records = candidate["records"].as_sequence().unwrap();
    assert_eq!(records.len(), RECORDS_PER_FILE);
    for (row, record) in records.iter().enumerate() {
        for column in 0..COLUMNS {
            let expected = if row < PASTE_ROWS && (1..=PASTE_COLUMNS).contains(&column) {
                1_000_000 + row * PASTE_COLUMNS + column - 1
            } else {
                row + column
            };
            assert_eq!(record[field_name(column)].as_u64(), Some(expected as u64));
        }
    }
    assert_eq!(
        fs::read_to_string(project.path().join(relative_path)).unwrap(),
        base_source
    );

    let started = Instant::now();
    let validation = service
        .validate(Some(project.path()), current_dir)
        .expect("validation");
    let validation_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(validation.valid);
    assert!(validation.diagnostics.is_empty());

    println!(
        "records={RECORD_COUNT} split_files={FILE_COUNT} columns={COLUMNS} paste_cells={PASTE_CELLS} paste_columns={PASTE_COLUMNS}"
    );
    println!("load_ms={load_ms:.3}");
    println!("query_ms={query_ms:.3}");
    println!("preview_ms={preview_ms:.3}");
    println!("validation_ms={validation_ms:.3}");
    println!(
        "oracle=rows-query-all-paste-cells-untargeted-cells-diagnostics-no-implicit-save-pass"
    );
}

fn write_project(root: &std::path::Path) {
    fs::create_dir_all(root.join("sources/data")).expect("data directory");
    fs::create_dir_all(root.join("sources/schemas")).expect("schema directory");
    fs::write(root.join("masterdata.toml"), config_source()).expect("config");
    fs::write(root.join("sources/schemas/items.yaml"), schema_source()).expect("schema");
    for file_index in 0..FILE_COUNT {
        let mut source = String::with_capacity(RECORDS_PER_FILE * COLUMNS * 12);
        writeln!(source, "kind: data\ntable: items\nrecords:").expect("header");
        for row in 0..RECORDS_PER_FILE {
            let id = file_index * RECORDS_PER_FILE + row;
            writeln!(source, "  - id: {id}").expect("id");
            for column in 1..COLUMNS {
                writeln!(source, "    {}: {}", field_name(column), id + column).expect("field");
            }
        }
        fs::write(
            root.join(format!("sources/data/items-{file_index:02}.yaml")),
            source,
        )
        .expect("data source");
    }
}

fn config_source() -> &'static str {
    "[project]\nid = \"desktop-v1-performance\"\nname = \"Desktop v1 performance\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n"
}

fn schema_source() -> String {
    let mut source = String::from("kind: schema\ntable: items\nfields:\n");
    for column in 0..COLUMNS {
        writeln!(source, "  - key: {column}").expect("key");
        writeln!(source, "    name: {}", field_name(column)).expect("name");
        writeln!(source, "    type: int").expect("type");
    }
    source.push_str("primaryKey:\n  fields: [id]\nsecondaryKeys: []\n");
    source
}

fn field_name(column: usize) -> String {
    if column == 0 {
        "id".to_owned()
    } else {
        format!("field{column:02}")
    }
}
