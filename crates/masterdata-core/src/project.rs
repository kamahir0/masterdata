use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{ProjectConfig, PublishTargetKind};
use crate::document::{ProjectDocuments, parse_yaml_document};
use crate::error::{ErrorKind, MasterdataError, Result, io_error};
use crate::validation::{ValidationReport, validate_documents};

pub const PROJECT_CONFIG_FILENAME: &str = "masterdata.toml";

#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
    config_path: PathBuf,
    config: ProjectConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InitOptions {
    pub project_id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProjectInfo {
    pub project_root: PathBuf,
    pub config_path: PathBuf,
    pub project_id: String,
    pub name: String,
    pub version: String,
    pub source_roots: Vec<PathBuf>,
    pub artifact_root: PathBuf,
    pub csharp_output: PathBuf,
    pub binary_output: PathBuf,
    pub cache: PathBuf,
    pub publish_targets: Vec<PublishTargetInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PublishTargetInfo {
    pub kind: PublishTargetKind,
    pub path: String,
    pub resolved_path: PathBuf,
}

impl Project {
    /// Resolve an explicit project path, or search for `masterdata.toml` from
    /// `start` toward the filesystem root.
    pub fn discover(explicit_project: Option<&Path>, start: &Path) -> Result<Self> {
        let start = absolute_path(
            start,
            &std::env::current_dir().map_err(|error| io_error(Path::new("."), error))?,
        );
        let config_path = if let Some(explicit_project) = explicit_project {
            let explicit_project = absolute_path(explicit_project, &start);
            let candidate = if explicit_project.is_file()
                || explicit_project.file_name().and_then(|name| name.to_str())
                    == Some(PROJECT_CONFIG_FILENAME)
            {
                explicit_project
            } else {
                explicit_project.join(PROJECT_CONFIG_FILENAME)
            };
            if !candidate.is_file() {
                return Err(MasterdataError::new(
                    "E-PROJECT-NOT-FOUND",
                    ErrorKind::ProjectNotFound,
                    format!(
                        "could not find {PROJECT_CONFIG_FILENAME} at {}",
                        candidate.display()
                    ),
                )
                .with_source(candidate)
                .with_related_requirement("PROJECT-001")
                .with_related_requirement("PROJECT-002"));
            }
            candidate
        } else {
            find_config_upwards(&start)?.ok_or_else(|| {
                MasterdataError::new(
                    "E-PROJECT-NOT-FOUND",
                    ErrorKind::ProjectNotFound,
                    format!(
                        "could not find {PROJECT_CONFIG_FILENAME} from {} or its parents",
                        start.display()
                    ),
                )
                .with_related_requirement("PROJECT-001")
                .with_related_requirement("PROJECT-003")
                .with_related_requirement("PROJECT-004")
            })?
        };

        Self::from_config_path(config_path)
    }

    pub fn from_config_path(config_path: PathBuf) -> Result<Self> {
        let config_path = absolute_path(
            &config_path,
            &std::env::current_dir().map_err(|error| io_error(Path::new("."), error))?,
        );
        let content =
            fs::read_to_string(&config_path).map_err(|error| io_error(&config_path, error))?;
        let raw: toml::Value = toml::from_str(&content).map_err(|error| {
            MasterdataError::new(
                "E-PROJECT-CONFIG-PARSE",
                ErrorKind::Config,
                format!("could not parse TOML: {error}"),
            )
            .with_source(config_path.clone())
        })?;
        reject_legacy_build_paths(&raw, &config_path)?;
        let config: ProjectConfig = raw.try_into().map_err(|error| {
            MasterdataError::new(
                "E-PROJECT-CONFIG-PARSE",
                ErrorKind::Config,
                format!("could not parse project configuration: {error}"),
            )
            .with_source(config_path.clone())
        })?;
        config.validate().map_err(|error| {
            let diagnostic = error.diagnostic().clone();
            MasterdataError {
                diagnostic: Box::new(diagnostic.with_source(config_path.clone())),
            }
        })?;
        let root = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        validate_project_paths(&root, &config)?;
        Ok(Self {
            root,
            config_path,
            config,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    pub fn info(&self) -> ProjectInfo {
        ProjectInfo {
            project_root: self.root.clone(),
            config_path: self.config_path.clone(),
            project_id: self.config.project.id.clone(),
            name: self.config.project.name.clone(),
            version: self.config.project.version.clone(),
            source_roots: self
                .config
                .sources
                .roots
                .iter()
                .map(|root| resolve_project_path(&self.root, root))
                .collect(),
            artifact_root: self.artifact_root_path(),
            csharp_output: self.csharp_output_path(),
            binary_output: self.binary_output_path(),
            cache: self.cache_path(),
            publish_targets: self
                .config
                .publish
                .targets
                .iter()
                .map(|target| PublishTargetInfo {
                    kind: target.kind,
                    path: target.path.clone(),
                    resolved_path: resolve_project_path(&self.root, &target.path),
                })
                .collect(),
        }
    }

    pub fn source_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for root in &self.config.sources.roots {
            let root_path = resolve_project_path(&self.root, root);
            if !root_path.exists() {
                return Err(MasterdataError::new(
                    "E-PROJECT-SOURCE-ROOT-MISSING",
                    ErrorKind::Config,
                    format!(
                        "configured source root does not exist: {}",
                        root_path.display()
                    ),
                )
                .with_source(root_path));
            }
            collect_yaml_files(&root_path, &mut files)?;
        }
        files.sort();
        files.dedup();
        Ok(files)
    }

    pub fn load_documents(&self) -> Result<ProjectDocuments> {
        let mut documents = ProjectDocuments::default();
        for path in self.source_files()? {
            let content = fs::read_to_string(&path).map_err(|error| io_error(&path, error))?;
            documents.files.push(parse_yaml_document(path, &content)?);
        }
        Ok(documents)
    }

    pub fn validate(&self) -> Result<ValidationReport> {
        let documents = self.load_documents()?;
        Ok(validate_documents(&documents))
    }

    pub fn artifact_root_path(&self) -> PathBuf {
        resolve_project_path(&self.root, &self.config.build.artifact_dir)
    }

    pub fn csharp_output_path(&self) -> PathBuf {
        self.artifact_root_path().join("csharp")
    }

    pub fn binary_output_path(&self) -> PathBuf {
        self.artifact_root_path().join("masterdata.bytes")
    }

    pub fn cache_path(&self) -> PathBuf {
        resolve_project_path(&self.root, &self.config.build.cache)
    }
}

/// Create a new project marker and the default source root.
///
/// This operation is deliberately small: Unity project detection may provide
/// future hints, but `masterdata.toml` remains the identity boundary.
pub fn initialize_project(root: &Path, options: &InitOptions) -> Result<ProjectInfo> {
    let current_dir = std::env::current_dir().map_err(|error| io_error(Path::new("."), error))?;
    let root = absolute_path(root, &current_dir);
    fs::create_dir_all(&root).map_err(|error| io_error(&root, error))?;
    let config_path = root.join(PROJECT_CONFIG_FILENAME);
    if config_path.exists() {
        return Err(MasterdataError::new(
            "E-PROJECT-ALREADY-INITIALIZED",
            ErrorKind::Config,
            format!("project config already exists at {}", config_path.display()),
        )
        .with_source(config_path));
    }
    let config = ProjectConfig {
        project: crate::ProjectMetadata {
            id: options.project_id.clone(),
            name: options.name.clone(),
            version: options.version.clone(),
        },
        sources: Default::default(),
        build: Default::default(),
        publish: Default::default(),
    };
    config.validate()?;
    let content = toml::to_string_pretty(&config).map_err(|error| {
        MasterdataError::new(
            "E-PROJECT-CONFIG-SERIALIZE",
            ErrorKind::Config,
            format!("could not serialize project config: {error}"),
        )
    })?;
    fs::write(&config_path, content).map_err(|error| io_error(&config_path, error))?;
    let source_root = root.join("sources");
    fs::create_dir_all(&source_root).map_err(|error| io_error(&source_root, error))?;
    Project::from_config_path(config_path).map(|project| project.info())
}

fn find_config_upwards(start: &Path) -> Result<Option<PathBuf>> {
    let mut directory = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = directory.join(PROJECT_CONFIG_FILENAME);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
        if !directory.pop() {
            return Ok(None);
        }
    }
}

fn collect_yaml_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|error| io_error(path, error))?;
    if metadata.is_file() {
        if is_yaml(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(path).map_err(|error| io_error(path, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(path, error))?;
        let entry_path = entry.path();
        // Do not follow symlink entries. This is an internal safety guard
        // against directory cycles; the product-level symlink policy is
        // intentionally still an Open Question in the source-discovery docs.
        if entry
            .file_type()
            .map_err(|error| io_error(&entry_path, error))?
            .is_symlink()
        {
            continue;
        }
        collect_yaml_files(&entry_path, files)?;
    }
    Ok(())
}

fn is_yaml(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "yaml" | "yml"))
}

fn resolve_project_path(root: &Path, configured: &str) -> PathBuf {
    let configured = Path::new(configured);
    if configured.is_absolute() {
        normalize_path(configured)
    } else {
        normalize_path(&root.join(configured))
    }
}

fn absolute_path(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&base.join(path))
    }
}

