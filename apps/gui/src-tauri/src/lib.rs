use std::path::{Path, PathBuf};

use masterdata_app::ApplicationService;
use masterdata_core::{Diagnostic, ErrorKind, MasterdataError, ProjectInfo};
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
    ApplicationService::new()
        .project_info(explicit_path, &current_dir)
        .map_err(ApiError::from)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildResponse {
    project: ProjectInfo,
    schema_source_content_hash: String,
    generated_output: PathBuf,
    binary_output: Option<PathBuf>,
    cache_directory: PathBuf,
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
    let execution = ApplicationService::new()
        .build(explicit_path, &current_dir, dry_run)
        .map_err(ApiError::from)?;
    Ok(BuildResponse {
        project: execution.plan.project.clone(),
        schema_source_content_hash: execution.plan.schema_source_content_hash,
        generated_output: execution.plan.generated_output,
        binary_output: execution.plan.binary_output,
        cache_directory: execution.plan.cache_directory,
        generated_files: execution.written_files,
        dry_run,
    })
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![project_info, build])
        .run(tauri::generate_context!())
        .expect("error while running masterdata GUI");
}

#[cfg(test)]
mod tests {
    use super::DiagnosticDto;
    use masterdata_core::{Diagnostic, ErrorKind};

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
}
