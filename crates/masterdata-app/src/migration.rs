use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use masterdata_core::{
    Diagnostic, ErrorKind, MasterdataError, MigrationCommand, MigrationDryRun, MigrationPlan,
    Project, ProjectDocuments, Result, dry_run_migration, parse_yaml_document,
};
use tempfile::{Builder, TempDir};

use crate::NativeApplicationService;

/// Native preparation for a verified in-memory Migration result plus the exact
/// filesystem inputs that must still match immediately before commit.
///
/// This is an internal Rust application boundary, not a wire/CLI result schema.
#[derive(Debug, Clone)]
pub struct NativeMigrationPreparation {
    pub dry_run: MigrationDryRun,
    base: MigrationBaseSnapshot,
}

impl NativeMigrationPreparation {
    pub fn plan(&self) -> &MigrationPlan {
        &self.dry_run.plan
    }
}

#[derive(Debug, Clone)]
struct MigrationBaseSnapshot {
    config_path: PathBuf,
    config_source: Vec<u8>,
    source_files: Vec<PathBuf>,
    sources: BTreeMap<PathBuf, Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCommitStatus {
    Success,
    StalePlan,
    RolledBack,
    RecoveryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCommittedFileState {
    Old,
    New,
    Missing,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationCommitFileReport {
    pub path: PathBuf,
    pub state: MigrationCommittedFileState,
    /// Internal recovery locations retained only when automatic rollback could
    /// not restore a usable source set. Their layout is not a public contract.
    pub recovery_artifacts: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationCommitReport {
    pub status: MigrationCommitStatus,
    pub files: Vec<MigrationCommitFileReport>,
}

#[derive(Debug, Clone)]
pub struct MigrationCommitFailure {
    pub report: MigrationCommitReport,
    pub error: MasterdataError,
}

impl MigrationCommitFailure {
    pub fn report(&self) -> &MigrationCommitReport {
        &self.report
    }

    pub fn diagnostic(&self) -> &Diagnostic {
        self.error.diagnostic()
    }

    pub fn into_error(self) -> MasterdataError {
        self.error
    }
}

impl fmt::Display for MigrationCommitFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for MigrationCommitFailure {}

/// Deterministic failure points for focused transaction regressions.
/// Production callers use [`NativeApplicationService::commit_migration`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCommitFailurePoint {
    BeforeInstall,
    RollbackRestore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationCommitFailureInjection {
    pub file_index: usize,
    pub point: MigrationCommitFailurePoint,
}

impl NativeApplicationService {
    /// Resolve a native project snapshot, run the pure Migration engine, and
    /// retain the exact config/file-set/source identities needed by MIGRATION-016.
    pub fn prepare_migration(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
    ) -> Result<NativeMigrationPreparation> {
        prepare_native_migration(explicit_project, current_dir, command)
    }

    /// Commit a previously verified Migration preparation to canonical YAML.
    ///
    /// No build, publish, generated C#, binary, or receipt update is started.
    pub fn commit_migration(
        &self,
        preparation: &NativeMigrationPreparation,
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        self.commit_migration_with_failures(preparation, &[])
    }

    /// Deterministic failure seam for transaction / rollback regressions.
    #[doc(hidden)]
    pub fn commit_migration_with_failures(
        &self,
        preparation: &NativeMigrationPreparation,
        injections: &[MigrationCommitFailureInjection],
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        execute_commit(preparation, injections)
    }
}

fn prepare_native_migration(
    explicit_project: Option<&Path>,
    current_dir: &Path,
    command: &MigrationCommand,
) -> Result<NativeMigrationPreparation> {
    // WHY: Migration planning must use one stable native snapshot while the
    // pure engine remains filesystem-independent.
    // IF REMOVED: config/source membership could be read from one revision and
    // source bytes from another before the MIGRATION-016 commit preflight.
    // EVIDENCE: docs/specs/schema-migration.md (MIGRATION-005, MIGRATION-012, MIGRATION-016);
    // docs/specs/runtime-hosts.md (RUNTIME-HOST-006, RUNTIME-HOST-013).
    // Regression: stale_config_is_rejected_without_source_mutation;
    // stale_source_membership_is_rejected_without_source_mutation;
    // stale_loaded_source_is_rejected_without_source_mutation.
    const ATTEMPTS: usize = 3;
    for _ in 0..ATTEMPTS {
        let discovered = Project::discover(explicit_project, current_dir)?;
        let config_path = discovered.config_path().to_path_buf();
        let config_before = read_bytes(&config_path)?;
        let project = Project::from_config_path(config_path.clone())?;
        let config_after = read_bytes(&config_path)?;
        if config_before != config_after {
            continue;
        }

        let source_files = project.source_files()?;
        let mut documents = ProjectDocuments::default();
        let mut sources = BTreeMap::new();
        let mut unstable = false;
        for path in &source_files {
            let source = fs::read_to_string(path).map_err(|error| native_io_error(path, error))?;
            let loaded = parse_yaml_document(path.clone(), &source)?;
            sources.insert(path.clone(), source.as_bytes().to_vec());
            documents.files.push(loaded);
        }

        if read_bytes(&config_path)? != config_after || project.source_files()? != source_files {
            continue;
        }
        for path in &source_files {
            if read_bytes(path)? != sources[path] {
                unstable = true;
                break;
            }
        }
        if unstable {
            continue;
        }

        let dry_run = dry_run_migration(&documents, command)?;
        return Ok(NativeMigrationPreparation {
            dry_run,
            base: MigrationBaseSnapshot {
                config_path,
                config_source: config_after,
                source_files,
                sources,
            },
        });
    }

    Err(MasterdataError::new(
        "E-MIGRATION-SNAPSHOT-UNSTABLE",
        ErrorKind::Validation,
        "native project inputs changed while the Migration snapshot was being acquired",
    )
    .with_related_requirement("MIGRATION-005")
    .with_related_requirement("MIGRATION-016"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetPhase {
    Staged,
    BackedUp,
    Installing,
    Installed,
}

struct CommitTarget {
    path: PathBuf,
    original: Vec<u8>,
    transformed: Vec<u8>,
    workspace: Option<TempDir>,
    stage_path: PathBuf,
    backup_path: PathBuf,
    failed_new_path: PathBuf,
    recovery_dir: Option<PathBuf>,
    phase: TargetPhase,
}

fn execute_commit(
    preparation: &NativeMigrationPreparation,
    injections: &[MigrationCommitFailureInjection],
) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
    let mut targets = match build_commit_targets(preparation) {
        Ok(targets) => targets,
        Err(error) => {
            return Err(MigrationCommitFailure {
                report: report_for(preparation, MigrationCommitStatus::RolledBack, &[]),
                error,
            });
        }
    };

    if let Err(error) = verify_preflight(preparation, &targets) {
        cleanup_workspaces(&mut targets);
        return Err(MigrationCommitFailure {
            report: report_for(preparation, MigrationCommitStatus::StalePlan, &targets),
            error,
        });
    }

    // WHY: The final stale check runs after staging but before the first
    // canonical source mutation, and each target is rechecked immediately
    // before it is moved aside. The moved backup is also byte-verified.
    // IF REMOVED: a plan could overwrite config/source/file-set changes made
    // after planning, or a TOCTOU update could be mistaken for the old base.
    // EVIDENCE: docs/specs/schema-migration.md (MIGRATION-013, MIGRATION-016).
    // Regression: stale_loaded_source_is_rejected_without_source_mutation.
    if let Err(error) = verify_preflight(preparation, &targets) {
        cleanup_workspaces(&mut targets);
        return Err(MigrationCommitFailure {
            report: report_for(preparation, MigrationCommitStatus::StalePlan, &targets),
            error,
        });
    }

    for index in 0..targets.len() {
        if let Err(error) = verify_expected_workspace(preparation, &targets) {
            return fail_after_mutation(
                preparation,
                &mut targets,
                error,
                true,
                injections,
            );
        }

        let path = targets[index].path.clone();
        if let Err(error) = fs::rename(&path, &targets[index].backup_path) {
            return fail_after_mutation(
                preparation,
                &mut targets,
                commit_io_error(&path, "could not secure the previous source before commit", error),
                false,
                injections,
            );
        }
        targets[index].phase = TargetPhase::BackedUp;

        let backup_metadata = match fs::symlink_metadata(&targets[index].backup_path) {
            Ok(metadata) if metadata.file_type().is_file() => metadata,
            Ok(_) => {
                let error = stale_error(
                    &path,
                    "source identity changed to a non-regular file immediately before commit",
                );
                return fail_after_mutation(
                    preparation,
                    &mut targets,
                    error,
                    true,
                    injections,
                );
            }
            Err(error) => {
                return fail_after_mutation(
                    preparation,
                    &mut targets,
                    commit_io_error(
                        &path,
                        "could not inspect secured source during commit",
                        error,
                    ),
                    false,
                    injections,
                );
            }
        };
        match fs::read(&targets[index].backup_path) {
            Ok(bytes) if bytes == targets[index].original => {}
            Ok(_) => {
                let error = stale_error(
                    &path,
                    "source bytes changed between the final preflight and commit",
                );
                return fail_after_mutation(
                    preparation,
                    &mut targets,
                    error,
                    true,
                    injections,
                );
            }
            Err(error) => {
                return fail_after_mutation(
                    preparation,
                    &mut targets,
                    commit_io_error(
                        &path,
                        "could not verify secured source during commit",
                        error,
                    ),
                    false,
                    injections,
                );
            }
        }

        if injected(injections, index, MigrationCommitFailurePoint::BeforeInstall) {
            return fail_after_mutation(
                preparation,
                &mut targets,
                injected_commit_error(&path, "before installing transformed source"),
                false,
                injections,
            );
        }

        let mut source = match File::open(&targets[index].stage_path) {
            Ok(file) => file,
            Err(error) => {
                return fail_after_mutation(
                    preparation,
                    &mut targets,
                    commit_io_error(&path, "could not open staged transformed source", error),
                    false,
                    injections,
                );
            }
        };
        let mut destination = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) => {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    return recovery_required_without_rollback(
                        preparation,
                        &mut targets,
                        stale_error(
                            &path,
                            "another source appeared after the old source was secured",
                        ),
                        "canonical source path was concurrently recreated",
                    );
                }
                return fail_after_mutation(
                    preparation,
                    &mut targets,
                    commit_io_error(&path, "could not create transformed source", error),
                    false,
                    injections,
                );
            }
        };
        targets[index].phase = TargetPhase::Installing;
        if let Err(error) = io::copy(&mut source, &mut destination) {
            drop(destination);
            return fail_after_mutation(
                preparation,
                &mut targets,
                commit_io_error(&path, "could not write transformed source", error),
                false,
                injections,
            );
        }
        if let Err(error) = destination.sync_all() {
            drop(destination);
            return fail_after_mutation(
                preparation,
                &mut targets,
                commit_io_error(&path, "could not sync transformed source", error),
                false,
                injections,
            );
        }
        drop(destination);
        if let Err(error) = fs::set_permissions(&path, backup_metadata.permissions()) {
            return fail_after_mutation(
                preparation,
                &mut targets,
                commit_io_error(&path, "could not preserve source permissions", error),
                false,
                injections,
            );
        }
        targets[index].phase = TargetPhase::Installed;
    }

    if let Err(error) = verify_expected_workspace(preparation, &targets) {
        return fail_after_mutation(preparation, &mut targets, error, true, injections);
    }

    let report = report_for(preparation, MigrationCommitStatus::Success, &targets);
    cleanup_workspaces(&mut targets);
    Ok(report)
}

fn build_commit_targets(preparation: &NativeMigrationPreparation) -> Result<Vec<CommitTarget>> {
    let affected = preparation
        .dry_run
        .plan
        .affected_files
        .iter()
        .map(|plan| plan.path.clone())
        .collect::<BTreeSet<_>>();

    let transformed = preparation
        .dry_run
        .transformed_documents
        .files
        .iter()
        .map(|loaded| (loaded.path.clone(), loaded.source.as_bytes().to_vec()))
        .collect::<BTreeMap<_, _>>();

    let mut targets = Vec::with_capacity(affected.len());
    for path in affected {
        let original = preparation.base.sources.get(&path).cloned().ok_or_else(|| {
            MasterdataError::new(
                "E-MIGRATION-COMMIT-PLAN",
                ErrorKind::Validation,
                format!("affected source `{}` is missing from the base snapshot", path.display()),
            )
            .with_source(path.clone())
            .with_related_requirement("MIGRATION-015")
        })?;
        let transformed = transformed.get(&path).cloned().ok_or_else(|| {
            MasterdataError::new(
                "E-MIGRATION-COMMIT-PLAN",
                ErrorKind::Validation,
                format!(
                    "affected source `{}` is missing from the transformed snapshot",
                    path.display()
                ),
            )
            .with_source(path.clone())
            .with_related_requirement("MIGRATION-015")
        })?;
        let parent = path.parent().ok_or_else(|| {
            MasterdataError::new(
                "E-MIGRATION-COMMIT-STAGING",
                ErrorKind::Io,
                "affected source has no parent directory",
            )
            .with_source(path.clone())
            .with_related_requirement("MIGRATION-010")
        })?;
        let workspace = Builder::new()
            .prefix(".masterdata-migration-")
            .tempdir_in(parent)
            .map_err(|error| {
                commit_io_error(&path, "could not create source commit staging area", error)
            })?;
        let stage_path = workspace.path().join("new");
        fs::write(&stage_path, &transformed).map_err(|error| {
            commit_io_error(&path, "could not stage transformed source", error)
        })?;
        let backup_path = workspace.path().join("old");
        let failed_new_path = workspace.path().join("failed-new");
        targets.push(CommitTarget {
            path,
            original,
            transformed,
            workspace: Some(workspace),
            stage_path,
            backup_path,
            failed_new_path,
            recovery_dir: None,
            phase: TargetPhase::Staged,
        });
    }
    Ok(targets)
}

fn verify_preflight(
    preparation: &NativeMigrationPreparation,
    targets: &[CommitTarget],
) -> Result<()> {
    verify_config_and_membership(preparation)?;
    for (path, expected) in &preparation.base.sources {
        let current = fs::read(path).map_err(|error| stale_access_error(path, error))?;
        if &current != expected {
            return Err(stale_error(
                path,
                "loaded canonical source differs from the Migration base snapshot",
            ));
        }
    }
    for target in targets {
        if !preparation.base.sources.contains_key(&target.path) {
            return Err(stale_error(
                &target.path,
                "affected source is not part of the Migration base snapshot",
            ));
        }
    }
    Ok(())
}

fn verify_expected_workspace(
    preparation: &NativeMigrationPreparation,
    targets: &[CommitTarget],
) -> Result<()> {
    verify_config_and_membership(preparation)?;
    let installed = targets
        .iter()
        .filter(|target| target.phase == TargetPhase::Installed)
        .map(|target| (target.path.clone(), target.transformed.as_slice()))
        .collect::<BTreeMap<_, _>>();
    for (path, original) in &preparation.base.sources {
        let expected = installed
            .get(path)
            .copied()
            .unwrap_or_else(|| original.as_slice());
        let current = fs::read(path).map_err(|error| stale_access_error(path, error))?;
        if current != expected {
            return Err(stale_error(
                path,
                "canonical source changed while the Migration commit was in progress",
            ));
        }
    }
    Ok(())
}

fn verify_config_and_membership(preparation: &NativeMigrationPreparation) -> Result<()> {
    let config = fs::read(&preparation.base.config_path)
        .map_err(|error| stale_access_error(&preparation.base.config_path, error))?;
    if config != preparation.base.config_source {
        return Err(stale_error(
            &preparation.base.config_path,
            "project configuration differs from the Migration base snapshot",
        ));
    }

    let project = Project::from_config_path(preparation.base.config_path.clone()).map_err(|error| {
        MasterdataError::new(
            "E-MIGRATION-STALE-PLAN",
            ErrorKind::Validation,
            format!("project configuration can no longer be resolved: {error}"),
        )
        .with_source(preparation.base.config_path.clone())
        .with_related_requirement("MIGRATION-016")
    })?;
    let current_files = project.source_files().map_err(|error| {
        MasterdataError::new(
            "E-MIGRATION-STALE-PLAN",
            ErrorKind::Validation,
            format!("migration-relevant source file set can no longer be resolved: {error}"),
        )
        .with_source(preparation.base.config_path.clone())
        .with_related_requirement("MIGRATION-016")
    })?;
    if current_files != preparation.base.source_files {
        return Err(stale_error(
            &preparation.base.config_path,
            "migration-relevant source file set membership changed after planning",
        ));
    }
    Ok(())
}

fn fail_after_mutation(
    preparation: &NativeMigrationPreparation,
    targets: &mut [CommitTarget],
    original_error: MasterdataError,
    stale: bool,
    injections: &[MigrationCommitFailureInjection],
) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
    // WHY: A normal commit failure is not allowed to expose a partial migrated
    // source set. Rollback proceeds in reverse deterministic order; the first
    // rollback failure transitions to Recovery Required and stops mutation.
    // IF REMOVED: multi-file AddField could be reported as success or continue
    // writing after the OLD/NEW boundary had been lost.
    // EVIDENCE: docs/specs/schema-migration.md (MIGRATION-010).
    // Regression: write_failure_rolls_back_complete_old_source_set;
    // rollback_failure_reports_recovery_required_and_stops.
    if let Err(rollback_error) = rollback_targets(targets, injections) {
        retain_workspaces(targets);
        let error = recovery_required_error(&original_error, &rollback_error);
        return Err(MigrationCommitFailure {
            report: report_for(
                preparation,
                MigrationCommitStatus::RecoveryRequired,
                targets,
            ),
            error,
        });
    }

    let status = if stale {
        MigrationCommitStatus::StalePlan
    } else {
        MigrationCommitStatus::RolledBack
    };
    let report = report_for(preparation, status, targets);
    cleanup_workspaces(targets);
    Err(MigrationCommitFailure {
        report,
        error: if stale {
            original_error
        } else {
            rolled_back_error(&original_error)
        },
    })
}

fn rollback_targets(
    targets: &mut [CommitTarget],
    injections: &[MigrationCommitFailureInjection],
) -> Result<()> {
    for index in (0..targets.len()).rev() {
        if targets[index].phase == TargetPhase::Staged {
            continue;
        }
        if injected(
            injections,
            index,
            MigrationCommitFailurePoint::RollbackRestore,
        ) {
            return Err(injected_commit_error(
                &targets[index].path,
                "while restoring the previous source",
            ));
        }

        if matches!(
            targets[index].phase,
            TargetPhase::Installing | TargetPhase::Installed
        ) {
            match fs::symlink_metadata(&targets[index].path) {
                Ok(_) => fs::rename(&targets[index].path, &targets[index].failed_new_path)
                    .map_err(|error| {
                        commit_io_error(
                            &targets[index].path,
                            "could not retain failed transformed source for rollback",
                            error,
                        )
                    })?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(commit_io_error(
                        &targets[index].path,
                        "could not inspect transformed source during rollback",
                        error,
                    ));
                }
            }
        }

        if let Err(error) = fs::rename(&targets[index].backup_path, &targets[index].path) {
            return Err(commit_io_error(
                &targets[index].path,
                "could not restore previous source during rollback",
                error,
            ));
        }
        targets[index].phase = TargetPhase::Staged;
    }
    Ok(())
}

fn recovery_required_without_rollback(
    preparation: &NativeMigrationPreparation,
    targets: &mut [CommitTarget],
    original_error: MasterdataError,
    reason: &str,
) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
    retain_workspaces(targets);
    let recovery = MasterdataError::new(
        "E-MIGRATION-RECOVERY-REQUIRED",
        ErrorKind::Io,
        format!(
            "Migration commit cannot continue safely: {reason}; original failure [{}]: {}",
            original_error.diagnostic().code,
            original_error.diagnostic().message
        ),
    )
    .with_source(
        original_error
            .diagnostic()
            .source
            .clone()
            .unwrap_or_else(|| preparation.base.config_path.clone()),
    )
    .with_related_requirement("MIGRATION-010")
    .with_related_requirement("MIGRATION-016");
    Err(MigrationCommitFailure {
        report: report_for(
            preparation,
            MigrationCommitStatus::RecoveryRequired,
            targets,
        ),
        error: recovery,
    })
}

