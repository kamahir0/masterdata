use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{
    AddFieldCommand, FieldDefinition, MigrationCommand, MigrationCommitFailureInjection,
    MigrationCommitState, MigrationFileCommitState, Project, ProjectDocuments, ProjectService,
    dry_run_migration,
};
use serde_yaml::Value;
use tempfile::{TempDir, tempdir};

const SCHEMA_SOURCE: &str = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: int
primaryKey:
  fields: [id]
"#;

const DATA_SOURCE: &str = r#"kind: data
table: item
records:
  - id: 1
"#;

const UNRELATED_SOURCE: &str = r#"kind: data
table: other
records: []
"#;

struct PreparedMigration {
    directory: TempDir,
    project: Project,
    snapshot: ProjectDocuments,
    dry_run: masterdata_core::MigrationDryRun,
    schema_path: PathBuf,
    data_path: PathBuf,
}

fn add_field_command() -> MigrationCommand {
    MigrationCommand::AddField(AddFieldCommand {
        table: "item".to_owned(),
        field: FieldDefinition {
            key: 1,
            name: "label".to_owned(),
            type_name: "string".to_owned(),
            nullable: false,
            array: false,
        },
        initializer: Some(Value::String("Potion".to_owned())),
    })
}

fn prepared_migration(include_unrelated: bool) -> PreparedMigration {
    let directory = tempdir().expect("temporary project directory");
    let source_root = directory.path().join("sources");
    fs::create_dir_all(&source_root).expect("source root");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"migration.test\"\nname = \"Migration\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n",
    )
    .expect("project configuration");

    let schema_path = source_root.join("item-schema.yaml");
    let data_path = source_root.join("item-data.yaml");
    fs::write(&schema_path, SCHEMA_SOURCE).expect("schema source");
    fs::write(&data_path, DATA_SOURCE).expect("data source");
    if include_unrelated {
        fs::write(source_root.join("other-data.yaml"), UNRELATED_SOURCE).expect("unrelated source");
    }

    let project = Project::discover(Some(directory.path()), directory.path()).expect("project");
    let snapshot = project.load_documents().expect("source snapshot");
    let dry_run = dry_run_migration(&snapshot, &add_field_command()).expect("migration dry-run");
    PreparedMigration {
        directory,
        project,
        snapshot,
        dry_run,
        schema_path,
        data_path,
    }
}

fn source_bytes(snapshot: &ProjectDocuments, path: &Path) -> Vec<u8> {
    snapshot
        .files
        .iter()
        .find(|loaded| loaded.path == path)
        .expect("source in snapshot")
        .source
        .as_bytes()
        .to_vec()
}

fn assert_report_state(
    report: &masterdata_core::MigrationCommitReport,
    state: MigrationCommitState,
) {
    assert_eq!(report.state, state);
}

#[test]
fn successful_multi_file_commit_leaves_complete_new_source_set() {
    let prepared = prepared_migration(false);

    let report =
        masterdata_core::commit_migration(&prepared.project, &prepared.snapshot, &prepared.dry_run)
            .expect("source commit");

    assert_report_state(&report, MigrationCommitState::Success);
    assert!(report.recovery_workspace.is_none());
    assert!(
        report
            .files
            .iter()
            .all(|file| file.state == MigrationFileCommitState::New)
    );
    assert_eq!(
        fs::read(&prepared.schema_path).expect("committed schema"),
        source_bytes(
            &prepared.dry_run.transformed_documents,
            &prepared.schema_path
        )
    );
    assert_eq!(
        fs::read(&prepared.data_path).expect("committed data"),
        source_bytes(&prepared.dry_run.transformed_documents, &prepared.data_path)
    );
    assert!(!prepared.directory.path().join(".masterdata").exists());
}

#[test]
fn stale_source_snapshot_rejects_without_mutation() {
    let prepared = prepared_migration(false);
    let original_schema = fs::read(&prepared.schema_path).expect("original schema");
    let changed_data = b"kind: data\ntable: item\nrecords:\n  - id: 2\n";
    fs::write(&prepared.data_path, changed_data).expect("concurrent source edit");

    let failure =
        masterdata_core::commit_migration(&prepared.project, &prepared.snapshot, &prepared.dry_run)
            .expect_err("stale source must be rejected");

    assert_report_state(&failure.report, MigrationCommitState::NotStarted);
    assert_eq!(failure.error.diagnostic().code, "E-MIGRATION-PATCH-INVALID");
    assert_eq!(
        fs::read(&prepared.schema_path).expect("schema unchanged"),
        original_schema
    );
    assert_eq!(
        fs::read(&prepared.data_path).expect("edited data"),
        changed_data
    );
}

#[test]
fn stale_project_config_rejects_without_mutation() {
    let prepared = prepared_migration(false);
    let original_schema = fs::read(&prepared.schema_path).expect("original schema");
    let config_path = prepared.project.config_path().to_owned();
    let mut changed_config = fs::read(&config_path).expect("configuration");
    changed_config.extend_from_slice(b"\n# concurrent formatting change\n");
    fs::write(&config_path, changed_config).expect("concurrent configuration edit");

    let failure =
        masterdata_core::commit_migration(&prepared.project, &prepared.snapshot, &prepared.dry_run)
            .expect_err("stale project configuration must be rejected");

    assert_report_state(&failure.report, MigrationCommitState::NotStarted);
    assert_eq!(failure.error.diagnostic().code, "E-MIGRATION-PATCH-INVALID");
    assert_eq!(
        fs::read(&prepared.schema_path).expect("schema unchanged"),
        original_schema
    );
}

