use std::fs;

use masterdata_app::NativeApplicationService;
use masterdata_core::{CompatibilityChangeKind, InitOptions, initialize_project};

fn project(root: &std::path::Path, id: &str, value: &str) {
    initialize_project(
        root,
        &InitOptions {
            project_id: id.to_owned(),
            name: "Compatibility project".to_owned(),
            version: "1.0.0".to_owned(),
        },
    )
    .expect("initialize project");
    fs::write(
        root.join("sources/schema.yaml"),
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: value\n    type: string\nprimaryKey:\n  fields: [id]\n",
    )
    .expect("schema");
    fs::write(
        root.join("sources/data.yaml"),
        format!("kind: data\ntable: item\nrecords:\n  - id: 1\n    value: {value}\n"),
    )
    .expect("data");
}

#[test]
fn application_operation_compares_explicit_projects_and_is_read_only() {
    let baseline = tempfile::tempdir().expect("baseline directory");
    let current = tempfile::tempdir().expect("current directory");
    project(baseline.path(), "compat.application", "before");
    project(current.path(), "compat.application", "after");

    let before_schema = fs::read(current.path().join("sources/schema.yaml")).expect("schema");
    let before_config = fs::read(current.path().join("masterdata.toml")).expect("config");
    let before_data = fs::read(current.path().join("sources/data.yaml")).expect("data");
    let report = NativeApplicationService::new()
        .analyze_compatibility(baseline.path(), current.path(), current.path())
        .expect("compatibility report");

    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::DataChanged)
    );
    assert_eq!(
        fs::read(current.path().join("sources/schema.yaml")).expect("schema"),
        before_schema
    );
    assert_eq!(
        fs::read(current.path().join("masterdata.toml")).expect("config"),
        before_config
    );
    assert_eq!(
        fs::read(current.path().join("sources/data.yaml")).expect("data"),
        before_data
    );
}

#[test]
fn application_operation_keeps_project_mismatch_as_input_error() {
    let baseline = tempfile::tempdir().expect("baseline directory");
    let current = tempfile::tempdir().expect("current directory");
    project(baseline.path(), "compat.baseline", "before");
    project(current.path(), "compat.current", "after");

    let error = NativeApplicationService::new()
        .analyze_compatibility(baseline.path(), current.path(), current.path())
        .expect_err("project mismatch");
    assert_eq!(error.diagnostic().code, "E-COMPAT-PROJECT-MISMATCH");
}