fn report_for(
    preparation: &NativeMigrationPreparation,
    status: MigrationCommitStatus,
    targets: &[CommitTarget],
) -> MigrationCommitReport {
    let transformed = preparation
        .dry_run
        .transformed_documents
        .files
        .iter()
        .map(|loaded| (loaded.path.clone(), loaded.source.as_bytes()))
        .collect::<BTreeMap<_, _>>();
    let files = preparation
        .dry_run
        .plan
        .affected_files
        .iter()
        .map(|plan| {
            let path = plan.path.clone();
            let current = fs::read(&path);
            let old = preparation.base.sources.get(&path).map(Vec::as_slice);
            let new = transformed.get(&path).copied();
            let state = match current {
                Ok(bytes) if old.is_some_and(|old| bytes == old) => MigrationCommittedFileState::Old,
                Ok(bytes) if new.is_some_and(|new| bytes == new) => MigrationCommittedFileState::New,
                Ok(_) => MigrationCommittedFileState::Other,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    MigrationCommittedFileState::Missing
                }
                Err(_) => MigrationCommittedFileState::Other,
            };
            let recovery_artifacts = if status == MigrationCommitStatus::RecoveryRequired {
                targets
                    .iter()
                    .find(|target| target.path == path)
                    .and_then(|target| target.recovery_dir.clone())
                    .map(|directory| vec![directory])
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            MigrationCommitFileReport {
                path,
                state,
                recovery_artifacts,
            }
        })
        .collect();
    MigrationCommitReport { status, files }
}

