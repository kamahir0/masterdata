//! Shared same-root source path mutation for Desktop authoring.
//!
//! The GUI supplies logical project-relative paths only.  Root selection,
//! symlink/traversal checks, race rechecks, and the no-overwrite filesystem
//! operation live here so every host observes the same mutation contract.

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use masterdata_core::{Diagnostic, ErrorKind, MasterdataError, Project, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::NativeApplicationService;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePathMutationRequest {
    pub source_path: String,
    pub destination_path: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourcePathMutationStatus {
    Success,
    Conflict,
    Failure,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourcePathEntryState {
    pub path: String,
    pub exists: bool,
    pub regular_file: bool,
    pub symlink: bool,
    pub content_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePathMutationReport {
    pub status: SourcePathMutationStatus,
    pub source_path: String,
    pub destination_path: String,
    pub source_state: Option<SourcePathEntryState>,
    pub destination_state: Option<SourcePathEntryState>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePathStateReport {
    pub source: SourcePathEntryState,
    pub destination: SourcePathEntryState,
}

#[derive(Debug)]
struct ResolvedEntry {
    root_index: usize,
    absolute: PathBuf,
    parent_absolute: PathBuf,
    parent: Dir,
    name: String,
    logical_path: String,
}

struct MutationStage {
    name: String,
    dir: Dir,
}

const STAGED_SOURCE_NAME: &str = "source";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutationPoint {
    BeforeDestinationCreate,
    BeforeSourceStage,
    AfterDestinationCreate,
    AfterSourceRemove,
}

static STAGE_ID: AtomicU64 = AtomicU64::new(0);

impl NativeApplicationService {
    pub fn rename_source_file(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        request: &SourcePathMutationRequest,
    ) -> Result<SourcePathMutationReport> {
        let project = Project::discover(explicit_project, current_dir)?;
        rename_source_file_impl(&project, request, &mut |_| Ok(()))
    }

    pub fn source_path_state(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        request: &SourcePathMutationRequest,
    ) -> Result<SourcePathStateReport> {
        let project = Project::discover(explicit_project, current_dir)?;
        let (source, destination) = resolve_pair(&project, request)?;
        let source = read_entry_state(&source)?;
        let destination = read_entry_state(&destination)?;
        Ok(SourcePathStateReport {
            source,
            destination,
        })
    }
}

fn rename_source_file_impl(
    project: &Project,
    request: &SourcePathMutationRequest,
    hook: &mut dyn FnMut(MutationPoint) -> io::Result<()>,
) -> Result<SourcePathMutationReport> {
    let (source, destination) = resolve_pair(project, request)?;
    if source.logical_path == destination.logical_path {
        return Err(path_error(
            "E-SOURCE-PATH-SAME",
            "source and destination must be different paths",
            &source.absolute,
            "SOURCE-PATH-002",
        ));
    }

    let source_initial = read_entry_state(&source)?;
    if !source_initial.exists {
        return Ok(report(
            &source,
            &destination,
            SourcePathMutationStatus::Conflict,
            Some(path_diagnostic(
                "E-SOURCE-PATH-CONFLICT",
                "source file disappeared before the rename started",
                &source.absolute,
                "SOURCE-PATH-003",
            )),
        ));
    }
    if source_initial.symlink || !source_initial.regular_file {
        return Err(path_error(
            "E-SOURCE-PATH-SYMLINK",
            "source must be an existing regular non-symlink file",
            &source.absolute,
            "SOURCE-PATH-002",
        ));
    }

    let destination_initial = read_entry_state(&destination)?;
    let destination_alias = if destination_initial.exists {
        None
    } else {
        find_case_insensitive_alias(&destination.parent, &destination.name)?
    };
    let destination_alias_regular = destination_alias.as_deref().is_some_and(|alias| {
        destination
            .parent
            .symlink_metadata(alias)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
    });
    let case_only_self = (destination_initial.exists || destination_alias.is_some())
        && (destination_initial.regular_file || destination_alias_regular)
        && source.name != destination.name
        && source.name.eq_ignore_ascii_case(&destination.name)
        && source.parent_absolute == destination.parent_absolute
        && same_case_only_entry(&source, &destination, destination_alias.as_deref());
    let distinct_destination_exists = destination_initial.exists || destination_alias.is_some();
    if distinct_destination_exists && !case_only_self {
        return Ok(report(
            &source,
            &destination,
            SourcePathMutationStatus::Conflict,
            Some(path_diagnostic(
                "E-SOURCE-PATH-CONFLICT",
                "destination already exists; overwrite is not supported",
                &destination.absolute,
                "SOURCE-PATH-003",
            )),
        ));
    }

    // The host can recheck the source and destination immediately before the
    // filesystem mutation.  Held directory capabilities keep the operation in
    // the configured root even if an ancestor is swapped after this point.
    if let Err(error) = hook(MutationPoint::BeforeDestinationCreate) {
        return Ok(report(
            &source,
            &destination,
            SourcePathMutationStatus::Failure,
            Some(path_diagnostic(
                "E-SOURCE-PATH-FAILURE",
                format!("rename preflight could not complete: {error}"),
                &source.absolute,
                "SOURCE-PATH-005",
            )),
        ));
    }
    let source_before_mutation = read_entry_state(&source)?;
    if source_before_mutation != source_initial {
        return Ok(report(
            &source,
            &destination,
            SourcePathMutationStatus::Conflict,
            Some(path_diagnostic(
                "E-SOURCE-PATH-CONFLICT",
                "source content changed during rename preflight",
                &source.absolute,
                "SOURCE-PATH-003",
            )),
        ));
    }
    let destination_before_mutation = read_entry_state(&destination)?;
    if case_only_self {
        let destination_alias_before_mutation = if destination_before_mutation.exists {
            None
        } else {
            find_case_insensitive_alias(&destination.parent, &destination.name)?
        };
        if !same_case_only_entry(
            &source,
            &destination,
            destination_alias_before_mutation.as_deref(),
        ) || destination_before_mutation != destination_initial
        {
            return Ok(report(
                &source,
                &destination,
                SourcePathMutationStatus::Conflict,
                Some(path_diagnostic(
                    "E-SOURCE-PATH-CONFLICT",
                    "source or destination changed during rename preflight",
                    &destination.absolute,
                    "SOURCE-PATH-003",
                )),
            ));
        }
        let source_bytes = read_file_nofollow(&source.parent, &source.name, &source.absolute)?;
        let source_identity = bytes_identity(&source_bytes);
        if source_initial.content_identity.as_deref() != Some(source_identity.as_str()) {
            return Ok(report(
                &source,
                &destination,
                SourcePathMutationStatus::Conflict,
                Some(path_diagnostic(
                    "E-SOURCE-PATH-CONFLICT",
                    "source content changed during rename preflight",
                    &source.absolute,
                    "SOURCE-PATH-003",
                )),
            ));
        }
        return mutate_with_stage(&source, &destination, &source_bytes, hook);
    }
    if destination_before_mutation.exists {
        return Ok(report(
            &source,
            &destination,
            SourcePathMutationStatus::Conflict,
            Some(path_diagnostic(
                "E-SOURCE-PATH-CONFLICT",
                "destination appeared during rename preflight; overwrite is not supported",
                &destination.absolute,
                "SOURCE-PATH-003",
            )),
        ));
    }

    let source_bytes = read_file_nofollow(&source.parent, &source.name, &source.absolute)?;
    let source_identity = bytes_identity(&source_bytes);
    if source_initial.content_identity.as_deref() != Some(source_identity.as_str()) {
        return Ok(report(
            &source,
            &destination,
            SourcePathMutationStatus::Conflict,
            Some(path_diagnostic(
                "E-SOURCE-PATH-CONFLICT",
                "source content changed during rename preflight",
                &source.absolute,
                "SOURCE-PATH-003",
            )),
        ));
    }
    mutate_with_stage(&source, &destination, &source_bytes, hook)
}

fn mutate_with_stage(
    source: &ResolvedEntry,
    destination: &ResolvedEntry,
    source_bytes: &[u8],
    hook: &mut dyn FnMut(MutationPoint) -> io::Result<()>,
) -> Result<SourcePathMutationReport> {
    // WHY: stage the source entry before publishing the destination. A
    // hard-link-then-unlink sequence could delete a replacement created at
    // the old path between its last identity check and unlink.
    // IF REMOVED: a source/destination TOCTOU can lose an unrelated source.
    // EVIDENCE: SOURCE-PATH-003, SOURCE-PATH-005 and the replacement-race test.
    let expected_identity = bytes_identity(source_bytes);
    let stage = match create_stage_dir(&source.parent, "move") {
        Ok(stage) => stage,
        Err(error) => {
            return Ok(report(
                source,
                destination,
                SourcePathMutationStatus::Failure,
                Some(path_diagnostic(
                    "E-SOURCE-PATH-FAILURE",
                    format!("could not create source mutation staging area: {error}"),
                    &source.absolute,
                    "SOURCE-PATH-005",
                )),
            ));
        }
    };
    if let Err(error) = hook(MutationPoint::BeforeSourceStage) {
        let cleanup = remove_stage_dir(&source.parent, stage);
        let status = if cleanup.is_ok() {
            SourcePathMutationStatus::Failure
        } else {
            SourcePathMutationStatus::OutcomeUnknown
        };
        return Ok(report(
            source,
            destination,
            status,
            Some(path_diagnostic(
                if status == SourcePathMutationStatus::Failure {
                    "E-SOURCE-PATH-FAILURE"
                } else {
                    "E-SOURCE-PATH-OUTCOME-UNKNOWN"
                },
                format!("source mutation could not start: {error}"),
                &source.absolute,
                "SOURCE-PATH-005",
            )),
        ));
    }
    if let Err(error) = source
        .parent
        .rename(&source.name, &stage.dir, STAGED_SOURCE_NAME)
    {
        let cleanup = remove_stage_dir(&source.parent, stage);
        let conflict = error.kind() == io::ErrorKind::NotFound;
        let status = if cleanup.is_ok() && conflict {
            SourcePathMutationStatus::Conflict
        } else if cleanup.is_ok() {
            SourcePathMutationStatus::Failure
        } else {
            SourcePathMutationStatus::OutcomeUnknown
        };
        return Ok(report(
            source,
            destination,
            status,
            Some(path_diagnostic(
                if status == SourcePathMutationStatus::Conflict {
                    "E-SOURCE-PATH-CONFLICT"
                } else if status == SourcePathMutationStatus::Failure {
                    "E-SOURCE-PATH-FAILURE"
                } else {
                    "E-SOURCE-PATH-OUTCOME-UNKNOWN"
                },
                format!("could not stage the source without overwriting another entry: {error}"),
                &source.absolute,
                "SOURCE-PATH-005",
            )),
        ));
    }

    let staged_state = match read_named_entry_state(
        &stage.dir,
        STAGED_SOURCE_NAME,
        &source.logical_path,
        &source.absolute,
    ) {
        Ok(state) => state,
        Err(error) => {
            return Ok(final_report_after_stage_error(
                source,
                destination,
                stage,
                format!("staged source state could not be verified: {error}"),
                false,
            ));
        }
    };
    if !staged_state.regular_file
        || staged_state.symlink
        || staged_state.content_identity.as_deref() != Some(expected_identity.as_str())
    {
        return Ok(final_report_after_stage_error(
            source,
            destination,
            stage,
            "source content changed during rename preflight".to_owned(),
            true,
        ));
    }

    if let Err(error) =
        create_destination_from(&stage.dir, STAGED_SOURCE_NAME, destination, source_bytes)
    {
        let destination_state = read_entry_state(destination).ok();
        let restored = restore_staged_source(source, stage);
        let status = if error.kind() == io::ErrorKind::AlreadyExists && restored.is_ok() {
            SourcePathMutationStatus::Conflict
        } else if destination_state.as_ref().is_some_and(|state| state.exists) {
            SourcePathMutationStatus::OutcomeUnknown
        } else if restored.is_ok() {
            SourcePathMutationStatus::Failure
        } else {
            SourcePathMutationStatus::OutcomeUnknown
        };
        return Ok(report(
            source,
            destination,
            status,
            Some(path_diagnostic(
                if status == SourcePathMutationStatus::Conflict {
                    "E-SOURCE-PATH-CONFLICT"
                } else if status == SourcePathMutationStatus::OutcomeUnknown {
                    "E-SOURCE-PATH-OUTCOME-UNKNOWN"
                } else {
                    "E-SOURCE-PATH-FAILURE"
                },
                format!("could not create the destination without overwrite: {error}"),
                &destination.absolute,
                "SOURCE-PATH-005",
            )),
        ));
    }

    if let Err(error) = hook(MutationPoint::AfterDestinationCreate) {
        return Ok(final_report_after_staged_mutation_error(
            source,
            destination,
            &stage,
            &expected_identity,
            format!("destination was created but source removal could not be completed: {error}"),
        ));
    }
    if let Err(error) = stage.dir.remove_file(STAGED_SOURCE_NAME) {
        return Ok(final_report_after_staged_mutation_error(
            source,
            destination,
            &stage,
            &expected_identity,
            format!("could not remove the staged original source: {error}"),
        ));
    }
    if let Err(error) = hook(MutationPoint::AfterSourceRemove) {
        return Ok(final_report_after_staged_mutation_error(
            source,
            destination,
            &stage,
            &expected_identity,
            format!("rename completion could not be confirmed: {error}"),
        ));
    }
    if let Err(error) = remove_stage_dir(&source.parent, stage) {
        return Ok(report(
            source,
            destination,
            SourcePathMutationStatus::OutcomeUnknown,
            Some(path_diagnostic(
                "E-SOURCE-PATH-OUTCOME-UNKNOWN",
                format!("rename completed but staging cleanup could not be confirmed: {error}"),
                &destination.absolute,
                "SOURCE-PATH-005",
            )),
        ));
    }

    let final_source = match read_entry_state(source) {
        Ok(state) => state,
        Err(error) => {
            return Ok(report(
                source,
                destination,
                SourcePathMutationStatus::OutcomeUnknown,
                Some(path_diagnostic(
                    "E-SOURCE-PATH-OUTCOME-UNKNOWN",
                    format!("rename completed but old path state could not be read: {error}"),
                    &source.absolute,
                    "SOURCE-PATH-005",
                )),
            ));
        }
    };
    let final_destination = match read_entry_state(destination) {
        Ok(state) => state,
        Err(error) => {
            return Ok(report(
                source,
                destination,
                SourcePathMutationStatus::OutcomeUnknown,
                Some(path_diagnostic(
                    "E-SOURCE-PATH-OUTCOME-UNKNOWN",
                    format!("rename completed but new path state could not be read: {error}"),
                    &destination.absolute,
                    "SOURCE-PATH-005",
                )),
            ));
        }
    };
    if !final_source.exists
        && final_destination.regular_file
        && final_destination.content_identity.as_deref() == Some(expected_identity.as_str())
    {
        return Ok(report(
            source,
            destination,
            SourcePathMutationStatus::Success,
            None,
        ));
    }
    Ok(report(
        source,
        destination,
        SourcePathMutationStatus::OutcomeUnknown,
        Some(path_diagnostic(
            "E-SOURCE-PATH-OUTCOME-UNKNOWN",
            "source mutation completed but the final old/new state could not be verified",
            &destination.absolute,
            "SOURCE-PATH-005",
        )),
    ))
}

fn create_destination_from(
    source_dir: &Dir,
    source_name: &str,
    destination: &ResolvedEntry,
    bytes: &[u8],
) -> io::Result<()> {
    // A hard link publishes complete bytes without overwriting an existing
    // destination. Cross-device filesystems fall back to a complete staged
    // copy published by an exclusive hard link.
    // EVIDENCE: SOURCE-PATH-001, SOURCE-PATH-003, SOURCE-PATH-005.
    match source_dir.hard_link(source_name, &destination.parent, &destination.name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(error),
        Err(_) => exclusive_copy(destination, bytes),
    }
}

fn exclusive_copy(destination: &ResolvedEntry, bytes: &[u8]) -> io::Result<()> {
    let stage = unique_stage_name("copy");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = destination.parent.open_with(&stage, &options)?;
    let result = (|| {
        std::io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;
        destination
            .parent
            .hard_link(&stage, &destination.parent, &destination.name)
    })();
    drop(file);
    let cleanup = destination.parent.remove_file(&stage);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn create_stage_dir(parent: &Dir, kind: &str) -> io::Result<MutationStage> {
    for _ in 0..32 {
        let name = unique_stage_name(kind);
        match parent.create_dir(&name) {
            Ok(()) => {
                let dir = match parent.open_dir_nofollow(&name) {
                    Ok(dir) => dir,
                    Err(error) => {
                        let _ = parent.remove_dir(&name);
                        return Err(error);
                    }
                };
                return Ok(MutationStage { name, dir });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique source mutation staging directory",
    ))
}

fn remove_stage_dir(parent: &Dir, stage: MutationStage) -> io::Result<()> {
    let name = stage.name;
    drop(stage.dir);
    parent.remove_dir(name)
}

fn restore_staged_source(source: &ResolvedEntry, stage: MutationStage) -> io::Result<()> {
    // Re-linking the staged entry publishes the original source only when the
    // old path is still empty. This avoids replacing a file that appeared
    // while the mutation was in progress.
    stage
        .dir
        .hard_link(STAGED_SOURCE_NAME, &source.parent, &source.name)?;
    stage.dir.remove_file(STAGED_SOURCE_NAME)?;
    remove_stage_dir(&source.parent, stage)
}

fn final_report_after_stage_error(
    source: &ResolvedEntry,
    destination: &ResolvedEntry,
    stage: MutationStage,
    message: String,
    conflict_if_restored: bool,
) -> SourcePathMutationReport {
    let restored = restore_staged_source(source, stage).is_ok();
    let status = if restored && conflict_if_restored {
        SourcePathMutationStatus::Conflict
    } else {
        SourcePathMutationStatus::OutcomeUnknown
    };
    report(
        source,
        destination,
        status,
        Some(path_diagnostic(
            if status == SourcePathMutationStatus::Conflict {
                "E-SOURCE-PATH-CONFLICT"
            } else {
                "E-SOURCE-PATH-OUTCOME-UNKNOWN"
            },
            message,
            &source.absolute,
            "SOURCE-PATH-005",
        )),
    )
}

fn final_report_after_staged_mutation_error(
    source: &ResolvedEntry,
    destination: &ResolvedEntry,
    stage: &MutationStage,
    expected_identity: &str,
    message: String,
) -> SourcePathMutationReport {
    let stage_file_exists = stage.dir.try_exists(STAGED_SOURCE_NAME).unwrap_or(true);
    let stage_dir_exists = exact_entry_exists(&source.parent, &stage.name).unwrap_or(true);
    let source_state = read_entry_state(source).ok();
    let destination_state = read_entry_state(destination).ok();
    let success = !stage_file_exists
        && !stage_dir_exists
        && source_state.as_ref().is_some_and(|state| !state.exists)
        && destination_state.as_ref().is_some_and(|state| {
            state.exists
                && state.regular_file
                && state.content_identity.as_deref() == Some(expected_identity)
        });
    report(
        source,
        destination,
        if success {
            SourcePathMutationStatus::Success
        } else {
            SourcePathMutationStatus::OutcomeUnknown
        },
        Some(path_diagnostic(
            if success {
                "E-SOURCE-PATH-VERIFY"
            } else {
                "E-SOURCE-PATH-OUTCOME-UNKNOWN"
            },
            message,
            &destination.absolute,
            "SOURCE-PATH-005",
        )),
    )
}

fn resolve_pair(
    project: &Project,
    request: &SourcePathMutationRequest,
) -> Result<(ResolvedEntry, ResolvedEntry)> {
    let source = resolve_entry(project, &request.source_path)?;
    let destination = resolve_entry(project, &request.destination_path)?;
    if source.root_index != destination.root_index {
        return Err(path_error(
            "E-SOURCE-PATH-CROSS-ROOT",
            "source and destination must be inside the same configured source root",
            &destination.absolute,
            "SOURCE-PATH-002",
        ));
    }
    Ok((source, destination))
}

fn resolve_entry(project: &Project, logical_path: &str) -> Result<ResolvedEntry> {
    let relative = validated_relative_path(logical_path)?;
    if !is_source_extension(&relative) {
        return Err(path_error(
            "E-SOURCE-PATH-EXTENSION",
            "source path must end in .yaml or .yml",
            &project.root().join(&relative),
            "SOURCE-PATH-002",
        ));
    }
    let absolute = project.root().join(&relative);
    let (root_index, root) = project
        .info()
        .source_roots
        .iter()
        .enumerate()
        .filter_map(|(index, root)| {
            absolute
                .strip_prefix(root)
                .ok()
                .filter(|rest| !rest.as_os_str().is_empty())
                .map(|_| (index, root.clone()))
        })
        .max_by_key(|(_, root)| root.components().count())
        .ok_or_else(|| {
            path_error(
                "E-SOURCE-PATH-ROOT",
                "source path is outside the configured source roots",
                &absolute,
                "SOURCE-PATH-002",
            )
        })?;
    let relative_to_root = absolute
        .strip_prefix(&root)
        .expect("root selected from absolute path")
        .to_path_buf();
    let parent_relative = relative_to_root
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new(""));
    let parent_absolute = root.join(parent_relative);
    let parent = open_parent(&root, parent_relative, &absolute)?;
    let name = relative_to_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            path_error(
                "E-SOURCE-PATH-UTF8",
                "source path must use valid UTF-8 file names",
                &absolute,
                "SOURCE-PATH-002",
            )
        })?
        .to_owned();
    Ok(ResolvedEntry {
        root_index,
        absolute,
        parent_absolute,
        parent,
        name,
        logical_path: logical_path.to_owned(),
    })
}

fn validated_relative_path(path: &str) -> Result<PathBuf> {
    if path.is_empty()
        || path.contains(['\\', ':', '\0'])
        || path.split('/').any(|part| part.is_empty())
        || Path::new(path).is_absolute()
    {
        return Err(path_error(
            "E-SOURCE-PATH-UNSAFE",
            "source path must be project-relative and must not use traversal or alternate separators",
            Path::new(path),
            "SOURCE-PATH-002",
        ));
    }
    let mut result = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) if !part.is_empty() => result.push(part),
            _ => {
                return Err(path_error(
                    "E-SOURCE-PATH-UNSAFE",
                    "source path must contain only normal relative components",
                    Path::new(path),
                    "SOURCE-PATH-002",
                ));
            }
        }
    }
    if result.as_os_str().is_empty() {
        return Err(path_error(
            "E-SOURCE-PATH-UNSAFE",
            "source path must not be empty",
            Path::new(path),
            "SOURCE-PATH-002",
        ));
    }
    Ok(result)
}

fn is_source_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "yaml" | "yml"))
}

