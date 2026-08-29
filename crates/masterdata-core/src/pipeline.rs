use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::document::ProjectDocuments;
use crate::error::{ErrorKind, MasterdataError, Result, io_error};
use crate::project::{Project, ProjectInfo};
use crate::validation::ValidationReport;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    ReadyForDotnet,
    MasterMemoryBinaryNotImplemented,
}

#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub project: ProjectInfo,
    pub documents: ProjectDocuments,
    pub validation: ValidationReport,
    pub schema_source_content_hash: String,
    pub generated_output: std::path::PathBuf,
    pub binary_output: Option<std::path::PathBuf>,
    pub cache_directory: std::path::PathBuf,
    pub status: BuildStatus,
}

pub fn prepare_build(project: &Project) -> Result<BuildPlan> {
    let documents = project.load_documents()?;
    let validation = crate::validate_documents(&documents);
    if !validation.valid {
        return Err(MasterdataError::new(
            "E-BUILD-VALIDATION-FAILED",
            ErrorKind::Validation,
            format!(
                "project validation failed with {} diagnostic(s)",
                validation.diagnostics.len()
            ),
        )
        .with_source(project.root().to_path_buf()));
    }
    let schema_source_content_hash =
        compute_schema_source_content_hash(&documents, project.root())?;
    let info = project.info();
    Ok(BuildPlan {
        binary_output: info.build_binary_output.clone(),
        cache_directory: info.build_cache.clone(),
        project: info,
        documents,
        validation,
        schema_source_content_hash,
        generated_output: project.build_output_path(),
        status: BuildStatus::ReadyForDotnet,
    })
}

/// Hash schema source bytes in deterministic path order. This is deliberately
/// a schema source-content hash, not a semantic schema hash or builder cache
/// key.
pub fn compute_schema_source_content_hash(
    documents: &ProjectDocuments,
    _project_root: &Path,
) -> Result<String> {
    let mut schema_paths: Vec<_> = documents.schemas().map(|(path, _)| path).collect();
    schema_paths.sort();
    let mut hasher = Sha256::new();
    for path in schema_paths {
        let bytes = fs::read(path).map_err(|error| io_error(path, error))?;
        hasher.update(bytes);
        hasher.update([0u8]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