fn cleanup_workspaces(targets: &mut [CommitTarget]) {
    for target in targets {
        target.workspace.take();
    }
}

fn retain_workspaces(targets: &mut [CommitTarget]) {
    for target in targets {
        if let Some(workspace) = target.workspace.take() {
            let retained = workspace.keep();
            target.stage_path = retained.join("new");
            target.backup_path = retained.join("old");
            target.failed_new_path = retained.join("failed-new");
            target.recovery_dir = Some(retained);
        }
    }
}

fn injected(
    injections: &[MigrationCommitFailureInjection],
    file_index: usize,
    point: MigrationCommitFailurePoint,
) -> bool {
    injections
        .iter()
        .any(|injection| injection.file_index == file_index && injection.point == point)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| native_io_error(path, error))
}

fn native_io_error(path: &Path, error: io::Error) -> MasterdataError {
    MasterdataError::new(
        "E-MIGRATION-SNAPSHOT-IO",
        ErrorKind::Io,
        format!("could not read Migration snapshot input: {error}"),
    )
    .with_source(path.to_path_buf())
    .with_related_requirement("MIGRATION-005")
}

fn stale_access_error(path: &Path, error: io::Error) -> MasterdataError {
    stale_error(
        path,
        format!("Migration base input is no longer readable: {error}"),
    )
}

