//! Reproducible navigation input; Desktop uses the same generated files.
use masterdata_app::{NativeApplicationService, TableAuthoringSession};
use masterdata_core::read_trace::{measure_read, read_span};
use std::{fmt::Write as _, fs, path::Path, time::Instant};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| arg == "--write-fixture") {
        write_fixture(Path::new(args.get(2).expect("destination")));
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write_fixture(root);
    let service = NativeApplicationService::new();
    sample("A-cold-project", || {
        service.authoring_workspace(Some(root), root).unwrap()
    });
    let session = TableAuthoringSession::default();
    let navigate = |path: &str, data: bool| {
        if data {
            let snapshot = service.open_data_file(Some(root), root, path).unwrap();
            let _span = read_span("serialization");
            serde_json::to_vec(&snapshot).unwrap();
        }
        let context = session.open_context(root, path).unwrap();
        let _span = read_span("serialization");
        serde_json::to_vec(&context).unwrap();
    };
    sample("B-first-source", || navigate("sources/a-1.yaml", true));
    sample("C-revisit-frontend-buffer", || {
        navigate("sources/a-1.yaml", false)
    });
    sample("D-same-table-record-source", || {
        navigate("sources/a-2.yaml", true)
    });
    sample("E-cross-table", || navigate("sources/b-schema.yaml", true));
    sample("F-schema-redirect", || {
        session.open_context(root, "sources/c-schema.yaml").unwrap();
        navigate("sources/c-1.yaml", true);
    });
    sample("G-rapid-four", || {
        for path in [
            "sources/a-1.yaml",
            "sources/a-2.yaml",
            "sources/b-schema.yaml",
            "sources/c-1.yaml",
        ] {
            navigate(path, true);
        }
    });
}
fn sample<T>(label: &str, operation: impl FnOnce() -> T) {
    let started = Instant::now();
    let (_, metrics) = measure_read(operation);
    println!(
        "{}",
        serde_json::json!({"label":label,"wallMs":started.elapsed().as_secs_f64()*1000.0,"metrics":metrics})
    );
}
fn write_fixture(root: &Path) {
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::write(root.join("masterdata.toml"), "[project]\nid = \"navigation-evidence\"\nname = \"Navigation Evidence\"\nversion = \"0.1.0\"\n[sources]\nroots = [\"sources\"]\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n").unwrap();
    for table in ["a", "b", "c"] {
        let mut schema = format!("kind: schema\ntable: {table}\nfields:\n");
        for col in 0..20 {
            writeln!(
                schema,
                "  - key: {col}\n    name: {}\n    type: int",
                field(col)
            )
            .unwrap();
        }
        schema.push_str("primaryKey:\n  fields: [id]\nsecondaryKeys: []\n");
        if table == "b" {
            schema.push_str(&records(0));
        }
        fs::write(root.join(format!("sources/{table}-schema.yaml")), schema).unwrap();
        for index in 1..=2 {
            if table == "b" && index == 1 {
                continue;
            }
            fs::write(
                root.join(format!("sources/{table}-{index}.yaml")),
                format!("kind: data\ntable: {table}\n{}", records(index * 2000)),
            )
            .unwrap();
        }
    }
}
fn records(offset: usize) -> String {
    let mut source = String::from("records:\n");
    for row in 0..2000 {
        writeln!(source, "  - id: {}", offset + row).unwrap();
        for col in 1..20 {
            writeln!(source, "    {}: {}", field(col), offset + row + col).unwrap();
        }
    }
    source
}
fn field(col: usize) -> String {
    if col == 0 {
        "id".into()
    } else {
        format!("field{col:02}")
    }
}
