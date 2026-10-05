use masterdata_engine::{project::Project, workspace::Workspace};
use std::{collections::BTreeMap, fs, path::PathBuf};
#[test]
fn saved_problem_locates_the_exact_base_occurrence_after_dirty_reorder_delete_and_save() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/rewrite-oracle/v1/consumer/minimal");
    let temp = tempfile::tempdir().unwrap();
    fs::copy(
        fixture.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(fixture.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    let file = temp.path().join("sources/data.yaml");
    fs::write(
        &file,
        fs::read_to_string(&file)
            .unwrap()
            .replace("direct: 3002", "direct: broken"),
    )
    .unwrap();
    let saved = Project::open(temp.path()).unwrap();
    let diagnostics = saved.validate(None).unwrap().0;
    let problem = diagnostics
        .iter()
        .find(|d| d.source == "sources/data.yaml" && d.occurrence == Some(2))
        .unwrap();
    let inputs = saved
        .sources
        .iter()
        .map(|(path, source)| (path.clone(), source.identity.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut workspace = Workspace::open(temp.path()).unwrap();
    let p = workspace.select("sources/data.yaml", 0, 32).unwrap();
    let row = p.rows[1].id.clone();
    let order = p
        .rows
        .iter()
        .rev()
        .map(|r| r.id.clone())
        .collect::<Vec<_>>();
    workspace
        .reorder_rows("sources/data.yaml", p.revision, p.generation, &order)
        .unwrap();
    let (target, work) = masterdata_engine::instrument::measure(|| {
        workspace
            .saved_problem_target(problem, &saved.config_identity, &inputs)
            .unwrap()
            .unwrap()
    });
    assert_eq!(target.row, row);
    assert_eq!(target.view_index, Some(0));
    assert_eq!(work.work.project_yaml_parse, 0);
    assert_eq!(work.work.project_validation, 0);
    let p = workspace.select("sources/data.yaml", 0, 32).unwrap();
    workspace
        .delete_row("sources/data.yaml", p.revision, p.generation, &row)
        .unwrap();
    let pending = workspace
        .saved_problem_target(problem, &saved.config_identity, &inputs)
        .unwrap()
        .unwrap();
    assert_eq!(pending.row, row);
    assert!(pending.editor_path.is_none());
    assert_eq!(
        workspace
            .save_paths(
                vec!["sources/data.yaml".into()],
                masterdata_engine::native::Fault::None
            )
            .unwrap()[0]
            .outcome,
        masterdata_engine::native::Outcome::Success
    );
    assert_eq!(
        workspace
            .saved_problem_target(problem, &saved.config_identity, &inputs)
            .unwrap_err()
            .code,
        "E-BUILD-PROBLEM-STALE"
    );
    let new_saved = Project::open(temp.path()).unwrap();
    let inputs = new_saved
        .sources
        .iter()
        .map(|(path, source)| (path.clone(), source.identity.clone()))
        .collect::<BTreeMap<_, _>>();
    // A fresh saved ordinal still maps to the surviving ID, not to a hash-based
    // ID invented after Save. The row may now be the former second occurrence.
    let mut first = problem.clone();
    first.occurrence = Some(1);
    let target = workspace
        .saved_problem_target(&first, &new_saved.config_identity, &inputs)
        .unwrap()
        .unwrap();
    assert_eq!(target.row, p.rows[1].id);
}
