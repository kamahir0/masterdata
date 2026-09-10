//! Shared application workflows for adapters such as the CLI and Tauri GUI.
//!
//! This crate orchestrates project discovery, validation, C# generation, and
//! the .NET boundary. Domain semantics remain in masterdata-core, C# syntax
//! remains in masterdata-codegen-csharp, and process invocation remains in
//! masterdata-dotnet.

use std::fs;
use std::path::{Path, PathBuf};

use masterdata_codegen_csharp::{CSharpGenerationPlan, CSharpGenerator};
use masterdata_core::{
    BuildPlan, BuildSelection, ErrorKind, InitOptions, MasterdataError, MigrationCommand,
    MigrationCommitFailure, MigrationCommitFailureInjection, MigrationCommitReport,
    MigrationDryRun, NativeProjectService, ProjectDocuments, ProjectInfo, Result, ValidationReport,
};
use masterdata_dotnet::{
    BridgeSmokeReport, DotnetBridge, MasterMemoryBuildReport, MasterMemoryBuildRequest,
    MasterMemorySpikeReport,
};
use tempfile::TempDir;

mod authoring;
mod publish;
mod receipt;

pub use authoring::*;
pub use publish::{
    BinaryPublishPreflight, CSharpPublishPreflight, PUBLISH_MANIFEST_FILENAME,
    PublishExecutionFailure, PublishExecutionReport, PublishFailureInjection, PublishFailurePoint,
    PublishPreflightPlan, PublishTargetPreflight, PublishTargetResult, PublishTargetStatus,
    preflight_publish,
};
pub use receipt::{
    ARTIFACT_HASH_ALGORITHM, ARTIFACT_RECEIPT_FILENAME, ARTIFACT_SET_RECEIPT_FILENAME,
    ARTIFACT_SET_RECEIPT_VERSION, ArtifactReceiptEntry, ArtifactSetReceipt, CANONICAL_BINARY_PATH,
    ValidatedArtifact, ValidatedArtifactSet, create_artifact_set_receipt, validate_artifact_set,
    write_artifact_set_receipt,
};

#[derive(Debug, Clone)]
pub struct NativeApplicationService {
    project: NativeProjectService,
    generator: CSharpGenerator,
    dotnet: DotnetBridge,
}

// WHY: CLI, Tauri, and a future Connected Web adapter must use one native
// application workflow while the CLI remains direct and in-process.
// IF REMOVED: host adapters can fork build/.NET/artifact semantics or force
// the CLI through a transport boundary.
// EVIDENCE: docs/specs/runtime-hosts.md; docs/adr/0006-host-capability-composition.md
impl Default for NativeApplicationService {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeApplicationService {
    pub fn new() -> Self {
        Self {
            project: NativeProjectService::new(),
            generator: CSharpGenerator::default(),
            dotnet: DotnetBridge::default(),
        }
    }

    pub fn with_dotnet(dotnet: DotnetBridge) -> Self {
        Self {
            project: NativeProjectService::new(),
            generator: CSharpGenerator::default(),
            dotnet,
        }
    }

