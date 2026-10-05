//! Read-only destination namespace proofs, including missing tails. No probe
//! creates files in external destinations before ALL publish preflight succeeds.
use super::*;
use std::{ffi::OsString, path::Component};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Case {
    sensitive: bool,
    byte_based: bool,
}
impl Case {
    const BYTES: Self = Self {
        sensitive: true,
        byte_based: true,
    };
    fn unicode(sensitive: bool) -> Self {
        Self {
            sensitive,
            byte_based: false,
        }
    }
}
#[derive(Clone, Debug)]
struct Slot {
    name: OsString,
    object: Option<Arc<Identity>>,
    case: Case,
}
#[derive(Clone, Debug)]
pub(super) struct Namespace {
    pub path: PathBuf,
    slots: Vec<Slot>,
    pub prefix: Vec<(PathBuf, Arc<Identity>)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Relation {
    Same,
    Ancestor,
    Descendant,
    Separate,
}
fn error(message: impl Into<String>) -> Error {
    Error::new("E-DESTINATION-NAMESPACE", message)
}
pub(super) fn absolute(base: &Path, path: &Path) -> Result<PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component)
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(error("path escapes filesystem root"));
                }
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(error("absolute Project base required"));
    }
    Ok(normalized)
}
impl Namespace {
    pub fn resolve(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err(error("absolute destination required"));
        }
        let mut current = PathBuf::new();
        let mut slots = vec![];
        let mut prefix = vec![];
        let mut missing = false;
        let mut case = Case::BYTES;
        for component in path.components() {
            if !matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            ) {
                return Err(error("normalized destination required"));
            }
            current.push(component);
            if matches!(component, Component::Prefix(_)) {
                continue;
            }
            let name = component.as_os_str().to_owned();
            if missing {
                slots.push(Slot {
                    name,
                    object: None,
                    case,
                });
                continue;
            }
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if path_alias(&metadata) {
                        return Err(error(format!(
                            "{}: symlink/reparse ancestor or target",
                            current.display()
                        )));
                    }
                    if !metadata.is_file() && !metadata.is_dir() {
                        return Err(error(format!(
                            "{}: unexpected filesystem type",
                            current.display()
                        )));
                    }
                    let identity = Arc::new(path_identity(&current).map_err(io_error)?);
                    slots.push(Slot {
                        name,
                        object: Some(identity.clone()),
                        case,
                    });
                    if metadata.is_dir() {
                        prefix.push((current.clone(), identity));
                        case = directory_case(&current)?;
                    } else if current != path {
                        return Err(error("file cannot be a destination ancestor"));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    missing = true;
                    slots.push(Slot {
                        name,
                        object: None,
                        case,
                    });
                    #[cfg(windows)]
                    {
                        case = Case::unicode(false);
                    }
                }
                Err(e) => return Err(io_error(e)),
            }
        }
        Ok(Self {
            path: path.to_path_buf(),
            slots,
            prefix,
        })
    }
    pub fn relation(&self, other: &Self) -> Result<Relation> {
        // Different spelling of an existing path is resolved by actual identity,
        // including case/Unicode aliases and hard links, before tail comparison.
        if self
            .slots
            .last()
            .and_then(|s| s.object.as_ref())
            .zip(other.slots.last().and_then(|s| s.object.as_ref()))
            .is_some_and(|(a, b)| a == b)
        {
            return Ok(Relation::Same);
        }
        for (a, b) in self.slots.iter().zip(&other.slots) {
            if let (Some(a), Some(b)) = (&a.object, &b.object) {
                if a == b {
                    continue;
                }
                return Ok(Relation::Separate);
            }
            if a.name == b.name {
                continue;
            }
            if a.object.is_some() != b.object.is_some() {
                // Lookup under the same actual parent established absence for one
                // spelling and existence for the other; they cannot be one slot.
                return Ok(Relation::Separate);
            }
            if a.case.byte_based && b.case.byte_based {
                return Ok(Relation::Separate);
            }
            let left = a
                .name
                .to_str()
                .ok_or_else(|| error("non-UTF8 missing tail equivalence is unknown"))?;
            let right = b
                .name
                .to_str()
                .ok_or_else(|| error("non-UTF8 missing tail equivalence is unknown"))?;
            if !left.is_ascii() || !right.is_ascii() {
                return Err(error(
                    "filesystem Unicode equivalence for distinct missing components could not be proved",
                ));
            }
            if a.case.sensitive && b.case.sensitive {
                return Ok(Relation::Separate);
            }
            if !left.eq_ignore_ascii_case(right) {
                return Ok(Relation::Separate);
            }
        }
        Ok(match self.slots.len().cmp(&other.slots.len()) {
            std::cmp::Ordering::Equal => Relation::Same,
            std::cmp::Ordering::Less => Relation::Ancestor,
            std::cmp::Ordering::Greater => Relation::Descendant,
        })
    }
    pub fn fresh_with_created(&self, created: &[Arc<Identity>]) -> Result<()> {
        for (path, expected) in &self.prefix {
            let metadata = fs::symlink_metadata(path).map_err(io_error)?;
            if path_alias(&metadata)
                || !metadata.is_dir()
                || path_identity(path).map_err(io_error)?.ne(expected.as_ref())
            {
                return Err(error("destination existing prefix changed"));
            }
        }
        let now = Self::resolve(&self.path)?;
        for (expected, actual) in self.slots.iter().zip(&now.slots) {
            if expected.case != actual.case {
                return Err(error("destination lookup capability changed"));
            }
            if expected.object.is_some() && expected.object != actual.object {
                return Err(error("destination object changed"));
            }
            if expected.object.is_none()
                && actual
                    .object
                    .as_ref()
                    .is_some_and(|object| !created.contains(object))
            {
                return Err(error("destination was concurrently created"));
            }
        }
        Ok(())
    }
}

