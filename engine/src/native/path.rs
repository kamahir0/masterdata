use super::*;

#[derive(Clone, Debug)]
pub struct MovePlan {
    pub source: String,
    pub destination: String,
    pub base: Snapshot,
    root: PathBuf,
    selected_root: PathBuf,
    target: PathBuf,
    parent: Arc<Identity>,
}
fn exact_entry(path: &Path) -> Result<bool> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::new("E-PATH-KIND", "file name required"))?;
    Ok(fs::read_dir(path.parent().unwrap())
        .map_err(io_error)?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(io_error)?
        .iter()
        .any(|e| e.file_name() == name))
}
fn destination_available(plan: &MovePlan) -> Result<()> {
    match fs::symlink_metadata(&plan.target) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(e)),
        Ok(_) => {
            // A second hard link is a distinct destination even when file IDs
            // match. Only an absent exact spelling in the same directory can
            // be the source's case-insensitive lookup alias.
            let same_lookup = plan.base.path.parent() == plan.target.parent()
                && !exact_entry(&plan.target)?
                && capture(
                    &plan.root,
                    std::slice::from_ref(&plan.selected_root),
                    &plan.destination,
                )
                .is_ok_and(|s| s.file == plan.base.file && s.parent == plan.base.parent);
            if same_lookup || plan.source == plan.destination {
                Ok(())
            } else {
                Err(Error::new(
                    "E-PATH-CONFLICT",
                    "destination already exists; no overwrite",
                ))
            }
        }
    }
}
pub fn prepare_move(
    root: &Path,
    roots: &[PathBuf],
    source: &str,
    destination: &str,
    expected: &Snapshot,
) -> Result<MovePlan> {
    let base = preflight(root, roots, source, expected)?;
    if !exact_entry(&base.path)? {
        return Err(Error::new(
            "E-PATH-STALE",
            "source spelling is no longer current",
        ));
    }
    let selected_root = roots
        .iter()
        .find(|r| base.physical.starts_with(r))
        .ok_or_else(|| Error::new("E-PATH-SCOPE", "configured source root required"))?
        .clone();
    let target = creation_path(root, &selected_root, destination)?;
    if !target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e, "yaml" | "yml"))
    {
        return Err(Error::new(
            "E-PATH-EXTENSION",
            "destination must be .yaml or .yml",
        ));
    }
    let parent = Arc::new(path_identity(target.parent().unwrap()).map_err(io_error)?);
    let plan = MovePlan {
        source: source.into(),
        destination: destination.into(),
        base,
        root: root.into(),
        selected_root,
        target,
        parent,
    };
    destination_available(&plan)?;
    Ok(plan)
}
impl MovePlan {
    pub fn destination_matches(&self, snapshot: &Snapshot) -> bool {
        snapshot.file == self.base.file
            && snapshot.parent == self.parent
            && snapshot.bytes == self.base.bytes
    }
    fn fresh(&self) -> Result<()> {
        preflight(
            &self.root,
            std::slice::from_ref(&self.selected_root),
            &self.source,
            &self.base,
        )?;
        if creation_path(&self.root, &self.selected_root, &self.destination)? != self.target
            || path_identity(self.target.parent().unwrap())
                .map_err(io_error)?
                .ne(self.parent.as_ref())
            || !exact_entry(&self.base.path)?
        {
            return Err(Error::new(
                "E-PATH-STALE",
                "source or destination parent changed",
            ));
        }
        destination_available(self)
    }
}

// Standard fs::rename replaces a racing destination on Unix. Never fall back
// to it when an exclusive filesystem capability is unavailable.
fn rename_exclusive(plan: &MovePlan) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::{
            ffi::CString,
            os::{fd::AsRawFd, unix::ffi::OsStrExt},
        };
        let from = CString::new(plan.base.path.file_name().unwrap().as_bytes())?;
        let to = CString::new(plan.target.file_name().unwrap().as_bytes())?;
        let Identity::Unix(from_parent) = plan.base.parent.as_ref();
        let Identity::Unix(to_parent) = plan.parent.as_ref();
        // Anchor to freshly checked parent objects so a late directory swap
        // cannot redirect this syscall into a replacement directory.
        #[cfg(target_os = "macos")]
        let result = unsafe {
            libc::renameatx_np(
                from_parent.as_raw_fd(),
                from.as_ptr(),
                to_parent.as_raw_fd(),
                to.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                from_parent.as_raw_fd(),
                from.as_ptr(),
                to_parent.as_raw_fd(),
                to.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (from, to);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "exclusive rename unavailable",
            ));
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
        let from = plan
            .base
            .path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let to = plan
            .target
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let result = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) };
        if result != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

pub fn observe_move(plan: &MovePlan) -> WriteResult {
    let new = capture(
        &plan.root,
        std::slice::from_ref(&plan.selected_root),
        &plan.destination,
    );
    let old = capture(
        &plan.root,
        std::slice::from_ref(&plan.selected_root),
        &plan.source,
    );
    let old_entry = exact_entry(&plan.base.path);
    let new_entry = exact_entry(&plan.target);
    let outcome = if matches!(new_entry, Ok(true))
        && new.as_ref().is_ok_and(|s| plan.destination_matches(s))
        && (plan.source == plan.destination || matches!(old_entry, Ok(false)))
    {
        Outcome::Success
    } else if matches!(old_entry, Ok(true))
        && old.as_ref().is_ok_and(|s| s.matches(&plan.base))
        && (matches!(new_entry, Ok(false)) || !new.as_ref().is_ok_and(|s| s.file == plan.base.file))
    {
        Outcome::Failure
    } else {
        Outcome::OutcomeUnknown
    };
    WriteResult::new(
        &plan.source,
        outcome,
        format!("{} → {}", plan.source, plan.destination),
    )
}

pub fn commit_move(
    plan: &MovePlan,
    fault: Fault,
    authorize: impl FnOnce() -> Result<()>,
) -> WriteResult {
    if let Err(e) = authorize().and_then(|_| plan.fresh()) {
        return WriteResult::new(&plan.source, Outcome::Conflict, e.to_string());
    }
    if plan.source == plan.destination {
        return observe_move(plan);
    }
    #[cfg(feature = "oracle-faults")]
    if matches!(fault, Fault::BeforeCommit) {
        return WriteResult::new(&plan.source, Outcome::Failure, "injected precommit failure");
    }
    if let Err(e) = rename_exclusive(plan) {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            return WriteResult::new(
                &plan.source,
                Outcome::Conflict,
                "destination appeared before exclusive rename",
            );
        }
        let mut result = observe_move(plan);
        // An I/O failure cannot be relabeled Success solely from a later lookup.
        if result.outcome == Outcome::Success {
            result.outcome = Outcome::OutcomeUnknown;
        }
        result.message = e.to_string();
        return result;
    }
    #[cfg(feature = "oracle-faults")]
    if matches!(fault, Fault::AfterCommitObservation) {
        return WriteResult::new(
            &plan.source,
            Outcome::OutcomeUnknown,
            "observation failed after rename",
        );
    }
    let _ = fault;
    observe_move(plan)
}
