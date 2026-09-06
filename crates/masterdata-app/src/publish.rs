use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::{fmt, io};

use masterdata_core::{
    Diagnostic, ErrorKind, MasterdataError, ProjectInfo, PublishTargetInfo, PublishTargetKind,
    Result,
};
use same_file::is_same_file;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::{Builder, TempDir};

use crate::receipt::ValidatedArtifactSet;

pub const PUBLISH_MANIFEST_FILENAME: &str = ".masterdata-publish-manifest.json";

const PUBLISH_MANIFEST_VERSION: u32 = 1;

/// Read-only information needed by a later publish execution phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPreflightPlan {
    pub artifact_set: ValidatedArtifactSet,
    pub targets: Vec<PublishTargetPreflight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishTargetStatus {
    NotAttempted,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishTargetResult {
    pub index: usize,
    pub kind: PublishTargetKind,
    pub configured_path: String,
    pub destination: PathBuf,
    pub status: PublishTargetStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishExecutionReport {
    pub targets: Vec<PublishTargetResult>,
}

#[derive(Debug, Clone)]
pub struct PublishExecutionFailure {
    pub report: PublishExecutionReport,
    pub error: MasterdataError,
}

impl PublishExecutionFailure {
    pub fn report(&self) -> &PublishExecutionReport {
        &self.report
    }

    pub fn diagnostic(&self) -> &Diagnostic {
        self.error.diagnostic()
    }
}

impl fmt::Display for PublishExecutionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for PublishExecutionFailure {}

/// Deterministic failure and filesystem-mutation points used by
/// application-layer execution tests.
/// The normal [`NativeApplicationService`](crate::NativeApplicationService)
/// workflow passes no injections and has no synthetic failure behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishFailurePoint {
    ToctouBeforeTarget,
    MutateProtectedRegionBeforeTarget,
    AliasProtectedRegionBeforeTarget,
    CSharpWhileStagingNew,
    CSharpAfterManagedReplacement,
    CSharpWhileRetiringStale,
    CSharpBeforeManifest,
    CSharpWhilePublishingManifest,
    CSharpRollback,
    BinaryBeforePublication,
    BinaryAfterPreviousSecured,
    BinaryWhilePublishingNew,
    BinaryAfterPublication,
    BinaryRollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishFailureInjection {
    pub target_index: usize,
    pub point: PublishFailurePoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishTargetPreflight {
    pub kind: PublishTargetKind,
    pub configured_path: String,
    pub destination: PathBuf,
    pub csharp: Option<CSharpPublishPreflight>,
    pub binary: Option<BinaryPublishPreflight>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpPublishPreflight {
    pub manifest_path: PathBuf,
    pub manifest_exists: bool,
    pub previous_managed_paths: Vec<String>,
    pub current_generated_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryPublishPreflight {
    pub destination: PathBuf,
    pub existing_regular_file: bool,
}

#[derive(Debug, Clone)]
struct TargetAnalysis {
    namespace: ResolvedPath,
    plan: PublishTargetPreflight,
    csharp: Option<CSharpTargetAnalysis>,
}

#[derive(Debug, Clone)]
struct CSharpTargetAnalysis {
    plan: CSharpPublishPreflight,
    previous: Vec<ManifestPath>,
    current: Vec<ManifestPath>,
}

#[derive(Debug, Clone)]
struct ResolvedPath {
    logical_path: PathBuf,
    canonical_path: PathBuf,
    existing_prefix: PathBuf,
    missing_tail: Vec<OsString>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamespaceRelation {
    Disjoint,
    Same,
    LeftAncestor,
    RightAncestor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtectedRegionKind {
    ProjectRoot,
    CriticalTree,
}

#[derive(Debug, Clone, Copy)]
enum SymlinkPolicy<'a> {
    Reject,
    ProjectRelativeToRoot(&'a Path),
}

#[derive(Debug, Clone)]
struct ProtectedRegion {
    path: ResolvedPath,
    label: String,
    kind: ProtectedRegionKind,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishManifest {
    version: u32,
    files: Vec<String>,
}

/// Preflight all configured publish targets after the canonical artifact set
/// has already been validated by the application service.
pub fn preflight_publish(
    project: &ProjectInfo,
    artifacts: ValidatedArtifactSet,
) -> Result<PublishPreflightPlan> {
    if project.publish_targets.is_empty() {
        return Ok(PublishPreflightPlan {
            artifact_set: artifacts,
            targets: Vec::new(),
        });
    }

    let protected_regions = protected_regions(project)?;
    let mut analyses = Vec::with_capacity(project.publish_targets.len());
    let mut first_error = None;

    for target in &project.publish_targets {
        match preflight_target(project, &artifacts, target, &protected_regions) {
            Ok(analysis) => analyses.push(analysis),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    for (index, left) in analyses.iter().enumerate() {
        for right in analyses.iter().skip(index + 1) {
            if namespace_relation(&left.namespace, &right.namespace)? != NamespaceRelation::Disjoint
            {
                return Err(publish_error(
                    "E-PUBLISH-TARGET-COLLISION",
                    ErrorKind::Validation,
                    &right.plan.destination,
                    format!(
                        "publish targets `{}` and `{}` overlap in the destination filesystem",
                        left.plan.destination.display(),
                        right.plan.destination.display()
                    ),
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-008", "PUBLISH-EXEC-001"],
                ));
            }
        }
    }

    Ok(PublishPreflightPlan {
        artifact_set: artifacts,
        targets: analyses.into_iter().map(|analysis| analysis.plan).collect(),
    })
}

pub(crate) fn execute_publish_plan(
    project: &ProjectInfo,
    plan: PublishPreflightPlan,
    injections: &[PublishFailureInjection],
) -> std::result::Result<PublishExecutionReport, PublishExecutionFailure> {
    let mut report = report_for_project(project, PublishTargetStatus::NotAttempted, None);
    if project.publish_targets.len() != plan.targets.len() {
        let error = publish_error(
            "E-PUBLISH-EXECUTION-PLAN",
            ErrorKind::Validation,
            &project.project_root,
            "publish preflight plan does not match the configured target order",
            &["PUBLISH-EXEC-001", "PUBLISH-EXEC-002"],
        );
        return Err(PublishExecutionFailure { report, error });
    }
    if project.publish_targets.is_empty() {
        return Ok(report);
    }

    // WHY: report every configured target in order and continue after a
    // target-local failure, because the approved execution contract has no
    // cross-target transaction or global rollback.
    // IF REMOVED: the first failure would hide later outcomes and callers
    // could not distinguish partial success from a preflight failure.
    // EVIDENCE: docs/specs/build-pipeline.md; Regression: execution_failure_continues_to_later_targets; publish_reports_per_target_status; successful_target_is_not_rolled_back_by_later_failure.
    for (index, (target, expected_plan)) in project
        .publish_targets
        .iter()
        .zip(plan.targets.iter())
        .enumerate()
    {
        let mutation = match begin_protected_region_mutation(project, target, index, injections) {
            Ok(mutation) => mutation,
            Err(error) => {
                let target_result = &mut report.targets[index];
                target_result.status = PublishTargetStatus::Failed;
                target_result.failure = Some(error.diagnostic().clone());
                continue;
            }
        };
        let failure = if failure_injected(
            injections,
            index,
            PublishFailurePoint::ToctouBeforeTarget,
        ) {
            Some(publish_error(
                "E-PUBLISH-TOCTOU",
                ErrorKind::Validation,
                &target.resolved_path,
                "publish target changed between preflight and execution",
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            ))
        } else {
            // WHY: Phase 3 runs after Phase 2 has completed, so a protected
            // region can change between target attempts. Resolve it for each
            // target instead of treating the Phase 2 snapshot as a global
            // trust decision.
            // IF REMOVED: a later target could publish through a protected
            // region that became aliased or otherwise unsafe after an earlier
            // target succeeded.
            // EVIDENCE: docs/specs/build-pipeline.md; Regression: protected_region_revalidation_failure_is_target_local_and_continues; protected_region_overlap_after_phase2_is_target_local.
            protected_regions(project)
                .and_then(|protected_regions| {
                    validate_artifact_bytes(&plan.artifact_set)?;
                    let analysis = preflight_target(
                        project,
                        &plan.artifact_set,
                        target,
                        &protected_regions,
                    )?;
                    if analysis.plan != *expected_plan {
                        return Err(publish_error(
                            "E-PUBLISH-TOCTOU",
                            ErrorKind::Validation,
                            &target.resolved_path,
                            "publish target ownership or namespace changed between preflight and execution",
                            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                        ));
                    }
                    let context = ExecutionContext {
                        project,
                        artifacts: &plan.artifact_set,
                        injections,
                        target_index: index,
                    };
                    execute_target(&context, target, expected_plan, analysis)
                })
                .err()
        };
        let failure = restore_protected_region_mutation(mutation, failure);

        let target_result = &mut report.targets[index];
        match failure {
            Some(error) => {
                target_result.status = PublishTargetStatus::Failed;
                target_result.failure = Some(error.diagnostic().clone());
            }
            None => target_result.status = PublishTargetStatus::Succeeded,
        }
    }

    if report
        .targets
        .iter()
        .any(|target| target.status == PublishTargetStatus::Failed)
    {
        let error = aggregate_execution_error(&report);
        Err(PublishExecutionFailure { report, error })
    } else {
        Ok(report)
    }
}

pub(crate) fn report_for_project(
    project: &ProjectInfo,
    status: PublishTargetStatus,
    failure: Option<&Diagnostic>,
) -> PublishExecutionReport {
    PublishExecutionReport {
        targets: project
            .publish_targets
            .iter()
            .enumerate()
            .map(|(index, target)| PublishTargetResult {
                index,
                kind: target.kind,
                configured_path: target.path.clone(),
                destination: target.resolved_path.clone(),
                status,
                failure: failure.cloned(),
            })
            .collect(),
    }
}

fn failure_injected(
    injections: &[PublishFailureInjection],
    target_index: usize,
    point: PublishFailurePoint,
) -> bool {
    injections
        .iter()
        .any(|injection| injection.target_index == target_index && injection.point == point)
}

// WHY: the protected-region regressions must change the filesystem state that
// Phase 3 resolves, rather than merely returning a synthetic TOCTOU error.
// IF REMOVED: tests could pass while execution still reused a stale protected
// region snapshot.
// EVIDENCE: docs/specs/build-pipeline.md; Regression: protected_region_revalidation_failure_is_target_local_and_continues; protected_region_overlap_after_phase2_is_target_local.
struct ProtectedRegionMutation {
    replaced_path: PathBuf,
    backup_path: PathBuf,
}

impl ProtectedRegionMutation {
    fn restore(self) -> Result<()> {
        let metadata = fs::symlink_metadata(&self.replaced_path).map_err(|error| {
            publish_io_error(
                &self.replaced_path,
                format!(
                    "could not inspect the protected-region test mutation during restore: {error}"
                ),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            return Err(publish_error(
                "E-PUBLISH-ROLLBACK-FAILED",
                ErrorKind::Io,
                &self.replaced_path,
                "protected-region test mutation was replaced by a directory before restore",
                &["PUBLISH-EXEC-003"],
            ));
        }
        fs::remove_file(&self.replaced_path).map_err(|error| {
            publish_io_error(
                &self.replaced_path,
                format!(
                    "could not remove the protected-region test mutation during restore: {error}"
                ),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;
        fs::rename(&self.backup_path, &self.replaced_path).map_err(|error| {
            publish_io_error(
                &self.replaced_path,
                format!("could not restore the protected-region test mutation: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })
    }
}

fn begin_protected_region_mutation(
    project: &ProjectInfo,
    target: &PublishTargetInfo,
    target_index: usize,
    injections: &[PublishFailureInjection],
) -> Result<Option<ProtectedRegionMutation>> {
    if failure_injected(
        injections,
        target_index,
        PublishFailurePoint::MutateProtectedRegionBeforeTarget,
    ) {
        return begin_missing_parent_mutation(project, target_index).map(Some);
    }
    if failure_injected(
        injections,
        target_index,
        PublishFailurePoint::AliasProtectedRegionBeforeTarget,
    ) {
        return begin_alias_mutation(project, target, target_index).map(Some);
    }
    Ok(None)
}

fn restore_protected_region_mutation(
    mutation: Option<ProtectedRegionMutation>,
    failure: Option<MasterdataError>,
) -> Option<MasterdataError> {
    let Some(mutation) = mutation else {
        return failure;
    };
    match mutation.restore() {
        Ok(()) => failure,
        Err(restore) => match failure {
            Some(original) => Some(combine_target_rollback_error(original, restore, None)),
            None => Some(restore),
        },
    }
}

fn protected_region_for_test(project: &ProjectInfo) -> Result<&Path> {
    project
        .source_roots
        .first()
        .map(PathBuf::as_path)
        .ok_or_else(|| {
            injected_failure(
                &project.project_root,
                "protected-region test mutation requires a configured source root",
                "PUBLISH-EXEC-002",
            )
        })
}

fn protected_region_test_backup(project: &ProjectInfo, target_index: usize) -> Result<PathBuf> {
    let backup = project.project_root.join(format!(
        ".masterdata-publish-protected-region-{target_index}"
    ));
    match fs::symlink_metadata(&backup) {
        Ok(_) => Err(injected_failure(
            &backup,
            "protected-region test mutation backup already exists",
            "PUBLISH-EXEC-002",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(backup),
        Err(error) => Err(publish_io_error(
            &backup,
            format!("could not inspect protected-region test mutation backup: {error}"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )),
    }
}

fn begin_missing_parent_mutation(
    project: &ProjectInfo,
    target_index: usize,
) -> Result<ProtectedRegionMutation> {
    let source_root = protected_region_for_test(project)?;
    let parent = source_root.parent().ok_or_else(|| {
        injected_failure(
            source_root,
            "protected-region test mutation source root has no parent",
            "PUBLISH-EXEC-002",
        )
    })?;
    if parent == project.project_root || !parent.starts_with(&project.project_root) {
        return Err(injected_failure(
            source_root,
            "protected-region test mutation requires a nested source root",
            "PUBLISH-EXEC-002",
        ));
    }
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        publish_io_error(
            parent,
            format!("could not inspect protected-region test mutation parent: {error}"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(injected_failure(
            parent,
            "protected-region test mutation parent is not a real directory",
            "PUBLISH-EXEC-002",
        ));
    }
    let backup = protected_region_test_backup(project, target_index)?;
    fs::rename(parent, &backup).map_err(|error| {
        publish_io_error(
            parent,
            format!("could not stage protected-region test mutation: {error}"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )
    })?;
    if let Err(error) = fs::write(parent, b"protected-region mutation") {
        let restore = fs::rename(&backup, parent);
        return Err(match restore {
            Ok(()) => publish_io_error(
                parent,
                format!("could not create protected-region test mutation: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            ),
            Err(restore) => combine_target_rollback_error(
                publish_io_error(
                    parent,
                    format!("could not create protected-region test mutation: {error}"),
                    &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                ),
                publish_io_error(
                    parent,
                    format!("could not restore protected-region test mutation: {restore}"),
                    &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                ),
                None,
            ),
        });
    }
    Ok(ProtectedRegionMutation {
        replaced_path: parent.to_path_buf(),
        backup_path: backup,
    })
}

fn begin_alias_mutation(
    project: &ProjectInfo,
    _target: &PublishTargetInfo,
    target_index: usize,
) -> Result<ProtectedRegionMutation> {
    let source_root = protected_region_for_test(project)?;
    if !source_root.starts_with(&project.project_root) {
        return Err(injected_failure(
            source_root,
            "protected-region alias test requires a project-local source root",
            "PUBLISH-EXEC-002",
        ));
    }
    let metadata = fs::symlink_metadata(source_root).map_err(|error| {
        publish_io_error(
            source_root,
            format!("could not inspect protected-region alias test source root: {error}"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(injected_failure(
            source_root,
            "protected-region alias test source root is not a real directory",
            "PUBLISH-EXEC-002",
        ));
    }
    let backup = protected_region_test_backup(project, target_index)?;
    fs::rename(source_root, &backup).map_err(|error| {
        publish_io_error(
            source_root,
            format!("could not stage protected-region alias test: {error}"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        if let Err(error) = symlink(&_target.resolved_path, source_root) {
            let restore = fs::rename(&backup, source_root);
            return Err(match restore {
                Ok(()) => publish_io_error(
                    source_root,
                    format!("could not create protected-region alias test: {error}"),
                    &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                ),
                Err(restore) => combine_target_rollback_error(
                    publish_io_error(
                        source_root,
                        format!("could not create protected-region alias test: {error}"),
                        &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                    ),
                    publish_io_error(
                        source_root,
                        format!("could not restore protected-region alias test: {restore}"),
                        &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                    ),
                    None,
                ),
            });
        }
    }

    #[cfg(not(unix))]
    {
        let restore = fs::rename(&backup, source_root);
        return Err(match restore {
            Ok(()) => injected_failure(
                source_root,
                "protected-region alias test is only available on Unix",
                "PUBLISH-EXEC-002",
            ),
            Err(restore) => combine_target_rollback_error(
                injected_failure(
                    source_root,
                    "protected-region alias test is only available on Unix",
                    "PUBLISH-EXEC-002",
                ),
                publish_io_error(
                    source_root,
                    format!("could not restore protected-region alias test: {restore}"),
                    &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                ),
                None,
            ),
        });
    }

    Ok(ProtectedRegionMutation {
        replaced_path: source_root.to_path_buf(),
        backup_path: backup,
    })
}

fn aggregate_execution_error(report: &PublishExecutionReport) -> MasterdataError {
    let failed = report
        .targets
        .iter()
        .filter(|target| target.status == PublishTargetStatus::Failed)
        .collect::<Vec<_>>();
    let first = failed.first().and_then(|target| target.failure.as_ref());
    let source = first
        .and_then(|diagnostic| diagnostic.source.clone())
        .or_else(|| failed.first().map(|target| target.destination.clone()));
    let summary = first
        .map(|diagnostic| {
            format!(
                "first failure [{}]: {}",
                diagnostic.code, diagnostic.message
            )
        })
        .unwrap_or_else(|| "one or more publish targets failed".to_owned());
    let mut error = MasterdataError::new(
        "E-PUBLISH-EXECUTION-FAILED",
        ErrorKind::Io,
        format!(
            "publish execution failed for {} target(s); {summary}",
            failed.len()
        ),
    );
    if let Some(source) = source {
        error = error.with_source(source);
    }
    for requirement in ["PUBLISH-EXEC-002", "PUBLISH-EXEC-003", "PUBLISH-EXEC-005"] {
        error = error.with_related_requirement(requirement);
    }
    error
}

struct ExecutionContext<'a> {
    project: &'a ProjectInfo,
    artifacts: &'a ValidatedArtifactSet,
    injections: &'a [PublishFailureInjection],
    target_index: usize,
}

fn execute_target(
    context: &ExecutionContext<'_>,
    target: &PublishTargetInfo,
    expected_plan: &PublishTargetPreflight,
    analysis: TargetAnalysis,
) -> Result<()> {
    match target.kind {
        PublishTargetKind::CSharp => {
            execute_csharp_target(context, target, expected_plan, analysis)
        }
        PublishTargetKind::Binary => {
            execute_binary_target(context, target, expected_plan, analysis)
        }
    }
}

fn validate_artifact_bytes(artifacts: &ValidatedArtifactSet) -> Result<()> {
    for artifact in artifacts
        .csharp
        .iter()
        .chain(std::iter::once(&artifacts.binary))
    {
        read_validated_artifact(artifact)?;
    }
    Ok(())
}

fn read_validated_artifact(artifact: &crate::receipt::ValidatedArtifact) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(&artifact.path).map_err(|error| {
        artifact_toctou_error(
            &artifact.path,
            format!("canonical artifact changed after receipt validation: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(artifact_toctou_error(
            &artifact.path,
            "canonical artifact is no longer a regular file",
        ));
    }
    let bytes = fs::read(&artifact.path).map_err(|error| {
        artifact_toctou_error(
            &artifact.path,
            format!("could not reread canonical artifact after receipt validation: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != artifact.hash {
        return Err(artifact_toctou_error(
            &artifact.path,
            "canonical artifact bytes changed after receipt validation",
        ));
    }
    Ok(bytes)
}

fn artifact_toctou_error(path: &Path, message: impl Into<String>) -> MasterdataError {
    publish_error(
        "E-PUBLISH-ARTIFACT-TOCTOU",
        ErrorKind::Validation,
        path,
        message,
        &["ARTIFACT-SET-004", "ARTIFACT-SET-006", "PUBLISH-EXEC-002"],
    )
}

fn ensure_directory_chain(
    path: &Path,
    label: &str,
    symlink_policy: SymlinkPolicy<'_>,
) -> Result<Vec<PathBuf>> {
    let mut missing = Vec::new();
    let mut probe = path.to_path_buf();
    let existing = loop {
        match fs::symlink_metadata(&probe) {
            Ok(metadata)
                if (metadata.file_type().is_symlink()
                    && !trusted_project_root_alias(&probe, symlink_policy))
                    || (!metadata.is_dir()
                        && !trusted_project_root_alias(&probe, symlink_policy)) =>
            {
                return Err(publish_error(
                    "E-PUBLISH-TARGET-PATH-UNSAFE",
                    ErrorKind::Validation,
                    &probe,
                    format!("{label} has a non-directory component"),
                    &["PUBLISH-PATH-002", "PUBLISH-PATH-003", "PUBLISH-EXEC-002"],
                ));
            }
            Ok(_) => break probe,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if probe.file_name().is_none() {
                    return Err(publish_error(
                        "E-PUBLISH-FILESYSTEM-IDENTITY",
                        ErrorKind::Io,
                        path,
                        format!("could not resolve {label} before execution"),
                        &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-002"],
                    ));
                }
                missing.push(probe.clone());
                if !probe.pop() {
                    return Err(publish_error(
                        "E-PUBLISH-FILESYSTEM-IDENTITY",
                        ErrorKind::Io,
                        path,
                        format!("could not resolve {label} before execution"),
                        &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-002"],
                    ));
                }
            }
            Err(error) => {
                return Err(publish_io_error(
                    &probe,
                    format!("could not inspect {label} before execution: {error}"),
                    &["PUBLISH-PATH-002", "PUBLISH-PATH-003", "PUBLISH-EXEC-002"],
                ));
            }
        }
    };

    let _ = existing;
    missing.reverse();
    let mut created = Vec::with_capacity(missing.len());
    for directory in missing {
        match fs::create_dir(&directory) {
            Ok(()) => {
                let metadata = fs::symlink_metadata(&directory).map_err(|error| {
                    publish_io_error(
                        &directory,
                        format!("could not verify created {label} directory: {error}"),
                        &["PUBLISH-PATH-002", "PUBLISH-PATH-003", "PUBLISH-EXEC-002"],
                    )
                })?;
                if (metadata.file_type().is_symlink()
                    && !trusted_project_root_alias(&directory, symlink_policy))
                    || (!metadata.is_dir()
                        && !trusted_project_root_alias(&directory, symlink_policy))
                {
                    return Err(publish_error(
                        "E-PUBLISH-TARGET-PATH-UNSAFE",
                        ErrorKind::Validation,
                        &directory,
                        format!("created {label} component is not a real directory"),
                        &["PUBLISH-PATH-002", "PUBLISH-PATH-003", "PUBLISH-EXEC-002"],
                    ));
                }
                created.push(directory);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&directory).map_err(|inspect_error| {
                    publish_io_error(
                        &directory,
                        format!(
                            "could not inspect concurrently-created {label} directory: {inspect_error}"
                        ),
                        &["PUBLISH-PATH-002", "PUBLISH-PATH-003", "PUBLISH-EXEC-002"],
                    )
                })?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(publish_error(
                        "E-PUBLISH-TOCTOU",
                        ErrorKind::Validation,
                        &directory,
                        format!("{label} changed while creating its parent directories"),
                        &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                    ));
                }
            }
            Err(error) => {
                return Err(publish_io_error(
                    &directory,
                    format!("could not create {label} directory: {error}"),
                    &["PUBLISH-PATH-002", "PUBLISH-EXEC-002"],
                ));
            }
        }
    }
    Ok(created)
}

// WHY: parent creation must use the same narrow project-root alias rule as
// Phase 2, otherwise a missing project-local target can fail on an OS-level
// temporary-directory alias even though its target namespace was preflighted.
// IF REMOVED: valid project-relative targets under the established root could
// be rejected during Phase 3, while broad absolute/external aliases remain
// unsafe because SymlinkPolicy::Reject does not trust them.
// EVIDENCE: docs/specs/build-pipeline.md; Regression: publish_path_accepts_absolute_target; publish_path_rejects_external_csharp_target_through_project_root_symlink; publish_path_rejects_external_binary_target_through_project_root_symlink.
fn trusted_project_root_alias(path: &Path, symlink_policy: SymlinkPolicy<'_>) -> bool {
    match symlink_policy {
        SymlinkPolicy::ProjectRelativeToRoot(project_root) => {
            is_strict_ancestor(path, project_root)
        }
        SymlinkPolicy::Reject => false,
    }
}

fn cleanup_created_directories(created: &[PathBuf]) -> Result<()> {
    for directory in created.iter().rev() {
        match fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(publish_error(
                    "E-PUBLISH-ROLLBACK-FAILED",
                    ErrorKind::Io,
                    directory,
                    "a created publish directory became a symlink during rollback",
                    &["PUBLISH-EXEC-003"],
                ));
            }
            Ok(metadata) if metadata.is_dir() => match fs::remove_dir(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => {}
                Err(error) => {
                    return Err(publish_io_error(
                        directory,
                        format!("could not clean an empty publish directory: {error}"),
                        &["PUBLISH-EXEC-003"],
                    ));
                }
            },
            Ok(_) => {
                return Err(publish_error(
                    "E-PUBLISH-ROLLBACK-FAILED",
                    ErrorKind::Io,
                    directory,
                    "a created publish directory changed type during rollback",
                    &["PUBLISH-EXEC-003"],
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(publish_io_error(
                    directory,
                    format!(
                        "could not inspect a created publish directory during rollback: {error}"
                    ),
                    &["PUBLISH-EXEC-003"],
                ));
            }
        }
    }
    Ok(())
}

fn revalidate_target_after_directories(
    project: &ProjectInfo,
    artifacts: &ValidatedArtifactSet,
    target: &PublishTargetInfo,
    expected_plan: &PublishTargetPreflight,
) -> Result<TargetAnalysis> {
    let protected_regions = protected_regions(project)?;
    let analysis = preflight_target(project, artifacts, target, &protected_regions)?;
    if analysis.plan != *expected_plan {
        return Err(publish_error(
            "E-PUBLISH-TOCTOU",
            ErrorKind::Validation,
            &target.resolved_path,
            "publish target changed while preparing its destination",
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        ));
    }
    Ok(analysis)
}

fn execute_csharp_target(
    context: &ExecutionContext<'_>,
    target: &PublishTargetInfo,
    expected_plan: &PublishTargetPreflight,
    analysis: TargetAnalysis,
) -> Result<()> {
    let symlink_policy = symlink_policy_for_target(context.project, target);
    let created = ensure_directory_chain(
        &analysis.namespace.logical_path,
        "C# publish target",
        symlink_policy,
    )?;
    let refreshed = match revalidate_target_after_directories(
        context.project,
        context.artifacts,
        target,
        expected_plan,
    ) {
        Ok(analysis) => analysis,
        Err(error) => {
            let cleanup = cleanup_created_directories(&created);
            return match cleanup {
                Ok(()) => Err(error),
                Err(rollback) => Err(combine_target_rollback_error(error, rollback, None)),
            };
        }
    };
    let csharp = refreshed.csharp.as_ref().ok_or_else(|| {
        publish_error(
            "E-PUBLISH-EXECUTION-PLAN",
            ErrorKind::Validation,
            &target.resolved_path,
            "C# target execution is missing its preflight analysis",
            &["PUBLISH-EXEC-001", "PUBLISH-EXEC-002"],
        )
    })?;
    let parent = refreshed.namespace.logical_path.parent().ok_or_else(|| {
        publish_error(
            "E-PUBLISH-TARGET-PATH-UNSAFE",
            ErrorKind::Validation,
            &refreshed.namespace.logical_path,
            "C# publish target has no parent directory",
            &["PUBLISH-PATH-001", "PUBLISH-EXEC-002"],
        )
    })?;
    let workspace = Builder::new()
        .prefix(".masterdata-publish-")
        .tempdir_in(parent)
        .map_err(|error| {
            publish_io_error(
                parent,
                format!("could not create C# publish staging workspace: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;

    let result = execute_csharp_transaction(
        context.artifacts,
        csharp,
        workspace,
        context.injections,
        context.target_index,
    );
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            if let Err(cleanup) = cleanup_created_directories(&created) {
                return Err(combine_target_rollback_error(error, cleanup, None));
            }
            Err(error)
        }
    }
}

// WHY: C# publication stages only generated files, moves previous manifest-
// owned files into a target-local backup, and publishes the manifest last.
// IF REMOVED: an I/O failure after a partial replacement could leave a mixed
// managed set or cause rollback cleanup to delete unmanaged user content.
// EVIDENCE: docs/specs/build-pipeline.md; Regression: execution_failure_rolls_back_only_failed_target; csharp_failure_preserves_previous_managed_set; csharp_manifest_failure_rolls_back_managed_set; csharp_publish_preserves_unmanaged_files_and_meta; csharp_publish_handles_nested_generated_paths.
fn execute_csharp_transaction(
    artifacts: &ValidatedArtifactSet,
    analysis: &CSharpTargetAnalysis,
    workspace: TempDir,
    injections: &[PublishFailureInjection],
    target_index: usize,
) -> Result<()> {
    let workspace_path = workspace.path().to_path_buf();
    let (original, state) = match run_csharp_transaction(
        artifacts,
        analysis,
        &workspace_path,
        injections,
        target_index,
    ) {
        Ok(_state) => return Ok(()),
        Err(failure) => *failure,
    };

    if failure_injected(
        injections,
        target_index,
        PublishFailurePoint::CSharpRollback,
    ) {
        let retained = workspace.keep();
        return Err(combine_target_rollback_error(
            original,
            publish_error(
                "E-PUBLISH-ROLLBACK-FAILED",
                ErrorKind::Io,
                &retained,
                "rollback failure was injected for C# publish",
                &["PUBLISH-EXEC-003", "PUBLISH-EXEC-005"],
            ),
            Some(retained),
        ));
    }

    match rollback_csharp_transaction(analysis, &state) {
        Ok(()) => Err(original),
        Err(rollback) => {
            let retained = workspace.keep();
            Err(combine_target_rollback_error(
                original,
                rollback,
                Some(retained),
            ))
        }
    }
}

struct CSharpTransactionState {
    moved: Vec<(PathBuf, PathBuf)>,
    published: Vec<(PathBuf, String)>,
    manifest_backup: Option<PathBuf>,
    manifest_published: Option<String>,
    created_directories: Vec<PathBuf>,
}

fn run_csharp_transaction(
    artifacts: &ValidatedArtifactSet,
    analysis: &CSharpTargetAnalysis,
    workspace: &Path,
    injections: &[PublishFailureInjection],
    target_index: usize,
) -> std::result::Result<CSharpTransactionState, Box<(MasterdataError, CSharpTransactionState)>> {
    let mut state = CSharpTransactionState {
        moved: Vec::new(),
        published: Vec::new(),
        manifest_backup: None,
        manifest_published: None,
        created_directories: Vec::new(),
    };
    let result: Result<()> = (|| {
        fs::create_dir(workspace.join("backup")).map_err(|error| {
            publish_io_error(
                workspace,
                format!("could not create C# publish backup area: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;
        let staged_root = workspace.join("new");
        fs::create_dir(&staged_root).map_err(|error| {
            publish_io_error(
                workspace,
                format!("could not create C# publish staging area: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;
        let mut staged_files = Vec::with_capacity(analysis.current.len());

        for current in &analysis.current {
            let artifact = artifacts
                .csharp
                .iter()
                .find(|artifact| artifact.relative_path == current.relative_path)
                .ok_or_else(|| {
                    publish_error(
                        "E-PUBLISH-EXECUTION-PLAN",
                        ErrorKind::Validation,
                        &current.resolved.logical_path,
                        "receipt artifact is missing from the current C# execution set",
                        &["ARTIFACT-SET-004", "PUBLISH-EXEC-002"],
                    )
                })?;
            let managed = matching_previous_entry(&analysis.previous, &current.resolved)?;
            let bytes = read_validated_artifact(artifact)?;
            let staged_relative = current
                .relative_path
                .replace('/', std::path::MAIN_SEPARATOR_STR);
            let staged_path = staged_root.join(Path::new(&staged_relative));
            if failure_injected(
                injections,
                target_index,
                PublishFailurePoint::CSharpWhileStagingNew,
            ) && managed.is_none()
            {
                return Err(injected_failure(
                    &current.resolved.logical_path,
                    "C# staging failure was injected before publishing a new managed file",
                    "PUBLISH-EXEC-003",
                ));
            }
            if let Some(parent) = staged_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    publish_io_error(
                        parent,
                        format!("could not create C# staging parent: {error}"),
                        &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                    )
                })?;
            }
            fs::write(&staged_path, bytes).map_err(|error| {
                publish_io_error(
                    &staged_path,
                    format!("could not stage generated C# bytes: {error}"),
                    &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
                )
            })?;
            staged_files.push((
                staged_path,
                current.resolved.logical_path.clone(),
                artifact.hash.clone(),
                managed,
            ));
        }

        let manifest_path = &analysis.plan.manifest_path;
        let staged_manifest = workspace.join("new-manifest.json");
        let manifest_bytes = serde_json::to_vec_pretty(&PublishManifest {
            version: PUBLISH_MANIFEST_VERSION,
            files: analysis.plan.current_generated_paths.clone(),
        })
        .map_err(|error| {
            publish_error(
                "E-PUBLISH-MANIFEST-WRITE",
                ErrorKind::Io,
                manifest_path,
                format!("could not serialize current publish manifest: {error}"),
                &["PUBLISH-PATH-005", "PUBLISH-EXEC-003"],
            )
        })?;
        fs::write(&staged_manifest, &manifest_bytes).map_err(|error| {
            publish_io_error(
                &staged_manifest,
                format!("could not stage current publish manifest: {error}"),
                &["PUBLISH-PATH-005", "PUBLISH-EXEC-003"],
            )
        })?;

        for (index, (staged_path, destination, hash, managed)) in staged_files.iter().enumerate() {
            if let Some(parent) = destination.parent() {
                state.created_directories.extend(ensure_directory_chain(
                    parent,
                    "generated C# destination parent",
                    SymlinkPolicy::Reject,
                )?);
            }
            if managed.is_some() {
                let backup = workspace.join("backup").join(format!("managed-{index}"));
                move_managed_entry(destination, &backup)?;
                state.moved.push((backup, destination.to_path_buf()));
            } else {
                ensure_destination_absent(destination, "new C# destination")?;
            }
            fs::rename(staged_path, destination).map_err(|error| {
                publish_io_error(
                    destination,
                    format!("could not publish generated C# file: {error}"),
                    &["PUBLISH-EXEC-003"],
                )
            })?;
            state
                .published
                .push((destination.to_path_buf(), hash.clone()));
            if managed.is_some()
                && failure_injected(
                    injections,
                    target_index,
                    PublishFailurePoint::CSharpAfterManagedReplacement,
                )
            {
                return Err(injected_failure(
                    destination,
                    "C# failure was injected after replacing a managed file",
                    "PUBLISH-EXEC-003",
                ));
            }
        }

        for (index, previous) in analysis.previous.iter().enumerate() {
            if matching_previous_entry(&analysis.current, &previous.resolved)?.is_none() {
                let backup = workspace.join("backup").join(format!("stale-{index}"));
                move_managed_entry(&previous.resolved.logical_path, &backup)?;
                state
                    .moved
                    .push((backup, previous.resolved.logical_path.clone()));
                if failure_injected(
                    injections,
                    target_index,
                    PublishFailurePoint::CSharpWhileRetiringStale,
                ) {
                    return Err(injected_failure(
                        &previous.resolved.logical_path,
                        "C# failure was injected while retiring a stale managed file",
                        "PUBLISH-EXEC-003",
                    ));
                }
            }
        }

        if failure_injected(
            injections,
            target_index,
            PublishFailurePoint::CSharpBeforeManifest,
        ) {
            return Err(injected_failure(
                manifest_path,
                "C# failure was injected before manifest publication",
                "PUBLISH-EXEC-003",
            ));
        }
        let old_manifest = workspace.join("backup").join("manifest.previous");
        if analysis.plan.manifest_exists {
            move_managed_entry(manifest_path, &old_manifest)?;
            state.manifest_backup = Some(old_manifest);
        } else {
            ensure_destination_absent(manifest_path, "new publish manifest")?;
        }
        if failure_injected(
            injections,
            target_index,
            PublishFailurePoint::CSharpWhilePublishingManifest,
        ) {
            return Err(injected_failure(
                manifest_path,
                "C# failure was injected while publishing the manifest",
                "PUBLISH-EXEC-003",
            ));
        }
        fs::rename(&staged_manifest, manifest_path).map_err(|error| {
            publish_io_error(
                manifest_path,
                format!("could not publish current C# manifest: {error}"),
                &["PUBLISH-PATH-005", "PUBLISH-EXEC-003"],
            )
        })?;
        state.manifest_published = Some(hash_bytes(&manifest_bytes));
        Ok(())
    })();

    match result {
        Ok(()) => Ok(state),
        Err(error) => Err(Box::new((error, state))),
    }
}

fn rollback_csharp_transaction(
    analysis: &CSharpTargetAnalysis,
    state: &CSharpTransactionState,
) -> Result<()> {
    if let Some(hash) = &state.manifest_published {
        remove_published_file(&analysis.plan.manifest_path, hash)?;
    }
    if let Some(backup) = &state.manifest_backup {
        ensure_destination_absent(&analysis.plan.manifest_path, "publish manifest restore")?;
        fs::rename(backup, &analysis.plan.manifest_path).map_err(|error| {
            publish_io_error(
                &analysis.plan.manifest_path,
                format!("could not restore the previous publish manifest: {error}"),
                &["PUBLISH-EXEC-003"],
            )
        })?;
    }
    for (path, hash) in state.published.iter().rev() {
        remove_published_file(path, hash)?;
    }
    for (backup, destination) in state.moved.iter().rev() {
        ensure_destination_absent(destination, "managed publish rollback")?;
        fs::rename(backup, destination).map_err(|error| {
            publish_io_error(
                destination,
                format!("could not restore the previous managed publish entry: {error}"),
                &["PUBLISH-EXEC-003"],
            )
        })?;
    }
    cleanup_created_directories(&state.created_directories)?;
    Ok(())
}

fn matching_previous_entry(entries: &[ManifestPath], path: &ResolvedPath) -> Result<Option<usize>> {
    for (index, entry) in entries.iter().enumerate() {
        match namespace_relation(&entry.resolved, path)? {
            NamespaceRelation::Same => return Ok(Some(index)),
            NamespaceRelation::Disjoint => {}
            NamespaceRelation::LeftAncestor | NamespaceRelation::RightAncestor => {
                return Err(publish_error(
                    "E-PUBLISH-MANAGED-PATH-OVERLAP",
                    ErrorKind::Validation,
                    &path.logical_path,
                    "publish execution encountered an overlapping managed path",
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-004", "PUBLISH-EXEC-003"],
                ));
            }
        }
    }
    Ok(None)
}

fn move_managed_entry(source: &Path, backup: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(|error| {
        publish_io_error(
            source,
            format!("could not inspect managed publish entry before moving it: {error}"),
            &["PUBLISH-PATH-004", "PUBLISH-EXEC-003"],
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(publish_error(
            "E-PUBLISH-TOCTOU",
            ErrorKind::Validation,
            source,
            "managed publish entry changed from a regular file before mutation",
            &["PUBLISH-PATH-004", "PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        ));
    }
    ensure_destination_absent(backup, "publish backup")?;
    fs::rename(source, backup).map_err(|error| {
        publish_io_error(
            source,
            format!("could not move the previous managed publish entry to backup: {error}"),
            &["PUBLISH-EXEC-003"],
        )
    })
}

fn ensure_destination_absent(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(publish_error(
            "E-PUBLISH-TOCTOU",
            ErrorKind::Validation,
            path,
            format!("{label} appeared or was replaced before publication"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(publish_io_error(
            path,
            format!("could not inspect {label}: {error}"),
            &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
        )),
    }
}

fn remove_published_file(path: &Path, expected_hash: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        publish_error(
            "E-PUBLISH-ROLLBACK-FAILED",
            ErrorKind::Io,
            path,
            format!("could not inspect a published file during rollback: {error}"),
            &["PUBLISH-EXEC-003"],
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(publish_error(
            "E-PUBLISH-ROLLBACK-FAILED",
            ErrorKind::Io,
            path,
            "published file changed type before rollback",
            &["PUBLISH-EXEC-003"],
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        publish_io_error(
            path,
            format!("could not read a published file during rollback: {error}"),
            &["PUBLISH-EXEC-003"],
        )
    })?;
    if hash_bytes(&bytes) != expected_hash {
        return Err(publish_error(
            "E-PUBLISH-ROLLBACK-FAILED",
            ErrorKind::Io,
            path,
            "published file changed before rollback and will not be removed",
            &["PUBLISH-EXEC-003"],
        ));
    }
    fs::remove_file(path).map_err(|error| {
        publish_io_error(
            path,
            format!("could not remove a published file during rollback: {error}"),
            &["PUBLISH-EXEC-003"],
        )
    })
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn injected_failure(path: &Path, message: &str, requirement: &str) -> MasterdataError {
    publish_error(
        "E-PUBLISH-INJECTED-FAILURE",
        ErrorKind::Io,
        path,
        message,
        &[requirement],
    )
}

fn combine_target_rollback_error(
    original: MasterdataError,
    rollback: MasterdataError,
    retained_workspace: Option<PathBuf>,
) -> MasterdataError {
    let source = original
        .diagnostic()
        .source
        .clone()
        .or_else(|| rollback.diagnostic().source.clone());
    let retained = retained_workspace
        .map(|path| format!("; rollback workspace retained at {}", path.display()))
        .unwrap_or_default();
    let mut error = MasterdataError::new(
        "E-PUBLISH-ROLLBACK-FAILED",
        ErrorKind::Io,
        format!(
            "publish target failed [{}]: {}; rollback failed [{}]: {}{}",
            original.diagnostic().code,
            original.diagnostic().message,
            rollback.diagnostic().code,
            rollback.diagnostic().message,
            retained
        ),
    );
    if let Some(source) = source {
        error = error.with_source(source);
    }
    for requirement in ["PUBLISH-EXEC-003", "PUBLISH-EXEC-005"] {
        error = error.with_related_requirement(requirement);
    }
    error
}

fn execute_binary_target(
    context: &ExecutionContext<'_>,
    target: &PublishTargetInfo,
    expected_plan: &PublishTargetPreflight,
    analysis: TargetAnalysis,
) -> Result<()> {
    let parent = analysis.namespace.logical_path.parent().ok_or_else(|| {
        publish_error(
            "E-PUBLISH-TARGET-PATH-UNSAFE",
            ErrorKind::Validation,
            &analysis.namespace.logical_path,
            "binary publish target has no parent directory",
            &["PUBLISH-PATH-001", "PUBLISH-EXEC-002"],
        )
    })?;
    let symlink_policy = symlink_policy_for_target(context.project, target);
    let created = ensure_directory_chain(parent, "binary publish target parent", symlink_policy)?;
    let refreshed = match revalidate_target_after_directories(
        context.project,
        context.artifacts,
        target,
        expected_plan,
    ) {
        Ok(analysis) => analysis,
        Err(error) => {
            let cleanup = cleanup_created_directories(&created);
            return match cleanup {
                Ok(()) => Err(error),
                Err(rollback) => Err(combine_target_rollback_error(error, rollback, None)),
            };
        }
    };
    let workspace = Builder::new()
        .prefix(".masterdata-publish-")
        .tempdir_in(parent)
        .map_err(|error| {
            publish_io_error(
                parent,
                format!("could not create binary publish staging workspace: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;
    let result = execute_binary_transaction(
        context.artifacts,
        &refreshed,
        workspace,
        context.injections,
        context.target_index,
    );
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            if let Err(cleanup) = cleanup_created_directories(&created) {
                return Err(combine_target_rollback_error(error, cleanup, None));
            }
            Err(error)
        }
    }
}

struct BinaryTransactionState {
    destination: PathBuf,
    previous_backup: Option<PathBuf>,
    published_hash: Option<String>,
}

// WHY: binary publication secures the previous explicit file before the
// replacement rename and removes only a verified new file during rollback.
// IF REMOVED: an initial or replacement failure could leave a partial binary
// or destroy the last usable binary while touching unrelated siblings.
// EVIDENCE: docs/specs/build-pipeline.md; Regression: binary_failure_preserves_previous_file; initial_binary_failure_does_not_publish_partial_file; binary_publish_preserves_siblings.
fn execute_binary_transaction(
    artifacts: &ValidatedArtifactSet,
    analysis: &TargetAnalysis,
    workspace: TempDir,
    injections: &[PublishFailureInjection],
    target_index: usize,
) -> Result<()> {
    let workspace_path = workspace.path().to_path_buf();
    let (original, state) = match run_binary_transaction(
        artifacts,
        analysis,
        &workspace_path,
        injections,
        target_index,
    ) {
        Ok(()) => return Ok(()),
        Err(failure) => failure,
    };

    if failure_injected(
        injections,
        target_index,
        PublishFailurePoint::BinaryRollback,
    ) {
        let retained = workspace.keep();
        return Err(combine_target_rollback_error(
            original,
            publish_error(
                "E-PUBLISH-ROLLBACK-FAILED",
                ErrorKind::Io,
                &retained,
                "rollback failure was injected for binary publish",
                &["PUBLISH-EXEC-003", "PUBLISH-EXEC-005"],
            ),
            Some(retained),
        ));
    }

    match rollback_binary_transaction(&state) {
        Ok(()) => Err(original),
        Err(rollback) => {
            let retained = workspace.keep();
            Err(combine_target_rollback_error(
                original,
                rollback,
                Some(retained),
            ))
        }
    }
}

fn run_binary_transaction(
    artifacts: &ValidatedArtifactSet,
    analysis: &TargetAnalysis,
    workspace: &Path,
    injections: &[PublishFailureInjection],
    target_index: usize,
) -> std::result::Result<(), (MasterdataError, BinaryTransactionState)> {
    let destination = analysis.namespace.logical_path.clone();
    let mut state = BinaryTransactionState {
        destination: destination.clone(),
        previous_backup: None,
        published_hash: None,
    };
    let result: Result<()> = (|| {
        let bytes = read_validated_artifact(&artifacts.binary)?;
        let staged = workspace.join("new-binary");
        fs::write(&staged, bytes).map_err(|error| {
            publish_io_error(
                &staged,
                format!("could not stage binary publish bytes: {error}"),
                &["PUBLISH-EXEC-002", "PUBLISH-EXEC-003"],
            )
        })?;
        if failure_injected(
            injections,
            target_index,
            PublishFailurePoint::BinaryBeforePublication,
        ) {
            return Err(injected_failure(
                &destination,
                "binary failure was injected before publication",
                "PUBLISH-EXEC-003",
            ));
        }

        let existing = analysis
            .plan
            .binary
            .as_ref()
            .map(|binary| binary.existing_regular_file)
            .unwrap_or(false);
        let backup = workspace.join("previous-binary");
        if existing {
            move_managed_entry(&destination, &backup)?;
            state.previous_backup = Some(backup);
            if failure_injected(
                injections,
                target_index,
                PublishFailurePoint::BinaryAfterPreviousSecured,
            ) {
                return Err(injected_failure(
                    &destination,
                    "binary failure was injected after securing the previous file",
                    "PUBLISH-EXEC-003",
                ));
            }
        } else {
            ensure_destination_absent(&destination, "new binary destination")?;
        }

        if failure_injected(
            injections,
            target_index,
            PublishFailurePoint::BinaryWhilePublishingNew,
        ) {
            return Err(injected_failure(
                &destination,
                "binary failure was injected while publishing the new file",
                "PUBLISH-EXEC-003",
            ));
        }
        fs::rename(&staged, &destination).map_err(|error| {
            publish_io_error(
                &destination,
                format!("could not publish binary artifact: {error}"),
                &["PUBLISH-EXEC-003"],
            )
        })?;
        state.published_hash = Some(artifacts.binary.hash.clone());
        if failure_injected(
            injections,
            target_index,
            PublishFailurePoint::BinaryAfterPublication,
        ) {
            return Err(injected_failure(
                &destination,
                "binary failure was injected after publication",
                "PUBLISH-EXEC-003",
            ));
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err((error, state)),
    }
}

fn rollback_binary_transaction(state: &BinaryTransactionState) -> Result<()> {
    if let Some(hash) = &state.published_hash {
        remove_published_file(&state.destination, hash)?;
    }
    if let Some(backup) = &state.previous_backup {
        ensure_destination_absent(&state.destination, "binary rollback")?;
        fs::rename(backup, &state.destination).map_err(|error| {
            publish_io_error(
                &state.destination,
                format!("could not restore the previous binary artifact: {error}"),
                &["PUBLISH-EXEC-003"],
            )
        })?;
    }
    Ok(())
}

fn preflight_target(
    project: &ProjectInfo,
    artifacts: &ValidatedArtifactSet,
    target: &masterdata_core::PublishTargetInfo,
    protected_regions: &[ProtectedRegion],
) -> Result<TargetAnalysis> {
    let label = format!("publish target `{}`", target.path);
    let symlink_policy = symlink_policy_for_target(project, target);
    let namespace = resolve_path(&target.resolved_path, &label, true, symlink_policy)?;

    match target.kind {
        PublishTargetKind::CSharp => validate_csharp_target_type(&namespace)?,
        PublishTargetKind::Binary => validate_binary_target_type(&namespace)?,
    }

    for region in protected_regions {
        let relation = namespace_relation(&namespace, &region.path)?;
        let overlaps = match region.kind {
            // A project-local child such as project/dist is an independent
            // destination. A target equal to or containing the project root
            // would own the project itself and remains forbidden.
            ProtectedRegionKind::ProjectRoot => matches!(
                relation,
                NamespaceRelation::Same | NamespaceRelation::LeftAncestor
            ),
            ProtectedRegionKind::CriticalTree => relation != NamespaceRelation::Disjoint,
        };
        if overlaps {
            return Err(publish_error(
                "E-PUBLISH-PROTECTED-PATH",
                ErrorKind::Validation,
                &target.resolved_path,
                format!(
                    "publish target `{}` overlaps protected {} `{}`",
                    target.path,
                    region.label,
                    region.path.logical_path.display()
                ),
                &["PUBLISH-PATH-001", "PUBLISH-PATH-007", "PUBLISH-EXEC-001"],
            ));
        }
    }

    let (plan, csharp) = match target.kind {
        PublishTargetKind::CSharp => {
            let csharp = preflight_csharp_target(&namespace, artifacts, symlink_policy)?;
            let plan = PublishTargetPreflight {
                kind: target.kind,
                configured_path: target.path.clone(),
                destination: target.resolved_path.clone(),
                csharp: Some(csharp.plan.clone()),
                binary: None,
            };
            (plan, Some(csharp))
        }
        PublishTargetKind::Binary => {
            let binary = BinaryPublishPreflight {
                destination: target.resolved_path.clone(),
                existing_regular_file: namespace.missing_tail.is_empty(),
            };
            (
                PublishTargetPreflight {
                    kind: target.kind,
                    configured_path: target.path.clone(),
                    destination: target.resolved_path.clone(),
                    csharp: None,
                    binary: Some(binary),
                },
                None,
            )
        }
    };

    Ok(TargetAnalysis {
        namespace,
        plan,
        csharp,
    })
}

fn symlink_policy_for_target<'a>(
    project: &'a ProjectInfo,
    target: &masterdata_core::PublishTargetInfo,
) -> SymlinkPolicy<'a> {
    let configured_path = Path::new(&target.path);
    if !configured_path.is_absolute() && target.resolved_path.starts_with(&project.project_root) {
        SymlinkPolicy::ProjectRelativeToRoot(&project.project_root)
    } else {
        SymlinkPolicy::Reject
    }
}

fn protected_regions(project: &ProjectInfo) -> Result<Vec<ProtectedRegion>> {
    let mut regions = Vec::with_capacity(4 + project.source_roots.len());
    regions.push(ProtectedRegion {
        path: resolve_path(
            &project.project_root,
            "project root",
            false,
            SymlinkPolicy::Reject,
        )?,
        label: "project root".to_owned(),
        kind: ProtectedRegionKind::ProjectRoot,
    });
    regions.push(ProtectedRegion {
        path: resolve_path(
            &project.config_path,
            "masterdata.toml",
            false,
            SymlinkPolicy::Reject,
        )?,
        label: "masterdata.toml".to_owned(),
        kind: ProtectedRegionKind::CriticalTree,
    });
    for source_root in &project.source_roots {
        regions.push(ProtectedRegion {
            path: resolve_path(
                source_root,
                "configured source root",
                false,
                SymlinkPolicy::Reject,
            )?,
            label: "configured source root".to_owned(),
            kind: ProtectedRegionKind::CriticalTree,
        });
    }
    regions.push(ProtectedRegion {
        path: resolve_path(
            &project.artifact_root,
            "canonical artifact root",
            false,
            SymlinkPolicy::Reject,
        )?,
        label: "canonical artifact root".to_owned(),
        kind: ProtectedRegionKind::CriticalTree,
    });
    regions.push(ProtectedRegion {
        path: resolve_path(&project.cache, "build cache", false, SymlinkPolicy::Reject)?,
        label: "build cache".to_owned(),
        kind: ProtectedRegionKind::CriticalTree,
    });
    Ok(regions)
}

fn validate_csharp_target_type(path: &ResolvedPath) -> Result<()> {
    if !path.missing_tail.is_empty() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&path.logical_path).map_err(|error| {
        publish_io_error(
            &path.logical_path,
            format!("could not inspect C# publish target: {error}"),
            &["PUBLISH-PATH-002", "PUBLISH-PATH-003", "PUBLISH-EXEC-001"],
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(publish_error(
            "E-PUBLISH-TARGET-TYPE",
            ErrorKind::Validation,
            &path.logical_path,
            "C# publish target must be a real directory",
            &["PUBLISH-PATH-003", "PUBLISH-EXEC-001"],
        ));
    }
    Ok(())
}

fn validate_binary_target_type(path: &ResolvedPath) -> Result<()> {
    if !path.missing_tail.is_empty() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&path.logical_path).map_err(|error| {
        publish_io_error(
            &path.logical_path,
            format!("could not inspect binary publish target: {error}"),
            &["PUBLISH-PATH-002", "PUBLISH-PATH-006", "PUBLISH-EXEC-001"],
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(publish_error(
            "E-PUBLISH-TARGET-TYPE",
            ErrorKind::Validation,
            &path.logical_path,
            "binary publish target must be a regular file or a missing path",
            &["PUBLISH-PATH-006", "PUBLISH-EXEC-001"],
        ));
    }
    Ok(())
}

fn preflight_csharp_target(
    target_root: &ResolvedPath,
    artifacts: &ValidatedArtifactSet,
    symlink_policy: SymlinkPolicy<'_>,
) -> Result<CSharpTargetAnalysis> {
    let manifest_path = target_root.logical_path.join(PUBLISH_MANIFEST_FILENAME);
    let (manifest_exists, previous_managed_paths) = if target_root.missing_tail.is_empty() {
        read_publish_manifest_state(&manifest_path)?
    } else {
        (false, Vec::new())
    };

    let manifest_resolved = resolve_path(&manifest_path, "publish manifest", true, symlink_policy)?;
    let previous = resolve_manifest_paths(target_root, &previous_managed_paths, symlink_policy)?;

    for (index, left) in previous.iter().enumerate() {
        for right in previous.iter().skip(index + 1) {
            if namespace_relation(&left.resolved, &right.resolved)? != NamespaceRelation::Disjoint {
                return Err(publish_error(
                    "E-PUBLISH-MANIFEST-ALIAS",
                    ErrorKind::Validation,
                    &manifest_path,
                    "publish manifest contains filesystem-equivalent or overlapping managed paths",
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
                ));
            }
        }
    }

    for entry in &previous {
        if namespace_relation(&entry.resolved, &manifest_resolved)? != NamespaceRelation::Disjoint {
            return Err(publish_error(
                "E-PUBLISH-MANIFEST-COLLISION",
                ErrorKind::Validation,
                &entry.resolved.logical_path,
                "previous managed path collides with the reserved publish manifest",
                &["PUBLISH-PATH-004", "PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
            ));
        }
        ensure_previous_managed_entry(&entry.resolved, &manifest_path)?;
    }

    let current_generated_paths = artifacts
        .csharp
        .iter()
        .map(|artifact| artifact.relative_path.clone())
        .collect::<Vec<_>>();
    let current = resolve_current_paths(target_root, &current_generated_paths, symlink_policy)?;

    for (index, left) in current.iter().enumerate() {
        for right in current.iter().skip(index + 1) {
            if namespace_relation(&left.resolved, &right.resolved)? != NamespaceRelation::Disjoint {
                return Err(publish_error(
                    "E-PUBLISH-CURRENT-PATH-COLLISION",
                    ErrorKind::Validation,
                    &target_root.logical_path,
                    "current generated C# paths overlap in the destination filesystem",
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-004", "PUBLISH-EXEC-001"],
                ));
            }
        }
    }

    for current_entry in &current {
        if namespace_relation(&current_entry.resolved, &manifest_resolved)?
            != NamespaceRelation::Disjoint
        {
            return Err(publish_error(
                "E-PUBLISH-MANIFEST-COLLISION",
                ErrorKind::Validation,
                &current_entry.resolved.logical_path,
                "current generated C# path collides with the reserved publish manifest",
                &["PUBLISH-PATH-004", "PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
            ));
        }

        let mut managed = false;
        for previous_entry in &previous {
            match namespace_relation(&current_entry.resolved, &previous_entry.resolved)? {
                NamespaceRelation::Disjoint => {}
                NamespaceRelation::Same => managed = true,
                NamespaceRelation::LeftAncestor | NamespaceRelation::RightAncestor => {
                    return Err(publish_error(
                        "E-PUBLISH-MANAGED-PATH-OVERLAP",
                        ErrorKind::Validation,
                        &current_entry.resolved.logical_path,
                        "current generated C# path overlaps a previous managed path with a different shape",
                        &["PUBLISH-PATH-001", "PUBLISH-PATH-004", "PUBLISH-EXEC-001"],
                    ));
                }
            }
        }

        if current_entry.resolved.missing_tail.is_empty() {
            let metadata =
                fs::symlink_metadata(&current_entry.resolved.logical_path).map_err(|error| {
                    publish_io_error(
                        &current_entry.resolved.logical_path,
                        format!("could not inspect current C# destination entry: {error}"),
                        &["PUBLISH-PATH-001", "PUBLISH-PATH-004", "PUBLISH-EXEC-001"],
                    )
                })?;
            if !metadata.is_file() || !managed {
                return Err(publish_error(
                    "E-PUBLISH-UNMANAGED-COLLISION",
                    ErrorKind::Validation,
                    &current_entry.resolved.logical_path,
                    "current generated C# path collides with an unmanaged or non-regular destination entry",
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-004", "PUBLISH-EXEC-001"],
                ));
            }
        }
    }

    let plan = CSharpPublishPreflight {
        manifest_path,
        manifest_exists,
        previous_managed_paths,
        current_generated_paths,
    };
    Ok(CSharpTargetAnalysis {
        plan,
        previous,
        current,
    })
}

#[derive(Debug, Clone)]
struct ManifestPath {
    relative_path: String,
    resolved: ResolvedPath,
}

fn resolve_manifest_paths(
    target_root: &ResolvedPath,
    paths: &[String],
    symlink_policy: SymlinkPolicy<'_>,
) -> Result<Vec<ManifestPath>> {
    let mut result = Vec::with_capacity(paths.len());
    let mut seen = BTreeSet::new();
    for relative_path in paths {
        validate_relative_path(relative_path, &target_root.logical_path)?;
        if relative_path == PUBLISH_MANIFEST_FILENAME {
            return Err(publish_error(
                "E-PUBLISH-MANIFEST-COLLISION",
                ErrorKind::Validation,
                &target_root.logical_path,
                "publish manifest cannot own its reserved filename",
                &["PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
            ));
        }
        if !seen.insert(relative_path.clone()) {
            return Err(publish_error(
                "E-PUBLISH-MANIFEST-ALIAS",
                ErrorKind::Validation,
                &target_root.logical_path,
                "publish manifest contains a duplicate managed path",
                &["PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
            ));
        }
        let path = target_root
            .logical_path
            .join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        result.push(ManifestPath {
            relative_path: relative_path.clone(),
            resolved: resolve_path(&path, "managed publish path", true, symlink_policy)?,
        });
    }
    Ok(result)
}

fn resolve_current_paths(
    target_root: &ResolvedPath,
    paths: &[String],
    symlink_policy: SymlinkPolicy<'_>,
) -> Result<Vec<ManifestPath>> {
    paths
        .iter()
        .map(|relative_path| {
            validate_relative_path(relative_path, &target_root.logical_path)?;
            let path = target_root
                .logical_path
                .join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
            Ok(ManifestPath {
                relative_path: relative_path.clone(),
                resolved: resolve_path(&path, "current generated C# path", true, symlink_policy)?,
            })
        })
        .collect()
}

fn ensure_previous_managed_entry(path: &ResolvedPath, manifest_path: &Path) -> Result<()> {
    if !path.missing_tail.is_empty() {
        return Err(publish_error(
            "E-PUBLISH-MANAGED-OWNERSHIP",
            ErrorKind::Validation,
            &path.logical_path,
            format!(
                "previous managed path is missing: {}",
                manifest_path.display()
            ),
            &["PUBLISH-PATH-004", "PUBLISH-EXEC-001"],
        ));
    }

    let metadata = fs::symlink_metadata(&path.logical_path).map_err(|error| {
        publish_io_error(
            &path.logical_path,
            format!("could not inspect previous managed entry: {error}"),
            &["PUBLISH-PATH-004", "PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
        )
    })?;
    if !metadata.is_file() {
        return Err(publish_error(
            "E-PUBLISH-MANAGED-OWNERSHIP",
            ErrorKind::Validation,
            &path.logical_path,
            format!(
                "previous managed path is not a regular file: {}",
                manifest_path.display()
            ),
            &["PUBLISH-PATH-004", "PUBLISH-EXEC-001"],
        ));
    }
    Ok(())
}

fn read_publish_manifest_state(path: &Path) -> Result<(bool, Vec<String>)> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((false, Vec::new()));
        }
        Err(error) => {
            return Err(publish_io_error(
                path,
                format!("could not inspect publish manifest: {error}"),
                &["PUBLISH-PATH-002", "PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
            ));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(publish_error(
            "E-PUBLISH-MANIFEST-TYPE",
            ErrorKind::Validation,
            path,
            "publish manifest must be a regular file",
            &["PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
        ));
    }

    let bytes = fs::read(path).map_err(|error| {
        publish_io_error(
            path,
            format!("could not read publish manifest: {error}"),
            &["PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
        )
    })?;
    let manifest: PublishManifest = serde_json::from_slice(&bytes).map_err(|error| {
        publish_error(
            "E-PUBLISH-MANIFEST-MALFORMED",
            ErrorKind::Validation,
            path,
            format!("could not parse publish manifest: {error}"),
            &["PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
        )
    })?;
    if manifest.version != PUBLISH_MANIFEST_VERSION {
        return Err(publish_error(
            "E-PUBLISH-MANIFEST-VERSION",
            ErrorKind::Validation,
            path,
            format!(
                "unsupported publish manifest version {}; expected {}",
                manifest.version, PUBLISH_MANIFEST_VERSION
            ),
            &["PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
        ));
    }
    for relative_path in &manifest.files {
        validate_relative_path(relative_path, path)?;
    }
    Ok((true, manifest.files))
}

fn validate_relative_path(path: &str, source: &Path) -> Result<()> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path.split('/').any(|component| {
            component.is_empty()
                || component == "."
                || component == ".."
                || is_windows_drive_prefix(component)
        })
    {
        return Err(publish_error(
            "E-PUBLISH-MANIFEST-PATH-UNSAFE",
            ErrorKind::Validation,
            source,
            format!("publish relative path is unsafe: {path}"),
            &["PUBLISH-PATH-004", "PUBLISH-PATH-005", "PUBLISH-EXEC-001"],
        ));
    }
    Ok(())
}

fn is_windows_drive_prefix(component: &str) -> bool {
    component.len() >= 2
        && component.as_bytes()[0].is_ascii_alphabetic()
        && component.as_bytes()[1] == b':'
}

fn resolve_path(
    path: &Path,
    label: &str,
    reject_symlinks: bool,
    symlink_policy: SymlinkPolicy<'_>,
) -> Result<ResolvedPath> {
    let mut probe = path.to_path_buf();
    let mut missing_tail = Vec::new();

    let existing_prefix = loop {
        match fs::symlink_metadata(&probe) {
            Ok(metadata) => {
                if !missing_tail.is_empty()
                    && !metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                {
                    return Err(publish_error(
                        "E-PUBLISH-TARGET-PATH-UNSAFE",
                        ErrorKind::Validation,
                        &probe,
                        format!("{label} has a non-directory component before its missing tail"),
                        &["PUBLISH-PATH-002", "PUBLISH-PATH-009", "PUBLISH-EXEC-001"],
                    ));
                }
                break probe.clone();
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = probe.file_name() else {
                    return Err(publish_error(
                        "E-PUBLISH-FILESYSTEM-IDENTITY",
                        ErrorKind::Io,
                        path,
                        format!("could not resolve {label} for filesystem identity safety"),
                        &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
                    ));
                };
                missing_tail.push(name.to_os_string());
                if !probe.pop() {
                    return Err(publish_error(
                        "E-PUBLISH-FILESYSTEM-IDENTITY",
                        ErrorKind::Io,
                        path,
                        format!("could not resolve {label} for filesystem identity safety"),
                        &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
                    ));
                }
            }
            Err(error) => {
                return Err(publish_io_error(
                    &probe,
                    format!("could not inspect {label}: {error}"),
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
                ));
            }
        }
    };

    missing_tail.reverse();
    if reject_symlinks {
        validate_existing_ancestors(&existing_prefix, label, symlink_policy)?;
    }

    let canonical_prefix = fs::canonicalize(&existing_prefix).map_err(|error| {
        publish_io_error(
            &existing_prefix,
            format!("could not canonicalize {label}: {error}"),
            &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
        )
    })?;
    let mut canonical_path = canonical_prefix.clone();
    for component in &missing_tail {
        canonical_path.push(component);
    }

    Ok(ResolvedPath {
        logical_path: path.to_path_buf(),
        canonical_path,
        existing_prefix,
        missing_tail,
    })
}

fn validate_existing_ancestors(
    path: &Path,
    label: &str,
    symlink_policy: SymlinkPolicy<'_>,
) -> Result<()> {
    let mut ancestors = Vec::new();
    let mut current = path.to_path_buf();
    loop {
        ancestors.push(current.clone());
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent.to_path_buf();
    }

    for ancestor in ancestors.into_iter().rev() {
        let metadata = fs::symlink_metadata(&ancestor).map_err(|error| {
            publish_io_error(
                &ancestor,
                format!("could not inspect {label} ancestor: {error}"),
                &[
                    "PUBLISH-PATH-002",
                    "PUBLISH-PATH-003",
                    "PUBLISH-PATH-006",
                    "PUBLISH-EXEC-001",
                ],
            )
        })?;
        // WHY: a project-relative target that remains below the established
        // project root may reuse the root's filesystem identity for aliases
        // already traversed before reaching that root. The target-specific
        // namespace begins at the project root, so only strict ancestors of
        // that root are trusted; symlinks at or below the root remain unsafe.
        // IF REMOVED: valid project-local targets under an OS-level temporary
        // directory alias such as macOS /var -> /private/var would fail.
        // BOUNDARY: absolute targets and relative targets outside the project
        // root use SymlinkPolicy::Reject and inspect their full ancestor chain.
        // EVIDENCE: docs/specs/build-pipeline.md; Regression: publish_path_accepts_absolute_target; publish_path_rejects_csharp_ancestor_symlink; publish_path_rejects_external_csharp_target_through_project_root_symlink; publish_path_rejects_external_binary_target_through_project_root_symlink.
        let trusted_project_root_ancestor = match symlink_policy {
            SymlinkPolicy::ProjectRelativeToRoot(project_root) => {
                is_strict_ancestor(&ancestor, project_root)
            }
            SymlinkPolicy::Reject => false,
        };
        if metadata.file_type().is_symlink() && !trusted_project_root_ancestor {
            return Err(publish_error(
                "E-PUBLISH-SYMLINK-ANCESTOR",
                ErrorKind::Validation,
                &ancestor,
                format!("{label} must not traverse a symlink"),
                &["PUBLISH-PATH-003", "PUBLISH-PATH-006", "PUBLISH-EXEC-001"],
            ));
        }
        if ancestor != path && !metadata.is_dir() && !trusted_project_root_ancestor {
            return Err(publish_error(
                "E-PUBLISH-TARGET-PATH-UNSAFE",
                ErrorKind::Validation,
                &ancestor,
                format!("{label} has a non-directory ancestor"),
                &["PUBLISH-PATH-002", "PUBLISH-PATH-009", "PUBLISH-EXEC-001"],
            ));
        }
    }
    Ok(())
}

fn is_strict_ancestor(candidate: &Path, target: &Path) -> bool {
    candidate != target && target.starts_with(candidate)
}

fn namespace_relation(left: &ResolvedPath, right: &ResolvedPath) -> Result<NamespaceRelation> {
    if left.canonical_path == right.canonical_path {
        return Ok(NamespaceRelation::Same);
    }
    if left.canonical_path.starts_with(&right.canonical_path) {
        return Ok(NamespaceRelation::RightAncestor);
    }
    if right.canonical_path.starts_with(&left.canonical_path) {
        return Ok(NamespaceRelation::LeftAncestor);
    }

    if left.missing_tail.is_empty() && right.missing_tail.is_empty() {
        let same =
            is_same_file(&left.existing_prefix, &right.existing_prefix).map_err(|error| {
                publish_error(
                    "E-PUBLISH-FILESYSTEM-IDENTITY",
                    ErrorKind::Io,
                    &right.logical_path,
                    format!("could not compare filesystem identity: {error}"),
                    &["PUBLISH-PATH-001", "PUBLISH-EXEC-001"],
                )
            })?;
        return Ok(if same {
            NamespaceRelation::Same
        } else {
            NamespaceRelation::Disjoint
        });
    }

    if !same_existing_prefix(left, right)? {
        return Ok(NamespaceRelation::Disjoint);
    }

    relation_for_missing_tails(left, right)
}

fn same_existing_prefix(left: &ResolvedPath, right: &ResolvedPath) -> Result<bool> {
    if left.canonical_path == right.canonical_path {
        return Ok(true);
    }
    is_same_file(&left.existing_prefix, &right.existing_prefix).map_err(|error| {
        publish_error(
            "E-PUBLISH-FILESYSTEM-IDENTITY",
            ErrorKind::Io,
            &right.logical_path,
            format!("could not compare existing filesystem prefixes: {error}"),
            &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
        )
    })
}

fn relation_for_missing_tails(
    left: &ResolvedPath,
    right: &ResolvedPath,
) -> Result<NamespaceRelation> {
    let common_length = left.missing_tail.len().min(right.missing_tail.len());
    for index in 0..common_length {
        if left.missing_tail[index] == right.missing_tail[index] {
            continue;
        }
        let equivalent = if index == 0 {
            components_are_equivalent(
                &left.existing_prefix,
                &left.missing_tail[index],
                &right.missing_tail[index],
            )?
        } else {
            components_are_equivalent_below_missing_tail(
                &left.logical_path,
                &left.missing_tail[index],
                &right.missing_tail[index],
            )?
        };
        if !equivalent {
            return Ok(NamespaceRelation::Disjoint);
        }
    }

    match left.missing_tail.len().cmp(&right.missing_tail.len()) {
        std::cmp::Ordering::Equal => Ok(NamespaceRelation::Same),
        std::cmp::Ordering::Less => Ok(NamespaceRelation::LeftAncestor),
        std::cmp::Ordering::Greater => Ok(NamespaceRelation::RightAncestor),
    }
}

fn components_are_equivalent(parent: &Path, left: &OsString, right: &OsString) -> Result<bool> {
    let (Some(left), Some(right)) = (left.to_str(), right.to_str()) else {
        return Err(publish_error(
            "E-PUBLISH-FILESYSTEM-IDENTITY",
            ErrorKind::Io,
            parent,
            "could not prove filesystem namespace identity for non-UTF-8 path components",
            &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
        ));
    };
    if left == right {
        return Ok(true);
    }
    if left.is_ascii() && right.is_ascii() && left.eq_ignore_ascii_case(right) {
        return match detect_case_behavior(parent)? {
            CaseBehavior::Insensitive => Ok(true),
            CaseBehavior::Sensitive => Ok(false),
            CaseBehavior::Unknown => Err(publish_error(
                "E-PUBLISH-FILESYSTEM-IDENTITY",
                ErrorKind::Io,
                parent,
                "could not determine filesystem case behavior for a missing path tail",
                &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
            )),
        };
    }
    if left.is_ascii() && right.is_ascii() {
        return Ok(false);
    }
    Err(publish_error(
        "E-PUBLISH-FILESYSTEM-IDENTITY",
        ErrorKind::Io,
        parent,
        "could not prove filesystem namespace identity for distinct Unicode path spellings",
        &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
    ))
}

fn components_are_equivalent_below_missing_tail(
    source: &Path,
    left: &OsString,
    right: &OsString,
) -> Result<bool> {
    let (Some(left), Some(right)) = (left.to_str(), right.to_str()) else {
        return Err(publish_error(
            "E-PUBLISH-FILESYSTEM-IDENTITY",
            ErrorKind::Io,
            source,
            "could not prove filesystem namespace identity below an unresolved path tail",
            &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
        ));
    };
    if left == right {
        return Ok(true);
    }
    if left.is_ascii() && right.is_ascii() && left.eq_ignore_ascii_case(right) {
        return Err(publish_error(
            "E-PUBLISH-FILESYSTEM-IDENTITY",
            ErrorKind::Io,
            source,
            "could not determine filesystem case behavior below an unresolved path tail",
            &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
        ));
    }
    if left.is_ascii() && right.is_ascii() {
        return Ok(false);
    }
    Err(publish_error(
        "E-PUBLISH-FILESYSTEM-IDENTITY",
        ErrorKind::Io,
        source,
        "could not prove filesystem namespace identity for distinct Unicode path spellings",
        &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaseBehavior {
    Sensitive,
    Insensitive,
    Unknown,
}

fn detect_case_behavior(directory: &Path) -> Result<CaseBehavior> {
    let names = fs::read_dir(directory)
        .map_err(|error| {
            publish_io_error(
                directory,
                format!("could not inspect destination namespace: {error}"),
                &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
            )
        })?
        .map(|entry| {
            entry.map(|entry| entry.file_name()).map_err(|error| {
                publish_io_error(
                    directory,
                    format!("could not inspect destination namespace entry: {error}"),
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;

    for name in &names {
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.is_ascii() || !name.bytes().any(|byte| byte.is_ascii_alphabetic()) {
            continue;
        }
        let mut variant = name.as_bytes().to_vec();
        let Some(index) = variant.iter().position(u8::is_ascii_alphabetic) else {
            continue;
        };
        let byte = variant[index];
        variant[index] = if byte.is_ascii_lowercase() {
            byte.to_ascii_uppercase()
        } else {
            byte.to_ascii_lowercase()
        };
        let Ok(variant) = String::from_utf8(variant) else {
            continue;
        };
        if names
            .iter()
            .any(|entry| entry.to_str() == Some(variant.as_str()))
        {
            return Ok(CaseBehavior::Sensitive);
        }
        match fs::symlink_metadata(directory.join(&variant)) {
            Ok(_) => return Ok(CaseBehavior::Insensitive),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CaseBehavior::Sensitive);
            }
            Err(error) => {
                return Err(publish_io_error(
                    directory,
                    format!("could not inspect filesystem case behavior: {error}"),
                    &["PUBLISH-PATH-001", "PUBLISH-PATH-002", "PUBLISH-EXEC-001"],
                ));
            }
        }
    }
    Ok(CaseBehavior::Unknown)
}

fn publish_error(
    code: &str,
    kind: ErrorKind,
    source: &Path,
    message: impl Into<String>,
    requirements: &[&str],
) -> MasterdataError {
    let mut error = MasterdataError::new(code, kind, message).with_source(source.to_path_buf());
    for requirement in requirements {
        error = error.with_related_requirement(*requirement);
    }
    error
}

fn publish_io_error(
    source: &Path,
    message: impl Into<String>,
    requirements: &[&str],
) -> MasterdataError {
    publish_error(
        "E-PUBLISH-FILESYSTEM-INSPECTION",
        ErrorKind::Io,
        source,
        message,
        requirements,
    )
}
