use crate::{
    NativeApplicationService,
    authoring::{load_authoring_documents, project_relative_string},
};
use cap_fs_ext::DirExt;
use cap_std::fs::{Dir, OpenOptions};
use masterdata_core::{
    CreationChoices, Diagnostic, ErrorKind, MasterdataError, Project, SourceCreation,
    creation_choices, prepare_source_creation,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreationRequest {
    pub source_root: String,
    pub destination: String,
    pub artifact: SourceCreation,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationContext {
    pub roots: Vec<CreationRoot>,
    pub choices: CreationChoices,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationRoot {
    pub index: usize,
    pub label: String,
    pub folders: Vec<String>,
}
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CreationStatus {
    Success,
    Conflict,
    Failure,
    OutcomeUnknown,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationReport {
    pub status: CreationStatus,
    pub path: String,
    pub folder: bool,
    pub diagnostic: Option<Diagnostic>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationDestinationState {
    pub exists: bool,
    pub folder: bool,
    pub source: Option<String>,
}

impl NativeApplicationService {
    pub fn creation_context(
        &self,
        explicit: Option<&Path>,
        cwd: &Path,
    ) -> masterdata_core::Result<CreationContext> {
        let project = Project::discover(explicit, cwd)?;
        let (documents, _) = load_authoring_documents(&project, None)?;
        let mut roots = Vec::new();
        for (index, root) in project.info().source_roots.iter().enumerate() {
            let dir = open_root(root)?;
            let mut folders = vec![String::new()];
            collect_folders(&dir, "", &mut folders).map_err(|error| io_error(root, error))?;
            folders.sort();
            roots.push(CreationRoot {
                index,
                label: root.display().to_string(),
                folders,
            });
        }
        Ok(CreationContext {
            roots,
            choices: creation_choices(&documents),
        })
    }

    pub fn recheck_creation(
        &self,
        explicit: Option<&Path>,
        cwd: &Path,
        request: &CreationRequest,
    ) -> masterdata_core::Result<CreationDestinationState> {
        let project = Project::discover(explicit, cwd)?;
        let (parent, name, target) = destination(&project, request)?;
        match parent.symlink_metadata(&name) {
            Ok(metadata) => Ok(CreationDestinationState {
                exists: true,
                folder: metadata.is_dir(),
                source: if metadata.is_file() && !metadata.file_type().is_symlink() {
                    Some(
                        parent
                            .read_to_string(&name)
                            .map_err(|error| io_error(&target, error))?,
                    )
                } else {
                    None
                },
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(CreationDestinationState {
                exists: false,
                folder: false,
                source: None,
            }),
            Err(error) => Err(io_error(&target, error)),
        }
    }

    pub fn create_source(
        &self,
        explicit: Option<&Path>,
        cwd: &Path,
        request: &CreationRequest,
    ) -> masterdata_core::Result<CreationReport> {
        let project = Project::discover(explicit, cwd)?;
        let (parent, name, target) = destination(&project, request)?;
        let folder = matches!(request.artifact, SourceCreation::Folder);
        let report = |status, diagnostic| CreationReport {
            status,
            path: project_relative_string(project.root(), &target),
            folder,
            diagnostic,
        };
        match parent.symlink_metadata(&name) {
            Ok(_) => {
                return Ok(report(
                    CreationStatus::Conflict,
                    Some(
                        create_error(
                            "E-SOURCE-CREATE-CONFLICT",
                            "destination already exists",
                            &target,
                        )
                        .diagnostic()
                        .clone(),
                    ),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&target, error)),
        }
        let source = if folder {
            None
        } else {
            let (documents, _) = load_authoring_documents(&project, None)?;
            let plan = prepare_source_creation(&documents, &target, &request.artifact)?;
            masterdata_codegen_csharp::validate_creation_names(
                &plan.document.document,
                &documents,
            )?;
            Some(plan.source)
        };
        let result = match &source {
            Some(source) => exclusive_file(&parent, &name, source.as_bytes()),
            None => parent.create_dir(&name),
        };
        if let Err(error) = result {
            let status = if error.kind() == io::ErrorKind::AlreadyExists {
                CreationStatus::Conflict
            } else {
                match parent.symlink_metadata(&name) {
                    Err(check) if check.kind() == io::ErrorKind::NotFound => {
                        CreationStatus::Failure
                    }
                    _ => CreationStatus::OutcomeUnknown,
                }
            };
            return Ok(report(
                status,
                Some(io_error(&target, error).diagnostic().clone()),
            ));
        }
        let verified = match source {
            Some(source) => parent
                .read(&name)
                .is_ok_and(|bytes| bytes == source.as_bytes()),
            None => parent
                .symlink_metadata(&name)
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink()),
        };
        if !verified {
            return Ok(report(
                CreationStatus::OutcomeUnknown,
                Some(
                    create_error(
                        "E-SOURCE-CREATE-OUTCOME-UNKNOWN",
                        "created destination could not be verified; recheck before retry",
                        &target,
                    )
                    .diagnostic()
                    .clone(),
                ),
            ));
        }
        Ok(report(CreationStatus::Success, None))
    }
}

fn destination(
    project: &Project,
    request: &CreationRequest,
) -> masterdata_core::Result<(Dir, String, PathBuf)> {
    let info = project.info();
    let root = info
        .source_roots
        .iter()
        .find(|root| root.to_string_lossy() == request.source_root)
        .ok_or_else(|| {
            create_error(
                "E-SOURCE-CREATE-PATH",
                "select a configured source root",
                project.root(),
            )
        })?;
    // The wire path uses '/' independently of the host OS. Reject alternate
    // separators, drive/stream syntax and traversal before entering the Dir capability.
    let path = Path::new(&request.destination);
    if request.destination.is_empty()
        || request.destination.contains(['\\', ':', '\0'])
        || path.is_absolute()
        || request
            .destination
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(create_error(
            "E-SOURCE-CREATE-PATH",
            "destination must be a relative path inside the selected root without traversal",
            root,
        ));
    }
    if !matches!(request.artifact, SourceCreation::Folder)
        && !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("yaml" | "yml")
        )
    {
        return Err(create_error(
            "E-SOURCE-CREATE-EXTENSION",
            "source file extension must be .yaml or .yml",
            &root.join(path),
        ));
    }
    let mut dir = open_root(root)?;
    let parent_path = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // Open one component at a time without following links. A check-then-open
    // alone permits a symlink swap; held directory capabilities preserve scope.
    // EVIDENCE: SOURCE-CREATE-002.
    let mut prefix = PathBuf::new();
    for component in parent_path
        .components()
        .filter(|part| !matches!(part, Component::CurDir))
    {
        prefix.push(component);
        let metadata = dir
            .symlink_metadata(component.as_os_str())
            .map_err(|error| io_error(&root.join(&prefix), error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(create_error(
                "E-SOURCE-CREATE-PATH",
                "parent must be an existing non-symlink directory",
                &root.join(&prefix),
            ));
        }
        dir = dir
            .open_dir_nofollow(component.as_os_str())
            .map_err(|error| io_error(&root.join(&prefix), error))?;
    }
    let parent = dir;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("validated UTF-8 destination")
        .to_owned();
    Ok((parent, name, root.join(path)))
}
pub(super) fn open_root(path: &Path) -> masterdata_core::Result<Dir> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(create_error(
            "E-SOURCE-CREATE-PATH",
            "source root must be a non-symlink directory",
            path,
        ));
    }
    Dir::open_ambient_dir(path, cap_std::ambient_authority()).map_err(|error| io_error(path, error))
}
pub(super) fn collect_folders(
    dir: &Dir,
    prefix: &str,
    folders: &mut Vec<String>,
) -> io::Result<()> {
    for entry in dir.entries()? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        folders.push(path.clone());
        collect_folders(&dir.open_dir_nofollow(name)?, &path, folders)?;
    }
    Ok(())
}
static STAGE_ID: AtomicU64 = AtomicU64::new(0);
fn exclusive_file(parent: &Dir, target: &str, bytes: &[u8]) -> io::Result<()> {
    exclusive_file_with_hook(parent, target, bytes, || Ok(()))
}
fn exclusive_file_with_hook(
    parent: &Dir,
    target: &str,
    bytes: &[u8],
    before_publish: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    // WHY: publish a complete staged inode with an exclusive hard link. Opening
    // the destination for writing would expose partial bytes; rename could
    // overwrite an entry created after preflight. Dir keeps parent I/O scoped.
    // EVIDENCE: SOURCE-CREATE-002, SOURCE-CREATE-012, SOURCE-CREATE-013.
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let (stage, mut file) = loop {
        let stage = format!(
            ".masterdata-create-{}-{}.tmp",
            std::process::id(),
            STAGE_ID.fetch_add(1, Ordering::Relaxed)
        );
        match parent.open_with(&stage, &options) {
            Ok(file) => break (stage, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        before_publish()?;
        parent.hard_link(&stage, parent, target)
    })();
    drop(file);
    let _ = parent.remove_file(stage);
    result
}
fn create_error(code: &str, message: impl Into<String>, path: &Path) -> MasterdataError {
    let mut error =
        MasterdataError::new(code, ErrorKind::Validation, message).with_source(path.to_path_buf());
    error.diagnostic.related_requirements.push(
        match code {
            "E-SOURCE-CREATE-CONFLICT" => "SOURCE-CREATE-012",
            "E-SOURCE-CREATE-OUTCOME-UNKNOWN" => "SOURCE-CREATE-014",
            "E-SOURCE-CREATE-EXTENSION" => "SOURCE-CREATE-003",
            _ => "SOURCE-CREATE-002",
        }
        .into(),
    );
    error
}
fn io_error(path: &Path, error: io::Error) -> MasterdataError {
    MasterdataError::new("E-SOURCE-CREATE-IO", ErrorKind::Io, error.to_string())
        .with_source(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_created_after_staging_is_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let dir = open_root(temp.path()).unwrap();
        let error = exclusive_file_with_hook(&dir, "new.yaml", b"candidate", || {
            dir.write("new.yaml", b"external")?;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(dir.read("new.yaml").unwrap(), b"external");
        assert_eq!(dir.entries().unwrap().count(), 1);
    }
    #[test]
    fn staged_io_failure_leaves_no_partial_destination() {
        let temp = tempfile::tempdir().unwrap();
        let dir = open_root(temp.path()).unwrap();
        let error = exclusive_file_with_hook(&dir, "new.yaml", b"candidate", || {
            Err(io::Error::other("injected staging failure"))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(!dir.try_exists("new.yaml").unwrap());
        assert_eq!(dir.entries().unwrap().count(), 0);
    }
}
