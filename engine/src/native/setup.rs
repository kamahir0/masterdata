//! New project scaffold, shared by CLI and Desktop. Existing configuration and
//! .gitignore are never rewritten; tool-state directories are created lazily.
use super::{namespace::Namespace, namespace::create_parents, *};
use crate::project::{BuildConfig, Config, Metadata, PublishConfig, Sources};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReport {
    pub outcome: Outcome,
    pub root: PathBuf,
    pub kept_gitignore: bool,
}
struct Initialized {
    report: InitReport,
    root_identity: Arc<Identity>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCreation {
    pub outcome: Outcome,
    pub root: PathBuf,
    pub remaining: Vec<String>,
    pub unconfirmed: Vec<String>,
    pub message: String,
}
#[derive(Clone, Copy, Default)]
pub enum InitFault {
    #[default]
    None,
    BeforeConfig,
    AfterConfig,
    ConfigRace,
}

/// GUI creation is stricter than CLI init: only an empty existing directory or
/// one missing directory under an actual parent. Validate the original traversal
/// before normalization; canonicalizing first would erase symlink evidence.
pub fn create_project(path: &Path, metadata: Metadata) -> Result<ProjectCreation> {
    create_project_with_fault(path, metadata, InitFault::None)
}
pub fn create_project_with_fault(
    path: &Path,
    metadata: Metadata,
    fault: InitFault,
) -> Result<ProjectCreation> {
    if !matches!(fault, InitFault::None) && !cfg!(feature = "oracle-faults") {
        return Err(Error::new(
            "E-FAULT-DISABLED",
            "init fault adapter is disabled",
        ));
    }
    let cwd = std::env::current_dir().map_err(io_error)?;
    let original = if path.is_absolute() {
        path.to_owned()
    } else {
        cwd.join(path)
    };
    let mut prefix = PathBuf::new();
    for component in original.components() {
        prefix.push(component);
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&prefix) {
            Ok(m) if path_alias(&m) => {
                return Err(Error::new(
                    "E-INIT-PATH",
                    "symlink/reparse traversal is not a creation destination",
                ));
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(io_error(e)),
        }
    }
    let root = namespace::absolute(&cwd, path)?;
    let proof = Namespace::resolve(&root)?;
    match fs::symlink_metadata(&root) {
        Ok(m) if m.is_dir() && !path_alias(&m) => {
            if fs::read_dir(&root).map_err(io_error)?.next().is_some() {
                return Err(Error::new(
                    "E-INIT-NONEMPTY",
                    "destination must be an empty directory",
                ));
            }
        }
        Ok(_) => return Err(Error::new("E-INIT-PATH", "directory destination required")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = root
                .parent()
                .ok_or_else(|| Error::new("E-INIT-PATH", "existing parent required"))?;
            let m = fs::symlink_metadata(parent).map_err(io_error)?;
            if !m.is_dir() || path_alias(&m) {
                return Err(Error::new(
                    "E-INIT-PATH",
                    "existing directory parent required",
                ));
            }
        }
        Err(e) => return Err(io_error(e)),
    }
    // Metadata failure is a precondition, never a partially created Project.
    let expected_config = configuration(metadata.clone())?;
    proof.fresh_with_created(&[])?;
    let result =
        initialize_at_root(root.clone(), metadata, fault, Some(&proof)).and_then(|initialized| {
            let project = crate::project::Project::open(&root)?;
            if path_identity(&root)
                .map_err(io_error)?
                .ne(initialized.root_identity.as_ref())
                || project.config_bytes.as_ref() != expected_config
            {
                return Err(Error::new(
                    "E-INIT-CONFLICT",
                    "created Project binding/config changed during resolution",
                ));
            }
            Ok(project)
        });
    let mut remaining = vec![];
    let mut unconfirmed = vec![];
    for entry in [
        "",
        "sources",
        "sources/schemas",
        "sources/types",
        "sources/data",
        "masterdata.toml",
        ".gitignore",
    ] {
        let path = root.join(entry);
        match Namespace::resolve(&path).and_then(|_| missing(&path)) {
            Ok(false) => remaining.push(if entry.is_empty() {
                ".".into()
            } else {
                entry.into()
            }),
            Ok(true) => (),
            Err(_) => unconfirmed.push(if entry.is_empty() {
                ".".into()
            } else {
                entry.into()
            }),
        }
    }
    let (outcome, message) = match result {
        Ok(_) => (
            Outcome::Success,
            "Project scaffold created and resolved".into(),
        ),
        Err(e) => (
            if e.code == "E-INIT-UNKNOWN" || e.message.contains("E-INIT-UNKNOWN") {
                Outcome::OutcomeUnknown
            } else {
                Outcome::Failure
            },
            e.to_string(),
        ),
    };
    // These are observed leftovers, not inferred ownership. A concurrent entry
    // may be present; no failed creation cleans up or retries the target.
    Ok(ProjectCreation {
        outcome,
        root,
        remaining,
        unconfirmed,
        message,
    })
}

fn root_path(path: &Path) -> Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(io_error)?;
    let path = namespace::absolute(&cwd, path)?;
    // Resolve the caller-selected existing root/parent once. Below that root,
    // namespace witnesses reject replaced or aliased scaffold descendants.
    let mut ancestor = path.as_path();
    let mut tail = vec![];
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tail.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| Error::new("E-INIT-PATH", "directory parent required"))?
                        .to_owned(),
                );
                ancestor = ancestor.parent().unwrap();
            }
            Err(e) => return Err(io_error(e)),
        }
    }
    let mut root = ancestor.canonicalize().map_err(io_error)?;
    if !root.is_dir() {
        return Err(Error::new(
            "E-INIT-PATH",
            "Project root must be a directory",
        ));
    }
    for component in tail.into_iter().rev() {
        root.push(component);
    }
    Ok(root)
}
fn missing(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(io_error(e)),
    }
}
fn create_file(proof: &Namespace, bytes: &[u8], created: &[Arc<Identity>]) -> Result<()> {
    let parent = proof.path.parent().unwrap();
    proof.fresh_with_created(created)?;
    let parent_identity = path_identity(parent).map_err(io_error)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    staged.write_all(bytes).map_err(io_error)?;
    staged.as_file().sync_all().map_err(io_error)?;
    let candidate_identity = file_identity(staged.as_file()).map_err(io_error)?;
    proof.fresh_with_created(created)?;
    if path_identity(parent).map_err(io_error)? != parent_identity {
        return Err(Error::new("E-INIT-CONFLICT", "Project parent changed"));
    }
    staged
        .persist_noclobber(&proof.path)
        .map_err(|e| Error::new("E-INIT-WRITE", e.to_string()))?;
    let observed = (|| -> Result<bool> {
        Ok(path_identity(parent).map_err(io_error)? == parent_identity
            && path_identity(&proof.path).map_err(io_error)? == candidate_identity
            && fs::read(&proof.path).map_err(io_error)? == bytes)
    })();
    if !matches!(observed, Ok(true)) {
        return Err(Error::new(
            "E-INIT-UNKNOWN",
            format!(
                "{}: creation could not be confirmed; inspect before retry: {:?}",
                proof.path.display(),
                observed
            ),
        ));
    }
    Ok(())
}