#[test]
fn stale_source_membership_rejects_without_mutation() {
    let prepared = prepared_migration(false);
    let original_schema = fs::read(&prepared.schema_path).expect("original schema");
    fs::write(
        prepared.project.root().join("sources/extra-data.yaml"),
        UNRELATED_SOURCE,
    )
    .expect("concurrent source addition");

    let failure =
        masterdata_core::commit_migration(&prepared.project, &prepared.snapshot, &prepared.dry_run)
            .expect_err("source membership change must be rejected");

    assert_report_state(&failure.report, MigrationCommitState::NotStarted);
    assert_eq!(failure.error.diagnostic().code, "E-MIGRATION-PATCH-INVALID");
    assert_eq!(
        fs::read(&prepared.schema_path).expect("schema unchanged"),
        original_schema
    );
}

#[test]
fn unrelated_source_change_does_not_block_commit() {
    let prepared = prepared_migration(true);
    let unrelated_path = prepared.project.root().join("sources/other-data.yaml");
    let changed_unrelated = b"kind: data\ntable: other\nrecords:\n  - id: 99\n";
    fs::write(&unrelated_path, changed_unrelated).expect("unrelated source edit");

    assert_eq!(
        prepared.dry_run.plan.source_inputs,
        vec![prepared.data_path.clone(), prepared.schema_path.clone()]
    );
    let report =
        masterdata_core::commit_migration(&prepared.project, &prepared.snapshot, &prepared.dry_run)
            .expect("unrelated source changes are outside the closure");

    assert_report_state(&report, MigrationCommitState::Success);
    assert_eq!(
        fs::read(unrelated_path).expect("unrelated source"),
        changed_unrelated
    );
    assert_eq!(
        fs::read(&prepared.schema_path).expect("committed schema"),
        source_bytes(
            &prepared.dry_run.transformed_documents,
            &prepared.schema_path
        )
    );
}

#[test]
fn commit_failure_rolls_back_complete_old_source_set() {
    let prepared = prepared_migration(false);
    let original_schema = fs::read(&prepared.schema_path).expect("original schema");
    let original_data = fs::read(&prepared.data_path).expect("original data");

    let failure = masterdata_core::commit_migration_with_failures(
        &prepared.project,
        &prepared.snapshot,
        &prepared.dry_run,
        &[MigrationCommitFailureInjection::write_file(1)],
    )
    .expect_err("injected commit failure");

    assert_report_state(&failure.report, MigrationCommitState::RolledBack);
    assert_eq!(failure.error.diagnostic().code, "E-IO-ACCESS");
    assert!(failure.report.recovery_workspace.is_none());
    assert_eq!(
        fs::read(&prepared.schema_path).expect("rolled-back schema"),
        original_schema
    );
    assert_eq!(
        fs::read(&prepared.data_path).expect("rolled-back data"),
        original_data
    );
    assert!(
        failure
            .report
            .files
            .iter()
            .any(|file| file.state == MigrationFileCommitState::Old)
    );
    assert!(
        failure
            .report
            .files
            .iter()
            .any(|file| file.state == MigrationFileCommitState::Unchanged)
    );
}

#[test]
fn rollback_failure_reports_recovery_required_and_retains_recovery_workspace() {
    let prepared = prepared_migration(false);
    let original_schema = fs::read(&prepared.schema_path).expect("original schema");
    let transformed_data =
        source_bytes(&prepared.dry_run.transformed_documents, &prepared.data_path);

    let failure = masterdata_core::commit_migration_with_failures(
        &prepared.project,
        &prepared.snapshot,
        &prepared.dry_run,
        &[
            MigrationCommitFailureInjection::write_file(1),
            MigrationCommitFailureInjection::rollback_file(0),
        ],
    )
    .expect_err("injected rollback failure");

    assert_report_state(&failure.report, MigrationCommitState::RecoveryRequired);
    assert_eq!(failure.error.diagnostic().code, "E-IO-ACCESS");
    let recovery_workspace = failure
        .report
        .recovery_workspace
        .as_ref()
        .expect("recovery workspace");
    assert!(recovery_workspace.join("journal.txt").is_file());
    assert!(recovery_workspace.join("old").is_dir());
    assert!(recovery_workspace.join("new").is_dir());
    assert_eq!(
        fs::read(&prepared.data_path).expect("new data retained"),
        transformed_data
    );
    assert_eq!(
        fs::read(&prepared.schema_path).expect("unrestored schema"),
        original_schema
    );
    assert!(
        failure
            .report
            .files
            .iter()
            .any(|file| file.state == MigrationFileCommitState::New)
    );
    assert!(
        failure
            .report
            .files
            .iter()
            .any(|file| file.state == MigrationFileCommitState::Unchanged)
    );

    fs::remove_dir_all(recovery_workspace).expect("cleanup retained recovery workspace");
}

#[test]
fn application_service_exposes_frontend_independent_commit_path() {
    let prepared = prepared_migration(false);
    let service = ProjectService::new();

    let report = service
        .commit_migration_snapshot(&prepared.project, &prepared.snapshot, &prepared.dry_run)
        .expect("application source commit");

    assert_eq!(report.state, MigrationCommitState::Success);
}
