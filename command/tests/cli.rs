//! Public process boundaries: exit status, diagnostics, exact bytes and filesystem
//! effects. No command-parser or private engine topology assertions.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn invoke(root: &Path, args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_masterdata"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn result(root: &Path, args: &[&OsStr], passed: bool) -> Value {
    let output = invoke(root, args);
    assert_eq!(
        output.status.success(),
        passed,
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn oracle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1")
}
fn fixture(id: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(oracle().join(id).join("input")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    fs::copy(
        oracle().join("save-both/input/masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    temp
}
fn source_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(root.join("sources"))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name().into(), fs::read(entry.path()).unwrap())
        })
        .collect()
}
#[test]
fn init_scaffolds_without_tool_state_and_preserves_existing_gitignore_and_marker() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("new authoring project");
    let created = result(
        temp.path(),
        &[
            "init".as_ref(),
            "--project".as_ref(),
            root.as_os_str(),
            "--id".as_ref(),
            "game.data".as_ref(),
            "--name".as_ref(),
            "Game \"Data\"".as_ref(),
            "--json".as_ref(),
        ],
        true,
    );
    assert_eq!(created["outcome"], "Success");
    for directory in ["sources/schemas", "sources/types", "sources/data"] {
        assert!(root.join(directory).is_dir());
    }
    assert_eq!(
        fs::read(root.join(".gitignore")).unwrap(),
        b"/.masterdata/\n"
    );
    assert!(!root.join(".masterdata").exists());
    let config = fs::read(root.join("masterdata.toml")).unwrap();
    let loaded = masterdata_engine::project::Project::open(&root).unwrap();
    assert_eq!(loaded.config.project.name, "Game \"Data\"");
    assert_eq!(loaded.config.build.artifact_dir, ".masterdata/output");
    assert_eq!(loaded.config.build.cache, ".masterdata/cache");
    assert!(loaded.config.publish.targets.is_empty());
    let duplicate = result(
        temp.path(),
        &["init".as_ref(), "--project".as_ref(), root.as_os_str()],
        false,
    );
    assert_eq!(duplicate["diagnostic"]["code"], "E-INIT-EXISTS");
    assert_eq!(fs::read(root.join("masterdata.toml")).unwrap(), config);
    let existing = temp.path().join("existing");
    fs::create_dir(&existing).unwrap();
    let gitignore = b"# user policy\r\ncustom/**\r\n";
    fs::write(existing.join(".gitignore"), gitignore).unwrap();
    let created = result(
        temp.path(),
        &[
            "init".as_ref(),
            "--project".as_ref(),
            existing.join("masterdata.toml").as_os_str(),
        ],
        true,
    );
    assert_eq!(created["keptGitignore"], true);
    assert_eq!(fs::read(existing.join(".gitignore")).unwrap(), gitignore);
    assert!(!existing.join(".masterdata").exists());
}
#[test]
fn init_preflights_scaffold_collisions_and_invalid_metadata_before_any_mutation() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("sources"), b"user file").unwrap();
    let denied = result(temp.path(), &["init".as_ref()], false);
    assert_eq!(denied["diagnostic"]["code"], "E-INIT-PATH");
    assert_eq!(fs::read(temp.path().join("sources")).unwrap(), b"user file");
    assert!(!temp.path().join("masterdata.toml").exists());
    assert!(!temp.path().join(".gitignore").exists());
    let missing = temp.path().join("missing");
    result(
        temp.path(),
        &[
            "init".as_ref(),
            "--project".as_ref(),
            missing.as_os_str(),
            "--id".as_ref(),
            "   ".as_ref(),
        ],
        false,
    );
    assert!(!missing.exists());
}
#[test]
fn validate_discovers_ancestors_and_explicit_config_and_never_materializes_artifacts() {
    let temp = fixture("migration-add");
    let before = source_bytes(temp.path());
    let nested = temp.path().join("sources/nested");
    fs::create_dir(&nested).unwrap();
    let validated = result(&nested, &["validate".as_ref(), "--json".as_ref()], true);
    assert_eq!(validated["projectId"], "rewrite.oracle");
    result(
        &nested,
        &[
            "validate".as_ref(),
            "--project".as_ref(),
            temp.path().join("masterdata.toml").as_os_str(),
        ],
        true,
    );
    result(
        &nested,
        &[
            "validate".as_ref(),
            "--profile".as_ref(),
            "missing".as_ref(),
        ],
        false,
    );
    fs::remove_dir(&nested).unwrap();
    assert_eq!(source_bytes(temp.path()), before);
    assert!(!temp.path().join(".masterdata").exists());
    let outside = tempfile::tempdir().unwrap();
    let missing = result(outside.path(), &["validate".as_ref()], false);
    assert_eq!(missing["diagnostic"]["code"], "E-PROJECT-NOT-FOUND");
}
#[test]
fn invalid_validation_is_structured_and_does_not_modify_sources_or_outputs() {
    let temp = fixture("migration-add");
    let path = temp.path().join("sources/unknown.yaml");
    fs::write(path, b"kind: [invalid").unwrap();
    let before = source_bytes(temp.path());
    let invalid = result(temp.path(), &["validate".as_ref()], false);
    assert!(
        invalid["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E-YAML-PARSE")
    );
    assert_eq!(source_bytes(temp.path()), before);
    assert!(!temp.path().join(".masterdata").exists());
}
#[test]
fn migration_plan_is_read_only_and_apply_matches_independent_exact_bytes() {
    for (id, command, destructive) in [
        (
            "migration-add",
            json!({"operation":"addField","table":"item","declaration":{"key":"2","name":"rank","typeName":"int","nullable":false,"array":false},"initializer":{"kind":"literal","value":"7"},"position":null}),
            false,
        ),
        (
            "migration-rename",
            json!({"operation":"renameField","table":"item","field":"id","newName":"itemId"}),
            false,
        ),
        (
            "migration-drop",
            json!({"operation":"dropField","table":"item","field":"note"}),
            true,
        ),
    ] {
        let temp = fixture(id);
        let before = source_bytes(temp.path());
        let input = temp.path().join("command.json");
        fs::write(&input, serde_json::to_vec(&command).unwrap()).unwrap();
        let planned = result(
            temp.path(),
            &["migrate".as_ref(), "--command".as_ref(), input.as_os_str()],
            true,
        );
        assert_eq!(planned["outcome"], "NotAttempted");
        assert_eq!(planned["plan"]["destructive"], destructive);
        assert!(!planned["plan"]["sources"].as_array().unwrap().is_empty());
        assert_eq!(source_bytes(temp.path()), before);
        assert!(!temp.path().join(".masterdata").exists());
        let dry = result(
            temp.path(),
            &[
                "migrate".as_ref(),
                "--command".as_ref(),
                input.as_os_str(),
                "--apply".as_ref(),
                "--allow-destructive".as_ref(),
                "--dry-run".as_ref(),
            ],
            true,
        );
        assert_eq!(dry["plan"], planned["plan"]);
        assert_eq!(source_bytes(temp.path()), before);
        if destructive {
            let denied = result(
                temp.path(),
                &[
                    "migrate".as_ref(),
                    "--command".as_ref(),
                    input.as_os_str(),
                    "--apply".as_ref(),
                ],
                false,
            );
            assert_eq!(denied["diagnostic"]["code"], "E-MIGRATION-AUTHORIZATION");
            assert_eq!(source_bytes(temp.path()), before);
        }
        let applied = result(
            temp.path(),
            &[
                "migrate".as_ref(),
                "--command".as_ref(),
                input.as_os_str(),
                "--apply".as_ref(),
                "--allow-destructive".as_ref(),
            ],
            true,
        );
        assert_eq!(applied["result"]["outcome"], "Success");
        for entry in fs::read_dir(oracle().join(id).join("expected")).unwrap() {
            let entry = entry.unwrap();
            assert_eq!(
                fs::read(temp.path().join("sources").join(entry.file_name())).unwrap(),
                fs::read(entry.path()).unwrap(),
                "{id}"
            );
        }
        assert!(!temp.path().join(".masterdata/output").exists());
    }
}
#[test]
fn doctor_reports_environment_failure_without_building_or_mutating() {
    let temp = fixture("migration-add");
    let before = source_bytes(temp.path());
    let output = Command::new(env!("CARGO_BIN_EXE_masterdata"))
        .current_dir(temp.path())
        .args(["doctor", "--json"])
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let doctor: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        doctor["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E-DOTNET-UNAVAILABLE")
    );
    assert_eq!(source_bytes(temp.path()), before);
    assert!(!temp.path().join(".masterdata").exists());
}
#[test]
fn unsupported_surface_and_ambiguous_flags_are_rejected_before_side_effects() {
    let temp = fixture("migration-add");
    let before = source_bytes(temp.path());
    for args in [
        vec!["generate"],
        vec!["project-info"],
        vec!["build", "-p"],
        vec!["build", "--publish", "--dry-run"],
        vec!["publish", "--profile", "release"],
        vec!["validate", "--publish"],
        vec!["init", "--project"],
        vec!["migrate", "--apply"],
        vec!["migrate", "--restore", "missing"],
        vec!["build", "--profile", "one", "--profile", "two"],
    ] {
        result(
            temp.path(),
            &args.iter().map(OsStr::new).collect::<Vec<_>>(),
            false,
        );
    }
    let help = invoke(temp.path(), &["--help".as_ref()]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    for command in ["init", "doctor", "validate", "build", "publish", "migrate"] {
        assert!(text.contains(command));
    }
    assert_eq!(source_bytes(temp.path()), before);
    assert!(!temp.path().join(".masterdata").exists());
}