fn reject_legacy_build_paths(raw: &toml::Value, config_path: &Path) -> Result<()> {
    let Some(build) = raw.get("build").and_then(toml::Value::as_table) else {
        return Ok(());
    };

    if build.contains_key("output") {
        return Err(legacy_build_path_error(
            config_path,
            "build.output",
            "use build.artifact_dir for project-local canonical artifacts, or add a kind = \"csharp\" publish target for an external destination",
            "E-CONFIG-LEGACY-BUILD-OUTPUT",
        ));
    }
    if build.contains_key("binary_output") {
        return Err(legacy_build_path_error(
            config_path,
            "build.binary_output",
            "the canonical binary is .masterdata/output/masterdata.bytes; use a kind = \"binary\" publish target for an external destination",
            "E-CONFIG-LEGACY-BINARY-OUTPUT",
        ));
    }
    Ok(())
}

fn legacy_build_path_error(
    config_path: &Path,
    field: &str,
    guidance: &str,
    code: &str,
) -> MasterdataError {
    MasterdataError::new(
        code,
        ErrorKind::Config,
        format!("{field} is no longer supported by the canonical artifact model"),
    )
    .with_source(config_path.to_path_buf())
    .with_suggestion(guidance)
    .with_related_requirement("PROJECT-CONFIG-004")
    .with_related_requirement("PROJECT-CONFIG-005")
}

