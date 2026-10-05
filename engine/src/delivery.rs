//! Saved-source Build capture. Authoring overlays and read caches are not Build
//! inputs; the Desktop job and CLI use the same immutable validated Plan.
use crate::native::{Outcome, artifact, dotnet};
use crate::{
    Error, Result, codegen,
    project::{Dataset, Project},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct SavedConfig {
    pub root: PathBuf,
    pub config: crate::project::Config,
    pub(crate) snapshot: crate::native::Snapshot,
}
impl SavedConfig {
    /// Publish loads configuration without inspecting source YAML or requiring
    /// a still-existing source tree. The receipt owns artifact eligibility.
    pub fn load(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(crate::project::io_error)?;
        let snapshot =
            crate::native::capture(&root, std::slice::from_ref(&root), "masterdata.toml")?;
        let config = crate::project::config(&snapshot.bytes)?;
        Ok(Self {
            root,
            config,
            snapshot,
        })
    }
    pub(crate) fn fresh(&self) -> Result<()> {
        crate::native::preflight(
            &self.root,
            std::slice::from_ref(&self.root),
            "masterdata.toml",
            &self.snapshot,
        )
        .map(|_| ())
    }
}

#[derive(Default)]
pub struct BuildOptions {
    pub profile: Option<String>,
    pub dry_run: bool,
    pub fault: artifact::ArtifactFault,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    pub outcome: Outcome,
    pub project_id: String,
    pub profile: Option<String>,
    pub dry_run: bool,
    pub artifact_root: PathBuf,
    pub row_counts: BTreeMap<String, usize>,
    pub receipt: artifact::Receipt,
    pub native_evidence: String,
    pub config_identity: String,
    pub input_identity: BTreeMap<String, String>,
    pub message: String,
    pub retained_backup: Option<PathBuf>,
}

pub fn build(root: &Path, options: &BuildOptions) -> Result<BuildReport> {
    let plan = BuildPlan::capture(root, options.profile.as_deref())?;
    // Capture actual output/config identity before compilation. A successful
    // native process cannot grant permission to replace another concurrent set.
    let guard = artifact::Guard::prepare(&plan)?;
    let built = dotnet::build(&plan)?;
    let receipt = artifact::receipt(&plan.project.config.project.id, &plan.csharp, &built.binary);
    let committed = if options.dry_run {
        artifact::CommitResult {
            outcome: Outcome::Success,
            message: "dry run compiled and reloaded the candidate; no artifacts were changed"
                .into(),
            retained_backup: None,
        }
    } else {
        guard.commit(&plan.csharp, &built.binary, &receipt, options.fault)?
    };
    Ok(BuildReport {
        outcome: committed.outcome,
        project_id: plan.project.config.project.id.clone(),
        profile: plan.profile.clone(),
        dry_run: options.dry_run,
        artifact_root: guard.target().to_path_buf(),
        row_counts: plan.row_counts(),
        receipt,
        native_evidence: built.native_evidence,
        config_identity: plan.project.config_identity.clone(),
        input_identity: plan
            .project
            .sources
            .iter()
            .map(|(path, source)| (path.clone(), source.identity.clone()))
            .collect(),
        message: committed.message,
        retained_backup: committed.retained_backup,
    })
}

pub struct BuildPlan {
    project: Project,
    profile: Option<String>,
    rows: Dataset,
    csharp: BTreeMap<String, String>,
}
impl BuildPlan {
    pub fn capture(root: &Path, profile: Option<&str>) -> Result<Self> {
        let project = Project::open(root)?;
        if crate::native::has_pending_recovery(&project.root)? {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source-set recovery must be established before Build",
            ));
        }
        crate::native::confirm_saved_input(&project)?;
        let (diagnostics, rows) = project.validate(profile)?;
        if let Some(error) = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "error")
        {
            return Err(Error::new(
                "E-BUILD-INVALID",
                format!(
                    "{}:{}:{} {}: {}",
                    error.source, error.line, error.column, error.code, error.message
                ),
            ));
        }
        let csharp = codegen::generate(&project)?;
        Ok(Self {
            project,
            profile: profile.map(str::to_owned),
            rows,
            csharp,
        })
    }
    pub fn csharp(&self) -> &BTreeMap<String, String> {
        &self.csharp
    }
    pub fn project(&self) -> &Project {
        &self.project
    }
    pub fn row_counts(&self) -> BTreeMap<String, usize> {
        self.rows
            .iter()
            .map(|(name, rows)| (name.clone(), rows.len()))
            .collect()
    }
    pub(crate) fn builder(&self) -> String {
        codegen::builder(&self.project)
    }
    pub(crate) fn write_request(&self, writer: impl std::io::Write) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            version: u32,
            profile: &'a Option<String>,
            schema_source_content_hashes: BTreeMap<&'a str, &'a str>,
            tables: &'a Dataset,
        }
        let hashes = self
            .project
            .sources
            .iter()
            .filter(|(_, source)| matches!(source.kind.as_deref(), Some("schema" | "type")))
            .map(|(path, source)| (path.as_str(), source.identity.as_str()))
            .collect();
        serde_json::to_writer(
            writer,
            &Request {
                version: 1,
                profile: &self.profile,
                schema_source_content_hashes: hashes,
                tables: &self.rows,
            },
        )
        .map_err(|error| Error::new("E-BUILDER-REQUEST", error.to_string()))
    }
}
