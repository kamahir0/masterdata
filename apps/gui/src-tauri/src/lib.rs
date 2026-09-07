use std::path::{Path, PathBuf};

use masterdata_app::NativeApplicationService;
use masterdata_core::{Diagnostic, ErrorKind, MasterdataError, ProjectInfo, ValidationReport};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticDto {
    code: String,
    kind: ErrorKind,
    message: String,
    source: Option<PathBuf>,
    line: Option<usize>,
    column: Option<usize>,
    schema_path: Option<String>,
    record_identity: Option<String>,
    suggestion: Option<String>,
    related_requirements: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    diagnostic: Box<DiagnosticDto>,
}

impl From<&Diagnostic> for DiagnosticDto {
    fn from(diagnostic: &Diagnostic) -> Self {
        Self {
            code: diagnostic.code.clone(),
            kind: diagnostic.kind,
            message: diagnostic.message.clone(),
            source: diagnostic.source.clone(),
            line: diagnostic.line,
            column: diagnostic.column,
            schema_path: diagnostic.schema_path.clone(),
            record_identity: diagnostic.record_identity.clone(),
            suggestion: diagnostic.suggestion.clone(),
            related_requirements: diagnostic.related_requirements.clone(),
        }
    }
}

impl From<MasterdataError> for ApiError {
    fn from(error: MasterdataError) -> Self {
        Self {
            diagnostic: Box::new(DiagnosticDto::from(error.diagnostic())),
        }
    }
}

fn current_directory() -> std::result::Result<PathBuf, ApiError> {
    std::env::current_dir().map_err(|error| {
        ApiError::from(MasterdataError::new(
            "E-GUI-CURRENT-DIRECTORY",
            ErrorKind::Io,
            format!("could not determine current directory: {error}"),
        ))
    })
}

#[tauri::command(rename_all = "camelCase")]
fn project_info(project_path: Option<String>) -> std::result::Result<ProjectInfo, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = project_path.or_else(|| std::env::var("MASTERDATA_PROJECT_PATH").ok());
    let explicit_path = configured_path.as_deref().map(Path::new);
    NativeApplicationService::new()
        .project_info(explicit_path, &current_dir)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn validate(project_path: Option<String>) -> std::result::Result<ValidationReport, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = project_path.or_else(|| std::env::var("MASTERDATA_PROJECT_PATH").ok());
    let explicit_path = configured_path.as_deref().map(Path::new);
    NativeApplicationService::new()
        .validate(explicit_path, &current_dir)
        .map_err(ApiError::from)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildResponse {
    project: ProjectInfo,
    schema_source_content_hash: String,
    artifact_root: PathBuf,
    csharp_output: PathBuf,
    binary_output: PathBuf,
    cache: PathBuf,
    generated_files: Vec<PathBuf>,
    dry_run: bool,
}

#[tauri::command(rename_all = "camelCase")]
fn build(
    project_path: Option<String>,
    dry_run: bool,
) -> std::result::Result<BuildResponse, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = project_path.or_else(|| std::env::var("MASTERDATA_PROJECT_PATH").ok());
    let explicit_path = configured_path.as_deref().map(Path::new);
    let execution = NativeApplicationService::new()
        .build(explicit_path, &current_dir, dry_run)
        .map_err(ApiError::from)?;
    Ok(BuildResponse {
        project: execution.plan.project.clone(),
        schema_source_content_hash: execution.plan.schema_source_content_hash,
        artifact_root: execution.plan.artifact_root,
        csharp_output: execution.plan.csharp_output,
        binary_output: execution.plan.binary_output,
        cache: execution.plan.cache_directory,
        generated_files: execution.written_files,
        dry_run,
    })
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![project_info, validate, build])
        .run(tauri::generate_context!())
        .expect("error while running masterdata GUI");
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::DiagnosticDto;
    use masterdata_core::{Diagnostic, ErrorKind, ValidationReport};

    #[test]
    fn diagnostic_dto_preserves_structured_fields() {
        let diagnostic =
            Diagnostic::new("E-SCHEMA-INVALID", ErrorKind::Validation, "invalid field")
                .with_schema_path("fields[0].type")
                .with_record_identity("item:1001")
                .with_suggestion("use a supported type")
                .with_related_requirement("SCHEMA-VO-001");
        let value =
            serde_json::to_value(DiagnosticDto::from(&diagnostic)).expect("diagnostic serializes");

        assert_eq!(value["code"], "E-SCHEMA-INVALID");
        assert_eq!(value["kind"], "validation");
        assert_eq!(value["schemaPath"], "fields[0].type");
        assert_eq!(value["recordIdentity"], "item:1001");
        assert_eq!(value["suggestion"], "use a supported type");
        assert_eq!(value["relatedRequirements"][0], "SCHEMA-VO-001");
    }

    #[test]
    fn validation_report_preserves_shared_structured_diagnostics() {
        let report = ValidationReport {
            valid: false,
            files_scanned: 1,
            schema_documents: 1,
            data_documents: 0,
            type_documents: 0,
            tables: vec!["item".to_owned()],
            types: Vec::new(),
            diagnostics: vec![
                Diagnostic::new("E-SCHEMA-INVALID", ErrorKind::Validation, "invalid field")
                    .with_source("sources/item.yaml")
                    .with_schema_path("fields[0].type")
                    .with_record_identity("item:1001")
                    .with_suggestion("use a supported type")
                    .with_related_requirement("SCHEMA-VO-001"),
            ],
        };
        let value = serde_json::to_value(report).expect("validation report serializes");

        assert_eq!(value["valid"], false);
        assert_eq!(value["files_scanned"], 1);
        assert_eq!(value["diagnostics"][0]["code"], "E-SCHEMA-INVALID");
        assert_eq!(value["diagnostics"][0]["schema_path"], "fields[0].type");
        assert_eq!(value["diagnostics"][0]["record_identity"], "item:1001");
        assert_eq!(
            value["diagnostics"][0]["related_requirements"][0],
            "SCHEMA-VO-001"
        );
    }

    #[test]
    fn validate_command_uses_shared_validation_service() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/minimal")
            .canonicalize()
            .expect("minimal fixture path");

        let report = super::validate(Some(project.to_string_lossy().into_owned()))
            .expect("validation command succeeds");

        assert!(report.valid, "{report:?}");
        assert!(report.tables.iter().any(|table| table == "item"));
    }

    #[test]
    fn build_command_uses_shared_build_service() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/minimal")
            .canonicalize()
            .expect("minimal fixture path");

        let response = super::build(Some(project.to_string_lossy().into_owned()), true)
            .expect("dry-run build command succeeds");
        let value = serde_json::to_value(&response).expect("build response serializes");

        assert_eq!(response.project.project_id, "fixture.minimal");
        assert!(response.dry_run);
        assert!(response.generated_files.is_empty());
        assert_eq!(value["dryRun"], true);
        assert_eq!(
            value["artifactRoot"],
            project
                .join(".masterdata/output")
                .to_string_lossy()
                .as_ref()
        );
        assert_eq!(
            value["binaryOutput"],
            project
                .join(".masterdata/output/masterdata.bytes")
                .to_string_lossy()
                .as_ref()
        );
    }
}