fn validate_project_paths(root: &Path, config: &ProjectConfig) -> Result<()> {
    let artifact_root =
        resolve_project_local_path(root, &config.build.artifact_dir, "build.artifact_dir")?;
    validate_no_symlink_components(&artifact_root, root, "build.artifact_dir")?;
    if let Ok(metadata) = fs::symlink_metadata(&artifact_root) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
            return Err(unsafe_project_path_error(
                &artifact_root,
                "canonical artifact root must be a real directory when it already exists",
            ));
        }
    }

    for source in &config.sources.roots {
        let source_path = resolve_project_path(root, source);
        if paths_overlap(&artifact_root, &source_path) {
            return Err(unsafe_project_path_error(
                &artifact_root,
                format!(
                    "canonical artifact root overlaps configured source root `{}`",
                    source_path.display()
                ),
            ));
        }
    }

    let cache_path = resolve_project_path(root, &config.build.cache);
    if paths_overlap(&artifact_root, &cache_path) {
        return Err(unsafe_project_path_error(
            &artifact_root,
            format!(
                "canonical artifact root overlaps build cache `{}`",
                cache_path.display()
            ),
        ));
    }
    Ok(())
}

fn resolve_project_local_path(root: &Path, configured: &str, field: &str) -> Result<PathBuf> {
    let configured_path = Path::new(configured);
    if configured_path.is_absolute() {
        return Err(unsafe_project_path_error(
            configured_path,
            format!("{field} must be a project-local relative path"),
        ));
    }
    let resolved = resolve_project_path(root, configured);
    if resolved == root || !resolved.starts_with(root) {
        return Err(unsafe_project_path_error(
            &resolved,
            format!("{field} must remain inside the project root"),
        ));
    }
    Ok(resolved)
}

fn validate_no_symlink_components(path: &Path, root: &Path, field: &str) -> Result<()> {
    let relative = path.strip_prefix(root).map_err(|_| {
        unsafe_project_path_error(
            path,
            format!("{field} cannot be resolved from project root"),
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let PathComponent::Normal(name) = component else {
            continue;
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(unsafe_project_path_error(
                    &current,
                    format!("{field} must not traverse a symlink"),
                ));
            }
            Ok(metadata) if !metadata.is_dir() && current != path => {
                return Err(unsafe_project_path_error(
                    &current,
                    format!("{field} contains a non-directory path component"),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(io_error(&current, error)),
        }
    }
    Ok(())
}

fn unsafe_project_path_error(path: &Path, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new("E-PROJECT-PATH-UNSAFE", ErrorKind::Config, message)
        .with_source(path.to_path_buf())
        .with_related_requirement("PROJECT-PATH-001")
        .with_related_requirement("BUILD-ARTIFACT-001")
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    is_path_ancestor_or_equal(left, right) || is_path_ancestor_or_equal(right, left)
}

fn is_path_ancestor_or_equal(candidate: &Path, target: &Path) -> bool {
    candidate
        .components()
        .eq(target.components().take(candidate.components().count()))
        && candidate.components().count() <= target.components().count()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    let mut rooted = false;
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                rooted = true;
                normalized.push(std::path::MAIN_SEPARATOR.to_string());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if normalized.file_name().is_some() {
                    normalized.pop();
                } else if !rooted {
                    normalized.push("..");
                }
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

use std::path::Component as PathComponent;
