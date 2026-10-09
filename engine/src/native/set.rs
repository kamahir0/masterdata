use super::*;
use crate::{migration, project::Project};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
struct Closure {
    root: PathBuf,
    roots: Vec<PathBuf>,
    bindings: Vec<String>,
    root_ids: Vec<Arc<Identity>>,
    config: Snapshot,
    sources: BTreeMap<String, Snapshot>,
}
pub(crate) fn confirm_saved_input(project: &Project) -> Result<()> {
    Closure::capture(project).map(|_| ()).map_err(|error| {
        Error::new(
            "E-BUILD-SNAPSHOT",
            format!("saved input changed during capture: {error}"),
        )
    })
}
fn stale(message: impl Into<String>) -> Error {
    Error::new("E-MIGRATION-STALE", message)
}
impl Closure {
    fn capture(project: &Project) -> Result<Self> {
        let config = capture(
            &project.root,
            std::slice::from_ref(&project.root),
            "masterdata.toml",
        )?;
        if config.bytes != project.config_bytes {
            return Err(stale("configuration changed during Plan derivation"));
        }
        let sources = project
            .sources
            .iter()
            .map(|(path, source)| {
                let actual = capture(&project.root, &project.roots, path)?;
                if actual.bytes != source.bytes {
                    return Err(stale(format!(
                        "{path}: source changed during Plan derivation"
                    )));
                }
                Ok((path.clone(), actual))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let closure = Self {
            root: project.root.clone(),
            roots: project.roots.clone(),
            bindings: project.config.sources.roots.clone(),
            config,
            sources,
            root_ids: project
                .roots
                .iter()
                .map(|p| path_identity(p).map(Arc::new).map_err(io_error))
                .collect::<Result<_>>()?,
        };
        closure.verify(&BTreeMap::new())?;
        Ok(closure)
    }
    fn environment(&self) -> Result<()> {
        preflight(
            &self.root,
            std::slice::from_ref(&self.root),
            "masterdata.toml",
            &self.config,
        )
        .map_err(|e| stale(e.to_string()))?;
        for ((binding, root), identity) in self.bindings.iter().zip(&self.roots).zip(&self.root_ids)
        {
            let path =
                checked_path(&self.root, &self.roots, binding).map_err(|e| stale(e.to_string()))?;
            if path.canonicalize().map_err(io_error)? != *root
                || path_identity(&path).map_err(io_error)? != **identity
            {
                return Err(stale("configured source root identity changed"));
            }
        }
        Ok(())
    }
    fn verify(&self, updated: &BTreeMap<String, Snapshot>) -> Result<()> {
        self.environment()?;
        let mut membership = BTreeSet::new();
        for root in &self.roots {
            crate::project::enumerate(root, &mut membership, &mut BTreeSet::new())?;
        }
        if membership != self.sources.values().map(|s| s.path.clone()).collect() {
            return Err(stale(
                "source-set membership changed; request a new Plan explicitly",
            ));
        }
        // All resolution inputs, including non-target sources, are witnessed.
        // A locally cached declaration or mtime cannot authorize a structural write.
        for (path, original) in &self.sources {
            preflight(
                &self.root,
                &self.roots,
                path,
                updated.get(path).unwrap_or(original),
            )
            .map_err(|e| stale(format!("{path}: {e}")))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SourceSetPlan {
    pub plan: migration::Plan,
    closure: Closure,
}
#[derive(Clone, Default)]
pub enum SetFault {
    #[default]
    None,
    #[cfg(feature = "oracle-faults")]
    CommitFailure {
        source: String,
        rollback_failure: Option<String>,
    },
    #[cfg(feature = "oracle-faults")]
    ObservationUnknown { source: String },
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFileState {
    pub source: String,
    pub state: String,
    pub commit: Option<WriteResult>,
    pub rollback: Option<WriteResult>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub id: String,
    pub directory: String,
    pub message: String,
    pub files: Vec<RecoveryFile>,
    #[serde(skip)]
    pub snapshots: BTreeMap<String, Snapshot>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryFile {
    pub source: String,
    pub state: String,
    pub old_copy: String,
    pub new_copy: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetResult {
    pub outcome: Outcome,
    pub message: String,
    pub files: Vec<SetFileState>,
    pub recovery: Option<RecoveryInfo>,
    #[serde(skip)]
    pub snapshots: BTreeMap<String, Snapshot>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ObjectId {
    filesystem: String,
    object: String,
    created: String,
}
fn object_id(file: &File) -> Result<ObjectId> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata().map_err(io_error)?;
        Ok(ObjectId {
            filesystem: metadata.dev().to_string(),
            object: metadata.ino().to_string(),
            created: String::new(),
        })
    }
    #[cfg(windows)]
    {
        let Identity::Windows {
            volume,
            id,
            created,
        } = file_identity(file).map_err(io_error)?;
        Ok(ObjectId {
            filesystem: volume.to_string(),
            object: format!("{id:x?}"),
            created: created.to_string(),
        })
    }
}
fn path_object_id(path: &Path) -> Result<ObjectId> {
    #[cfg(unix)]
    {
        object_id(&File::open(path).map_err(io_error)?)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        object_id(
            &fs::OpenOptions::new()
                .access_mode(0)
                .share_mode(7)
                .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
                .open(path)
                .map_err(io_error)?,
        )
    }
}
fn captured_object_id(identity: &Identity) -> Result<ObjectId> {
    #[cfg(unix)]
    {
        let Identity::Unix(handle) = identity;
        object_id(handle.as_file())
    }
    #[cfg(windows)]
    {
        let Identity::Windows {
            volume,
            id,
            created,
        } = identity;
        Ok(ObjectId {
            filesystem: volume.to_string(),
            object: format!("{id:x?}"),
            created: created.to_string(),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entry {
    source: String,
    old: String,
    new: String,
    new_object: ObjectId,
    parent: ObjectId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Journal {
    format: u32,
    status: String,
    message: String,
    entries: Vec<Entry>,
}
#[derive(Clone, Debug)]
struct Directory {
    path: PathBuf,
    identity: Arc<Identity>,
}
impl Directory {
    fn capture(path: PathBuf) -> Result<Self> {
        for ancestor in path.ancestors() {
            if path_alias(&fs::symlink_metadata(ancestor).map_err(io_error)?) {
                return Err(Error::new(
                    "E-RECOVERY-PATH",
                    "migration information ancestry must not contain aliases",
                ));
            }
        }
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if path_alias(&metadata) || !metadata.is_dir() {
            return Err(Error::new(
                "E-RECOVERY-PATH",
                "migration information directory must not be an alias",
            ));
        }
        Ok(Self {
            identity: Arc::new(path_identity(&path).map_err(io_error)?),
            path,
        })
    }
    fn check(&self) -> Result<()> {
        let actual = Self::capture(self.path.clone())?;
        if self.identity != actual.identity {
            return Err(Error::new(
                "E-RECOVERY-PATH",
                "migration information directory changed",
            ));
        }
        Ok(())
    }
    fn write(&self, name: &str, bytes: &[u8], exclusive: bool) -> Result<()> {
        self.check()?;
        let mut stage = tempfile::NamedTempFile::new_in(&self.path).map_err(io_error)?;
        stage.write_all(bytes).map_err(io_error)?;
        stage.as_file().sync_all().map_err(io_error)?;
        self.check()?;
        if exclusive {
            stage
                .persist_noclobber(self.path.join(name))
                .map_err(|e| io_error(e.error))?;
        } else {
            let target = self.path.join(name);
            if fs::symlink_metadata(&target).is_ok_and(|m| path_alias(&m) || !m.is_file()) {
                return Err(Error::new(
                    "E-RECOVERY-PATH",
                    "journal destination is not a regular file",
                ));
            }
            stage.persist(target).map_err(|e| io_error(e.error))?;
        }
        self.check()
    }
    fn journal(&self, journal: &Journal, exclusive: bool) -> Result<()> {
        self.write(
            "journal.json",
            &serde_json::to_vec_pretty(journal)
                .map_err(|e| Error::new("E-RECOVERY-JOURNAL", e.to_string()))?,
            exclusive,
        )
    }
    fn cleanup(&self, entries: usize) {
        if self.check().is_err() {
            return;
        }
        for i in 0..entries {
            for side in ["old", "new"] {
                let _ = fs::remove_file(self.path.join(format!("{i}.{side}.bytes")));
            }
        }
        let _ = fs::remove_file(self.path.join("journal.json"));
        // Never recursively delete a directory that may contain external files.
        let _ = fs::remove_dir(&self.path);
    }
}
fn storage(root: &Path, create: bool) -> Result<Option<Directory>> {
    let mut parent = Directory::capture(root.canonicalize().map_err(io_error)?)?;
    for component in [".masterdata", "migrations"] {
        let next = parent.path.join(component);
        parent.check()?;
        match fs::symlink_metadata(&next) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && create => {
                match fs::create_dir(&next) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(io_error(e)),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_error(e)),
        }
        parent.check()?;
        parent = Directory::capture(next)?;
    }
    Ok(Some(parent))
}
fn state(snapshot: Option<&Snapshot>, entry: &Entry) -> &'static str {
    match snapshot {
        Some(s) if s.content == entry.old => "OLD",
        Some(s) if s.content == entry.new => "NEW",
        Some(_) => "Diverged",
        None => "Unavailable",
    }
}
fn owned_new(snapshot: &Snapshot, entry: &Entry) -> bool {
    snapshot.content == entry.new
        && captured_object_id(&snapshot.file).is_ok_and(|id| id == entry.new_object)
        && captured_object_id(&snapshot.parent).is_ok_and(|id| id == entry.parent)
}
fn recovery_info(
    directory: &Directory,
    journal: &Journal,
    root: &Path,
    roots: &[PathBuf],
) -> RecoveryInfo {
    RecoveryInfo {
        snapshots: BTreeMap::new(),
        id: directory.path.file_name().unwrap().to_string_lossy().into(),
        directory: directory.path.to_string_lossy().into(),
        message: journal.message.clone(),
        files: journal
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| RecoveryFile {
                source: e.source.clone(),
                state: state(capture(root, roots, &e.source).ok().as_ref(), e).into(),
                old_copy: directory
                    .path
                    .join(format!("{i}.old.bytes"))
                    .to_string_lossy()
                    .into(),
                new_copy: directory
                    .path
                    .join(format!("{i}.new.bytes"))
                    .to_string_lossy()
                    .into(),
            })
            .collect(),
    }
}
impl SourceSetPlan {
    pub(crate) fn confirm_unattempted(&self) -> Result<SetResult> {
        self.closure.verify(&BTreeMap::new())?;
        Ok(SetResult {
            outcome: Outcome::NotAttempted,
            message: "No commit attempt was accepted for this Plan".into(),
            files: self
                .plan
                .candidates
                .keys()
                .map(|source| SetFileState {
                    source: source.clone(),
                    state: "OLD".into(),
                    commit: Some(WriteResult::new(
                        source,
                        Outcome::NotAttempted,
                        "no accepted attempt",
                    )),
                    rollback: None,
                })
                .collect(),
            recovery: None,
            snapshots: self.closure.sources.clone(),
        })
    }
    pub fn prepare(project: &Project, plan: migration::Plan) -> Result<Self> {
        if !pending_recovery(&project.root)?.is_empty() {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source-set recovery must be resolved first",
            ));
        }
        let closure = Closure::capture(project)?;
        for (path, candidate) in &plan.candidates {
            if closure
                .sources
                .get(path)
                .is_none_or(|s| s.bytes != candidate.before.bytes)
            {
                return Err(stale(
                    "candidate did not originate from this captured source set",
                ));
            }
        }
        Ok(Self { plan, closure })
    }
    pub fn commit(&self, authorize_destructive: bool, fault: SetFault) -> Result<SetResult> {
        self.commit_authorized(authorize_destructive, fault, || Ok(()))
    }
    pub fn commit_authorized(
        &self,
        authorize_destructive: bool,
        fault: SetFault,
        authorize: impl Fn() -> Result<()>,
    ) -> Result<SetResult> {
        if self.plan.destructive && !authorize_destructive {
            return Err(Error::new(
                "E-MIGRATION-AUTHORIZATION",
                "explicit destructive authorization required",
            ));
        }
        authorize()?;
        if !pending_recovery(&self.closure.root)?.is_empty() {
            return Err(Error::new("E-RECOVERY-REQUIRED", "writes are gated"));
        }
        self.closure.verify(&BTreeMap::new())?;
        let result = SetResult {
            outcome: Outcome::Success,
            message: String::new(),
            files: self
                .plan
                .candidates
                .keys()
                .map(|source| SetFileState {
                    source: source.clone(),
                    state: "OLD".into(),
                    commit: None,
                    rollback: None,
                })
                .collect(),
            recovery: None,
            snapshots: BTreeMap::new(),
        };
        if self.plan.candidates.is_empty() {
            return Ok(result);
        }
        let mut stages = Vec::new();
        let mut entries = Vec::new();
        // Stage every candidate before the first replacement. The staged object
        // identities, not candidate bytes alone, authorize any later rollback.
        for (path, candidate) in &self.plan.candidates {
            let original = &self.closure.sources[path];
            if original.permissions.readonly() {
                return Err(Error::new(
                    "E-MIGRATION-WRITE",
                    format!("{path}: source is read-only"),
                ));
            }
            let mut stage = tempfile::NamedTempFile::new_in(original.path.parent().unwrap())
                .map_err(io_error)?;
            stage
                .as_file()
                .set_permissions(original.permissions.clone())
                .map_err(io_error)?;
            stage
                .write_all(candidate.after.bytes.as_bytes())
                .map_err(io_error)?;
            stage.as_file().sync_all().map_err(io_error)?;
            entries.push(Entry {
                source: path.clone(),
                old: original.content.clone(),
                new: candidate.after.identity.clone(),
                new_object: object_id(stage.as_file())?,
                parent: path_object_id(original.path.parent().unwrap())?,
            });
            stages.push(stage);
        }
        let parent = storage(&self.closure.root, true)?.unwrap();
        parent.check()?;
        let directory = Directory::capture(
            tempfile::Builder::new()
                .prefix("set-")
                .tempdir_in(&parent.path)
                .map_err(io_error)?
                .keep(),
        )?;
        parent.check()?;
        let mut journal = Journal {
            format: 1,
            status: "active".into(),
            message: "Source-set commit has not been confirmed".into(),
            entries,
        };
        let backups = || -> Result<()> {
            for (i, (path, candidate)) in self.plan.candidates.iter().enumerate() {
                directory.write(
                    &format!("{i}.old.bytes"),
                    self.closure.sources[path].bytes.as_bytes(),
                    true,
                )?;
                directory.write(
                    &format!("{i}.new.bytes"),
                    candidate.after.bytes.as_bytes(),
                    true,
                )?;
            }
            directory.journal(&journal, true)
        };
        if let Err(error) = backups() {
            directory.cleanup(journal.entries.len());
            return Err(error);
        }
        self.execute(stages, directory, &mut journal, result, fault, authorize)
    }
    fn execute(
        &self,
        stages: Vec<tempfile::NamedTempFile>,
        directory: Directory,
        journal: &mut Journal,
        mut result: SetResult,
        fault: SetFault,
        authorize: impl Fn() -> Result<()>,
    ) -> Result<SetResult> {
        let mut updated = BTreeMap::new();
        let mut message = None;
        for (i, stage) in stages.into_iter().enumerate() {
            let entry = &journal.entries[i];
            if let Err(error) = authorize()
                .and_then(|_| directory.check())
                .and_then(|_| self.closure.verify(&updated))
            {
                result.files[i].commit = Some(WriteResult::new(
                    &entry.source,
                    Outcome::Conflict,
                    error.to_string(),
                ));
                message = Some(error.to_string());
                break;
            }
            #[cfg(feature = "oracle-faults")]
            if matches!(&fault, SetFault::CommitFailure { source, .. } if *source == entry.source) {
                result.files[i].commit = Some(WriteResult::new(
                    &entry.source,
                    Outcome::Failure,
                    "injected source replacement failure",
                ));
                message = Some("source replacement failed".into());
                break;
            }
            let original = &self.closure.sources[&entry.source];
            // The final source-specific preflight follows closure verification.
            if let Err(error) = preflight(
                &self.closure.root,
                &self.closure.roots,
                &entry.source,
                original,
            ) {
                result.files[i].commit = Some(WriteResult::new(
                    &entry.source,
                    Outcome::Conflict,
                    error.to_string(),
                ));
                message = Some(error.to_string());
                break;
            }
            let replacement = stage.persist(&original.path);
            #[cfg(feature = "oracle-faults")]
            let unobserved = matches!(&fault, SetFault::ObservationUnknown { source } if *source == entry.source);
            #[cfg(not(feature = "oracle-faults"))]
            let unobserved = false;
            let actual = if unobserved {
                None
            } else {
                capture(&self.closure.root, &self.closure.roots, &entry.source).ok()
            };
            if replacement.is_ok() && actual.as_ref().is_some_and(|s| owned_new(s, entry)) {
                result.files[i].commit =
                    Some(WriteResult::new(&entry.source, Outcome::Success, ""));
                updated.insert(entry.source.clone(), actual.unwrap());
            } else {
                let outcome = if replacement.is_err()
                    && actual.as_ref().is_some_and(|s| s.matches(original))
                {
                    Outcome::Failure
                } else {
                    Outcome::OutcomeUnknown
                };
                let error = replacement.err().map_or_else(
                    || "source replacement could not be confirmed".into(),
                    |e| e.to_string(),
                );
                result.files[i].commit = Some(WriteResult::new(&entry.source, outcome, &error));
                message = Some(error);
                break;
            }
        }
        if message.is_none()
            && let Err(error) = self.closure.verify(&updated)
        {
            message = Some(error.to_string());
        }
        if let Some(error) = message {
            for (i, entry) in journal.entries.iter().enumerate().rev() {
                let actual = capture(&self.closure.root, &self.closure.roots, &entry.source).ok();
                if actual.as_ref().is_some_and(|s| s.content == entry.old) {
                    continue;
                }
                if !actual.as_ref().is_some_and(|s| owned_new(s, entry)) {
                    result.files[i].rollback = Some(WriteResult::new(
                        &entry.source,
                        Outcome::Conflict,
                        "actual source is not the transaction's staged object",
                    ));
                    continue;
                }
                #[cfg(feature = "oracle-faults")]
                if matches!(&fault, SetFault::CommitFailure { rollback_failure: Some(source), .. } if *source == entry.source)
                {
                    result.files[i].rollback = Some(WriteResult::new(
                        &entry.source,
                        Outcome::Failure,
                        "injected rollback failure",
                    ));
                    continue;
                }
                if let Err(error) = self.closure.environment() {
                    result.files[i].rollback = Some(WriteResult::new(
                        &entry.source,
                        Outcome::Conflict,
                        error.to_string(),
                    ));
                    continue;
                }
                let original = &self.closure.sources[&entry.source];
                let (rollback, _) = commit(
                    &self.closure.root,
                    &self.closure.roots,
                    &entry.source,
                    actual.as_ref().unwrap(),
                    &original.bytes,
                    Fault::None,
                );
                result.files[i].rollback = Some(rollback);
            }
            result.message = error;
            result.outcome = Outcome::Failure;
        }
        let _ = fault;
        for (i, entry) in journal.entries.iter().enumerate() {
            let actual = capture(&self.closure.root, &self.closure.roots, &entry.source).ok();
            result.files[i].state = state(actual.as_ref(), entry).into();
            if let Some(actual) = actual {
                result.snapshots.insert(entry.source.clone(), actual);
            }
            if result.files[i].commit.is_none() {
                result.files[i].commit = Some(WriteResult::new(
                    &entry.source,
                    Outcome::NotAttempted,
                    "an earlier source failed",
                ));
            }
        }
        let expected = if result.outcome == Outcome::Success {
            "NEW"
        } else {
            "OLD"
        };
        if result.files.iter().any(|s| s.state != expected) {
            result.outcome = Outcome::RecoveryRequired;
            journal.status = "recovery".into();
            journal.message = result.message.clone();
            let _ = directory.journal(journal, false);
            result.recovery = Some(recovery_info(
                &directory,
                journal,
                &self.closure.root,
                &self.closure.roots,
            ));
        } else {
            journal.status = expected.into();
            journal.message = result.message.clone();
            if directory.journal(journal, false).is_ok() {
                directory.cleanup(journal.entries.len());
            } else {
                result.outcome = Outcome::RecoveryRequired;
                result.message =
                    "source set is observed, but the recovery journal could not be finalized"
                        .into();
                result.recovery = Some(recovery_info(
                    &directory,
                    journal,
                    &self.closure.root,
                    &self.closure.roots,
                ));
            }
        }
        Ok(result)
    }
}

fn journals(root: &Path) -> Result<Vec<(Directory, Journal)>> {
    let Some(storage) = storage(root, false)? else {
        return Ok(vec![]);
    };
    let mut journals = Vec::new();
    for entry in fs::read_dir(&storage.path).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        if !entry.file_name().to_string_lossy().starts_with("set-") {
            continue;
        }
        let directory = Directory::capture(entry.path())?;
        let path = directory.path.join("journal.json");
        match fs::symlink_metadata(&path) {
            Ok(m) if path_alias(&m) || !m.is_file() => {
                return Err(Error::new(
                    "E-RECOVERY-JOURNAL",
                    "journal is not a regular file",
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(io_error(e)),
            _ => {}
        }
        let journal: Journal = serde_json::from_slice(&fs::read(&path).map_err(io_error)?)
            .map_err(|e| Error::new("E-RECOVERY-JOURNAL", e.to_string()))?;
        if journal.format != 1
            || journal.entries.is_empty()
            || journal
                .entries
                .iter()
                .any(|e| relative_safe(&e.source).is_err())
        {
            return Err(Error::new(
                "E-RECOVERY-JOURNAL",
                "unsupported recovery information",
            ));
        }
        if !matches!(journal.status.as_str(), "OLD" | "NEW") {
            journals.push((directory, journal));
        }
    }
    storage.check()?;
    Ok(journals)
}
pub fn pending_recovery(root: &Path) -> Result<Vec<RecoveryInfo>> {
    let records = journals(root)?;
    if records.is_empty() {
        return Ok(vec![]);
    }
    let project = Project::open(root)?;
    Ok(records
        .iter()
        .map(|(directory, journal)| {
            recovery_info(directory, journal, &project.root, &project.roots)
        })
        .collect())
}
pub fn has_pending_recovery(root: &Path) -> Result<bool> {
    Ok(!journals(root)?.is_empty())
}
pub fn recheck_recovery(root: &Path, id: &str) -> Result<RecoveryInfo> {
    let (directory, mut journal) = journals(root)?
        .into_iter()
        .find(|(directory, _)| directory.path.file_name().unwrap().to_string_lossy() == id)
        .ok_or_else(|| Error::new("E-RECOVERY-MISSING", "recovery information is unavailable"))?;
    let project = Project::open(root)?;
    let info = recovery_info(&directory, &journal, &project.root, &project.roots);
    let closure = Closure::capture(&project)?;
    let complete = ["OLD", "NEW"]
        .into_iter()
        .find(|state| info.files.iter().all(|file| file.state == *state));
    let Some(complete) = complete else {
        return Err(Error::new(
            "E-RECOVERY-REQUIRED",
            "source set remains mixed or unavailable; inspect the retained OLD / NEW copies",
        ));
    };
    for (entry, file) in journal.entries.iter().zip(&info.files) {
        let actual = closure
            .sources
            .get(&file.source)
            .ok_or_else(|| stale("recovery source disappeared"))?;
        if state(Some(actual), entry) != complete {
            return Err(stale("recovery source changed during observation"));
        }
        crate::source::Document::parse(actual.bytes.clone())?;
    }
    closure.verify(&BTreeMap::new())?;
    journal.status = complete.into();
    journal.message = format!("Fresh actual source set confirmed {complete}");
    directory.journal(&journal, false)?;
    directory.cleanup(journal.entries.len());
    Ok(RecoveryInfo {
        message: journal.message,
        snapshots: info
            .files
            .iter()
            .map(|file| (file.source.clone(), closure.sources[&file.source].clone()))
            .collect(),
        ..info
    })
}

pub fn restore_recovery(root: &Path, id: &str, authorized: bool) -> Result<RecoveryInfo> {
    if !authorized {
        return Err(Error::new(
            "E-RECOVERY-AUTHORIZATION",
            "explicit Restore OLD authorization required",
        ));
    }
    let (directory, mut journal) = journals(root)?
        .into_iter()
        .find(|(directory, _)| directory.path.file_name().unwrap().to_string_lossy() == id)
        .ok_or_else(|| Error::new("E-RECOVERY-MISSING", "recovery information is unavailable"))?;
    let project = Project::open(root)?;
    let closure = Closure::capture(&project)?;
    let mut old_copies = BTreeMap::new();
    for (i, entry) in journal.entries.iter().enumerate() {
        let actual = closure
            .sources
            .get(&entry.source)
            .ok_or_else(|| stale("recovery source is unavailable"))?;
        if actual.content == entry.old {
            continue;
        }
        if !owned_new(actual, entry) {
            return Err(Error::new(
                "E-RECOVERY-CONFLICT",
                format!(
                    "{}: actual source is not the transaction's staged object; inspect the retained copies",
                    entry.source
                ),
            ));
        }
        directory.check()?;
        let path = directory.path.join(format!("{i}.old.bytes"));
        let logical = path
            .strip_prefix(&project.root)
            .map_err(|_| Error::new("E-RECOVERY-PATH", "backup escaped project"))?
            .to_string_lossy()
            .replace('\\', "/");
        let backup = capture(
            &project.root,
            std::slice::from_ref(&directory.path),
            &logical,
        )?;
        if backup.content != entry.old {
            return Err(Error::new(
                "E-RECOVERY-JOURNAL",
                "OLD copy no longer matches the reviewed source",
            ));
        }
        crate::source::Document::parse(backup.bytes.clone())?;
        old_copies.insert(entry.source.clone(), backup);
    }
    let mut updated = BTreeMap::new();
    for entry in &journal.entries {
        let Some(backup) = old_copies.get(&entry.source) else {
            continue;
        };
        directory.check()?;
        closure.verify(&updated)?;
        // Recovery may restore only a freshly confirmed object written by this
        // transaction. An external replacement with identical bytes is protected.
        let actual = &closure.sources[&entry.source];
        let (result, snapshot) = commit(
            &project.root,
            &project.roots,
            &entry.source,
            actual,
            &backup.bytes,
            Fault::None,
        );
        if result.outcome != Outcome::Success {
            journal.message = format!("Restore OLD stopped: {}", result.message);
            let _ = directory.journal(&journal, false);
            return Ok(recovery_info(
                &directory,
                &journal,
                &project.root,
                &project.roots,
            ));
        }
        updated.insert(entry.source.clone(), snapshot.unwrap());
    }
    closure.verify(&updated)?;
    recheck_recovery(root, id)
}
