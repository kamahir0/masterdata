//! Fresh native source identity and single-file commit authority.
pub mod artifact;
pub mod dotnet;
mod namespace;
mod path;
pub mod publish;
mod set;
use crate::{
    Error, Result,
    project::{io_error, relative_safe},
    source::content_identity,
};
pub use path::{MovePlan, commit_move, observe_move, prepare_move};
#[cfg(not(windows))]
use same_file::Handle;
use serde::Serialize;
pub(crate) use set::confirm_saved_input;
pub use set::{
    RecoveryInfo, SetFault, SetResult, SourceSetPlan, has_pending_recovery, pending_recovery,
    recheck_recovery, restore_recovery,
};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub physical: PathBuf,
    pub bytes: Arc<str>,
    pub content: String,
    file: Arc<Identity>,
    parent: Arc<Identity>,
    permissions: fs::Permissions,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedIdentity {
    physical: PathBuf,
    content: String,
    file: Arc<Identity>,
    parent: Arc<Identity>,
}
#[derive(Debug, PartialEq, Eq)]
enum Identity {
    #[cfg(not(windows))]
    Unix(Handle),
    #[cfg(windows)]
    Windows {
        volume: u64,
        id: [u8; 16],
        created: u64,
    },
}
#[cfg(not(windows))]
fn file_identity(file: &File) -> std::io::Result<Identity> {
    Handle::from_file(file.try_clone()?).map(Identity::Unix)
}
#[cfg(windows)]
fn file_identity(file: &File) -> std::io::Result<Identity> {
    use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ID_INFO, FileIdInfo, GetFileInformationByHandleEx,
    };
    let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // Retaining the target handle across a Windows replace prevents MoveFileExW
    // replacement even with delete sharing. Capture 128-bit identity + creation
    // identity instead; every authorization still opens and compares actual disk.
    Ok(Identity::Windows {
        volume: info.VolumeSerialNumber,
        id: info.FileId.Identifier,
        created: file.metadata()?.creation_time(),
    })
}
fn path_identity(path: &Path) -> std::io::Result<Identity> {
    #[cfg(not(windows))]
    {
        Handle::from_path(path).map(Identity::Unix)
    }
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
        file_identity(
            &OpenOptions::new()
                .access_mode(0)
                .share_mode(7)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                .open(path)?,
        )
    }
}
impl Snapshot {
    pub fn observed_identity(&self) -> ObservedIdentity {
        ObservedIdentity {
            physical: self.physical.clone(),
            content: self.content.clone(),
            file: self.file.clone(),
            parent: self.parent.clone(),
        }
    }
    pub fn matches(&self, other: &Self) -> bool {
        self.physical == other.physical
            && self.content == other.content
            && self.file == other.file
            && self.parent == other.parent
    }
}
pub fn checked_path(root: &Path, roots: &[PathBuf], logical: &str) -> Result<PathBuf> {
    let relative = relative_safe(logical)?;
    let mut p = root.to_path_buf();
    for part in relative.components() {
        p.push(part);
        let m = fs::symlink_metadata(&p).map_err(io_error)?;
        if path_alias(&m) {
            return Err(Error::new(
                "E-PATH-ALIAS",
                "symlink source path cannot authorize writes",
            ));
        }
    }
    let actual = p.canonicalize().map_err(io_error)?;
    if !roots.iter().any(|r| actual.starts_with(r)) {
        return Err(Error::new("E-PATH-SCOPE", "source escaped configured root"));
    }
    Ok(p)
}
fn path_alias(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

/// A new destination has no source snapshot. Authorize its existing parent and
/// selected root freshly, and commit exclusively instead of reusing Save replace.
pub fn creation_path(root: &Path, selected_root: &Path, logical: &str) -> Result<PathBuf> {
    if logical.contains(['\\', ':'])
        || logical
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(Error::new(
            "E-PATH-SCOPE",
            "logical paths use relative slash-separated components",
        ));
    }
    let relative = relative_safe(logical)?;
    let filename = relative
        .file_name()
        .ok_or_else(|| Error::new("E-PATH-SCOPE", "destination name required"))?;
    if filename.to_string_lossy().ends_with(['.', ' ']) {
        return Err(Error::new("E-PATH-ALIAS", "ambiguous destination name"));
    }
    let parent = relative
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = checked_path(
        root,
        &[selected_root.to_path_buf()],
        &parent.to_string_lossy(),
    )?;
    if !parent.is_dir() {
        return Err(Error::new(
            "E-PATH-KIND",
            "existing destination parent directory required",
        ));
    }
    Ok(parent.join(filename))
}

