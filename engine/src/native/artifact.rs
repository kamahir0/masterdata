//! Complete canonical artifact publication, separate from physical source writes.
//! Actual root/parent/content identity is checked again after native compilation.
use super::namespace::{Namespace, Relation, absolute};
use super::*;
use crate::{delivery::BuildPlan, project::Config};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::OpenOptions};

pub const RECEIPT: &str = ".masterdata-artifact-set.json";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileReceipt {
    pub path: String,
    pub hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub version: u32,
    pub project_id: String,
    pub hash_algorithm: String,
    pub csharp: Vec<FileReceipt>,
    pub binary: FileReceipt,
}
#[derive(Clone, Debug, Serialize)]
pub struct CommitResult {
    pub outcome: Outcome,
    pub message: String,
    pub retained_backup: Option<PathBuf>,
}
#[derive(Clone, Copy, Default)]
pub enum ArtifactFault {
    #[default]
    None,
    BeforeSwitch,
    AfterOldMoved,
    AfterNewMoved,
    RollbackFailure,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    identity: Arc<Identity>,
    hash: Option<String>,
}
type Tree = BTreeMap<PathBuf, Entry>;
pub struct Guard {
    root: PathBuf,
    target: PathBuf,
    config: Snapshot,
    prefix: Vec<(PathBuf, Arc<Identity>)>,
    previous: Option<Tree>,
}
pub fn receipt(project_id: &str, csharp: &BTreeMap<String, String>, binary: &[u8]) -> Receipt {
    Receipt {
        version: 1,
        project_id: project_id.into(),
        hash_algorithm: "sha256".into(),
        csharp: csharp
            .iter()
            .map(|(path, bytes)| FileReceipt {
                path: path.clone(),
                hash: content_identity(bytes.as_bytes()),
            })
            .collect(),
        binary: FileReceipt {
            path: "masterdata.bytes".into(),
            hash: content_identity(binary),
        },
    }
}
fn unsafe_path(message: impl Into<String>) -> Error {
    Error::new("E-ARTIFACT-PATH", message)
}
fn normalized_relative(path: &str) -> Result<PathBuf> {
    Ok(relative_safe(path)?
        .components()
        .filter(|part| !matches!(part, std::path::Component::CurDir))
        .collect())
}
fn walk(root: &Path, target: &Path) -> Result<Vec<(PathBuf, Arc<Identity>)>> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| unsafe_path("canonical root escaped Project"))?;
    let mut current = root.to_path_buf();
    let mut prefix = vec![];
    for part in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(part) = part {
            current.push(part);
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if path_alias(&metadata) || !metadata.is_dir() {
                    return Err(unsafe_path(format!(
                        "{}: actual directory without aliases required",
                        current.display()
                    )));
                }
                prefix.push((
                    current.clone(),
                    Arc::new(path_identity(&current).map_err(io_error)?),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(prefix)
}
fn protected(root: &Path, config: &Config, target: &Path) -> Result<()> {
    let namespace = Namespace::resolve(target)?;
    if target == root
        || namespace.relation(&Namespace::resolve(&root.join("masterdata.toml"))?)?
            != Relation::Separate
    {
        return Err(unsafe_path("canonical root overlaps Project or config"));
    }
    for path in config
        .sources
        .roots
        .iter()
        .chain(std::iter::once(&config.build.cache))
    {
        let configured = absolute(root, Path::new(path))?;
        if namespace.relation(&Namespace::resolve(&configured)?)? != Relation::Separate {
            return Err(unsafe_path(format!(
                "canonical root overlaps protected path {}",
                configured.display()
            )));
        }
    }
    Ok(())
}
fn tree(path: &Path) -> Result<Tree> {
    let mut result = BTreeMap::new();
    fn visit(root: &Path, path: &Path, result: &mut Tree) -> Result<()> {
        let metadata = fs::symlink_metadata(path).map_err(io_error)?;
        if path_alias(&metadata) || !(metadata.is_file() || metadata.is_dir()) {
            return Err(unsafe_path(format!(
                "{}: unexpected artifact type",
                path.display()
            )));
        }
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        result.insert(
            relative,
            Entry {
                identity: Arc::new(path_identity(path).map_err(io_error)?),
                hash: if metadata.is_file() {
                    Some(content_identity(&fs::read(path).map_err(io_error)?))
                } else {
                    None
                },
            },
        );
        if metadata.is_dir() {
            for entry in fs::read_dir(path).map_err(io_error)? {
                visit(root, &entry.map_err(io_error)?.path(), result)?;
            }
        }
        Ok(())
    }
    visit(path, path, &mut result)?;
    let expected = [
        Path::new("csharp"),
        Path::new("masterdata.bytes"),
        Path::new(RECEIPT),
    ];
    for (relative, entry) in &result {
        if relative.as_os_str().is_empty() {
            continue;
        }
        if relative.starts_with("csharp") {
            if relative == Path::new("csharp") && entry.hash.is_some()
                || entry.hash.is_some()
                    && relative
                        .extension()
                        .is_none_or(|extension| extension != "cs")
            {
                return Err(unsafe_path("unexpected canonical C# entry"));
            }
        } else if !expected.contains(&relative.as_path()) || entry.hash.is_none() {
            return Err(unsafe_path("unexpected canonical root entry"));
        }
    }
    Ok(result)
}
fn present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(error)),
    }
}
impl Guard {
    pub fn prepare(plan: &BuildPlan) -> Result<Self> {
        let root = plan.project().root.clone();
        let target = root.join(normalized_relative(
            &plan.project().config.build.artifact_dir,
        )?);
        if target == root {
            return Err(unsafe_path("canonical root may not replace Project"));
        }
        protected(&root, &plan.project().config, &target)?;
        let prefix = walk(&root, &target)?;
        let config = capture(&root, std::slice::from_ref(&root), "masterdata.toml")?;
        if config.content != plan.project().config_identity {
            return Err(Error::new(
                "E-BUILD-STALE",
                "config changed during Build capture",
            ));
        }
        let previous = if present(&target)? {
            Some(tree(&target)?)
        } else {
            None
        };
        Ok(Self {
            root,
            target,
            config,
            prefix,
            previous,
        })
    }
    pub fn target(&self) -> &Path {
        &self.target
    }
    fn fresh(&self) -> Result<()> {
        preflight(
            &self.root,
            std::slice::from_ref(&self.root),
            "masterdata.toml",
            &self.config,
        )?;
        let config = crate::project::config(&self.config.bytes)?;
        protected(&self.root, &config, &self.target)?;
        let current = walk(&self.root, &self.target)?;
        for (path, identity) in &self.prefix {
            if !current
                .iter()
                .any(|(candidate, now)| candidate == path && now == identity)
            {
                return Err(Error::new(
                    "E-BUILD-STALE",
                    "Project or output parent identity changed",
                ));
            }
        }
        let actual = if present(&self.target)? {
            Some(tree(&self.target)?)
        } else {
            None
        };
        if actual != self.previous {
            return Err(Error::new(
                "E-BUILD-CONFLICT",
                "canonical artifact root changed during Build",
            ));
        }
        Ok(())
    }
    pub fn commit(
        &self,
        csharp: &BTreeMap<String, String>,
        binary: &[u8],
        receipt: &Receipt,
        fault: ArtifactFault,
    ) -> Result<CommitResult> {
        self.fresh()?;
        #[cfg(feature = "oracle-faults")]
        if matches!(fault, ArtifactFault::BeforeSwitch) {
            return Ok(CommitResult {
                outcome: Outcome::Failure,
                message: "injected staging failure; previous set retained".into(),
                retained_backup: None,
            });
        }
        #[cfg(not(feature = "oracle-faults"))]
        let _ = fault;
        let parent = self
            .target
            .parent()
            .ok_or_else(|| unsafe_path("output parent required"))?;
        // Only a successful compiled/reloaded candidate reaches parent creation.
        fs::create_dir_all(parent).map_err(io_error)?;
        let prefix = walk(&self.root, parent)?;
        let parent_identity = prefix.last().unwrap().1.clone();
        let stage = tempfile::Builder::new()
            .prefix(".masterdata-build-")
            .tempdir_in(parent)
            .map_err(io_error)?;
        fs::create_dir(stage.path().join("csharp")).map_err(io_error)?;
        for (relative, bytes) in csharp {
            let relative = normalized_relative(relative)?;
            let target = stage.path().join("csharp").join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(io_error)?;
            file.write_all(bytes.as_bytes()).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
        }
        fs::write(stage.path().join("masterdata.bytes"), binary).map_err(io_error)?;
        fs::write(
            stage.path().join(RECEIPT),
            serde_json::to_vec_pretty(receipt)
                .map_err(|error| Error::new("E-ARTIFACT-RECEIPT", error.to_string()))?,
        )
        .map_err(io_error)?;
        for file in ["masterdata.bytes", RECEIPT] {
            File::open(stage.path().join(file))
                .map_err(io_error)?
                .sync_all()
                .map_err(io_error)?;
        }
        let new_tree = tree(stage.path())?;
        self.fresh()?;
        if path_identity(parent)
            .map_err(io_error)?
            .ne(parent_identity.as_ref())
        {
            return Err(Error::new(
                "E-BUILD-STALE",
                "output parent changed during staging",
            ));
        }
        let backup = parent.join(format!(
            "{}.previous",
            stage.path().file_name().unwrap().to_string_lossy()
        ));
        let mut old_moved = false;
        if self.previous.is_some() {
            rename_exclusive(&self.target, &backup, &parent_identity)?;
            old_moved = true;
        }
        #[cfg(feature = "oracle-faults")]
        let failed = matches!(
            fault,
            ArtifactFault::AfterOldMoved | ArtifactFault::RollbackFailure
        );
        #[cfg(not(feature = "oracle-faults"))]
        let failed = false;
        let switched = if failed {
            Err(Error::new(
                "E-ARTIFACT-FAULT",
                "injected complete-set switch failure",
            ))
        } else {
            rename_exclusive(stage.path(), &self.target, &parent_identity)
        };
        let published = switched.is_ok();
        #[cfg(feature = "oracle-faults")]
        let switched = if matches!(fault, ArtifactFault::AfterNewMoved) {
            Err(Error::new(
                "E-ARTIFACT-FAULT",
                "injected post-switch failure",
            ))
        } else {
            switched
        };
        if let Err(error) = switched {
            let restored = (|| -> Result<()> {
                #[cfg(feature = "oracle-faults")]
                if matches!(fault, ArtifactFault::RollbackFailure) {
                    return Err(Error::new(
                        "E-ARTIFACT-RECOVERY",
                        "injected rollback failure",
                    ));
                }
                if published {
                    if tree(&self.target)? != new_tree {
                        return Err(Error::new(
                            "E-ARTIFACT-RECOVERY",
                            "published set changed before rollback",
                        ));
                    }
                    rename_exclusive(&self.target, stage.path(), &parent_identity)?;
                }
                if old_moved {
                    if Some(tree(&backup)?) != self.previous {
                        return Err(Error::new(
                            "E-ARTIFACT-RECOVERY",
                            "previous set changed before rollback",
                        ));
                    }
                    rename_exclusive(&backup, &self.target, &parent_identity)?;
                }
                Ok(())
            })();
            return Ok(CommitResult {
                outcome: if restored.is_ok() {
                    Outcome::Failure
                } else {
                    Outcome::RecoveryRequired
                },
                message: match restored {
                    Ok(()) => format!("{error}; previous complete set retained"),
                    Err(restoration) => format!(
                        "{error}; artifact rollback could not be established: {restoration}"
                    ),
                },
                retained_backup: if old_moved && backup.exists() {
                    Some(backup)
                } else {
                    None
                },
            });
        }
        if preflight(
            &self.root,
            std::slice::from_ref(&self.root),
            "masterdata.toml",
            &self.config,
        )
        .is_err()
            || !path_identity(parent).is_ok_and(|identity| identity == *parent_identity)
            || !tree(&self.target).is_ok_and(|current| current == new_tree)
        {
            return Ok(CommitResult{outcome:Outcome::OutcomeUnknown,message:"set switch completed but current canonical path identity could not be established; do not retry automatically".into(),retained_backup:if old_moved{Some(backup)}else{None}});
        }
        let retained_backup = if old_moved {
            if tree(&backup).is_ok_and(|actual| Some(actual) == self.previous)
                && fs::remove_dir_all(&backup).is_ok()
            {
                None
            } else {
                Some(backup)
            }
        } else {
            None
        };
        Ok(CommitResult {
            outcome: Outcome::Success,
            message: "complete C# / binary / receipt published".into(),
            retained_backup,
        })
    }
}
fn rename_exclusive(from: &Path, to: &Path, parent: &Identity) -> Result<()> {
    super::rename_slots(from, parent, to, parent)
}

