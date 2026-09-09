use std::path::Path;

use crate::Result;
use crate::migration::{MigrationCommand, MigrationDryRun, dry_run_migration};
use crate::migration_commit::{
    MigrationCommitFailure, MigrationCommitFailureInjection, MigrationCommitReport,
    commit_migration, commit_migration_with_failures,
};
use crate::pipeline::{BuildPlan, prepare_build_from_documents};
use crate::project::{InitOptions, Project, ProjectInfo, initialize_project};
use crate::table::BuildSelection;
use crate::validation::ValidationReport;

/// Native project service used by native composition roots.
//
// WHY: project discovery, filesystem reads, and native path resolution are
// host I/O responsibilities. Naming this boundary explicitly prevents a
// future Browser Host from treating native project access as pure semantics.
// IF REMOVED: native filesystem authority can leak into shared/browser
// preparation and create a second, incompatible project-loading path.
// EVIDENCE: docs/specs/runtime-hosts.md; docs/adr/0006-host-capability-composition.md
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeProjectService;

impl NativeProjectService {
    pub fn new() -> Self {
        Self
    }

    pub fn project_info(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ProjectInfo> {
        Project::discover(explicit_project, current_dir).map(|project| project.info())
    }

    pub fn validate(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ValidationReport> {
        Project::discover(explicit_project, current_dir)?.validate()
    }

    pub fn prepare_build(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<BuildPlan> {
        let project = Project::discover(explicit_project, current_dir)?;
        let documents = project.load_documents()?;
        prepare_build_from_documents(project.info(), documents, &BuildSelection::unfiltered())
    }

    pub fn prepare_build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
    ) -> Result<BuildPlan> {
        let project = Project::discover(explicit_project, current_dir)?;
        let documents = project.load_documents()?;
        prepare_build_from_documents(project.info(), documents, selection)
    }

    /// Prepare a frontend-independent migration from a native project snapshot.
    pub fn prepare_migration(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
    ) -> Result<MigrationDryRun> {
        let project = Project::discover(explicit_project, current_dir)?;
        let documents = project.load_documents()?;
        dry_run_migration(&documents, command)
    }

    /// Commit a verified migration against the exact snapshot used to prepare it.
    ///
    /// The source commit is intentionally separate from semantic preparation;
    /// it does not trigger build, generated C# publication, or binary output.
    pub fn commit_migration(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        self.commit_migration_with_failures(explicit_project, current_dir, command, &[])
    }

    /// Deterministic failure seam for source-commit transaction regression tests.
    #[doc(hidden)]
    pub fn commit_migration_with_failures(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
        injections: &[MigrationCommitFailureInjection],
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        let project = match Project::discover(explicit_project, current_dir) {
            Ok(project) => project,
            Err(error) => return Err(MigrationCommitFailure::not_started(error)),
        };
        let documents = match project.load_documents() {
            Ok(documents) => documents,
            Err(error) => return Err(MigrationCommitFailure::not_started(error)),
        };
        let dry_run = match dry_run_migration(&documents, command) {
            Ok(dry_run) => dry_run,
            Err(error) => return Err(MigrationCommitFailure::not_started(error)),
        };
        commit_migration_with_failures(&project, &documents, &dry_run, injections)
    }

    /// Commit a caller-provided, already verified source snapshot and dry-run.
    pub fn commit_migration_snapshot(
        &self,
        project: &Project,
        source_snapshot: &crate::ProjectDocuments,
        dry_run: &MigrationDryRun,
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        commit_migration(project, source_snapshot, dry_run)
    }

    pub fn init(&self, root: &Path, options: &InitOptions) -> Result<ProjectInfo> {
        initialize_project(root, options)
    }
}

/// Compatibility name for existing core consumers.
pub type ProjectService = NativeProjectService;
