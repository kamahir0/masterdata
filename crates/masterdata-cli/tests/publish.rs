use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use masterdata_app::{ARTIFACT_SET_RECEIPT_FILENAME, write_artifact_set_receipt};
use tempfile::{Builder, TempDir};

#[test]
fn publish_command_is_exposed() {
    let output = run_cli(
        &std::env::current_dir().expect("current directory"),
        &["--help"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("publish"));
    assert!(!stdout.contains("generate"));
}

#[test]
fn publish_uses_receipt_valid_artifacts_without_loading_current_yaml() {
    let project = project_with_targets(&[("csharp", "published")]);
    fs::write(
        project.path().join("sources/current.yaml"),
        b"this is intentionally invalid masterdata YAML",
    )
    .expect("invalid source YAML");

    let project_path = project.path().to_str().expect("project path");
    let output = run_cli(project.path(), &["--project", project_path, "publish"]);

    assert!(output.status.success(), "{}", output_text(&output));
    assert!(output_text(&output).contains("publish: succeeded"));
    assert!(output_text(&output).contains("[0] csharp published succeeded"));
    assert_eq!(
        fs::read(project.path().join("published/Item.g.cs")).expect("published C#"),
        b"generated"
    );
}

#[test]
fn publish_missing_receipt_fails_without_target_mutation() {
    let project = project_with_targets(&[("csharp", "published")]);
    fs::remove_file(
        project
            .path()
            .join(".masterdata/output")
            .join(ARTIFACT_SET_RECEIPT_FILENAME),
    )
    .expect("receipt");

    let output = run_cli(project.path(), &["publish"]);

    assert!(!output.status.success());
    assert!(output_text(&output).contains("E-ARTIFACT-SET-RECEIPT-MISSING"));
    assert!(output_text(&output).contains("[0] csharp published not attempted"));
    assert!(!project.path().join("published").exists());
}

#[test]
fn publish_preflight_failure_mutates_no_targets() {
    let project = project_with_targets(&[("csharp", "first"), ("csharp", "first/nested")]);

    let output = run_cli(project.path(), &["publish"]);

    assert!(!output.status.success());
    assert!(output_text(&output).contains("E-PUBLISH-TARGET-COLLISION"));
    assert!(!project.path().join("first").exists());
    assert!(!project.path().join("first/nested").exists());
}

#[test]
fn publish_zero_targets_is_successful_noop() {
    let project = project_with_targets(&[]);

    let output = run_cli(project.path(), &["publish"]);

    assert!(output.status.success(), "{}", output_text(&output));
    assert!(output_text(&output).contains("publish: succeeded"));
    assert!(output_text(&output).contains("(no targets)"));
}

#[test]
fn publish_does_not_accept_unapproved_short_publish_flag() {
    let project = project_with_targets(&[]);

    let output = run_cli(project.path(), &["-p", "publish"]);

    assert!(!output.status.success());
    assert!(output_text(&output).contains("-p"));
    assert!(!project.path().join("published").exists());
}

fn run_cli(current_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_masterdata"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("run masterdata CLI")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn project_with_targets(targets: &[(&str, &str)]) -> TempDir {
    let project = Builder::new()
        .prefix("cli-publish-")
        .tempdir()
        .expect("project directory");
    fs::create_dir_all(project.path().join("sources")).expect("sources");

    let mut config = String::from(
        "[project]\nid = \"cli.publish\"\nname = \"CLI Publish\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
    );
    for (kind, path) in targets {
        config.push_str(&format!(
            "\n[[publish.targets]]\nkind = {kind:?}\npath = {}\n",
            toml_string(path)
        ));
    }
    fs::write(project.path().join("masterdata.toml"), config).expect("project config");

    let artifact_root = project.path().join(".masterdata/output");
    fs::create_dir_all(artifact_root.join("csharp")).expect("C# artifact root");
    fs::write(artifact_root.join("csharp/Item.g.cs"), b"generated").expect("C# artifact");
    fs::write(artifact_root.join("masterdata.bytes"), b"binary").expect("binary artifact");
    write_artifact_set_receipt(&artifact_root, "cli.publish").expect("receipt");
    project
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("TOML string")
}