fn open_parent(root: &Path, relative: &Path, target: &Path) -> Result<Dir> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(|error| path_io_error(root, error, "SOURCE-PATH-002"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(path_error(
            "E-SOURCE-PATH-SYMLINK",
            "configured source root must be a non-symlink directory",
            root,
            "SOURCE-PATH-002",
        ));
    }
    let mut dir = Dir::open_ambient_dir(root, cap_std::ambient_authority())
        .map_err(|error| path_io_error(root, error, "SOURCE-PATH-002"))?;
    let mut prefix = PathBuf::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(path_error(
                "E-SOURCE-PATH-UNSAFE",
                "source parent contains a non-normal path component",
                target,
                "SOURCE-PATH-002",
            ));
        };
        prefix.push(name);
        let metadata = dir.symlink_metadata(name).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                path_error(
                    "E-SOURCE-PATH-PARENT",
                    "source and destination parents must already exist",
                    &root.join(&prefix),
                    "SOURCE-PATH-002",
                )
            } else {
                path_io_error(&root.join(&prefix), error, "SOURCE-PATH-002")
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(path_error(
                "E-SOURCE-PATH-SYMLINK",
                "source and destination parents must be existing non-symlink directories",
                &root.join(&prefix),
                "SOURCE-PATH-002",
            ));
        }
        dir = dir
            .open_dir_nofollow(name)
            .map_err(|error| path_io_error(&root.join(&prefix), error, "SOURCE-PATH-002"))?;
    }
    Ok(dir)
}

