use sha2::{Digest, Sha256};

use crate::document::ProjectDocuments;
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::project::{Project, ProjectInfo};
use crate::table::{BuildSelection, ResolvedTable, resolve_tables};
use crate::type_system::{TypeSystem, resolve_type_system};
use crate::validation::{ValidationReport, validate_documents_with_selection};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    ReadyForDotnet,
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
    pub artifact_root: std::path::PathBuf,
    pub csharp_output: std::path::PathBuf,
    pub binary_output: std::path::PathBuf,
    pub cache_directory: std::path::PathBuf,
    pub status: BuildStatus,
}

/// The result of semantic preparation that does not require native I/O.
/// `ProjectInfo` and artifact paths are intentionally added only when this
/// model is lowered into a native `BuildPlan`.
#[derive(Debug, Clone)]
pub struct SemanticBuildPreparation {
    pub documents: ProjectDocuments,
    pub type_system: TypeSystem,
    pub selection: BuildSelection,
    pub tables: Vec<ResolvedTable>,
    pub validation: ValidationReport,
    pub schema_source_content_hash: String,
}

/// Native convenience wrapper that performs project document loading before
/// entering the snapshot-only semantic preparation boundary. Browser or
/// other non-native hosts should call [`prepare_semantic_build`] directly.
pub fn prepare_build_with_selection(
    project: &Project,
    selection: &BuildSelection,
) -> Result<BuildPlan> {
    let documents = project.load_documents()?;
    prepare_build_from_documents(project.info(), documents, selection)
}

/// Prepare semantic build state from a source snapshot supplied by a host.
/// This function is deliberately independent of native filesystem I/O so a
/// future Browser Host can provide the same loaded documents.
// WHY: Host I/O must stop at the loaded-document snapshot; validation,
// resolution, selection, and source-content hashing are shared semantics.
// IF REMOVED: semantic preparation would regain a hidden native filesystem
// dependency and connected/browser hosts could not share the same result.
// EVIDENCE: docs/specs/runtime-hosts.md; docs/adr/0006-host-capability-composition.md
// Regression: build_preparation_accepts_loaded_documents.
pub fn prepare_build_from_documents(
    project: ProjectInfo,
    documents: ProjectDocuments,
    selection: &BuildSelection,
) -> Result<BuildPlan> {
    let semantic = prepare_semantic_build(documents, selection).map_err(|error| {
        let mut diagnostic = error.diagnostic().clone();
        if diagnostic.source.is_none() {
            diagnostic.source = Some(project.project_root.clone());
        }
        MasterdataError {
            diagnostic: Box::new(diagnostic),
        }
    })?;
    let info = project;
    Ok(BuildPlan {
        artifact_root: info.artifact_root.clone(),
        csharp_output: info.csharp_output.clone(),
        binary_output: info.binary_output.clone(),
        cache_directory: info.cache.clone(),
        project: info,
        documents: semantic.documents,
        type_system: semantic.type_system,
        selection: semantic.selection,
        tables: semantic.tables,
        validation: semantic.validation,
        schema_source_content_hash: semantic.schema_source_content_hash,
        status: BuildStatus::ReadyForDotnet,
    })
}

/// Run validation, resolution, selection, and source-content hashing using
/// only the already-loaded source snapshot.
pub fn prepare_semantic_build(
    documents: ProjectDocuments,
    selection: &BuildSelection,
) -> Result<SemanticBuildPreparation> {
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
        .with_related_requirement("RUNTIME-HOST-013"));
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
    let schema_source_content_hash = compute_schema_source_content_hash(&documents);
    Ok(SemanticBuildPreparation {
        documents,
        type_system,
        selection: selection.clone(),
        tables,
        validation,
        schema_source_content_hash,
    })
}

/// Hash schema source bytes in deterministic path order. This is deliberately
/// a schema source-content hash, not a semantic schema hash or builder cache
/// key.
pub fn compute_schema_source_content_hash(documents: &ProjectDocuments) -> String {
    let mut schema_sources: Vec<_> = documents
        .files
        .iter()
        .filter_map(|loaded| match &loaded.document {
            crate::document::SourceDocument::Schema(_)
            | crate::document::SourceDocument::Type(_) => {
                Some((&loaded.path, loaded.source.as_bytes()))
            }
            crate::document::SourceDocument::Data(_) => None,
        })
        .collect();
    schema_sources.sort_by(|left, right| left.0.cmp(right.0));
    let mut hasher = Sha256::new();
    for (_, source) in schema_sources {
        hasher.update(source);
        hasher.update([0u8]);
    }
    format!("{:x}", hasher.finalize())
}