pub fn initialize_project(path: &Path, metadata: Metadata) -> Result<InitReport> {
    let root = root_path(path)?;
    initialize_at_root(root, metadata, InitFault::None, None).map(|created| created.report)
}
fn configuration(metadata: Metadata) -> Result<String> {
    let config = Config {
        project: metadata,
        sources: Sources {
            roots: vec!["sources".into()],
        },
        build: BuildConfig {
            artifact_dir: ".masterdata/output".into(),
            cache: ".masterdata/cache".into(),
            profiles: Default::default(),
        },
        publish: PublishConfig::default(),
    };
    let bytes =
        toml::to_string_pretty(&config).map_err(|e| Error::new("E-CONFIG", e.to_string()))?;
    crate::project::config(&bytes)?;
    Ok(bytes)
}
fn initialize_at_root(
    root: PathBuf,
    metadata: Metadata,
    fault: InitFault,
    gui_root: Option<&Namespace>,
) -> Result<Initialized> {
    let bytes = configuration(metadata)?;
    let marker = root.join("masterdata.toml");
    if !missing(&marker)? {
        return Err(Error::new(
            "E-INIT-EXISTS",
            "masterdata.toml already exists",
        ));
    }
    let directories = [
        "",
        "sources",
        "sources/schemas",
        "sources/types",
        "sources/data",
    ]
    .map(|relative| root.join(relative));
    let proofs = directories
        .iter()
        .map(|path| {
            if !missing(path)? && !fs::symlink_metadata(path).map_err(io_error)?.is_dir() {
                return Err(Error::new(
                    "E-INIT-PATH",
                    format!("{}: directory required", path.display()),
                ));
            }
            Namespace::resolve(path)
        })
        .collect::<Result<Vec<_>>>()?;
    let marker_proof = Namespace::resolve(&marker)?;
    let gitignore = root.join(".gitignore");
    let kept_gitignore = !missing(&gitignore)?;
    let gitignore_proof = if kept_gitignore {
        None
    } else {
        Some(Namespace::resolve(&gitignore)?)
    };
    let mut created = vec![];
    for proof in &proofs {
        if let Some(root) = gui_root {
            root.fresh_with_created(&created)?;
        }
        proof.fresh_with_created(&created)?;
        marker_proof.fresh_with_created(&created)?;
        create_parents(&proof.path, &mut created)?;
    }
    if matches!(fault, InitFault::BeforeConfig) {
        return Err(Error::new(
            "E-INIT-FAULT",
            "source directories were created; config was not attempted",
        ));
    }
    if matches!(fault, InitFault::ConfigRace) {
        fs::write(&marker, b"concurrently created config\n").map_err(io_error)?;
    }
    if let Some(root) = gui_root {
        root.fresh_with_created(&created)?;
    }
    create_file(&marker_proof, bytes.as_bytes(), &created)?;
    if matches!(fault, InitFault::AfterConfig) {
        return Err(Error::new(
            "E-INIT-FAULT",
            "config and source directories were created; .gitignore was not attempted",
        ));
    }
    if let Some(proof) = gitignore_proof
        && let Err(e) = create_file(&proof, b"/.masterdata/\n", &created)
    {
        return Err(Error::new(
            "E-INIT-PARTIAL",
            format!(
                "masterdata.toml and source directories were created; .gitignore was not confirmed: {e}"
            ),
        ));
    }
    let root_identity = Arc::new(path_identity(&root).map_err(io_error)?);
    if let Some(root) = gui_root {
        root.fresh_with_created(&created)?;
    }
    Ok(Initialized {
        root_identity,
        report: InitReport {
            outcome: Outcome::Success,
            root,
            kept_gitignore,
        },
    })
}
