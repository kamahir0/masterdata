use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::{NamedTempFile, TempDir, TempPath};

use crate::document::{LoadedDocument, ProjectDocuments};
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::migration::MigrationDryRun;
use crate::project::Project;

const IO_ERROR_CODE: &str = "E-IO-ACCESS";
const PLAN_ERROR_CODE: &str = "E-MIGRATION-PATCH-INVALID";

/// Observable state of the source set after a commit attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCommitState {
    /// No canonical source file was intentionally changed.
    NotStarted,
    /// Every affected source file contains the transformed bytes.
    Success,
    /// A failed commit was successfully restored to the complete old source set.
    RolledBack,
    /// The old and new source sets cannot both be established automatically.
    RecoveryRequired,
}

/// State of one affected source file in a commit report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationFileCommitState {
    /// The file was not changed by the attempted commit.
    Unchanged,
    /// The file contains the transformed source bytes.
    New,
    /// The file contains the original source bytes after rollback.
    Old,
    /// The file needs manual recovery because the transaction could not establish its state.
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationFileCommitStatus {
    pub path: PathBuf,
    pub state: MigrationFileCommitState,
}

/// Structured result for a source commit attempt.
///
/// This is an internal Rust use-case result, not a public CLI or JSON contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationCommitReport {
    pub state: MigrationCommitState,
    pub files: Vec<MigrationFileCommitStatus>,
    /// Retained staged and backup material when the state is
    /// [`MigrationCommitState::RecoveryRequired`].
    pub recovery_workspace: Option<PathBuf>,
}

/// Failure result that retains the structured source-set state for adapters.
#[derive(Debug)]
pub struct MigrationCommitFailure {
    pub report: MigrationCommitReport,
    pub error: MasterdataError,
}

impl MigrationCommitFailure {
    pub fn into_error(self) -> MasterdataError {
        self.error
    }

    pub(crate) fn not_started(error: MasterdataError) -> Self {
        Self {
            report: MigrationCommitReport {
                state: MigrationCommitState::NotStarted,
                files: Vec::new(),
                recovery_workspace: None,
            },
            error,
        }
    }
}

/// Deterministic fault-injection point used by source-commit regression tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationCommitFailurePoint {
    WriteFile(usize),
    RollbackFile(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationCommitFailureInjection {
    pub point: MigrationCommitFailurePoint,
}

impl MigrationCommitFailureInjection {
    pub const fn write_file(index: usize) -> Self {
        Self {
            point: MigrationCommitFailurePoint::WriteFile(index),
        }
    }

    pub const fn rollback_file(index: usize) -> Self {
        Self {
            point: MigrationCommitFailurePoint::RollbackFile(index),
        }
    }
}

/// Commit an already verified AddField dry-run against its exact source snapshot.
pub fn commit_migration(
    project: &Project,
    source_snapshot: &ProjectDocuments,
    dry_run: &MigrationDryRun,
) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
    commit_migration_with_failures(project, source_snapshot, dry_run, &[])
}

/// Commit an already verified AddField dry-run with deterministic test seams.
#[doc(hidden)]
pub fn commit_migration_with_failures(
    project: &Project,
    source_snapshot: &ProjectDocuments,
    dry_run: &MigrationDryRun,
    injections: &[MigrationCommitFailureInjection],
) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
    let mut report = report_for(dry_run);
    let mut transaction = match SourceCommitTransaction::stage(source_snapshot, dry_run) {
        Ok(transaction) => transaction,
        Err(error) => return Err(MigrationCommitFailure { report, error }),
    };

    // WHY: MIGRATION-016 requires the config, source-set membership, and every
    // source input used by resolution to match the exact snapshot immediately
    // before canonical mutation begins.
    // IF REMOVED: a plan could overwrite a concurrent edit even when the
    // affected file itself was not the edited closure input.
    // EVIDENCE: docs/specs/schema-migration.md; docs/spec-changes/0011-cli-surface-and-schema-migration.md
    // Regression: stale_source_snapshot_rejects_without_mutation;
    // stale_project_config_rejects_without_mutation;
    // stale_source_membership_rejects_without_mutation.
    if let Err(error) =
        preflight_source_snapshot(project, source_snapshot, &dry_run.plan.source_inputs)
    {
        return Err(MigrationCommitFailure { report, error });
    }

    for index in 0..transaction.entries.len() {
        if let Err(error) = transaction.install(index, injections) {
            return Err(finish_failed_commit(transaction, report, error, injections));
        }
        report.files[index].state = MigrationFileCommitState::New;
    }

    report.state = MigrationCommitState::Success;
    Ok(report)
}