fn read_entry_state(entry: &ResolvedEntry) -> Result<SourcePathEntryState> {
    read_named_entry_state(
        &entry.parent,
        &entry.name,
        &entry.logical_path,
        &entry.absolute,
    )
}

fn read_named_entry_state(
    parent: &Dir,
    name: &str,
    logical_path: &str,
    absolute: &Path,
) -> Result<SourcePathEntryState> {
    let exact_exists = exact_entry_exists(parent, name)
        .map_err(|error| path_io_error(absolute, error, "SOURCE-PATH-005"))?;
    if !exact_exists {
        return Ok(SourcePathEntryState {
            path: logical_path.to_owned(),
            exists: false,
            regular_file: false,
            symlink: false,
            content_identity: None,
        });
    }
    let metadata = match parent.symlink_metadata(name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(SourcePathEntryState {
                path: logical_path.to_owned(),
                exists: false,
                regular_file: false,
                symlink: false,
                content_identity: None,
            });
        }
        Err(error) => return Err(path_io_error(absolute, error, "SOURCE-PATH-005")),
    };
    let symlink = metadata.file_type().is_symlink();
    let regular_file = metadata.is_file() && !symlink;
    let content_identity = if regular_file {
        let bytes = read_file_nofollow(parent, name, absolute)?;
        Some(bytes_identity(&bytes))
    } else {
        None
    };
    Ok(SourcePathEntryState {
        path: logical_path.to_owned(),
        exists: true,
        regular_file,
        symlink,
        content_identity,
    })
}