fn stale_error(path: &Path, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new("E-MIGRATION-STALE-PLAN", ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement("MIGRATION-016")
}

fn commit_io_error(path: &Path, action: &str, error: impl fmt::Display) -> MasterdataError {
    MasterdataError::new(
        "E-MIGRATION-COMMIT-IO",
        ErrorKind::Io,
        format!("{action}: {error}"),
    )
    .with_source(path.to_path_buf())
    .with_related_requirement("MIGRATION-010")
}

fn injected_commit_error(path: &Path, point: &str) -> MasterdataError {
    MasterdataError::new(
        "E-MIGRATION-COMMIT-INJECTED",
        ErrorKind::Io,
        format!("injected Migration commit failure {point}"),
    )
    .with_source(path.to_path_buf())
    .with_related_requirement("MIGRATION-010")
}

fn rolled_back_error(original: &MasterdataError) -> MasterdataError {
    let mut error = MasterdataError::new(
        "E-MIGRATION-COMMIT-ROLLED-BACK",
        ErrorKind::Io,
        format!(
            "Migration commit failed [{}]: {}; complete OLD source set was restored",
            original.diagnostic().code,
            original.diagnostic().message
        ),
    )
    .with_related_requirement("MIGRATION-010");
    if let Some(source) = &original.diagnostic().source {
        error = error.with_source(source.clone());
    }
    error
}

fn recovery_required_error(
    original: &MasterdataError,
    rollback: &MasterdataError,
) -> MasterdataError {
    let mut error = MasterdataError::new(
        "E-MIGRATION-RECOVERY-REQUIRED",
        ErrorKind::Io,
        format!(
            "Migration commit failed [{}]: {}; rollback failed [{}]: {}; manual recovery is required",
            original.diagnostic().code,
            original.diagnostic().message,
            rollback.diagnostic().code,
            rollback.diagnostic().message
        ),
    )
    .with_related_requirement("MIGRATION-010");
    if let Some(source) = &rollback.diagnostic().source {
        error = error.with_source(source.clone());
    }
    error
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use masterdata_core::{
        AddFieldCommand, FieldDefinition, MigrationCommand, SourceDocument, parse_yaml_document,
    };
    use tempfile::{TempDir, tempdir};

    use super::{
        MigrationCommitFailureInjection, MigrationCommitFailurePoint, MigrationCommitStatus,
        MigrationCommittedFileState,
    };
    use crate::NativeApplicationService;

    struct Fixture {
        _temp: TempDir,
        root: PathBuf,
        schema: PathBuf,
        data_a: PathBuf,
        data_b: PathBuf,
        config: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempdir().expect("temp project");
            let root = temp.path().join("project");
            let sources = root.join("sources");
            fs::create_dir_all(&sources).expect("source root");
            let config = root.join("masterdata.toml");
            fs::write(
                &config,
                "[project]\nid = \"migration.test\"\nname = \"Migration Test\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
            )
            .expect("config");
            let schema = sources.join("item-schema.yaml");
            fs::write(
                &schema,
                "kind: schema\ntable: item\ncsharpName: ItemMaster\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n",
            )
            .expect("schema");
            let data_a = sources.join("items-a.yaml");
            fs::write(
                &data_a,
                "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Potion\n",
            )
            .expect("data a");
            let data_b = sources.join("items-b.yaml");
            fs::write(
                &data_b,
                "kind: data\ntable: item\nrecords:\n  - id: 1002\n    name: Hi-Potion\n",
            )
            .expect("data b");
            Self {
                _temp: temp,
                root,
                schema,
                data_a,
                data_b,
                config,
            }
        }

        fn snapshot(&self) -> BTreeMap<PathBuf, Vec<u8>> {
            [&self.schema, &self.data_a, &self.data_b]
                .into_iter()
                .map(|path| (path.clone(), fs::read(path).expect("source snapshot")))
                .collect()
        }
    }

    fn add_field_command() -> MigrationCommand {
        let value = parse_yaml_document(
            PathBuf::from("initializer.yaml"),
            "kind: data\ntable: helper\nrecords:\n  - value: 10\n",
        )
        .expect("initializer parse");
        let initializer = match value.document {
            SourceDocument::Data(data) => data.records[0]
                .get("value")
                .expect("initializer value")
                .clone(),
            _ => unreachable!("initializer document is data"),
        };
        MigrationCommand::AddField(AddFieldCommand {
            table: "item".to_owned(),
            field: FieldDefinition {
                key: 2,
                name: "price".to_owned(),
                type_name: "int".to_owned(),
                nullable: false,
                array: false,
            },
            initializer: Some(initializer),
        })
    }

    fn prepare(
        service: &NativeApplicationService,
        fixture: &Fixture,
    ) -> super::NativeMigrationPreparation {
        service
            .prepare_migration(Some(&fixture.root), &fixture.root, &add_field_command())
            .expect("migration preparation")
    }

    #[test]
    fn successful_multi_file_commit_leaves_complete_new_source_set() {
        let fixture = Fixture::new();
        let service = NativeApplicationService::new();
        let preparation = prepare(&service, &fixture);

        let report = service
            .commit_migration(&preparation)
            .expect("migration commit");

        assert_eq!(report.status, MigrationCommitStatus::Success);
        assert!(report
            .files
            .iter()
            .all(|file| file.state == MigrationCommittedFileState::New));
        assert!(fs::read_to_string(&fixture.schema).unwrap().contains("name: price"));
        assert!(fs::read_to_string(&fixture.data_a).unwrap().contains("price: 10"));
        assert!(fs::read_to_string(&fixture.data_b).unwrap().contains("price: 10"));
        assert!(!fixture.root.join(".masterdata/output").exists());
    }

    #[test]
    fn stale_loaded_source_is_rejected_without_source_mutation() {
        let fixture = Fixture::new();
        let service = NativeApplicationService::new();
        let preparation = prepare(&service, &fixture);
        let schema_before = fs::read(&fixture.schema).unwrap();
        fs::write(
            &fixture.data_a,
            "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Externally-Edited\n",
        )
        .expect("external edit");
        let external = fs::read(&fixture.data_a).unwrap();

        let failure = service
            .commit_migration(&preparation)
            .expect_err("stale plan");

        assert_eq!(failure.report.status, MigrationCommitStatus::StalePlan);
        assert_eq!(failure.diagnostic().code, "E-MIGRATION-STALE-PLAN");
        assert_eq!(fs::read(&fixture.schema).unwrap(), schema_before);
        assert_eq!(fs::read(&fixture.data_a).unwrap(), external);
    }

    #[test]
    fn stale_source_membership_is_rejected_without_source_mutation() {
        let fixture = Fixture::new();
        let service = NativeApplicationService::new();
        let preparation = prepare(&service, &fixture);
        let before = fixture.snapshot();
        fs::write(
            fixture.root.join("sources/other.yaml"),
            "kind: data\ntable: other\nrecords: []\n",
        )
        .expect("new source");

        let failure = service
            .commit_migration(&preparation)
            .expect_err("stale file set");

        assert_eq!(failure.report.status, MigrationCommitStatus::StalePlan);
        assert_eq!(failure.diagnostic().code, "E-MIGRATION-STALE-PLAN");
        assert_sources(&before);
    }

    #[test]
    fn stale_config_is_rejected_without_source_mutation() {
        let fixture = Fixture::new();
        let service = NativeApplicationService::new();
        let preparation = prepare(&service, &fixture);
        let before = fixture.snapshot();
        fs::create_dir_all(fixture.root.join("more")).unwrap();
        let mut config = fs::read_to_string(&fixture.config).unwrap();
        config = config.replace("roots = [\"sources\"]", "roots = [\"sources\", \"more\"]");
        fs::write(&fixture.config, config).unwrap();

        let failure = service
            .commit_migration(&preparation)
            .expect_err("stale config");

        assert_eq!(failure.report.status, MigrationCommitStatus::StalePlan);
        assert_eq!(failure.diagnostic().code, "E-MIGRATION-STALE-PLAN");
        assert_sources(&before);
    }

    #[test]
    fn write_failure_rolls_back_complete_old_source_set() {
        let fixture = Fixture::new();
        let service = NativeApplicationService::new();
        let preparation = prepare(&service, &fixture);
        let before = fixture.snapshot();

        let failure = service
            .commit_migration_with_failures(
                &preparation,
                &[MigrationCommitFailureInjection {
                    file_index: 1,
                    point: MigrationCommitFailurePoint::BeforeInstall,
                }],
            )
            .expect_err("injected commit failure");

        assert_eq!(failure.report.status, MigrationCommitStatus::RolledBack);
        assert_eq!(
            failure.diagnostic().code,
            "E-MIGRATION-COMMIT-ROLLED-BACK"
        );
        assert!(failure
            .report
            .files
            .iter()
            .all(|file| file.state == MigrationCommittedFileState::Old));
        assert_sources(&before);
    }

    #[test]
    fn rollback_failure_reports_recovery_required_and_stops() {
        let fixture = Fixture::new();
        let service = NativeApplicationService::new();
        let preparation = prepare(&service, &fixture);

        let failure = service
            .commit_migration_with_failures(
                &preparation,
                &[
                    MigrationCommitFailureInjection {
                        file_index: 1,
                        point: MigrationCommitFailurePoint::BeforeInstall,
                    },
                    MigrationCommitFailureInjection {
                        file_index: 0,
                        point: MigrationCommitFailurePoint::RollbackRestore,
                    },
                ],
            )
            .expect_err("recovery required");

        assert_eq!(
            failure.report.status,
            MigrationCommitStatus::RecoveryRequired
        );
        assert_eq!(
            failure.diagnostic().code,
            "E-MIGRATION-RECOVERY-REQUIRED"
        );
        assert_eq!(
            failure.report.files[0].state,
            MigrationCommittedFileState::New
        );
        assert_eq!(
            failure.report.files[1].state,
            MigrationCommittedFileState::Old
        );
        assert_eq!(
            failure.report.files[2].state,
            MigrationCommittedFileState::Old
        );
        assert!(failure
            .report
            .files
            .iter()
            .any(|file| !file.recovery_artifacts.is_empty()));
    }

    fn assert_sources(expected: &BTreeMap<PathBuf, Vec<u8>>) {
        for (path, bytes) in expected {
            assert_eq!(
                fs::read(path).expect("source bytes").as_slice(),
                bytes.as_slice()
            );
        }
    }
}