pub struct ArtifactSet {
    pub root: PathBuf,
    pub receipt: Receipt,
    pub csharp: BTreeMap<String, Vec<u8>>,
    pub binary: Vec<u8>,
    identity: Tree,
}
fn receipt_error(path: &Path, reason: impl std::fmt::Display) -> Error {
    Error::new(
        "E-ARTIFACT-RECEIPT",
        format!(
            "{}: {reason} [ARTIFACT-SET-004 / ARTIFACT-SET-006]; run a complete Build",
            path.display()
        ),
    )
}
pub(super) fn managed_relative(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.contains(['\\', ':'])
        || value
            .split('/')
            .any(|p| p.is_empty() || matches!(p, "." | "..") || p.ends_with(['.', ' ']))
    {
        return Err(unsafe_path("invalid managed relative path"));
    }
    normalized_relative(value)
}
fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl ArtifactSet {
    pub fn load(saved: &crate::delivery::SavedConfig) -> Result<Self> {
        let root = saved
            .root
            .join(normalized_relative(&saved.config.build.artifact_dir)?);
        let receipt_path = root.join(RECEIPT);
        let checked = (|| -> Result<Self> {
            protected(&saved.root, &saved.config, &root)?;
            walk(&saved.root, &root)?;
            let identity = tree(&root)?;
            let entries = fs::read_dir(&root)
                .map_err(io_error)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<std::io::Result<std::collections::BTreeSet<_>>>()
                .map_err(io_error)?;
            if entries
                != std::collections::BTreeSet::from([
                    "csharp".into(),
                    "masterdata.bytes".into(),
                    RECEIPT.into(),
                ])
            {
                return Err(unsafe_path(
                    "canonical root must contain exactly C#, binary and receipt",
                ));
            }
            let receipt: Receipt =
                serde_json::from_slice(&fs::read(&receipt_path).map_err(io_error)?)
                    .map_err(|e| unsafe_path(format!("invalid receipt shape: {e}")))?;
            if receipt.version != 1 || receipt.hash_algorithm != "sha256" {
                return Err(unsafe_path("unsupported receipt version or hash algorithm"));
            }
            if receipt.project_id != saved.config.project.id {
                return Err(unsafe_path("receipt project id mismatch"));
            }
            if receipt.binary.path != "masterdata.bytes" || !valid_hash(&receipt.binary.hash) {
                return Err(unsafe_path("invalid fixed binary path or hash"));
            }
            let mut csharp = BTreeMap::new();
            let mut paths = vec![];
            let mut objects = vec![];
            let mut previous: Option<&str> = None;
            for file in &receipt.csharp {
                let relative = managed_relative(&file.path)?;
                if relative
                    .extension()
                    .is_none_or(|extension| extension != "cs")
                    || !valid_hash(&file.hash)
                {
                    return Err(unsafe_path("invalid C# path or hash"));
                }
                if previous.is_some_and(|previous| previous >= file.path.as_str()) {
                    return Err(unsafe_path(
                        "C# paths must be unique and deterministically ordered",
                    ));
                }
                previous = Some(&file.path);
                let key = Path::new("csharp").join(&relative);
                let entry = identity
                    .get(&key)
                    .ok_or_else(|| unsafe_path(format!("missing C# file {}", file.path)))?;
                if entry.hash.as_ref() != Some(&file.hash) {
                    return Err(unsafe_path(format!("C# hash/type mismatch {}", file.path)));
                }
                if objects.contains(&entry.identity) {
                    return Err(unsafe_path("duplicate filesystem object in C# receipt"));
                }
                let namespace = Namespace::resolve(&root.join(&key))?;
                for other in &paths {
                    if namespace.relation(other)? != Relation::Separate {
                        return Err(unsafe_path("ambiguous C# receipt namespace"));
                    }
                }
                paths.push(namespace);
                objects.push(entry.identity.clone());
                let bytes = fs::read(root.join(&key)).map_err(io_error)?;
                if content_identity(&bytes) != file.hash {
                    return Err(unsafe_path("C# changed during receipt validation"));
                }
                csharp.insert(file.path.clone(), bytes);
            }
            let actual = identity
                .iter()
                .filter_map(|(relative, entry)| {
                    if relative.starts_with("csharp") && entry.hash.is_some() {
                        Some(
                            relative
                                .strip_prefix("csharp")
                                .unwrap()
                                .components()
                                .map(|component| component.as_os_str().to_str())
                                .collect::<Option<Vec<_>>>()
                                .map(|parts| parts.join("/")),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Option<std::collections::BTreeSet<_>>>()
                .ok_or_else(|| unsafe_path("non-UTF8 canonical C# path"))?;
            if actual != csharp.keys().cloned().collect() {
                return Err(unsafe_path(
                    "receipt does not describe complete C# file set",
                ));
            }
            let binary = fs::read(root.join("masterdata.bytes")).map_err(io_error)?;
            if content_identity(&binary) != receipt.binary.hash || tree(&root)? != identity {
                return Err(unsafe_path(
                    "canonical set changed during receipt validation",
                ));
            }
            saved.fresh()?;
            Ok(Self {
                root,
                receipt,
                csharp,
                binary,
                identity,
            })
        })();
        checked.map_err(|e| receipt_error(&receipt_path, e))
    }
    pub(super) fn fresh(&self) -> Result<()> {
        if tree(&self.root)? != self.identity {
            return Err(receipt_error(
                &self.root,
                "canonical set changed after preview",
            ));
        }
        Ok(())
    }
}