    pub fn project_info(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ProjectInfo> {
        self.project.project_info(explicit_project, current_dir)
    }

    pub fn validate(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ValidationReport> {
        self.project.validate(explicit_project, current_dir)
    }

    pub fn init(&self, root: &Path, options: &InitOptions) -> Result<ProjectInfo> {
        self.project.init(root, options)
    }

    pub fn prepare_build(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<BuildPlan> {
        self.project.prepare_build(explicit_project, current_dir)
    }

    pub fn prepare_build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
    ) -> Result<BuildPlan> {
        self.project
            .prepare_build_with_selection(explicit_project, current_dir, selection)
    }

    pub fn prepare_migration(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
    ) -> Result<MigrationDryRun> {
        self.project
            .prepare_migration(explicit_project, current_dir, command)
    }

    pub fn commit_migration(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        self.project
            .commit_migration(explicit_project, current_dir, command)
    }

    #[doc(hidden)]
    pub fn commit_migration_with_failures(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        command: &MigrationCommand,
        injections: &[MigrationCommitFailureInjection],
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        self.project.commit_migration_with_failures(
            explicit_project,
            current_dir,
            command,
            injections,
        )
    }

    pub fn commit_migration_snapshot(
        &self,
        project: &masterdata_core::Project,
        source_snapshot: &ProjectDocuments,
        dry_run: &MigrationDryRun,
    ) -> std::result::Result<MigrationCommitReport, MigrationCommitFailure> {
        self.project
            .commit_migration_snapshot(project, source_snapshot, dry_run)
    }

    pub fn validate_artifact_set(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ValidatedArtifactSet> {
        let info = self.project.project_info(explicit_project, current_dir)?;
        receipt::validate_artifact_set(&info.artifact_root, &info.project_id)
    }

    pub fn preflight_publish(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<PublishPreflightPlan> {
        let info = self.project.project_info(explicit_project, current_dir)?;
        let artifacts = receipt::validate_artifact_set(&info.artifact_root, &info.project_id)?;
        publish::preflight_publish(&info, artifacts)
    }

    pub fn publish(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> std::result::Result<PublishExecutionReport, PublishExecutionFailure> {
        self.publish_with_failures(explicit_project, current_dir, &[])
    }

    #[doc(hidden)]
    pub fn publish_with_failures(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        injections: &[PublishFailureInjection],
    ) -> std::result::Result<PublishExecutionReport, PublishExecutionFailure> {
        let info = match self.project.project_info(explicit_project, current_dir) {
            Ok(info) => info,
            Err(error) => {
                return Err(PublishExecutionFailure {
                    report: PublishExecutionReport { targets: Vec::new() },
                    error,
                });
            }
        };
        let report = publish::report_for_project(&info, publish::PublishTargetStatus::NotAttempted, None);
        let artifacts = match receipt::validate_artifact_set(&info.artifact_root, &info.project_id) {
            Ok(artifacts) => artifacts,
            Err(error) => return Err(PublishExecutionFailure { report, error }),
        };
        let plan = match publish::preflight_publish(&info, artifacts) {
            Ok(plan) => plan,
            Err(error) => return Err(PublishExecutionFailure { report, error }),
        };
        publish::execute_publish_plan(&info, plan, injections)
    }

    pub fn build_and_publish(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> std::result::Result<BuildAndPublishExecution, BuildAndPublishFailure> {
        let build = self
            .build(explicit_project, current_dir, false)
            .map_err(BuildAndPublishFailure::Build)?;
        let publish = match self.publish(explicit_project, current_dir) {
            Ok(report) => report,
            Err(failure) => {
                return Err(BuildAndPublishFailure::Publish {
                    build: Box::new(build),
                    failure,
                });
            }
        };
        Ok(BuildAndPublishExecution { build, publish })
    }

    pub fn plan_csharp(&self, plan: &BuildPlan) -> Result<CSharpGenerationPlan> {
        self.generator.plan(plan)
    }

    pub fn build(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        dry_run: bool,
    ) -> Result<BuildExecution> {
        self.build_with_selection(
            explicit_project,
            current_dir,
            &BuildSelection::unfiltered(),
            dry_run,
        )
    }

    pub fn build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
        dry_run: bool,
    ) -> Result<BuildExecution> {
        let plan = self.prepare_build_with_selection(explicit_project, current_dir, selection)?;
        let generation = self.generator.plan(&plan)?;
        let (written_files, binary) = if dry_run {
            (Vec::new(), None)
        } else {
            let final_artifact_root = &plan.artifact_root;
            let artifact_parent = final_artifact_root.parent().ok_or_else(|| {
                canonical_publish_error(final_artifact_root, "canonical artifact root has no parent directory")
            })?;
            validate_existing_artifact_root(final_artifact_root)?;
            fs::create_dir_all(artifact_parent).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-CANONICAL-STAGING",
                    ErrorKind::Io,
                    format!("could not create canonical artifact parent: {error}"),
                )
                .with_source(artifact_parent.to_path_buf())
            })?;
            let workspace = TempDir::new_in(artifact_parent).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-CANONICAL-STAGING",
                    ErrorKind::Io,
                    format!("could not create canonical build staging workspace: {error}"),
                )
                .with_source(artifact_parent.to_path_buf())
            })?;
            let staged_csharp = workspace.path().join("Generated");
            self.generator.write_to(&generation, &staged_csharp)?;
            let staged_binary = workspace.path().join("masterdata.bytes");
            let request = MasterMemoryBuildRequest::from_plan(&plan, &generation, staged_binary.clone())?;
            let mut report = self.dotnet.build_mastermemory(&request, &staged_csharp, workspace.path())?;
            let staged_root = workspace.path().join("output");
            fs::create_dir(&staged_root).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-CANONICAL-STAGING",
                    ErrorKind::Io,
                    format!("could not create staged canonical artifact root: {error}"),
                )
                .with_source(staged_root.clone())
            })?;
            let staged_root_csharp = staged_root.join("csharp");
            fs::rename(&staged_csharp, &staged_root_csharp).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-CANONICAL-STAGING",
                    ErrorKind::Io,
                    format!("could not assemble staged canonical C# output: {error}"),
                )
                .with_source(staged_root_csharp.clone())
            })?;
            let staged_root_binary = staged_root.join("masterdata.bytes");
            fs::rename(&staged_binary, &staged_root_binary).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-CANONICAL-STAGING",
                    ErrorKind::Io,
                    format!("could not assemble staged canonical binary: {error}"),
                )
                .with_source(staged_root_binary.clone())
            })?;
            publish_staged_artifact_set(
                &staged_root,
                final_artifact_root,
                &plan.project.project_id,
                receipt::write_artifact_set_receipt,
            )?;
            report.binary_path = plan.binary_output.clone();
            let written_files = generation
                .files
                .iter()
                .map(|file| plan.csharp_output.join(&file.relative_path))
                .collect();
            (written_files, Some(report))
        };
        Ok(BuildExecution {
            plan,
            generation,
            written_files,
            binary,
        })
    }

    pub fn bridge_smoke_test(&self, repository_root: &Path) -> Result<BridgeSmokeReport> {
        self.dotnet.smoke_test(repository_root)
    }

    pub fn mastermemory_spike(&self, repository_root: &Path) -> Result<MasterMemorySpikeReport> {
        self.dotnet.mastermemory_spike(repository_root)
    }
}

