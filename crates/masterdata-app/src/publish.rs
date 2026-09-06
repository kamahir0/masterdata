use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{ErrorKind, MasterdataError, ProjectInfo, PublishTargetKind, Result};
use same_file::is_same_file;
use serde::Deserialize;

use crate::receipt::ValidatedArtifactSet;

pub const PUBLISH_MANIFEST_FILENAME: &str = ".masterdata-publish-manifest.json";

const PUBLISH_MANIFEST_VERSION: u32 = 1;

/// Read-only information needed by a later publish execution phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPreflightPlan {
    pub artifact_set: ValidatedArtifactSet,
    pub targets: Vec<PublishTargetPreflight>,
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

#[derive(Debug, Deserialize)]
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

    let plan = match target.kind {
        PublishTargetKind::CSharp => {
            let csharp = preflight_csharp_target(&namespace, artifacts, symlink_policy)?;
            PublishTargetPreflight {
                kind: target.kind,
                configured_path: target.path.clone(),
                destination: target.resolved_path.clone(),
                csharp: Some(csharp),
                binary: None,
            }
        }
        PublishTargetKind::Binary => {
            let binary = BinaryPublishPreflight {
                destination: target.resolved_path.clone(),
                existing_regular_file: namespace.missing_tail.is_empty(),
            };
            PublishTargetPreflight {
                kind: target.kind,
                configured_path: target.path.clone(),
                destination: target.resolved_path.clone(),
                csharp: None,
                binary: Some(binary),
            }
        }
    };

    Ok(TargetAnalysis { namespace, plan })
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
) -> Result<CSharpPublishPreflight> {
    let manifest_path = target_root.logical_path.join(PUBLISH_MANIFEST_FILENAME);
    let previous_managed_paths = if target_root.missing_tail.is_empty() {
        read_publish_manifest(&manifest_path)?
    } else {
        Vec::new()
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

    Ok(CSharpPublishPreflight {
        manifest_path,
        previous_managed_paths,
        current_generated_paths,
    })
}

#[derive(Debug, Clone)]
struct ManifestPath {
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

fn read_publish_manifest(path: &Path) -> Result<Vec<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
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
    Ok(manifest.files)
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
