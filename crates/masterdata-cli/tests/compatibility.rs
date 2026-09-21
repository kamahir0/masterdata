use std::fs;
use std::process::Command;

use masterdata_core::{InitOptions, initialize_project};

#[test]
fn compatibility_command_accepts_explicit_snapshots_and_emits_structured_json() {
    let baseline = tempfile::tempdir().expect("baseline");
    let current = tempfile::tempdir().expect("current");
    for (root, value) in [(baseline.path(), "before"), (current.path(), "after")] {
        initialize_project(
            root,
            &InitOptions {
                project_id: "cli.compatibility".into(),
                name: "CLI Compatibility".into(),
                version: "1.0.0".into(),
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

    let output = Command::new(env!("CARGO_BIN_EXE_masterdata"))
        .current_dir(current.path())
        .args([
            "compatibility",
            "--baseline",
            baseline.path().to_str().expect("baseline path"),
            "--current",
            current.path().to_str().expect("current path"),
            "--json",
        ])
        .output()
        .expect("run compatibility command");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("structured compatibility JSON");
    assert_eq!(report["summary"]["changeCount"], 1);
    assert_eq!(report["changes"][0]["kind"], "data_changed");
    assert_eq!(report["changes"][0]["artifactBinary"], "rebuild_required");
}
