//! Receipt-only Publish with read-only all-target preflight and independent
//! target-local staged writes. Unmanaged content and Unity .meta are never owned.
use super::*;
use super::{
    artifact::{ArtifactSet, managed_relative},
    namespace::{Namespace, Relation, absolute, create_parents},
};
use crate::delivery::SavedConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MANIFEST: &str = ".masterdata-publish-manifest.json";
#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    files: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetPreview {
    pub kind: String,
    pub configured_path: String,
    pub destination: PathBuf,
    pub additions: Vec<String>,
    pub updates: Vec<String>,
    pub removals: Vec<String>,
    pub binary_replacement: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub artifact_root: PathBuf,
    pub artifact_identity: String,
    pub config_identity: String,
    pub project_id: String,
    pub source_freshness: &'static str,
    pub targets: Vec<TargetPreview>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetResult {
    pub kind: String,
    pub destination: PathBuf,
    pub outcome: Outcome,
    pub status: &'static str,
    pub message: String,
    pub recovery_directory: Option<PathBuf>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub outcome: Outcome,
    pub source_freshness: &'static str,
    pub unity_verification: &'static str,
    pub no_op: bool,
    pub targets: Vec<TargetResult>,
}
#[derive(Default, Clone, Copy)]
pub enum PublishFault {
    #[default]
    None,
    BeforeTarget(usize),
    AfterFile {
        target: usize,
        file: usize,
        rollback_failure: bool,
    },
    UnknownAfterCommit(usize),
}
#[derive(Clone)]
struct OwnedFile {
    bytes: Vec<u8>,
    identity: Arc<Identity>,
    hash: String,
    permissions: fs::Permissions,
}
fn error(message: impl Into<String>) -> Error {
    Error::new("E-PUBLISH-PREFLIGHT", message)
}
fn owned(path: &Path) -> Result<Option<OwnedFile>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if path_alias(&metadata) || !metadata.is_file() {
                return Err(error(format!(
                    "{}: expected managed regular file [PUBLISH-PATH-004]",
                    path.display()
                )));
            }
            let mut file = File::open(path).map_err(io_error)?;
            let identity = Arc::new(file_identity(&file).map_err(io_error)?);
            let mut bytes = vec![];
            file.read_to_end(&mut bytes).map_err(io_error)?;
            let hash = content_identity(&bytes);
            let current = fs::symlink_metadata(path).map_err(io_error)?;
            if path_alias(&current)
                || !current.is_file()
                || path_identity(path).map_err(io_error)?.ne(identity.as_ref())
            {
                return Err(error("managed file changed during preflight"));
            }
            Ok(Some(OwnedFile {
                bytes,
                identity,
                hash,
                permissions: metadata.permissions(),
            }))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(e)),
    }
}
fn same_file_state(first: Option<&OwnedFile>, second: Option<&OwnedFile>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(a), Some(b)) => a.identity == b.identity && a.hash == b.hash,
        _ => false,
    }
}
struct Change {
    previous_path: PathBuf,
    next_path: PathBuf,
    previous: Option<OwnedFile>,
    next: Option<Vec<u8>>,
    namespace: Namespace,
    previous_namespace: Namespace,
}
struct Target {
    namespace: Namespace,
    preview: TargetPreview,
    changes: Vec<Change>,
}
pub struct PublishPlan {
    saved: SavedConfig,
    artifact: ArtifactSet,
    targets: Vec<Target>,
    preview: Preview,
}
fn protected(saved: &SavedConfig, artifact: &ArtifactSet, target: &Namespace) -> Result<()> {
    let root = Namespace::resolve(&saved.root)?;
    if matches!(target.relation(&root)?, Relation::Same | Relation::Ancestor) {
        return Err(error("publish target contains Project [PUBLISH-PATH-007]"));
    }
    for path in std::iter::once(saved.root.join("masterdata.toml"))
        .chain(std::iter::once(artifact.root.clone()))
        .chain(
            saved
                .config
                .sources
                .roots
                .iter()
                .chain(std::iter::once(&saved.config.build.cache))
                .map(|path| absolute(&saved.root, Path::new(path)))
                .collect::<Result<Vec<_>>>()?,
        )
    {
        if target.relation(&Namespace::resolve(&path)?)? != Relation::Separate {
            return Err(error(format!(
                "publish target overlaps protected region {} [PUBLISH-PATH-007]",
                path.display()
            )));
        }
    }
    Ok(())
}
fn protected_object(saved: &SavedConfig, artifact: &ArtifactSet, path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_error(e)),
    };
    if !metadata.is_file() || path_alias(&metadata) {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() < 2 {
            return Ok(());
        }
    }
    let object = path_identity(path).map_err(io_error)?;
    fn contains(path: &Path, object: &Identity) -> Result<bool> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(io_error(e)),
        };
        if path_alias(&metadata) || !(metadata.is_file() || metadata.is_dir()) {
            return Ok(false);
        }
        if path_identity(path).map_err(io_error)? == *object {
            return Ok(true);
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path).map_err(io_error)? {
                if contains(&entry.map_err(io_error)?.path(), object)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    // Object aliases need a metadata-only ownership check. This never reads,
    // hashes or interprets YAML, and does not make source freshness a receipt key.
    for region in std::iter::once(artifact.root.clone()).chain(
        saved
            .config
            .sources
            .roots
            .iter()
            .chain(std::iter::once(&saved.config.build.cache))
            .map(|path| absolute(&saved.root, Path::new(path)))
            .collect::<Result<Vec<_>>>()?,
    ) {
        if contains(&region, &object)? {
            return Err(error(
                "managed destination aliases a protected object [PUBLISH-PATH-001 / PUBLISH-PATH-007]",
            ));
        }
    }
    Ok(())
}
fn manifest(root: &Path) -> Result<(Option<OwnedFile>, Vec<String>)> {
    let previous = owned(&root.join(MANIFEST))?;
    let Some(file) = &previous else {
        return Ok((None, vec![]));
    };
    let parsed: Manifest = serde_json::from_slice(&file.bytes)
        .map_err(|e| error(format!("invalid manifest shape: {e} [PUBLISH-PATH-005]")))?;
    if parsed.version != 1 {
        return Err(error("unsupported manifest version [PUBLISH-PATH-005]"));
    }
    let mut seen = vec![];
    let reserved = Namespace::resolve(&root.join(MANIFEST))?;
    for value in &parsed.files {
        let path = root.join(managed_relative(value)?);
        let namespace = Namespace::resolve(&path)?;
        if namespace.relation(&reserved)? != Relation::Separate {
            return Err(error(
                "manifest path collides with reserved manifest [PUBLISH-PATH-005]",
            ));
        }
        for existing in &seen {
            if namespace.relation(existing)? != Relation::Separate {
                return Err(error(
                    "duplicate/overlapping manifest namespace [PUBLISH-PATH-005]",
                ));
            }
        }
        seen.push(namespace);
        owned(&path)?;
    }
    Ok((previous, parsed.files))
}
impl Target {
    fn prepare(
        saved: &SavedConfig,
        artifact: &ArtifactSet,
        kind: &str,
        configured: &str,
    ) -> Result<Self> {
        let path = absolute(&saved.root, Path::new(configured))?;
        let namespace = Namespace::resolve(&path)?;
        protected(saved, artifact, &namespace)?;
        let mut preview = TargetPreview {
            kind: kind.into(),
            configured_path: configured.into(),
            destination: path.clone(),
            additions: vec![],
            updates: vec![],
            removals: vec![],
            binary_replacement: false,
        };
        let mut changes = vec![];
        if kind == "binary" {
            protected_object(saved, artifact, &path)?;
            let previous = owned(&path)?;
            preview.binary_replacement = previous.is_some();
            changes.push(Change {
                previous_path: path.clone(),
                next_path: path,
                previous,
                next: Some(artifact.binary.clone()),
                namespace: namespace.clone(),
                previous_namespace: namespace.clone(),
            });
        } else {
            if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_dir()) {
                return Err(error(
                    "C# target must be a real directory [PUBLISH-PATH-003]",
                ));
            }
            let (previous_manifest, files) = manifest(&path)?;
            protected_object(saved, artifact, &path.join(MANIFEST))?;
            let reserved = Namespace::resolve(&path.join(MANIFEST))?;
            let previous = files
                .iter()
                .map(|relative| {
                    let file = path.join(managed_relative(relative)?);
                    Ok((relative.clone(), Namespace::resolve(&file)?, owned(&file)?))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut used = BTreeSet::new();
            let mut next_namespaces = vec![];
            for (relative, bytes) in &artifact.csharp {
                let next_path = path.join(managed_relative(relative)?);
                protected_object(saved, artifact, &next_path)?;
                let next_namespace = Namespace::resolve(&next_path)?;
                if next_namespace.relation(&reserved)? != Relation::Separate {
                    return Err(error("generated C# collides with reserved manifest"));
                }
                for other in &next_namespaces {
                    if next_namespace.relation(other)? != Relation::Separate {
                        return Err(error(
                            "generated C# paths alias in destination [PUBLISH-PATH-001]",
                        ));
                    }
                }
                next_namespaces.push(next_namespace.clone());
                let matched = previous
                    .iter()
                    .enumerate()
                    .find_map(|(i, (_, old, _))| match next_namespace.relation(old) {
                        Ok(Relation::Same) => Some(Ok(i)),
                        Ok(Relation::Separate) => None,
                        Ok(_) => Some(Err(error("generated C# overlaps a previous managed path"))),
                        Err(e) => Some(Err(e)),
                    })
                    .transpose()?;
                let existing = owned(&next_path)?;
                let (old_path, old_namespace, old) = if let Some(index) = matched {
                    used.insert(index);
                    let (relative, old_namespace, old) = &previous[index];
                    preview.updates.push(relative.clone());
                    (
                        path.join(managed_relative(relative)?),
                        old_namespace.clone(),
                        old.clone(),
                    )
                } else {
                    if existing.is_some() {
                        return Err(error(format!(
                            "{}: generated C# collides with unmanaged file [PUBLISH-006]",
                            next_path.display()
                        )));
                    }
                    preview.additions.push(relative.clone());
                    (next_path.clone(), next_namespace.clone(), None)
                };
                changes.push(Change {
                    previous_path: old_path,
                    next_path,
                    previous: old,
                    next: Some(bytes.clone()),
                    namespace: next_namespace,
                    previous_namespace: old_namespace,
                });
            }
            for (index, (relative, old_namespace, old)) in previous.into_iter().enumerate() {
                protected_object(saved, artifact, &path.join(managed_relative(&relative)?))?;
                if !used.contains(&index) {
                    preview.removals.push(relative.clone());
                    let old_path = path.join(managed_relative(&relative)?);
                    changes.push(Change {
                        previous_path: old_path.clone(),
                        next_path: old_path,
                        previous: old,
                        next: None,
                        namespace: old_namespace.clone(),
                        previous_namespace: old_namespace,
                    });
                }
            }
            let next_manifest = serde_json::to_vec_pretty(&Manifest {
                version: 1,
                files: artifact.csharp.keys().cloned().collect(),
            })
            .map_err(|e| error(e.to_string()))?;
            changes.push(Change {
                previous_path: path.join(MANIFEST),
                next_path: path.join(MANIFEST),
                previous: previous_manifest,
                next: Some(next_manifest),
                namespace: reserved.clone(),
                previous_namespace: reserved,
            });
        }
        Ok(Self {
            namespace,
            preview,
            changes,
        })
    }
    fn fresh(&self, created: &[Arc<Identity>]) -> Result<()> {
        self.namespace.fresh_with_created(created)?;
        for change in &self.changes {
            change.namespace.fresh_with_created(created)?;
            change.previous_namespace.fresh_with_created(created)?;
            if !same_file_state(
                change.previous.as_ref(),
                owned(&change.previous_path)?.as_ref(),
            ) {
                return Err(error("managed content/ownership changed after preview"));
            }
        }
        Ok(())
    }
}
impl PublishPlan {
    pub fn prepare(root: &Path) -> Result<Self> {
        let saved = SavedConfig::load(root)?;
        // Receipt validation precedes ANY external destination inspection.
        let artifact = ArtifactSet::load(&saved)?;
        let mut targets: Vec<Target> = vec![];
        for configured in &saved.config.publish.targets {
            let target = Target::prepare(&saved, &artifact, &configured.kind, &configured.path)?;
            for other in &targets {
                if target.namespace.relation(&other.namespace)? != Relation::Separate {
                    return Err(error(
                        "publish ownership regions overlap [PUBLISH-PATH-008]",
                    ));
                }
            }
            targets.push(target);
        }
        saved.fresh()?;
        artifact.fresh()?;
        for target in &targets {
            target.fresh(&[])?;
        }
        let preview = Preview {
            artifact_root: artifact.root.clone(),
            artifact_identity: content_identity(
                &serde_json::to_vec(&artifact.receipt).map_err(|e| error(e.to_string()))?,
            ),
            config_identity: saved.snapshot.content.clone(),
            project_id: saved.config.project.id.clone(),
            source_freshness: "not_checked",
            targets: targets
                .iter()
                .map(|target| target.preview.clone())
                .collect(),
        };
        Ok(Self {
            saved,
            artifact,
            targets,
            preview,
        })
    }
    pub fn preview(&self) -> &Preview {
        &self.preview
    }
    pub fn execute(&self, fault: PublishFault) -> Result<Report> {
        // Confirm keeps exactly the reviewed config/artifact/managed set. All
        // destinations are witnessed again before any parent creation or write.
        (|| {
            self.saved.fresh()?;
            self.artifact.fresh()?;
            for target in &self.targets {target.fresh(&[])?;}
            Ok(())
        })().map_err(|error:Error|Error::new("E-PUBLISH-STALE",format!("Publish preview is no longer authorized; request and confirm a new preview: {error}")))?;
        let mut created = vec![];
        let mut results = vec![];
        for (index, target) in self.targets.iter().enumerate() {
            results.push(target.execute(index, fault, &mut created, &self.saved, &self.artifact));
        }
        let outcome = if results
            .iter()
            .any(|result| result.outcome == Outcome::RecoveryRequired)
        {
            Outcome::RecoveryRequired
        } else if results
            .iter()
            .any(|result| result.outcome == Outcome::OutcomeUnknown)
        {
            Outcome::OutcomeUnknown
        } else if results
            .iter()
            .all(|result| result.outcome == Outcome::Success)
        {
            Outcome::Success
        } else {
            Outcome::Failure
        };
        Ok(Report {
            outcome,
            source_freshness: "not_checked",
            unity_verification: "not_observed",
            no_op: results.is_empty(),
            targets: results,
        })
    }
}
struct Applied {
    index: usize,
    backup: PathBuf,
    old_moved: bool,
    new_identity: Option<Arc<Identity>>,
}
impl Target {
    fn execute(
        &self,
        index: usize,
        fault: PublishFault,
        created: &mut Vec<Arc<Identity>>,
        saved: &SavedConfig,
        artifact: &ArtifactSet,
    ) -> TargetResult {
        let mut outcome = Outcome::Failure;
        let mut recovery = None;
        let operation = (|| -> Result<()> {
            self.fresh(created)?;
            #[cfg(feature = "oracle-faults")]
            if matches!(fault,PublishFault::BeforeTarget(i) if i==index) {
                return Err(error("injected target execution failure"));
            }
            let directory = if self.preview.kind == "csharp" {
                self.namespace.path.as_path()
            } else {
                self.namespace
                    .path
                    .parent()
                    .ok_or_else(|| error("binary parent required"))?
            };
            create_parents(directory, created)?;
            let stage = tempfile::Builder::new()
                .prefix(".masterdata-publish-")
                .tempdir_in(directory)
                .map_err(io_error)?;
            let stage_identity = Arc::new(path_identity(stage.path()).map_err(io_error)?);
            let mut applied = vec![];
            let committed = (|| -> Result<()> {
                for (number, change) in self.changes.iter().enumerate() {
                    protected_object(saved, artifact, &change.previous_path)?;
                    protected_object(saved, artifact, &change.next_path)?;
                    if change.previous.is_none() && change.next.is_none() {
                        continue;
                    }
                    change.namespace.fresh_with_created(created)?;
                    change.previous_namespace.fresh_with_created(created)?;
                    if !same_file_state(
                        change.previous.as_ref(),
                        owned(&change.previous_path)?.as_ref(),
                    ) {
                        return Err(error("managed file changed at mutation boundary"));
                    }
                    let parent = change
                        .next_path
                        .parent()
                        .ok_or_else(|| error("managed parent required"))?;
                    create_parents(parent, created)?;
                    let parent_identity = path_identity(parent).map_err(io_error)?;
                    let old_parent =
                        path_identity(change.previous_path.parent().unwrap()).map_err(io_error)?;
                    let backup = stage.path().join(format!("old-{number}"));
                    let candidate = stage.path().join(format!("new-{number}"));
                    if let Some(bytes) = &change.next {
                        let mut file = fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&candidate)
                            .map_err(io_error)?;
                        if let Some(old) = &change.previous {
                            file.set_permissions(old.permissions.clone())
                                .map_err(io_error)?;
                        }
                        file.write_all(bytes).map_err(io_error)?;
                        file.sync_all().map_err(io_error)?;
                    }
                    let mut state = Applied {
                        index: number,
                        backup,
                        old_moved: false,
                        new_identity: None,
                    };
                    if change.previous.is_some() {
                        rename_slots(
                            &change.previous_path,
                            &old_parent,
                            &state.backup,
                            &stage_identity,
                        )?;
                        state.old_moved = true;
                    }
                    applied.push(state);
                    if change.next.is_some() {
                        rename_slots(
                            &candidate,
                            &stage_identity,
                            &change.next_path,
                            &parent_identity,
                        )?;
                        applied.last_mut().unwrap().new_identity = Some(Arc::new(
                            path_identity(&change.next_path).map_err(io_error)?,
                        ));
                    }
                    #[cfg(feature = "oracle-faults")]
                    if matches!(fault,PublishFault::AfterFile{target,file,..} if target==index && file==number)
                    {
                        return Err(error("injected target-local commit failure"));
                    }
                }
                for state in &applied {
                    let change = &self.changes[state.index];
                    if let Some(bytes) = &change.next {
                        let actual = owned(&change.next_path)?
                            .ok_or_else(|| error("published file disappeared"))?;
                        if Some(actual.identity) != state.new_identity
                            || actual.hash != content_identity(bytes)
                        {
                            return Err(error("published identity could not be established"));
                        }
                    }
                }
                Ok(())
            })();
            if let Err(failure) = committed {
                let rollback = (|| -> Result<()> {
                    #[cfg(feature = "oracle-faults")]
                    if matches!(fault,PublishFault::AfterFile{target,rollback_failure:true,..} if target==index)
                    {
                        return Err(error("injected target-local rollback failure"));
                    }
                    for state in applied.iter().rev() {
                        let change = &self.changes[state.index];
                        if let Some(identity) = &state.new_identity {
                            let actual = owned(&change.next_path)?
                                .ok_or_else(|| error("owned NEW missing during rollback"))?;
                            if actual.identity != *identity
                                || actual.hash != content_identity(change.next.as_ref().unwrap())
                            {
                                return Err(error("NEW changed before rollback; no overwrite"));
                            }
                            let parent = path_identity(change.next_path.parent().unwrap())
                                .map_err(io_error)?;
                            rename_slots(
                                &change.next_path,
                                &parent,
                                &stage.path().join(format!("removed-{}", state.index)),
                                &stage_identity,
                            )?;
                        }
                        if state.old_moved {
                            if !same_file_state(
                                change.previous.as_ref(),
                                owned(&state.backup)?.as_ref(),
                            ) {
                                return Err(error("OLD changed before rollback"));
                            }
                            let parent = path_identity(change.previous_path.parent().unwrap())
                                .map_err(io_error)?;
                            rename_slots(
                                &state.backup,
                                &stage_identity,
                                &change.previous_path,
                                &parent,
                            )?;
                        }
                    }
                    Ok(())
                })();
                if let Err(rollback) = rollback {
                    outcome = Outcome::RecoveryRequired;
                    recovery = Some(stage.keep());
                    return Err(Error::new(
                        "E-PUBLISH-RECOVERY",
                        format!("{failure}; rollback failed: {rollback}"),
                    ));
                }
                return Err(failure);
            }
            #[cfg(feature = "oracle-faults")]
            if matches!(fault,PublishFault::UnknownAfterCommit(i) if i==index) {
                outcome = Outcome::OutcomeUnknown;
                recovery = Some(stage.keep());
                return Err(error(
                    "injected unknown outcome; recheck actual target; no automatic retry",
                ));
            }
            #[cfg(not(feature = "oracle-faults"))]
            let _ = (index, fault);
            outcome = Outcome::Success;
            Ok(())
        })();
        TargetResult {
            kind: self.preview.kind.clone(),
            destination: self.namespace.path.clone(),
            status: if outcome == Outcome::Success {
                "succeeded"
            } else {
                "failed"
            },
            outcome,
            message: match operation {
                Ok(()) => "published receipt-valid artifacts".into(),
                Err(e) => e.to_string(),
            },
            recovery_directory: recovery,
        }
    }
}