fn report_for(dry_run: &MigrationDryRun) -> MigrationCommitReport {
    MigrationCommitReport {
        state: MigrationCommitState::NotStarted,
        files: dry_run
            .plan
            .affected_files
            .iter()
            .map(|file| MigrationFileCommitStatus {
                path: file.path.clone(),
                state: MigrationFileCommitState::Unchanged,
            })
            .collect(),
        recovery_workspace: None,
    }
}

fn finish_failed_commit(
    mut transaction: SourceCommitTransaction,
    mut report: MigrationCommitReport,
    commit_error: MasterdataError,
    injections: &[MigrationCommitFailureInjection],
) -> MigrationCommitFailure {
    match transaction.rollback(injections) {
        Ok(()) => {
            for (status, entry) in report.files.iter_mut().zip(&transaction.entries) {
                if entry.state == TransactionEntryState::Restored {
                    status.state = MigrationFileCommitState::Old;
                }
            }
            report.state = MigrationCommitState::RolledBack;
            MigrationCommitFailure {
                report,
                error: commit_error,
            }
        }
        Err(rollback_error) => {
            for (status, entry) in report.files.iter_mut().zip(&transaction.entries) {
                status.state = match entry.state {
                    TransactionEntryState::Pending => MigrationFileCommitState::Unchanged,
                    TransactionEntryState::Restored => MigrationFileCommitState::Old,
                    TransactionEntryState::NewInstalled => MigrationFileCommitState::New,
                    TransactionEntryState::OldStaged => MigrationFileCommitState::RecoveryRequired,
                };
            }
            let recovery_workspace = transaction.retain_recovery_workspace();
            report.state = MigrationCommitState::RecoveryRequired;
            report.recovery_workspace = Some(recovery_workspace.clone());
            let error = MasterdataError::new(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!(
                    "migration source commit failed [{}]: {}; rollback failed [{}]: {}; recovery workspace retained at {}",
                    commit_error.diagnostic().code,
                    commit_error.diagnostic().message,
                    rollback_error.diagnostic().code,
                    rollback_error.diagnostic().message,
                    recovery_workspace.display(),
                ),
            )
            .with_source(recovery_workspace)
            .with_related_requirement("MIGRATION-010")
            .with_related_requirement("MIGRATION-016");
            MigrationCommitFailure { report, error }
        }
    }
}

