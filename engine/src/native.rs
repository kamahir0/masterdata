//! Fresh native source identity and single-file commit authority.
use crate::{
    Error, Result,
    project::{io_error, relative_safe},
    source::content_identity,
};
#[cfg(not(windows))]
use same_file::Handle;
use serde::Serialize;
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
        if m.file_type().is_symlink() {
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
