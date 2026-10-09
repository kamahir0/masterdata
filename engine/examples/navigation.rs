use masterdata_engine::{instrument, workspace::Workspace};
use serde_json::{Value, json};
use std::{env, path::Path, time::Instant};
fn main() {
    let args: Vec<_> = env::args().collect();
    let project = Path::new(&args[1]);
    let (started, open) = instrument::measure(|| Workspace::open(project).unwrap());
    let mut workspace = started;
    assert!(
        workspace.read.declaration_problems.is_empty(),
        "{:?}",
        workspace.read.declaration_problems
    );
    let now = Instant::now();
    let first = workspace.select("sources/a-1.yaml", 0, 40).unwrap();
    let first_wall = now.elapsed().as_secs_f64() * 1000.0;
    let cases = [
        ("revisit", "sources/a-1.yaml"),
        ("sameTable", "sources/a-2.yaml"),
        ("crossTable", "sources/b-schema.yaml"),
        ("schema", "sources/c-schema.yaml"),
    ];
    let mut samples: Vec<Value> = vec![];
    for run in 0..3 {
        for n in 0..100 {
            for (case, path) in cases {
                let projection = workspace.select(path, 0, 40).unwrap();
                let encode = Instant::now();
                let serialized = serde_json::to_string(&projection).unwrap();
                let serialization = encode.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(projection.total_rows, 2000);
                assert_eq!(projection.columns.len(), 20);
                assert!(projection.rows.len() <= 40);
                let work = &projection.measurement.work;
                assert_eq!(
                    work.project_discovery
                        + work.project_enumeration
                        + work.project_yaml_parse
                        + work.project_validation,
                    0
                );
                samples.push(json!({"case":case,"run":run,"sample":n,"backend":projection.measurement,"serializationMs":serialization,"wireBytes":serialized.len(),"projectedRows":projection.rows.len()}));
            }
        }
    }
    let p = workspace.select("sources/a-1.yaml", 0, 40).unwrap();
    let edit = instrument::measure(|| {
        workspace
            .edit_text(
                "sources/a-1.yaml",
                p.revision,
                &p.rows[0].id,
                "id",
                "999999",
            )
            .unwrap()
    })
    .1;
    for run in 0..3 {
        for n in 0..100 {
            workspace.select("sources/a-2.yaml", 0, 40).unwrap();
            let p = workspace.select("sources/a-1.yaml", 0, 40).unwrap();
            assert!(p.dirty);
            assert_eq!(p.rows[0].cells[0].display, "999999");
            samples
                .push(json!({"case":"dirtyRevisit","run":run,"sample":n,"backend":p.measurement}));
        }
    }
    let undo = instrument::measure(|| workspace.undo("sources/a-1.yaml", false).unwrap()).1;
    let mut edits = vec![];
    let mut undos = vec![];
    for _ in 0..100 {
        let p = workspace.select("sources/a-1.yaml", 0, 40).unwrap();
        edits.push(
            instrument::measure(|| {
                workspace
                    .edit_text(
                        "sources/a-1.yaml",
                        p.revision,
                        &p.rows[0].id,
                        "id",
                        "999998",
                    )
                    .unwrap()
            })
            .1,
        );
        undos.push(instrument::measure(|| workspace.undo("sources/a-1.yaml", false).unwrap()).1);
    }
    let report = json!({"format":1,"kind":"native engine; actual Desktopとは別evidence","open":open,"first":first.measurement,"firstWallMs":first_wall,"edit":edit,"undo":undo,"editSamples":edits,"undoSamples":undos,"samples":samples});
    if let Some(output) = args.get(2) {
        std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    } else {
        println!("{report}");
    }
}