fn preflight_source_snapshot(
    project: &Project,
    source_snapshot: &ProjectDocuments,
    source_inputs: &[PathBuf],
) -> Result<()> {
    let config_metadata = fs::symlink_metadata(project.config_path()).map_err(|error| {
        stale_error(
            project.config_path(),
            format!("project configuration could not be inspected: {error}"),
        )
    })?;
    if config_metadata.file_type().is_symlink() || !config_metadata.file_type().is_file() {
        return Err(stale_error(
            project.config_path(),
            "project configuration is no longer the same regular file",
        ));
    }
    let config_source = fs::read(project.config_path()).map_err(|error| {
        stale_error(
            project.config_path(),
            format!("project configuration could not be read: {error}"),
        )
    })?;
    if config_source != project.config_source() {
        return Err(stale_error(
            project.config_path(),
            "project configuration changed after the migration snapshot",
        ));
    }

    let expected_paths = snapshot_paths(source_snapshot)?;
    let current_paths = project.source_files().map_err(|error| {
        stale_error(
            project.config_path(),
            format!(
                "source file set could not be confirmed: [{}] {}",
                error.diagnostic().code,
                error.diagnostic().message
            ),
        )
    })?;
    if current_paths != expected_paths {
        return Err(stale_error(
            project.config_path(),
            "configured source file membership changed after the migration snapshot",
        ));
    }

    for path in source_input_paths(source_snapshot, source_inputs)? {
        let loaded = find_document(source_snapshot, &path).ok_or_else(|| {
            commit_error(
                PLAN_ERROR_CODE,
                ErrorKind::Validation,
                format!(
                    "migration plan source input `{}` is missing from the source snapshot",
                    path.display()
                ),
                Some(path.clone()),
                "MIGRATION-016",
            )
        })?;
        let metadata = fs::symlink_metadata(&loaded.path).map_err(|error| {
            stale_error(
                &loaded.path,
                format!("source file could not be inspected: {error}"),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(stale_error(
                &loaded.path,
                "source snapshot path is no longer the same regular file",
            ));
        }
        let source = fs::read(&loaded.path).map_err(|error| {
            stale_error(
                &loaded.path,
                format!("source file could not be read: {error}"),
            )
        })?;
        if source != loaded.source.as_bytes() {
            return Err(stale_error(
                &loaded.path,
                "source file content changed after the migration snapshot",
            ));
        }
    }
    Ok(())
}

fn source_input_paths(
    source_snapshot: &ProjectDocuments,
    source_inputs: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let snapshot_paths = snapshot_paths(source_snapshot)?;
    let mut paths = source_inputs.to_vec();
    paths.sort();
    let mut unique = BTreeSet::new();
    for path in &paths {
        if !unique.insert(path.clone()) {
            return Err(commit_error(
                PLAN_ERROR_CODE,
                ErrorKind::Validation,
                format!(
                    "migration plan contains duplicate source input `{}`",
                    path.display()
                ),
                Some(path.clone()),
                "MIGRATION-016",
            ));
        }
        if snapshot_paths.binary_search(path).is_err() {
            return Err(commit_error(
                PLAN_ERROR_CODE,
                ErrorKind::Validation,
                format!(
                    "migration plan source input `{}` is not in the source snapshot",
                    path.display()
                ),
                Some(path.clone()),
                "MIGRATION-016",
            ));
        }
    }
    if paths.is_empty() {
        return Err(commit_error(
            PLAN_ERROR_CODE,
            ErrorKind::Validation,
            "migration plan does not identify any source input",
            None,
            "MIGRATION-016",
        ));
    }
    Ok(paths)
}

fn snapshot_paths(source_snapshot: &ProjectDocuments) -> Result<Vec<PathBuf>> {
    let mut paths = source_snapshot
        .files
        .iter()
        .map(|loaded| loaded.path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    let mut unique = BTreeSet::new();
    if paths.iter().any(|path| !unique.insert(path.clone())) {
        return Err(commit_error(
            PLAN_ERROR_CODE,
            ErrorKind::Validation,
            "migration source snapshot contains duplicate file paths",
            None,
            "MIGRATION-016",
        ));
    }
    Ok(paths)
}

fn stale_error(path: &Path, detail: impl Into<String>) -> MasterdataError {
    MasterdataError::new(
        PLAN_ERROR_CODE,
        ErrorKind::Validation,
        format!("migration plan is stale: {}", detail.into()),
    )
    .with_source(path.to_path_buf())
    .with_related_requirement("MIGRATION-016")
}

fn commit_error(
    code: &str,
    kind: ErrorKind,
    message: impl Into<String>,
    source: Option<PathBuf>,
    requirement: &str,
) -> MasterdataError {
    let mut error = MasterdataError::new(code, kind, message);
    if let Some(source) = source {
        error = error.with_source(source);
    }
    error.with_related_requirement(requirement)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionEntryState {
    Pending,
    OldStaged,
    NewInstalled,
    Restored,
}

struct TransactionEntry {
    path: PathBuf,
    old_bytes: Vec<u8>,
    new_bytes: Vec<u8>,
    staged_new: PathBuf,
    replacement: Option<TempPath>,
    live_backup: Option<TempPath>,
    state: TransactionEntryState,
}

struct SourceCommitTransaction {
    workspace: TempDir,
    entries: Vec<TransactionEntry>,
}

impl SourceCommitTransaction {
    fn stage(source_snapshot: &ProjectDocuments, dry_run: &MigrationDryRun) -> Result<Self> {
        let snapshot_paths = snapshot_paths(source_snapshot)?;
        let workspace = TempDir::new().map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not create migration staging workspace: {error}"),
                None,
                "MIGRATION-010",
            )
        })?;
        let staged_old_root = workspace.path().join("old");
        let staged_new_root = workspace.path().join("new");
        fs::create_dir_all(&staged_old_root).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not create migration backup staging area: {error}"),
                Some(staged_old_root.clone()),
                "MIGRATION-010",
            )
        })?;
        fs::create_dir_all(&staged_new_root).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not create migration transformed staging area: {error}"),
                Some(staged_new_root.clone()),
                "MIGRATION-010",
            )
        })?;

        let mut entries = Vec::with_capacity(dry_run.plan.affected_files.len());
        let mut planned_paths = BTreeSet::new();
        for (index, planned) in dry_run.plan.affected_files.iter().enumerate() {
            if !planned_paths.insert(planned.path.clone()) {
                return Err(commit_error(
                    PLAN_ERROR_CODE,
                    ErrorKind::Validation,
                    format!(
                        "migration plan contains duplicate affected file `{}`",
                        planned.path.display()
                    ),
                    Some(planned.path.clone()),
                    "MIGRATION-009",
                ));
            }
            if snapshot_paths.binary_search(&planned.path).is_err() {
                return Err(commit_error(
                    PLAN_ERROR_CODE,
                    ErrorKind::Validation,
                    format!(
                        "affected file `{}` is not in the source snapshot",
                        planned.path.display()
                    ),
                    Some(planned.path.clone()),
                    "MIGRATION-016",
                ));
            }
            let original = find_document(source_snapshot, &planned.path).ok_or_else(|| {
                commit_error(
                    PLAN_ERROR_CODE,
                    ErrorKind::Validation,
                    format!(
                        "source snapshot is missing affected file `{}`",
                        planned.path.display()
                    ),
                    Some(planned.path.clone()),
                    "MIGRATION-016",
                )
            })?;
            let transformed = find_document(&dry_run.transformed_documents, &planned.path)
                .ok_or_else(|| {
                    commit_error(
                        PLAN_ERROR_CODE,
                        ErrorKind::Validation,
                        format!(
                            "transformed snapshot is missing affected file `{}`",
                            planned.path.display()
                        ),
                        Some(planned.path.clone()),
                        "MIGRATION-015",
                    )
                })?;
            let staged_old = staged_old_root.join(format!("{index:04}.source"));
            let staged_new = staged_new_root.join(format!("{index:04}.source"));
            fs::write(&staged_old, original.source.as_bytes()).map_err(|error| {
                commit_error(
                    IO_ERROR_CODE,
                    ErrorKind::Io,
                    format!("could not stage original source bytes: {error}"),
                    Some(staged_old.clone()),
                    "MIGRATION-010",
                )
            })?;
            fs::write(&staged_new, transformed.source.as_bytes()).map_err(|error| {
                commit_error(
                    IO_ERROR_CODE,
                    ErrorKind::Io,
                    format!("could not stage transformed source bytes: {error}"),
                    Some(staged_new.clone()),
                    "MIGRATION-010",
                )
            })?;
            entries.push(TransactionEntry {
                path: planned.path.clone(),
                old_bytes: original.source.as_bytes().to_vec(),
                new_bytes: transformed.source.as_bytes().to_vec(),
                staged_new,
                replacement: None,
                live_backup: None,
                state: TransactionEntryState::Pending,
            });
        }

        let journal = workspace.path().join("journal.txt");
        let journal_content = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                format!(
                    "{index}\t{}\told/{index:04}.source\tnew/{index:04}.source\n",
                    entry.path.display()
                )
            })
            .collect::<String>();
        fs::write(&journal, journal_content).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not write migration transaction journal: {error}"),
                Some(journal),
                "MIGRATION-010",
            )
        })?;

        Ok(Self { workspace, entries })
    }

    fn install(
        &mut self,
        index: usize,
        injections: &[MigrationCommitFailureInjection],
    ) -> Result<()> {
        let entry = &mut self.entries[index];
        if injections
            .iter()
            .any(|injection| injection.point == MigrationCommitFailurePoint::WriteFile(index))
        {
            return Err(commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!(
                    "source commit write was injected for `{}`",
                    entry.path.display()
                ),
                Some(entry.path.clone()),
                "MIGRATION-010",
            ));
        }

        verify_source_bytes(
            &entry.path,
            &entry.old_bytes,
            "source changed during commit",
        )?;
        let parent = entry.path.parent().ok_or_else(|| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                "affected source path has no parent directory",
                Some(entry.path.clone()),
                "MIGRATION-010",
            )
        })?;
        validate_source_parent(parent)?;

        let staged_new = fs::read(&entry.staged_new).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not read staged transformed source bytes: {error}"),
                Some(entry.staged_new.clone()),
                "MIGRATION-010",
            )
        })?;
        if staged_new != entry.new_bytes {
            return Err(commit_error(
                PLAN_ERROR_CODE,
                ErrorKind::Validation,
                "staged transformed source bytes differ from the verified dry-run",
                Some(entry.staged_new.clone()),
                "MIGRATION-015",
            ));
        }

        // WHY: the replacement is created beside the source, and the old
        // source is moved to a same-parent backup before the new path is
        // installed. This keeps each rename on one filesystem and leaves a
        // verified backup available for reverse-order rollback.
        // IF REMOVED: a cross-device or in-place overwrite could leave a
        // mixed source set with no reliable OLD restoration path.
        // EVIDENCE: docs/specs/schema-migration.md; Regression: successful_multi_file_commit_leaves_complete_new_source_set; commit_failure_rolls_back_complete_old_source_set; rollback_failure_reports_recovery_required_and_retains_recovery_workspace.
        let replacement = NamedTempFile::new_in(parent).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not create source replacement file: {error}"),
                Some(parent.to_path_buf()),
                "MIGRATION-010",
            )
        })?;
        let mut replacement_file = replacement;
        replacement_file
            .write_all(&entry.new_bytes)
            .and_then(|()| replacement_file.as_file().sync_all())
            .map_err(|error| {
                commit_error(
                    IO_ERROR_CODE,
                    ErrorKind::Io,
                    format!("could not write source replacement file: {error}"),
                    Some(entry.path.clone()),
                    "MIGRATION-010",
                )
            })?;
        let replacement_path = replacement_file.into_temp_path();

        let backup = NamedTempFile::new_in(parent).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not create source backup path: {error}"),
                Some(parent.to_path_buf()),
                "MIGRATION-010",
            )
        })?;
        let backup_path = backup.into_temp_path();
        fs::remove_file(&backup_path).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not prepare source backup path: {error}"),
                Some(backup_path.to_path_buf()),
                "MIGRATION-010",
            )
        })?;

        verify_source_bytes(
            &entry.path,
            &entry.old_bytes,
            "source changed during commit",
        )?;
        fs::rename(&entry.path, &backup_path).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not stage original source file: {error}"),
                Some(entry.path.clone()),
                "MIGRATION-010",
            )
        })?;
        entry.live_backup = Some(backup_path);
        entry.replacement = Some(replacement_path);
        entry.state = TransactionEntryState::OldStaged;

        let replacement_path = entry
            .replacement
            .as_ref()
            .expect("replacement path is recorded before installation");
        if let Err(error) = fs::rename(replacement_path, &entry.path) {
            return Err(commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not install transformed source file: {error}"),
                Some(entry.path.clone()),
                "MIGRATION-010",
            ));
        }
        entry.state = TransactionEntryState::NewInstalled;
        Ok(())
    }

    fn rollback(&mut self, injections: &[MigrationCommitFailureInjection]) -> Result<()> {
        // Reverse order keeps the already-established suffix out of the way
        // before restoring an earlier file. A rollback failure stops this loop;
        // the retained workspace is then the explicit recovery boundary.
        for index in (0..self.entries.len()).rev() {
            if !matches!(
                self.entries[index].state,
                TransactionEntryState::OldStaged | TransactionEntryState::NewInstalled
            ) {
                continue;
            }
            if injections.iter().any(|injection| {
                injection.point == MigrationCommitFailurePoint::RollbackFile(index)
            }) {
                return Err(commit_error(
                    IO_ERROR_CODE,
                    ErrorKind::Io,
                    format!(
                        "source rollback was injected for `{}`",
                        self.entries[index].path.display()
                    ),
                    Some(self.entries[index].path.clone()),
                    "MIGRATION-010",
                ));
            }
            self.restore_entry(index)?;
        }
        Ok(())
    }

    fn restore_entry(&mut self, index: usize) -> Result<()> {
        let entry = &mut self.entries[index];
        match entry.state {
            TransactionEntryState::OldStaged => {
                ensure_absent(&entry.path, "rollback target")?;
            }
            TransactionEntryState::NewInstalled => {
                verify_source_bytes(
                    &entry.path,
                    &entry.new_bytes,
                    "new source changed before rollback",
                )?;
                fs::remove_file(&entry.path).map_err(|error| {
                    commit_error(
                        IO_ERROR_CODE,
                        ErrorKind::Io,
                        format!("could not remove transformed source during rollback: {error}"),
                        Some(entry.path.clone()),
                        "MIGRATION-010",
                    )
                })?;
            }
            TransactionEntryState::Pending | TransactionEntryState::Restored => return Ok(()),
        }

        let backup = entry.live_backup.as_ref().ok_or_else(|| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                "source rollback backup is missing",
                Some(entry.path.clone()),
                "MIGRATION-010",
            )
        })?;
        let backup_bytes = fs::read(backup).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not read source rollback backup: {error}"),
                Some(backup.to_path_buf()),
                "MIGRATION-010",
            )
        })?;
        if backup_bytes != entry.old_bytes {
            return Err(commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                "source rollback backup differs from the original snapshot",
                Some(backup.to_path_buf()),
                "MIGRATION-010",
            ));
        }
        fs::rename(backup, &entry.path).map_err(|error| {
            commit_error(
                IO_ERROR_CODE,
                ErrorKind::Io,
                format!("could not restore original source file: {error}"),
                Some(entry.path.clone()),
                "MIGRATION-010",
            )
        })?;
        entry.state = TransactionEntryState::Restored;
        Ok(())
    }

    fn retain_recovery_workspace(mut self) -> PathBuf {
        // WHY: ownership is retained only after rollback failure so staged
        // OLD/NEW bytes and live backups survive the returned failure.
        // IF REMOVED: normal transaction cleanup would silently delete the
        // only material needed to recover a mixed source set.
        // EVIDENCE: docs/specs/schema-migration.md; Regression: rollback_failure_reports_recovery_required_and_retains_recovery_workspace.
        for entry in &mut self.entries {
            if let Some(backup) = entry.live_backup.take() {
                let _ = backup.keep();
            }
        }
        let workspace = self.workspace.keep();
        drop(self.entries);
        workspace
    }
}

