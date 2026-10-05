use masterdata_engine::{
    creation::Declaration,
    migration::{self, Command},
    native::{self, Outcome, SourceSetPlan},
    project::Project,
    source::Value,
};
use std::{collections::BTreeMap, fs, path::PathBuf};
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
fn add() -> Command {
    Command::AddField {
        table: "item".into(),
        declaration: Declaration {
            key: "2".into(),
            name: "rank".into(),
            type_name: "int".into(),
            nullable: false,
            array: false,
        },
        initializer: Some(Value::Literal("7".into())),
        position: None,
    }
}
fn plan(temp: &tempfile::TempDir, command: Command) -> SourceSetPlan {
    let project = Project::open(temp.path()).unwrap();
    SourceSetPlan::prepare(&project, migration::derive(&project, command).unwrap()).unwrap()
}
fn disk(temp: &tempfile::TempDir) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(temp.path().join("sources"))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
#[test]
fn native_source_set_commit_matches_independent_bytes_and_requires_destructive_authorization() {
    for id in ["migration-add", "migration-rename", "migration-drop"] {
        let temp = fixture(id);
        let command = match id {
            "migration-add" => add(),
            "migration-rename" => Command::RenameField {
                table: "item".into(),
                field: "id".into(),
                new_name: "itemId".into(),
            },
            _ => Command::DropField {
                table: "item".into(),
                field: "note".into(),
            },
        };
        let before = disk(&temp);
        let plan = plan(&temp, command);
        assert_eq!(disk(&temp), before);
        assert!(!temp.path().join(".masterdata").exists());
        if id == "migration-drop" {
            assert_eq!(
                plan.commit(false, native::SetFault::None).unwrap_err().code,
                "E-MIGRATION-AUTHORIZATION"
            );
            assert_eq!(disk(&temp), before);
        }
        let result = plan
            .commit(id == "migration-drop", native::SetFault::None)
            .unwrap();
        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(result.snapshots.len(), 3);
        for entry in fs::read_dir(oracle().join(id).join("expected")).unwrap() {
            let entry = entry.unwrap();
            assert_eq!(
                fs::read(temp.path().join("sources").join(entry.file_name())).unwrap(),
                fs::read(entry.path()).unwrap()
            );
        }
        assert!(native::pending_recovery(temp.path()).unwrap().is_empty());
        assert!(plan.commit(false, native::SetFault::None).is_err());
    }
}
#[test]
fn a_plan_rejects_changed_non_target_input_membership_config_and_same_byte_replacement() {
    for change in ["source", "membership", "config", "identity"] {
        let temp = fixture("migration-add");
        fs::write(
            temp.path().join("sources/other.yaml"),
            "kind: type\nname: Count\ncategory: valueObject\nunderlying: int\n",
        )
        .unwrap();
        let plan = plan(&temp, add());
        let path = temp.path().join("sources/other.yaml");
        match change {
            "source" => {
                fs::write(&path, fs::read_to_string(&path).unwrap() + "# changed\n").unwrap()
            }
            "membership" => {
                fs::copy(&path, temp.path().join("sources/new.yaml")).unwrap();
            }
            "config" => {
                let path = temp.path().join("masterdata.toml");
                fs::write(&path, fs::read_to_string(&path).unwrap() + "\n# changed\n").unwrap();
            }
            _ => {
                let bytes = fs::read(&path).unwrap();
                let replacement = path.with_file_name("replace.tmp");
                fs::write(&replacement, bytes).unwrap();
                fs::remove_file(&path).unwrap();
                fs::rename(replacement, &path).unwrap();
            }
        }
        let changed = disk(&temp);
        assert_eq!(
            plan.commit(false, native::SetFault::None).unwrap_err().code,
            "E-MIGRATION-STALE",
            "{change}"
        );
        assert_eq!(disk(&temp), changed);
        assert!(!temp.path().join(".masterdata").exists());
    }
}
#[cfg(feature = "oracle-faults")]
#[test]
fn failure_rolls_back_complete_old_and_unknown_is_never_retried() {
    for fault in [
        native::SetFault::CommitFailure {
            source: "sources/two.yaml".into(),
            rollback_failure: None,
        },
        native::SetFault::ObservationUnknown {
            source: "sources/schema.yaml".into(),
        },
    ] {
        let temp = fixture("migration-add");
        let before = disk(&temp);
        let result = plan(&temp, add()).commit(false, fault).unwrap();
        assert_eq!(result.outcome, Outcome::Failure);
        assert!(result.files.iter().all(|f| f.state == "OLD"));
        assert_eq!(disk(&temp), before);
        assert!(native::pending_recovery(temp.path()).unwrap().is_empty());
        assert!(
            result
                .files
                .last()
                .unwrap()
                .commit
                .as_ref()
                .unwrap()
                .outcome
                != Outcome::Success
        );
    }
}
#[cfg(feature = "oracle-faults")]
#[test]
fn independent_rollback_fault_retains_structured_evidence_and_never_repairs_implicitly() {
    let scenario: serde_json::Value =
        serde_json::from_slice(&fs::read(oracle().join("faults.json")).unwrap()).unwrap();
    let expected = &scenario["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "migration-recovery-required")
        .unwrap()["expected"];
    let temp = fixture("migration-add");
    let result = plan(&temp, add())
        .commit(
            false,
            native::SetFault::CommitFailure {
                source: "sources/two.yaml".into(),
                rollback_failure: Some("sources/one.yaml".into()),
            },
        )
        .unwrap();
    assert_eq!(result.outcome, Outcome::RecoveryRequired);
    for (filename, asset) in expected["disk"].as_object().unwrap() {
        assert_eq!(
            fs::read(temp.path().join("sources").join(filename)).unwrap(),
            fs::read(oracle().join(asset.as_str().unwrap())).unwrap()
        );
    }
    let mixed = disk(&temp);
    let info = result.recovery.unwrap();
    assert_eq!(
        info.files
            .iter()
            .map(|f| f.state.as_str())
            .collect::<Vec<_>>(),
        ["NEW", "OLD", "OLD"]
    );
    for file in &info.files {
        assert!(PathBuf::from(&file.old_copy).is_file());
        assert!(PathBuf::from(&file.new_copy).is_file());
    }
    assert!(native::has_pending_recovery(temp.path()).unwrap());
    assert!(native::recheck_recovery(temp.path(), &info.id).is_err());
    assert_eq!(disk(&temp), mixed);
    let project = Project::open(temp.path()).unwrap();
    let rename = migration::derive(
        &project,
        Command::RenameField {
            table: "item".into(),
            field: "note".into(),
            new_name: "memo".into(),
        },
    )
    .unwrap();
    assert_eq!(
        SourceSetPlan::prepare(&project, rename).unwrap_err().code,
        "E-RECOVERY-REQUIRED"
    );
}
#[cfg(feature = "oracle-faults")]
#[test]
fn rollback_never_overwrites_an_external_replacement_with_candidate_bytes() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let temp = fixture("migration-add");
    let plan = plan(&temp, add());
    let calls = AtomicUsize::new(0);
    let target = temp.path().join("sources/one.yaml");
    let result = plan
        .commit_authorized(false, native::SetFault::None, || {
            if calls.fetch_add(1, Ordering::SeqCst) == 3 {
                let replacement = target.with_file_name("external.tmp");
                fs::write(&replacement, fs::read(&target).unwrap()).unwrap();
                fs::remove_file(&target).unwrap();
                fs::rename(&replacement, &target).unwrap();
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(result.outcome, Outcome::RecoveryRequired);
    assert_eq!(result.files[0].state, "NEW");
    assert_eq!(
        result.files[0].rollback.as_ref().unwrap().outcome,
        Outcome::Conflict
    );
    let roots = [temp.path().join("sources").canonicalize().unwrap()];
    let actual = native::capture(temp.path(), &roots, "sources/one.yaml").unwrap();
    let external_id = actual.observed_identity();
    assert!(native::recheck_recovery(temp.path(), &result.recovery.unwrap().id).is_err());
    assert_eq!(
        native::capture(temp.path(), &roots, "sources/one.yaml")
            .unwrap()
            .observed_identity(),
        external_id
    );
}