pub fn create_exclusive(
    root: &Path,
    selected_root: &Path,
    logical: &str,
    candidate: Option<&str>,
    fault: Fault,
    authorize: impl FnOnce() -> Result<()>,
) -> WriteResult {
    let failure = |e: Error| WriteResult::new(logical, Outcome::Failure, e.to_string());
    let target = match creation_path(root, selected_root, logical) {
        Ok(p) => p,
        Err(e) => return failure(e),
    };
    let parent = target.parent().unwrap();
    let parent_identity = match path_identity(parent) {
        Ok(i) => i,
        Err(e) => return failure(io_error(e)),
    };
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            return WriteResult::new(
                logical,
                Outcome::Conflict,
                "E-CREATE-CONFLICT: destination already exists",
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return failure(io_error(e)),
    }
    let mut stage = if let Some(bytes) = candidate {
        let prepare = || -> Result<tempfile::NamedTempFile> {
            let mut file = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
            file.write_all(bytes.as_bytes()).map_err(io_error)?;
            file.as_file().sync_all().map_err(io_error)?;
            Ok(file)
        };
        match prepare() {
            Ok(file) => Some(file),
            Err(e) => return failure(e),
        }
    } else {
        None
    };
    #[cfg(feature = "oracle-faults")]
    if matches!(fault, Fault::BeforeCommit) {
        return failure(Error::new("E-CREATE-FAULT", "injected precommit failure"));
    }
    if let Err(e) = authorize() {
        return WriteResult::new(logical, Outcome::Conflict, format!("E-CREATE-STALE: {e}"));
    }
    let current_path = creation_path(root, selected_root, logical);
    if !current_path.as_ref().is_ok_and(|path| path == &target)
        || path_identity(parent).ok().as_ref() != Some(&parent_identity)
    {
        return WriteResult::new(
            logical,
            Outcome::Conflict,
            "E-CREATE-STALE: destination parent identity changed",
        );
    }
    let commit = if let Some(file) = stage.take() {
        file.persist_noclobber(&target)
            .map(|_| ())
            .map_err(|e| e.error)
    } else {
        fs::create_dir(&target)
    };
    if let Err(e) = commit {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            return WriteResult::new(
                logical,
                Outcome::Conflict,
                "E-CREATE-CONFLICT: destination appeared before exclusive commit",
            );
        }
        let missing =
            fs::symlink_metadata(&target).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
        return WriteResult::new(
            logical,
            if missing {
                Outcome::Failure
            } else {
                Outcome::OutcomeUnknown
            },
            io_error(e).to_string(),
        );
    }
    #[cfg(feature = "oracle-faults")]
    if matches!(fault, Fault::AfterCommitObservation) {
        return WriteResult::new(
            logical,
            Outcome::OutcomeUnknown,
            "E-CREATE-FAULT: observation failed after exclusive commit",
        );
    }
    let _ = fault;
    let observed = if let Some(bytes) = candidate {
        capture(root, &[selected_root.to_path_buf()], logical)
            .is_ok_and(|s| s.bytes.as_ref() == bytes && s.parent.as_ref() == &parent_identity)
    } else {
        checked_path(root, &[selected_root.to_path_buf()], logical)
            .is_ok_and(|p| p == target && p.is_dir())
            && path_identity(parent).ok().as_ref() == Some(&parent_identity)
    };
    if observed {
        WriteResult::new(logical, Outcome::Success, "Created")
    } else {
        WriteResult::new(
            logical,
            Outcome::OutcomeUnknown,
            "E-CREATE-OBSERVATION: destination cannot be confirmed",
        )
    }
}
pub fn capture(root: &Path, roots: &[PathBuf], logical: &str) -> Result<Snapshot> {
    let path = checked_path(root, roots, logical)?;
    let physical = path.canonicalize().map_err(io_error)?;
    let mut file = File::open(&path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err(Error::new("E-PATH-KIND", "source must be a regular file"));
    }
    let handle = file_identity(&file).map_err(io_error)?;
    let parent = path_identity(path.parent().unwrap()).map_err(io_error)?;
    let mut bytes = String::new();
    file.read_to_string(&mut bytes).map_err(io_error)?;
    crate::instrument::count(crate::instrument::Kind::Bytes(bytes.len() as u64));
    if handle != path_identity(&path).map_err(io_error)?
        || parent != path_identity(path.parent().unwrap()).map_err(io_error)?
    {
        return Err(Error::new(
            "E-SOURCE-CHANGED",
            "source identity changed during read",
        ));
    }
    Ok(Snapshot {
        path,
        physical,
        content: content_identity(bytes.as_bytes()),
        bytes: bytes.into(),
        file: Arc::new(handle),
        parent: Arc::new(parent),
        permissions: metadata.permissions(),
    })
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum Outcome {
    Success,
    Conflict,
    Failure,
    OutcomeUnknown,
    NotAttempted,
    RecoveryRequired,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResult {
    pub source: String,
    pub outcome: Outcome,
    pub message: String,
    pub current_identity: Option<String>,
}
impl WriteResult {
    pub fn new(source: &str, outcome: Outcome, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            outcome,
            message: message.into(),
            current_identity: None,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub enum Fault {
    #[default]
    None,
    #[cfg(feature = "oracle-faults")]
    BeforeCommit,
    #[cfg(feature = "oracle-faults")]
    AfterCommitObservation,
}
pub fn preflight(
    root: &Path,
    roots: &[PathBuf],
    logical: &str,
    expected: &Snapshot,
) -> Result<Snapshot> {
    let current = capture(root, roots, logical)?;
    if !current.matches(expected) {
        return Err(Error::new(
            "E-SOURCE-CONFLICT",
            "actual source identity differs from authorized identity",
        ));
    }
    Ok(current)
}
pub fn commit(
    root: &Path,
    roots: &[PathBuf],
    logical: &str,
    expected: &Snapshot,
    candidate: &str,
    fault: Fault,
) -> (WriteResult, Option<Snapshot>) {
    let conflict = |message: String| (WriteResult::new(logical, Outcome::Conflict, message), None);
    let current = match preflight(root, roots, logical, expected) {
        Ok(s) => s,
        Err(e) => return conflict(e.to_string()),
    };
    if current.permissions.readonly() {
        return (
            WriteResult::new(logical, Outcome::Failure, "source is read-only"),
            Some(current),
        );
    }
    let prepare = || -> Result<tempfile::NamedTempFile> {
        let mut stage =
            tempfile::NamedTempFile::new_in(current.path.parent().unwrap()).map_err(io_error)?;
        stage
            .as_file()
            .set_permissions(current.permissions.clone())
            .map_err(io_error)?;
        stage.write_all(candidate.as_bytes()).map_err(io_error)?;
        stage.as_file().sync_all().map_err(io_error)?;
        Ok(stage)
    };
    let stage = match prepare() {
        Ok(s) => s,
        Err(e) => {
            return (
                WriteResult::new(logical, Outcome::Failure, e.to_string()),
                Some(current),
            );
        }
    };
    #[cfg(feature = "oracle-faults")]
    if matches!(fault, Fault::BeforeCommit) {
        return (
            WriteResult::new(logical, Outcome::Failure, "injected precommit failure"),
            Some(current),
        );
    }
    // Stage I/O may take time; a read-cache identity never authorizes the replace.
    if let Err(e) = preflight(root, roots, logical, expected) {
        return conflict(e.to_string());
    }
    let replacement = stage.persist(&current.path);
    #[cfg(feature = "oracle-faults")]
    if replacement.is_ok() && matches!(fault, Fault::AfterCommitObservation) {
        return (
            WriteResult::new(
                logical,
                Outcome::OutcomeUnknown,
                "commit observation unavailable",
            ),
            None,
        );
    }
    let _ = fault;
    let actual = capture(root, roots, logical);
    match (replacement, actual) {
        (Ok(_), Ok(new)) if new.bytes.as_ref() == candidate => {
            let mut result = WriteResult::new(logical, Outcome::Success, "");
            result.current_identity = Some(new.content.clone());
            (result, Some(new))
        }
        (Err(e), Ok(actual)) if actual.bytes.as_ref() == current.bytes.as_ref() => (
            WriteResult::new(logical, Outcome::Failure, e.to_string()),
            Some(actual),
        ),
        (_, actual) => (
            WriteResult::new(
                logical,
                Outcome::OutcomeUnknown,
                actual.err().map_or_else(
                    || "current source differs after commit".into(),
                    |e| e.to_string(),
                ),
            ),
            None,
        ),
    }
}

/// Exclusive native rename for owned staging/backup slots. Existing destinations
/// are never replaced; callers separately witness both actual parent identities.
fn rename_slots(
    from: &Path,
    from_parent: &Identity,
    to: &Path,
    to_parent: &Identity,
) -> Result<()> {
    #[cfg(unix)]
    {
        use std::{
            ffi::CString,
            os::{fd::AsRawFd, unix::ffi::OsStrExt},
        };
        let source = CString::new(from.file_name().unwrap().as_bytes())
            .map_err(|e| Error::new("E-PATH-SCOPE", e.to_string()))?;
        let target = CString::new(to.file_name().unwrap().as_bytes())
            .map_err(|e| Error::new("E-PATH-SCOPE", e.to_string()))?;
        let Identity::Unix(source_parent) = from_parent;
        let Identity::Unix(target_parent) = to_parent;
        #[cfg(target_os = "macos")]
        let status = unsafe {
            libc::renameatx_np(
                source_parent.as_raw_fd(),
                source.as_ptr(),
                target_parent.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        #[cfg(target_os = "linux")]
        let status = unsafe {
            libc::renameat2(
                source_parent.as_raw_fd(),
                source.as_ptr(),
                target_parent.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Err(Error::new("E-PATH-SCOPE", "exclusive rename unavailable"));
        if status != 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
        let _ = (from_parent, to_parent);
        let source = from
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let target = to
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0) } == 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}
