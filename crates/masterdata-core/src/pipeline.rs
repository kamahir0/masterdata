use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::document::ProjectDocuments;
use crate::error::{ErrorKind, MasterdataError, Result, io_error};
use crate::project::{Project, ProjectInfo};
use crate::table::{BuildSelection, ResolvedTable, resolve_tables};
use crate::type_system::{TypeSystem, resolve_type_system};
use crate::validation::{ValidationReport, validate_documents_with_selection};

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
    pub type_system: TypeSystem,
    pub selection: BuildSelection,
    pub tables: Vec<ResolvedTable>,
    pub validation: ValidationReport,
    pub schema_source_content_hash: String,
    pub generated_output: std::path::PathBuf,
    pub binary_output: Option<std::path::PathBuf>,
    pub cache_directory: std::path::PathBuf,
    pub status: BuildStatus,
}

pub fn prepare_build(project: &Project) -> Result<BuildPlan> {
    prepare_build_with_selection(project, &BuildSelection::unfiltered())
}

pub fn prepare_build_with_selection(
    project: &Project,
    selection: &BuildSelection,
) -> Result<BuildPlan> {
    let documents = project.load_documents()?;
    let validation = validate_documents_with_selection(&documents, selection);
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
    let type_system = resolve_type_system(&documents)?;
    let tables = resolve_tables(&documents, &type_system, selection)
        .model
        .ok_or_else(|| {
            MasterdataError::new(
                "E-TABLE-RESOLUTION-FAILED",
                ErrorKind::Validation,
                "validated project could not be lowered to logical Tables",
            )
        })?;
    let schema_source_content_hash =
        compute_schema_source_content_hash(&documents, project.root())?;
    let info = project.info();
    Ok(BuildPlan {
        binary_output: info.build_binary_output.clone(),
        cache_directory: info.build_cache.clone(),
        project: info,
        documents,
        type_system,
        selection: selection.clone(),
        tables,
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
    let mut schema_paths: Vec<_> = documents
        .schemas()
        .map(|(path, _)| path)
        .chain(documents.types().map(|(path, _)| path))
        .collect();
    schema_paths.sort();
    let mut hasher = Sha256::new();
    for path in schema_paths {
        let bytes = fs::read(path).map_err(|error| io_error(path, error))?;
        hasher.update(bytes);
        hasher.update([0u8]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