pub(super) fn create_parents(path: &Path, created: &mut Vec<Arc<Identity>>) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if !path_alias(&metadata) && metadata.is_dir() => {}
            Ok(_) => return Err(error("directory creation encountered an unexpected entry")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let parent = current
                    .parent()
                    .ok_or_else(|| error("directory parent required"))?;
                let identity = path_identity(parent).map_err(io_error)?;
                fs::create_dir(&current).map_err(io_error)?;
                if path_identity(parent).map_err(io_error)? != identity {
                    return Err(error("directory parent identity changed during creation"));
                }
                let metadata = fs::symlink_metadata(&current).map_err(io_error)?;
                if path_alias(&metadata) || !metadata.is_dir() {
                    return Err(error("new directory identity could not be established"));
                }
                created.push(Arc::new(path_identity(&current).map_err(io_error)?));
            }
            Err(e) => return Err(io_error(e)),
        }
    }
    Ok(())
}
#[cfg(target_os = "macos")]
fn directory_case(path: &Path) -> Result<Case> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    #[repr(C)]
    struct Capabilities {
        length: u32,
        capabilities: libc::vol_capabilities_attr_t,
    }
    let mut attributes: libc::attrlist = unsafe { std::mem::zeroed() };
    attributes.bitmapcount = 5;
    attributes.volattr = libc::ATTR_VOL_INFO | libc::ATTR_VOL_CAPABILITIES;
    let mut result: Capabilities = unsafe { std::mem::zeroed() };
    let name = CString::new(path.as_os_str().as_bytes()).map_err(|e| error(e.to_string()))?;
    let status = unsafe {
        libc::getattrlist(
            name.as_ptr(),
            (&mut attributes as *mut libc::attrlist).cast(),
            (&mut result as *mut Capabilities).cast(),
            std::mem::size_of::<Capabilities>(),
            0,
        )
    };
    if status != 0 {
        return Err(error(format!(
            "{}: volume namespace capability unavailable: {}",
            path.display(),
            std::io::Error::last_os_error()
        )));
    }
    if result.capabilities.valid[0] & libc::VOL_CAP_FMT_CASE_SENSITIVE == 0 {
        return Err(error("volume case capability is unknown"));
    }
    Ok(
        if result.capabilities.capabilities[0] & libc::VOL_CAP_FMT_CASE_SENSITIVE != 0 {
            Case::unicode(true)
        } else {
            Case::unicode(false)
        },
    )
}
#[cfg(windows)]
fn directory_case(path: &Path) -> Result<Case> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_CASE_SENSITIVE_INFO, FILE_FLAG_BACKUP_SEMANTICS, FileCaseSensitiveInfo,
        GetFileInformationByHandleEx,
    };
    let handle = fs::OpenOptions::new()
        .access_mode(0)
        .share_mode(7)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(io_error)?;
    let mut info: FILE_CASE_SENSITIVE_INFO = unsafe { std::mem::zeroed() };
    if unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle(),
            FileCaseSensitiveInfo,
            (&mut info as *mut FILE_CASE_SENSITIVE_INFO).cast(),
            std::mem::size_of::<FILE_CASE_SENSITIVE_INFO>() as u32,
        )
    } == 0
    {
        return Err(error(format!(
            "{}: directory case capability unavailable: {}",
            path.display(),
            std::io::Error::last_os_error()
        )));
    }
    // Existing entries use actual lookup. For a missing descendant, Win32's
    // newly-created directory need not inherit a WSL case-sensitive flag. Treat
    // future ASCII variants as possibly aliased instead of granting ownership.
    Ok(if info.Flags & 1 != 0 {
        Case::unicode(true)
    } else {
        Case::unicode(false)
    })
}
#[cfg(target_os = "linux")]
fn directory_case(path: &Path) -> Result<Case> {
    use std::os::fd::AsRawFd;
    let file = File::open(path).map_err(io_error)?;
    let mut status: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut status) } != 0 {
        return Err(io_error(std::io::Error::last_os_error()));
    }
    // Actual filesystem format + directory flags, not the OS name. ext4/f2fs
    // may enable Unicode casefold per directory; tmpfs uses byte lookup.
    match status.f_type as u64 {
        0xef53 | 0xf2f52010 => {
            let mut flags: libc::c_long = 0;
            let request: libc::c_ulong = 0x80000000
                | ((std::mem::size_of::<libc::c_long>() as u64) << 16)
                | (b'f' as u64) << 8
                | 1;
            if unsafe { libc::ioctl(file.as_raw_fd(), request, &mut flags) } != 0 {
                return Err(error("directory casefold flag is unavailable"));
            }
            Ok(if flags & 0x40000000 != 0 {
                Case::unicode(false)
            } else {
                Case::BYTES
            })
        }
        0x01021994 => Ok(Case::BYTES),
        _ => Err(error(format!(
            "filesystem {:x}: missing-tail namespace is unsupported",
            status.f_type
        ))),
    }
}
#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn directory_case(_: &Path) -> Result<Case> {
    Err(error("destination namespace unavailable on this platform"))
}