pub type ApplicationService = NativeApplicationService;

#[derive(Debug, Clone)]
pub struct BuildExecution {
    pub plan: BuildPlan,
    pub generation: CSharpGenerationPlan,
    pub written_files: Vec<PathBuf>,
    pub binary: Option<MasterMemoryBuildReport>,
}

#[derive(Debug)]
pub struct BuildAndPublishExecution {
    pub build: BuildExecution,
    pub publish: PublishExecutionReport,
}

#[derive(Debug)]
pub enum BuildAndPublishFailure {
    Build(MasterdataError),
    Publish {
        build: Box<BuildExecution>,
        failure: PublishExecutionFailure,
    },
}

impl BuildAndPublishFailure {
    pub fn into_error(self) -> MasterdataError {
        match self {
            Self::Build(error) => error,
            Self::Publish { failure, .. } => failure.error,
        }
    }
}

fn validate_existing_artifact_root(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(canonical_publish_error(path, "canonical artifact root is a symlink")),
        Ok(metadata) if !metadata.file_type().is_dir() => Err(canonical_publish_error(path, "canonical artifact root is not a directory")),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(canonical_publish_error(path, format!("could not inspect canonical artifact root: {error}"))),
    }
}

fn validate_staged_artifact_root(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| canonical_publish_error(path, format!("could not inspect staged canonical artifact root: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(canonical_publish_error(path, "staged canonical artifact root is not a real directory"));
    }
    let csharp = path.join("csharp");
    let csharp_metadata = fs::symlink_metadata(&csharp).map_err(|error| canonical_publish_error(&csharp, format!("staged canonical C# output is missing: {error}")))?;
    if csharp_metadata.file_type().is_symlink() || !csharp_metadata.file_type().is_dir() {
        return Err(canonical_publish_error(&csharp, "staged canonical C# output is not a real directory"));
    }
    let binary = path.join("masterdata.bytes");
    let binary_metadata = fs::symlink_metadata(&binary).map_err(|error| canonical_publish_error(&binary, format!("staged canonical binary is missing: {error}")))?;
    if binary_metadata.file_type().is_symlink() || !binary_metadata.file_type().is_file() {
        return Err(canonical_publish_error(&binary, "staged canonical binary is not a regular file"));
    }
    Ok(())
}

fn publish_staged_artifact_set<F>(staged_root: &Path, final_artifact_root: &Path, project_id: &str, write_receipt: F) -> Result<()>
where
    F: Fn(&Path, &str) -> Result<receipt::ArtifactSetReceipt>,
{
    validate_staged_artifact_root(staged_root)?;
    write_receipt(staged_root, project_id)?;
    receipt::validate_artifact_set(staged_root, project_id)?;
    publish_canonical_artifact_root(staged_root, final_artifact_root)
}

fn publish_canonical_artifact_root(staged: &Path, final_path: &Path) -> Result<()> {
    validate_staged_artifact_root(staged)?;
    validate_existing_artifact_root(final_path)?;
    let parent = final_path.parent().ok_or_else(|| canonical_publish_error(final_path, "canonical artifact root has no parent directory"))?;
    let backup_root = TempDir::new_in(parent).map_err(|error| canonical_publish_error(parent, format!("could not create canonical artifact rollback area: {error}")))?;
    let previous = backup_root.path().join("previous");
    let had_previous = match fs::symlink_metadata(final_path) {
        Ok(_) => {
            fs::rename(final_path, &previous).map_err(|error| canonical_publish_error(final_path, format!("could not stage previous canonical artifacts: {error}")))?;
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(canonical_publish_error(final_path, format!("could not inspect canonical artifact root: {error}"))),
    };
    if let Err(error) = fs::rename(staged, final_path) {
        let publication_error = canonical_publish_error(final_path, format!("could not publish canonical artifact root: {error}"));
        if had_previous && let Err(restore_error) = fs::rename(&previous, final_path) {
            let retained = backup_root.keep();
            return Err(combine_publication_errors(publication_error, format!("could not restore previous canonical artifacts: {restore_error}"), &retained));
        }
        return Err(publication_error);
    }
    Ok(())
}

fn canonical_publish_error(path: &Path, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new("E-BUILD-CANONICAL-PUBLISH", ErrorKind::Io, message).with_source(path.to_path_buf())
}

fn combine_publication_errors(original: MasterdataError, rollback_message: String, rollback_area: &Path) -> MasterdataError {
    MasterdataError::new(
        "E-BUILD-ARTIFACT-ROLLBACK",
        ErrorKind::Io,
        format!(
            "canonical artifact publication failed [{}]: {}; rollback failed: {}; rollback area retained at {}",
            original.diagnostic().code,
            original.diagnostic().message,
            rollback_message,
            rollback_area.display(),
        ),
    )
    .with_source(rollback_area.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use masterdata_core::{ErrorKind, MasterdataError};
    use tempfile::tempdir;
    use super::{publish_canonical_artifact_root, publish_staged_artifact_set, validate_existing_artifact_root};

    fn staged_root(root: &std::path::Path, marker: &[u8]) -> std::path::PathBuf {
        let staged = root.join("staged");
        fs::create_dir_all(staged.join("csharp")).expect("staged C# directory");
        fs::write(staged.join("csharp/Item.g.cs"), marker).expect("staged C# file");
        fs::write(staged.join("masterdata.bytes"), marker).expect("staged binary");
        staged
    }

    #[test]
    fn canonical_publication_replaces_complete_artifact_root() {
        let directory = tempdir().expect("temporary directory");
        let final_root = directory.path().join("output");
        fs::create_dir_all(final_root.join("csharp")).expect("final C# directory");
        fs::write(final_root.join("csharp/Old.g.cs"), b"OLD").expect("old C# file");
        fs::write(final_root.join("masterdata.bytes"), b"OLD").expect("old binary");
        let staged = staged_root(directory.path(), b"NEW");
        publish_canonical_artifact_root(&staged, &final_root).expect("publication");
        assert!(!final_root.join("csharp/Old.g.cs").exists());
        assert_eq!(fs::read(final_root.join("csharp/Item.g.cs")).unwrap(), b"NEW");
        assert_eq!(fs::read(final_root.join("masterdata.bytes")).unwrap(), b"NEW");
    }

    #[test]
    fn canonical_publication_failure_preserves_previous_root() {
        let directory = tempdir().expect("temporary directory");
        let final_root = directory.path().join("output");
        fs::create_dir_all(final_root.join("csharp")).expect("final C# directory");
        fs::write(final_root.join("csharp/Old.g.cs"), b"OLD").expect("old C# file");
        fs::write(final_root.join("masterdata.bytes"), b"OLD").expect("old binary");
        let missing_staged = directory.path().join("missing-staged");
        let error = publish_canonical_artifact_root(&missing_staged, &final_root).expect_err("missing staged root");
        assert_eq!(error.diagnostic().code, "E-BUILD-CANONICAL-PUBLISH");
        assert_eq!(fs::read(final_root.join("csharp/Old.g.cs")).unwrap(), b"OLD");
        assert_eq!(fs::read(final_root.join("masterdata.bytes")).unwrap(), b"OLD");
    }

    #[test]
    fn canonical_publication_rejects_existing_file_root() {
        let directory = tempdir().expect("temporary directory");
        let final_root = directory.path().join("output");
        fs::write(&final_root, b"not a directory").expect("file root");
        let error = validate_existing_artifact_root(&final_root).expect_err("file root");
        assert_eq!(error.diagnostic().code, "E-BUILD-CANONICAL-PUBLISH");
    }

    #[test]
    fn receipt_generation_failure_preserves_previous_set() {
        let directory = tempdir().expect("temporary directory");
        let final_root = directory.path().join("output");
        fs::create_dir_all(final_root.join("csharp")).expect("final C# directory");
        fs::write(final_root.join("csharp/Old.g.cs"), b"OLD").expect("old C# file");
        fs::write(final_root.join("masterdata.bytes"), b"OLD").expect("old binary");
        super::receipt::write_artifact_set_receipt(&final_root, "fixture").expect("old receipt");
        let before = snapshot_directory(&final_root);
        let staged = staged_root(directory.path(), b"NEW");
        let error = publish_staged_artifact_set(&staged, &final_root, "fixture", |_root, _id| {
            Err(MasterdataError::new("E-TEST-RECEIPT-GENERATION", ErrorKind::Io, "injected receipt generation failure"))
        }).expect_err("receipt generation failure");
        assert_eq!(error.diagnostic().code, "E-TEST-RECEIPT-GENERATION");
        assert_eq!(snapshot_directory(&final_root), before);
    }

    fn snapshot_directory(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        let mut snapshot = std::collections::BTreeMap::new();
        snapshot_directory_recursive(root, root, &mut snapshot);
        snapshot
    }

    fn snapshot_directory_recursive(root: &std::path::Path, directory: &std::path::Path, snapshot: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).expect("snapshot entry") {
            let entry = entry.expect("snapshot directory entry");
            let path = entry.path();
            if path.is_dir() {
                snapshot_directory_recursive(root, &path, snapshot);
            } else {
                let relative = path.strip_prefix(root).expect("snapshot relative path");
                snapshot.insert(relative.to_path_buf(), fs::read(path).expect("snapshot file"));
            }
        }
    }
}
