use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use masterdata_app::write_artifact_set_receipt;
use tempfile::{Builder, TempDir};

#[test]
fn cli_007_build_help_exposes_publish_without_short_alias() {
    let output = run_cli(
        &std::env::current_dir().expect("current directory"),
        &["build", "--help"],
    );
    let help = output_text(&output);

    assert!(output.status.success(), "{help}");
    assert!(help.contains("--publish"));
    assert!(!help.contains("-p,"));
}

#[test]
fn cli_007_build_failure_does_not_start_publish() {
    let project = project_with_invalid_source();

    let output = run_cli(project.path(), &["build", "--publish"]);

    assert!(!output.status.success());
    assert!(!project.path().join("published").exists());
    assert!(!output_text(&output).contains("publish:"));
}

#[test]
fn cli_007_build_publish_publishes_after_successful_build() {
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping build --publish integration test");
        return;
    }

    let project = copy_full_fixture("build publish success project");
    append_publish_target(project.path(), "csharp", "published");

    let output = run_cli(project.path(), &["build", "--publish"]);
    let text = output_text(&output);

    assert!(output.status.success(), "{text}");
    assert!(text.contains("wrote "));
    assert!(text.contains("built canonical MasterMemory binary:"));
    assert!(text.contains("publish: succeeded"));
    assert!(text.contains("[0] csharp published succeeded"));
    assert!(project.path().join("published/ItemMaster.g.cs").is_file());
    assert!(
        project
            .path()
            .join("published/.masterdata-publish-manifest.json")
            .is_file()
    );
}

#[test]
fn cli_007_build_publish_failure_keeps_successful_canonical_build() {
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping build --publish failure integration test");
        return;
    }

    let project = copy_full_fixture("build publish failure project");
    append_publish_target(project.path(), "csharp", "published");
    append_publish_target(project.path(), "csharp", "published/nested");

    let output = run_cli(project.path(), &["build", "--publish"]);
    let text = output_text(&output);
    let artifact_root = project.path().join(".masterdata/output");

    assert!(!output.status.success(), "{text}");
    assert!(text.contains("built canonical MasterMemory binary:"));
    assert!(text.contains("publish: failed"));
    assert!(text.contains("[0] csharp published not attempted"));
    assert!(artifact_root.join("csharp/ItemMaster.g.cs").is_file());
    assert!(artifact_root.join("masterdata.bytes").is_file());
    assert!(
        artifact_root
            .join(".masterdata-artifact-set.json")
            .is_file()
    );
    assert!(!project.path().join("published").exists());
}

#[test]
fn cli_007_build_without_publish_leaves_external_target_untouched() {
    if !dotnet_available() {
        eprintln!(".NET SDK is unavailable; skipping build-only publish isolation test");
        return;
    }

    let project = copy_full_fixture("build only project");
    append_publish_target(project.path(), "csharp", "published");

    let output = run_cli(project.path(), &["build"]);

    assert!(output.status.success(), "{}", output_text(&output));
    assert!(!project.path().join("published").exists());
}

#[test]
fn cli_007_dry_run_cannot_be_combined_with_publish() {
    let output = run_cli(
        &std::env::current_dir().expect("current directory"),
        &["build", "--dry-run", "--publish"],
    );

    assert!(!output.status.success());
    assert!(output_text(&output).contains("cannot be used with"));
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

fn project_with_invalid_source() -> TempDir {
    let project = Builder::new()
        .prefix("cli-build-publish-failure-")
        .tempdir()
        .expect("project directory");
    let project_id = "cli.build.publish.failure";
    fs::create_dir_all(project.path().join("sources")).expect("sources");
    fs::write(
        project.path().join("masterdata.toml"),
        project_config(project_id, &[("csharp", "published")]),
    )
    .expect("project config");
    fs::write(
        project.path().join("sources/current.yaml"),
        b"this is intentionally invalid masterdata YAML",
    )
    .expect("invalid source YAML");
    let artifact_root = project.path().join(".masterdata/output");
    fs::create_dir_all(artifact_root.join("csharp")).expect("C# artifact root");
    fs::write(
        artifact_root.join("csharp/Item.g.cs"),
        b"previous generated",
    )
    .expect("previous C# artifact");
    fs::write(artifact_root.join("masterdata.bytes"), b"previous binary")
        .expect("previous binary artifact");
    write_artifact_set_receipt(&artifact_root, project_id).expect("previous receipt");
    project
}

fn copy_full_fixture(label: &str) -> TempDir {
    let directory = Builder::new()
        .prefix(&format!("{label}-"))
        .tempdir()
        .expect("project directory");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("full");
    copy_directory(&fixture, directory.path());
    directory
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination");
    for entry in fs::read_dir(source).expect("fixture directory") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("fixture file");
        }
    }
}

fn append_publish_target(project: &Path, kind: &str, path: &str) {
    let config_path = project.join("masterdata.toml");
    let mut config = fs::read_to_string(&config_path).expect("project config");
    config.push_str(&format!(
        "\n[[publish.targets]]\nkind = {kind:?}\npath = {}\n",
        toml_string(path)
    ));
    fs::write(config_path, config).expect("updated project config");
}

fn project_config(project_id: &str, targets: &[(&str, &str)]) -> String {
    let mut config = format!(
        "[project]\nid = {project_id:?}\nname = \"CLI Build Publish\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n"
    );
    for (kind, path) in targets {
        config.push_str(&format!(
            "\n[[publish.targets]]\nkind = {kind:?}\npath = {}\n",
            toml_string(path)
        ));
    }
    config
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("TOML string")
}

fn dotnet_available() -> bool {
    let executable = std::env::var_os("MASTERDATA_DOTNET").unwrap_or_else(|| "dotnet".into());
    Command::new(executable)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}