fn find_document<'a>(documents: &'a ProjectDocuments, path: &Path) -> Option<&'a LoadedDocument> {
    documents.files.iter().find(|loaded| loaded.path == path)
}

fn validate_source_parent(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        commit_error(
            IO_ERROR_CODE,
            ErrorKind::Io,
            format!("source parent directory could not be inspected: {error}"),
            Some(path.to_path_buf()),
            "MIGRATION-010",
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(commit_error(
            IO_ERROR_CODE,
            ErrorKind::Io,
            "source parent path is not a real directory",
            Some(path.to_path_buf()),
            "MIGRATION-010",
        ));
    }
    Ok(())
}

fn verify_source_bytes(path: &Path, expected: &[u8], detail: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        commit_error(
            PLAN_ERROR_CODE,
            ErrorKind::Validation,
            format!("{detail}: source file could not be inspected: {error}"),
            Some(path.to_path_buf()),
            "MIGRATION-016",
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(commit_error(
            PLAN_ERROR_CODE,
            ErrorKind::Validation,
            format!("{detail}: source path is not the expected regular file"),
            Some(path.to_path_buf()),
            "MIGRATION-016",
        ));
    }
    let actual = fs::read(path).map_err(|error| {
        commit_error(
            PLAN_ERROR_CODE,
            ErrorKind::Validation,
            format!("{detail}: source file could not be read: {error}"),
            Some(path.to_path_buf()),
            "MIGRATION-016",
        )
    })?;
    if actual != expected {
        return Err(commit_error(
            PLAN_ERROR_CODE,
            ErrorKind::Validation,
            detail,
            Some(path.to_path_buf()),
            "MIGRATION-016",
        ));
    }
    Ok(())
}

fn ensure_absent(path: &Path, operation: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(commit_error(
            IO_ERROR_CODE,
            ErrorKind::Io,
            format!("{operation} is not absent: {}", path.display()),
            Some(path.to_path_buf()),
            "MIGRATION-010",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(commit_error(
            IO_ERROR_CODE,
            ErrorKind::Io,
            format!("could not inspect {operation}: {error}"),
            Some(path.to_path_buf()),
            "MIGRATION-010",
        )),
    }
}
