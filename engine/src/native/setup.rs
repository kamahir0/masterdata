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
        proof.fresh_with_created(&created)?;
        marker_proof.fresh_with_created(&created)?;
        create_parents(&proof.path, &mut created)?;
    }
    create_file(&marker_proof, bytes.as_bytes(), &created)?;
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
    Ok(InitReport {
        outcome: Outcome::Success,
        root,
        kept_gitignore,
    })
}