fn read_file_nofollow(parent: &Dir, name: &str, absolute: &Path) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = parent
        .open_with(name, &options)
        .map_err(|error| path_io_error(absolute, error, "SOURCE-PATH-005"))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut bytes)
        .map_err(|error| path_io_error(absolute, error, "SOURCE-PATH-005"))?;
    Ok(bytes)
}

fn exact_entry_exists(dir: &Dir, name: &str) -> io::Result<bool> {
    for entry in dir.entries()? {
        let entry = entry?;
        if entry.file_name() == Path::new(name).as_os_str() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn find_case_insensitive_alias(dir: &Dir, name: &str) -> Result<Option<String>> {
    let entries = dir
        .entries()
        .map_err(|error| path_io_error(Path::new(name), error, "SOURCE-PATH-005"))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| path_io_error(Path::new(name), error, "SOURCE-PATH-005"))?;
        let candidate = entry.file_name();
        let Some(candidate_text) = candidate.to_str() else {
            continue;
        };
        if candidate_text != name && candidate_text.eq_ignore_ascii_case(name) {
            return Ok(Some(candidate_text.to_owned()));
        }
    }
    Ok(None)
}

fn same_entry(left: &ResolvedEntry, right: &ResolvedEntry) -> bool {
    same_file::is_same_file(&left.absolute, &right.absolute).unwrap_or(false)
}

fn same_case_only_entry(
    source: &ResolvedEntry,
    destination: &ResolvedEntry,
    destination_alias: Option<&str>,
) -> bool {
    if source.parent_absolute != destination.parent_absolute {
        return false;
    }
    // On a case-sensitive filesystem the requested destination path does not
    // exist yet, so compare its enumerated alias with the exact source entry.
    // On a case-insensitive filesystem the exact lookup is sufficient.
    destination_alias == Some(source.name.as_str()) || same_entry(source, destination)
}

fn bytes_identity(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn unique_stage_name(kind: &str) -> String {
    format!(
        ".masterdata-source-path-{kind}-{}-{}.tmp",
        std::process::id(),
        STAGE_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn report(
    source: &ResolvedEntry,
    destination: &ResolvedEntry,
    status: SourcePathMutationStatus,
    diagnostic: Option<Diagnostic>,
) -> SourcePathMutationReport {
    SourcePathMutationReport {
        status,
        source_path: source.logical_path.clone(),
        destination_path: destination.logical_path.clone(),
        source_state: read_entry_state(source).ok(),
        destination_state: read_entry_state(destination).ok(),
        diagnostic,
    }
}

fn path_diagnostic(
    code: &str,
    message: impl Into<String>,
    path: &Path,
    requirement: &str,
) -> Diagnostic {
    Diagnostic::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement(requirement)
}

fn path_error(
    code: &str,
    message: impl Into<String>,
    path: &Path,
    requirement: &str,
) -> MasterdataError {
    MasterdataError {
        diagnostic: Box::new(path_diagnostic(code, message, path, requirement)),
    }
}

fn path_io_error(path: &Path, error: io::Error, requirement: &str) -> MasterdataError {
    MasterdataError {
        diagnostic: Box::new(
            Diagnostic::new("E-SOURCE-PATH-IO", ErrorKind::Io, error.to_string())
                .with_source(path.to_path_buf())
                .with_related_requirement(requirement),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn project() -> TempDir {
        let temp = tempfile::tempdir().expect("temp project");
        fs::create_dir_all(temp.path().join("sources/data/nested")).expect("source dirs");
        fs::create_dir_all(temp.path().join("other")).expect("other root");
        fs::write(
            temp.path().join("masterdata.toml"),
            r#"[project]
id = "source.path.test"
name = "Source Path"
version = "0.1.0"

[sources]
roots = ["sources", "other"]

[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"
"#,
        )
        .expect("config");
        fs::write(temp.path().join("sources/data/item.yaml"), b"old\n").expect("source");
        temp
    }

    fn request(source: &str, destination: &str) -> SourcePathMutationRequest {
        SourcePathMutationRequest {
            source_path: source.to_owned(),
            destination_path: destination.to_owned(),
        }
    }

    #[test]
    fn same_folder_rename_and_same_root_move_preserve_exact_bytes() {
        let temp = project();
        let service = NativeApplicationService::new();
        let report = service
            .rename_source_file(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/item.yaml", "sources/data/renamed.yaml"),
            )
            .expect("rename report");
        assert_eq!(report.status, SourcePathMutationStatus::Success);
        assert_eq!(
            fs::read(temp.path().join("sources/data/renamed.yaml")).unwrap(),
            b"old\n"
        );
        assert!(!temp.path().join("sources/data/item.yaml").exists());

        let report = service
            .rename_source_file(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/renamed.yaml", "sources/data/nested/moved.yml"),
            )
            .expect("move report");
        assert_eq!(report.status, SourcePathMutationStatus::Success);
        assert_eq!(
            fs::read(temp.path().join("sources/data/nested/moved.yml")).unwrap(),
            b"old\n"
        );
    }

    #[test]
    fn invalid_extension_cross_root_missing_parent_and_destination_conflict_are_rejected() {
        let temp = project();
        let service = NativeApplicationService::new();
        for (source, destination, code) in [
            (
                "sources/data/item.yaml",
                "sources/data/new.txt",
                "E-SOURCE-PATH-EXTENSION",
            ),
            (
                "sources/data/item.yaml",
                "other/item.yaml",
                "E-SOURCE-PATH-CROSS-ROOT",
            ),
            (
                "sources/data/item.yaml",
                "sources/missing/new.yaml",
                "E-SOURCE-PATH-PARENT",
            ),
            (
                "sources/data/item.yaml",
                "sources/data/../new.yaml",
                "E-SOURCE-PATH-UNSAFE",
            ),
            (
                "sources/data/item.yaml",
                "../outside.yaml",
                "E-SOURCE-PATH-UNSAFE",
            ),
        ] {
            let error = service
                .rename_source_file(
                    Some(temp.path()),
                    temp.path(),
                    &request(source, destination),
                )
                .expect_err("invalid path");
            assert_eq!(error.diagnostic().code, code);
        }
        fs::write(
            temp.path().join("sources/data/existing.yaml"),
            b"external\n",
        )
        .unwrap();
        let report = service
            .rename_source_file(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/item.yaml", "sources/data/existing.yaml"),
            )
            .unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::Conflict);
        assert_eq!(
            fs::read(temp.path().join("sources/data/item.yaml")).unwrap(),
            b"old\n"
        );
        assert_eq!(
            fs::read(temp.path().join("sources/data/existing.yaml")).unwrap(),
            b"external\n"
        );
    }

    #[test]
    fn source_and_destination_races_are_conflicts_before_mutation() {
        let temp = project();
        let project = Project::discover(Some(temp.path()), temp.path()).unwrap();
        let source = request("sources/data/item.yaml", "sources/data/raced.yaml");
        let (source_entry, destination_entry) = resolve_pair(&project, &source).unwrap();
        let mut hook = |point| {
            if point == MutationPoint::BeforeDestinationCreate {
                destination_entry
                    .parent
                    .write(&destination_entry.name, b"external\n")?;
            }
            Ok(())
        };
        let report = rename_source_file_impl(&project, &source, &mut hook).unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::Conflict);
        assert!(source_entry.absolute.exists());
        assert_eq!(fs::read(destination_entry.absolute).unwrap(), b"external\n");

        fs::remove_file(temp.path().join("sources/data/raced.yaml")).unwrap();
        let source = request("sources/data/item.yaml", "sources/data/raced.yaml");
        let mut hook = |point| {
            if point == MutationPoint::BeforeDestinationCreate {
                fs::write(temp.path().join("sources/data/item.yaml"), b"changed\n")?;
            }
            Ok(())
        };
        let report = rename_source_file_impl(&project, &source, &mut hook).unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::Conflict);
        assert_eq!(
            fs::read(temp.path().join("sources/data/item.yaml")).unwrap(),
            b"changed\n"
        );
        assert!(!temp.path().join("sources/data/raced.yaml").exists());
    }

    #[test]
    fn source_disappearing_at_stage_boundary_is_a_conflict() {
        let temp = project();
        let project = Project::discover(Some(temp.path()), temp.path()).unwrap();
        let mutation_request = request("sources/data/item.yaml", "sources/data/disappeared.yaml");
        let mut hook = |point| {
            if point == MutationPoint::BeforeSourceStage {
                fs::remove_file(temp.path().join("sources/data/item.yaml"))?;
            }
            Ok(())
        };
        let report = rename_source_file_impl(&project, &mutation_request, &mut hook).unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::Conflict);
        assert!(!temp.path().join("sources/data/item.yaml").exists());
        assert!(!temp.path().join("sources/data/disappeared.yaml").exists());
    }

    #[test]
    fn known_failure_and_outcome_unknown_are_distinct() {
        let temp = project();
        let project = Project::discover(Some(temp.path()), temp.path()).unwrap();
        let service = NativeApplicationService::new();
        let mutation_request = request("sources/data/item.yaml", "sources/data/failure.yaml");
        let mut hook = |point| {
            if point == MutationPoint::BeforeDestinationCreate {
                return Err(io::Error::other("injected pre-mutation failure"));
            }
            Ok(())
        };
        let report = rename_source_file_impl(&project, &mutation_request, &mut hook).unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::Failure);
        assert!(temp.path().join("sources/data/item.yaml").exists());
        assert!(!temp.path().join("sources/data/failure.yaml").exists());

        let mutation_request = request("sources/data/item.yaml", "sources/data/unknown.yaml");
        let mut hook = |point| {
            if point == MutationPoint::AfterDestinationCreate {
                return Err(io::Error::other("injected post-create failure"));
            }
            Ok(())
        };
        let report = rename_source_file_impl(&project, &mutation_request, &mut hook).unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::OutcomeUnknown);
        assert!(!temp.path().join("sources/data/item.yaml").exists());
        assert!(temp.path().join("sources/data/unknown.yaml").exists());
        let state = service
            .source_path_state(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/item.yaml", "sources/data/unknown.yaml"),
            )
            .expect("unknown outcome can be rechecked");
        assert!(!state.source.exists);
        assert!(state.destination.exists);
    }

    #[test]
    fn source_replacement_after_destination_create_is_not_deleted() {
        let temp = project();
        let project = Project::discover(Some(temp.path()), temp.path()).unwrap();
        let mutation_request = request("sources/data/item.yaml", "sources/data/moved.yaml");
        let mut hook = |point| {
            if point == MutationPoint::AfterDestinationCreate {
                fs::write(temp.path().join("sources/data/item.yaml"), b"external\n")?;
            }
            Ok(())
        };
        let report = rename_source_file_impl(&project, &mutation_request, &mut hook).unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::OutcomeUnknown);
        assert_eq!(
            fs::read(temp.path().join("sources/data/item.yaml")).unwrap(),
            b"external\n"
        );
        assert_eq!(
            fs::read(temp.path().join("sources/data/moved.yaml")).unwrap(),
            b"old\n"
        );
    }

    #[test]
    fn case_only_rename_is_a_normal_success() {
        let temp = project();
        let service = NativeApplicationService::new();
        fs::rename(
            temp.path().join("sources/data/item.yaml"),
            temp.path().join("sources/data/Item.yaml"),
        )
        .unwrap();
        let report = service
            .rename_source_file(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/Item.yaml", "sources/data/item.yaml"),
            )
            .unwrap();
        assert_eq!(report.status, SourcePathMutationStatus::Success);
        assert_eq!(
            fs::read(temp.path().join("sources/data/item.yaml")).unwrap(),
            b"old\n"
        );
        let leftovers = fs::read_dir(temp.path().join("sources/data"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".masterdata-source-path-"))
            .collect::<Vec<_>>();
        assert!(leftovers.is_empty(), "staging entries: {leftovers:?}");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_source_and_parent_are_rejected_without_following_links() {
        use std::os::unix::fs::symlink;
        let temp = project();
        symlink(
            temp.path().join("sources/data/item.yaml"),
            temp.path().join("sources/data/link.yaml"),
        )
        .unwrap();
        let service = NativeApplicationService::new();
        let error = service
            .rename_source_file(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/link.yaml", "sources/data/link2.yaml"),
            )
            .unwrap_err();
        assert_eq!(error.diagnostic().code, "E-SOURCE-PATH-SYMLINK");

        symlink(
            temp.path().join("sources/data"),
            temp.path().join("sources/link-dir"),
        )
        .unwrap();
        let error = service
            .rename_source_file(
                Some(temp.path()),
                temp.path(),
                &request("sources/data/item.yaml", "sources/link-dir/new.yaml"),
            )
            .unwrap_err();
        assert_eq!(error.diagnostic().code, "E-SOURCE-PATH-SYMLINK");
    }
}
