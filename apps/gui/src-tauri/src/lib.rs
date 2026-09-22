use std::path::{Path, PathBuf};

use masterdata_app::{
    AuthoringBatchCopyRequest, AuthoringBatchCopyResult, AuthoringBatchPreview,
    AuthoringBatchRequest, AuthoringClipboardShape, AuthoringEdit, AuthoringRecordDraft,
    AuthoringRecordMutation, AuthoringWorkspace, ComputedViewEditPreview, ComputedViewEditRequest,
    ComputedViewRemoveReport, ComputedViewSaveReport, ComputedViewSnapshot, ConfigSaveReport,
    CreationContext, CreationDestinationState, CreationReport, CreationRequest,
    DataFileQueryRequest, DataFileQueryResult, DataFileSnapshot, NativeApplicationService,
    ProjectConfigEditPreviewView, ProjectConfigEditRequest, ProjectConfigSnapshot,
    ProjectInitReport, ProjectInitRequest, PublishAggregateStatus, PublishExecutionReport,
    PublishPreview, RecordTagEditRequest, SourceContentState, SourceEditPreview,
    SourcePathMutationReport, SourcePathMutationRequest, SourcePathStateReport, SourceSaveReport,
    TableOverviewRequest, TableOverviewSnapshot,
};
use masterdata_core::{
    CompatibilityReport, Diagnostic, ErrorKind, MasterdataError, ProjectInfo, ValidationReport,
};
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
    value_path: Option<String>,
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
            value_path: diagnostic.value_path.clone(),
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

fn configured_project_path(project_path: Option<String>) -> Option<String> {
    project_path.or_else(|| std::env::var("MASTERDATA_PROJECT_PATH").ok())
}

fn active_operations() -> &'static std::sync::Mutex<std::collections::BTreeSet<PathBuf>> {
    static ACTIVE: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeSet<PathBuf>>> =
        std::sync::OnceLock::new();
    ACTIVE.get_or_init(Default::default)
}

struct OperationGuard {
    project_root: PathBuf,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = active_operations().lock() {
            active.remove(&self.project_root);
        }
    }
}

fn operation_guard(project_root: &Path) -> std::result::Result<OperationGuard, ApiError> {
    let key = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let mut active = active_operations().lock().map_err(|_| {
        ApiError::from(MasterdataError::new(
            "E-GUI-OPERATION-BUSY",
            ErrorKind::Io,
            "project operation guard is unavailable",
        ))
    })?;
    if !active.insert(key.clone()) {
        return Err(ApiError::from(MasterdataError::new(
            "E-GUI-OPERATION-BUSY",
            ErrorKind::Validation,
            "another operation is in progress for this project; retry after it completes",
        )));
    }
    Ok(OperationGuard { project_root: key })
}

fn table_session() -> std::result::Result<
    std::sync::MutexGuard<'static, masterdata_app::TableAuthoringSession>,
    ApiError,
> {
    static SESSION: std::sync::OnceLock<std::sync::Mutex<masterdata_app::TableAuthoringSession>> =
        std::sync::OnceLock::new();
    SESSION.get_or_init(Default::default).lock().map_err(|_| {
        ApiError::from(MasterdataError::new(
            "E-MIGRATION-SESSION",
            ErrorKind::Io,
            "Migration session unavailable",
        ))
    })
}
fn table_root(project_path: Option<String>) -> std::result::Result<PathBuf, ApiError> {
    let path = configured_project_path(project_path);
    Ok(NativeApplicationService::new()
        .project_info(path.as_deref().map(Path::new), &current_directory()?)
        .map_err(ApiError::from)?
        .project_root)
}

fn config_binding_root(project_path: Option<String>) -> std::result::Result<PathBuf, ApiError> {
    let configured = configured_project_path(project_path);
    let current = current_directory()?;
    let path = configured.map(PathBuf::from).unwrap_or(current.clone());
    let absolute = if path.is_absolute() {
        path
    } else {
        current.join(path)
    };
    if absolute.file_name().and_then(|name| name.to_str()) == Some("masterdata.toml") {
        Ok(absolute.parent().map(Path::to_path_buf).unwrap_or(absolute))
    } else {
        Ok(absolute)
    }
}
#[tauri::command(rename_all = "camelCase")]
fn open_table(
    project_path: Option<String>,
    relative_path: String,
) -> std::result::Result<masterdata_app::TableSnapshot, ApiError> {
    table_session()?
        .open_table(&table_root(project_path)?, &relative_path)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn plan_table_migration(
    project_path: Option<String>,
    input: serde_json::Value,
) -> std::result::Result<masterdata_app::TablePlanView, ApiError> {
    let input = serde_json::from_value(input).map_err(|error| {
        ApiError::from(MasterdataError::new(
            "E-MIGRATION-INPUT",
            ErrorKind::Validation,
            format!("invalid Migration input: {error}"),
        ))
    })?;
    table_session()?
        .plan(&table_root(project_path)?, input)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn open_type(
    project_path: Option<String>,
    relative_path: String,
) -> std::result::Result<masterdata_app::TypeSnapshot, ApiError> {
    table_session()?
        .open_type(&table_root(project_path)?, &relative_path)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn plan_type_migration(
    project_path: Option<String>,
    input: serde_json::Value,
) -> std::result::Result<masterdata_app::TypePlanView, ApiError> {
    let input = serde_json::from_value(input).map_err(|e| {
        ApiError::from(MasterdataError::new(
            "E-TYPE-EDITOR-INPUT",
            ErrorKind::Validation,
            e.to_string(),
        ))
    })?;
    table_session()?
        .plan_type(&table_root(project_path)?, input)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn apply_table_migration(
    project_path: Option<String>,
    token: String,
    allow_destructive: bool,
) -> std::result::Result<masterdata_app::TableApplyView, ApiError> {
    let root = table_root(project_path)?;
    let _operation = operation_guard(&root)?;
    table_session()?
        .apply(&root, &token, allow_destructive)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn migration_recovery_status(
    project_path: Option<String>,
) -> std::result::Result<Option<masterdata_app::TableApplyView>, ApiError> {
    table_session()?
        .recovery_status(&table_root(project_path)?)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn recheck_migration(
    project_path: Option<String>,
) -> std::result::Result<Option<masterdata_app::TableApplyView>, ApiError> {
    table_session()?
        .recheck(&table_root(project_path)?)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn project_info(project_path: Option<String>) -> std::result::Result<ProjectInfo, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .project_info(configured_path.as_deref().map(Path::new), &current_dir)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn create_project(request: ProjectInitRequest) -> std::result::Result<ProjectInitReport, ApiError> {
    let current = current_directory()?;
    let _operation = operation_guard(&current)?;
    NativeApplicationService::new()
        .create_project(&current, &request)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn authoring_workspace(
    project_path: Option<String>,
) -> std::result::Result<AuthoringWorkspace, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .authoring_workspace(configured_path.as_deref().map(Path::new), &current_dir)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn creation_context(
    project_path: Option<String>,
) -> std::result::Result<CreationContext, ApiError> {
    let current_dir = current_directory()?;
    let configured = configured_project_path(project_path);
    NativeApplicationService::new()
        .creation_context(configured.as_deref().map(Path::new), &current_dir)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn create_source(
    project_path: Option<String>,
    request: serde_json::Value,
) -> std::result::Result<CreationReport, ApiError> {
    // Decode inside the command so malformed form values remain a structured
    // preflight rejection, never an ambiguous transport/commit failure.
    // EVIDENCE: GUI-CREATE-ERR-001, SOURCE-CREATE-014.
    let request: CreationRequest = serde_json::from_value(request).map_err(|error| {
        ApiError::from(MasterdataError::new(
            "E-SOURCE-CREATE-REQUEST",
            ErrorKind::Validation,
            format!("invalid creation input: {error}"),
        ))
    })?;
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    // Hold the shared session guard across mutation so Apply cannot race another
    // source command or bypass a Recovery Required gate (GUI-SHELL-CAPABILITY-001).
    let session = table_session()?;
    session
        .ensure_mutation_allowed(&root)
        .map_err(ApiError::from)?;
    let current_dir = current_directory()?;
    let configured = configured_project_path(project_path);
    NativeApplicationService::new()
        .create_source(configured.as_deref().map(Path::new), &current_dir, &request)
        .map_err(ApiError::from)
}
#[tauri::command(rename_all = "camelCase")]
fn recheck_creation(
    project_path: Option<String>,
    request: CreationRequest,
) -> std::result::Result<CreationDestinationState, ApiError> {
    let current_dir = current_directory()?;
    let configured = configured_project_path(project_path);
    NativeApplicationService::new()
        .recheck_creation(configured.as_deref().map(Path::new), &current_dir, &request)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn rename_source_file(
    project_path: Option<String>,
    request: serde_json::Value,
) -> std::result::Result<SourcePathMutationReport, ApiError> {
    let request: SourcePathMutationRequest = serde_json::from_value(request).map_err(|error| {
        ApiError::from(MasterdataError::new(
            "E-SOURCE-PATH-REQUEST",
            ErrorKind::Validation,
            format!("invalid source path mutation input: {error}"),
        ))
    })?;
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    let session = table_session()?;
    session
        .ensure_mutation_allowed(&root)
        .map_err(ApiError::from)?;
    let current_dir = current_directory()?;
    let configured = configured_project_path(project_path);
    NativeApplicationService::new()
        .rename_source_file(configured.as_deref().map(Path::new), &current_dir, &request)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn source_path_state(
    project_path: Option<String>,
    request: SourcePathMutationRequest,
) -> std::result::Result<SourcePathStateReport, ApiError> {
    let current_dir = current_directory()?;
    let configured = configured_project_path(project_path);
    NativeApplicationService::new()
        .source_path_state(configured.as_deref().map(Path::new), &current_dir, &request)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn open_data_file(
    project_path: Option<String>,
    relative_path: String,
) -> std::result::Result<DataFileSnapshot, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .open_data_file(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn open_project_config(
    project_path: Option<String>,
) -> std::result::Result<ProjectConfigSnapshot, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .open_project_config(configured_path.as_deref().map(Path::new), &current_dir)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn preview_project_config_edit(
    base_source: String,
    base_content_identity: String,
    request: ProjectConfigEditRequest,
) -> std::result::Result<ProjectConfigEditPreviewView, ApiError> {
    NativeApplicationService::new()
        .preview_project_config_edit(&base_source, &base_content_identity, &request)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn save_project_config_edit(
    project_path: Option<String>,
    base_source: String,
    base_content_identity: String,
    requests: Vec<ProjectConfigEditRequest>,
) -> std::result::Result<ConfigSaveReport, ApiError> {
    let root = config_binding_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    table_session()?
        .ensure_mutation_allowed_at_root(&root)
        .map_err(ApiError::from)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .save_project_config_edit(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &base_source,
            &base_content_identity,
            &requests,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn authoring_clipboard_shape(
    clipboard_text: String,
) -> std::result::Result<AuthoringClipboardShape, ApiError> {
    NativeApplicationService::new()
        .authoring_clipboard_shape(&clipboard_text)
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn query_data_file(
    project_path: Option<String>,
    request: DataFileQueryRequest,
) -> std::result::Result<DataFileQueryResult, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .query_data_file(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &request,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn table_overview(
    project_path: Option<String>,
    request: TableOverviewRequest,
) -> std::result::Result<TableOverviewSnapshot, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .table_overview(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &request,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn open_computed_view(
    project_path: Option<String>,
    relative_path: String,
) -> std::result::Result<ComputedViewSnapshot, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .open_computed_view(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn preview_computed_view(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    request: ComputedViewEditRequest,
) -> std::result::Result<ComputedViewEditPreview, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .preview_computed_view(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &request,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn save_computed_view(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    base_content_identity: String,
    request: ComputedViewEditRequest,
) -> std::result::Result<ComputedViewSaveReport, ApiError> {
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    table_session()?
        .ensure_mutation_allowed(&root)
        .map_err(ApiError::from)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .save_computed_view(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &base_content_identity,
            &request,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn remove_computed_view(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    base_content_identity: String,
    confirmed: bool,
) -> std::result::Result<ComputedViewRemoveReport, ApiError> {
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    table_session()?
        .ensure_mutation_allowed(&root)
        .map_err(ApiError::from)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .remove_computed_view(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &base_content_identity,
            confirmed,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn publish_preview(project_path: Option<String>) -> std::result::Result<PublishPreview, ApiError> {
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .publish_preview(configured_path.as_deref().map(Path::new), &current_dir)
        .map_err(ApiError::from)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishExecutionView {
    status: &'static str,
    outcome: PublishAggregateStatus,
    unity_verification: &'static str,
    report: PublishExecutionReport,
    diagnostic: Option<DiagnosticDto>,
}

#[tauri::command(rename_all = "camelCase")]
fn publish_from_preview(
    project_path: Option<String>,
    artifact_set_identity: String,
    config_content_identity: String,
    publish_plan_identity: String,
) -> std::result::Result<PublishExecutionView, ApiError> {
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    match NativeApplicationService::new().publish_from_preview(
        configured_path.as_deref().map(Path::new),
        &current_dir,
        &artifact_set_identity,
        &config_content_identity,
        &publish_plan_identity,
    ) {
        Ok(report) => Ok(PublishExecutionView {
            status: "success",
            outcome: report.aggregate_status(),
            unity_verification: "not_observed",
            report,
            diagnostic: None,
        }),
        Err(failure) => {
            let diagnostic = DiagnosticDto::from(failure.diagnostic());
            Ok(PublishExecutionView {
                status: "failure",
                outcome: failure.report.aggregate_status(),
                unity_verification: "not_observed",
                report: failure.report,
                diagnostic: Some(diagnostic),
            })
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
fn preview_data_file(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    edits: Vec<AuthoringEdit>,
    added_records: Option<Vec<AuthoringRecordDraft>>,
    deleted_record_indices: Option<Vec<usize>>,
    tag_edits: Option<Vec<RecordTagEditRequest>>,
) -> std::result::Result<SourceEditPreview, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    let mutation = AuthoringRecordMutation {
        edits,
        added_records: added_records.unwrap_or_default(),
        deleted_record_indices: deleted_record_indices.unwrap_or_default(),
        tag_edits: tag_edits.unwrap_or_default(),
    };
    NativeApplicationService::new()
        .preview_data_file_mutation(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &mutation,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn preview_data_file_batch(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    current_mutation: AuthoringRecordMutation,
    request: AuthoringBatchRequest,
) -> std::result::Result<AuthoringBatchPreview, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .preview_data_file_batch(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &current_mutation,
            &request,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn copy_data_file_batch(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    request: AuthoringBatchCopyRequest,
) -> std::result::Result<AuthoringBatchCopyResult, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .copy_data_file_batch(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &request,
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn source_content(
    project_path: Option<String>,
    relative_path: String,
) -> std::result::Result<SourceContentState, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .source_content(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
        )
        .map_err(ApiError::from)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command(rename_all = "camelCase")]
fn save_data_file(
    project_path: Option<String>,
    relative_path: String,
    base_source: String,
    base_content_identity: String,
    edits: Vec<AuthoringEdit>,
    added_records: Option<Vec<AuthoringRecordDraft>>,
    deleted_record_indices: Option<Vec<usize>>,
    tag_edits: Option<Vec<RecordTagEditRequest>>,
    overwrite_expected_identity: Option<String>,
) -> std::result::Result<SourceSaveReport, ApiError> {
    let root = table_root(project_path.clone())?;
    // A normal source Save participates in the migration/session gate but not
    // the long-lived Build/Publish operation guard. Build capture holds this
    // session only until its immutable Plan is complete.
    let session = table_session()?;
    session
        .ensure_mutation_allowed(&root)
        .map_err(ApiError::from)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    let mutation = AuthoringRecordMutation {
        edits,
        added_records: added_records.unwrap_or_default(),
        deleted_record_indices: deleted_record_indices.unwrap_or_default(),
        tag_edits: tag_edits.unwrap_or_default(),
    };
    NativeApplicationService::new()
        .save_data_file_mutation(
            configured_path.as_deref().map(Path::new),
            &current_dir,
            &relative_path,
            &base_source,
            &base_content_identity,
            &mutation,
            overwrite_expected_identity.as_deref(),
        )
        .map_err(ApiError::from)
}

#[tauri::command(rename_all = "camelCase")]
fn validate(project_path: Option<String>) -> std::result::Result<ValidationReport, ApiError> {
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    NativeApplicationService::new()
        .validate(configured_path.as_deref().map(Path::new), &current_dir)
        .map_err(ApiError::from)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildResponse {
    project: ProjectInfo,
    profile: Option<String>,
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
    profile: Option<String>,
) -> std::result::Result<BuildResponse, ApiError> {
    let root = table_root(project_path.clone())?;
    let _operation = operation_guard(&root)?;
    let current_dir = current_directory()?;
    let configured_path = configured_project_path(project_path);
    let service = NativeApplicationService::new();
    let plan = {
        let session = table_session()?;
        session
            .ensure_mutation_allowed(&root)
            .map_err(ApiError::from)?;
        service
            .prepare_build_with_profile(
                configured_path.as_deref().map(Path::new),
                &current_dir,
                profile.as_deref(),
            )
            .map_err(ApiError::from)?
    };
    let execution = service
        .build_from_plan(plan, dry_run)
        .map_err(ApiError::from)?;
    Ok(BuildResponse {
        project: execution.plan.project.clone(),
        profile,
        schema_source_content_hash: execution.plan.schema_source_content_hash,
        artifact_root: execution.plan.artifact_root,
        csharp_output: execution.plan.csharp_output,
        binary_output: execution.plan.binary_output,
        cache: execution.plan.cache_directory,
        generated_files: execution.written_files,
        dry_run,
    })
}

#[tauri::command(rename_all = "camelCase")]
fn compatibility_report(
    baseline_project: String,
    current_project: String,
) -> std::result::Result<CompatibilityReport, ApiError> {
    let current_dir = current_directory()?;
    NativeApplicationService::new()
        .analyze_compatibility(
            Path::new(&baseline_project),
            Path::new(&current_project),
            &current_dir,
        )
        .map_err(ApiError::from)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_table,
            open_type,
            plan_type_migration,
            plan_table_migration,
            apply_table_migration,
            migration_recovery_status,
            recheck_migration,
            project_info,
            create_project,
            authoring_workspace,
            creation_context,
            create_source,
            recheck_creation,
            rename_source_file,
            source_path_state,
            open_project_config,
            preview_project_config_edit,
            save_project_config_edit,
            authoring_clipboard_shape,
            open_data_file,
            query_data_file,
            table_overview,
            open_computed_view,
            preview_computed_view,
            save_computed_view,
            remove_computed_view,
            publish_preview,
            publish_from_preview,
            preview_data_file,
            preview_data_file_batch,
            copy_data_file_batch,
            source_content,
            save_data_file,
            validate,
            build,
            compatibility_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running masterdata GUI");
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::DiagnosticDto;
    use masterdata_core::{Diagnostic, ErrorKind, ValidationReport};

    #[test]
    fn malformed_creation_input_is_a_structured_preflight_rejection() {
        let result = super::create_source(
            None,
            serde_json::json!({
                "sourceRoot":"/unused", "destination":"new.yaml",
                "artifact":{"category":"table", "table":"item",
                    "fields":[{"key":null,"name":"id","type":"int"}],
                    "primaryKey":{"fields":["id"]}}
            }),
        );
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("invalid key accepted"),
        };
        assert_eq!(error.diagnostic.code, "E-SOURCE-CREATE-REQUEST");
        assert_eq!(error.diagnostic.kind, ErrorKind::Validation);
    }

    #[test]
    fn table_commands_preserve_snapshot_and_preflight_diagnostics() {
        let project = minimal_project();
        let file = NativeTableFixture::schema_path(&project);
        let table = super::open_table(Some(project.to_string_lossy().into_owned()), file)
            .expect("Table snapshot");
        assert!(!table.schema.fields.is_empty());
        let error = super::plan_table_migration(
            Some(project.to_string_lossy().into_owned()),
            serde_json::json!({"operation":"drop","table":table.schema.table,"field":"missing"}),
        )
        .expect_err("missing field rejected");
        assert_eq!(error.diagnostic.kind, ErrorKind::Validation);
    }
    #[test]
    fn type_commands_preserve_exact_numbers_and_share_apply_boundary() {
        let dir = tempfile::tempdir().unwrap();
        masterdata_core::initialize_project(
            dir.path(),
            &masterdata_core::InitOptions {
                project_id: "test.types".into(),
                name: "Types".into(),
                version: "0.1.0".into(),
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("sources/type.yaml"), "kind: type\nname: Rarity\nenum:\n  underlying: ulong\n  members:\n    - name: Rare\n      value: 18446744073709551615\n").unwrap();
        let path = Some(dir.path().to_string_lossy().into_owned());
        let snapshot = super::open_type(path.clone(), "sources/type.yaml".into()).unwrap();
        assert_eq!(snapshot.members[0].value, "18446744073709551615");
        let plan = super::plan_type_migration(path.clone(), serde_json::json!({"operation":"add_enum","target":"Rarity","name":"High","value":"18446744073709551614"})).unwrap();
        assert!(plan.files[0].after.contains("18446744073709551614"));
        let outcome = super::apply_table_migration(path.clone(), plan.token, false).unwrap();
        assert_eq!(outcome.state, "success");
        let error = super::plan_type_migration(path, serde_json::json!({"operation":"add_enum","target":"Rarity","name":"Bad","value":18446744073709551615_u64})).unwrap_err();
        assert_eq!(error.diagnostic.code, "E-TYPE-EDITOR-INPUT");
    }

    #[test]
    fn compatibility_command_uses_explicit_projects_and_serializes_axis_report() {
        let baseline = tempfile::tempdir().expect("baseline");
        let current = tempfile::tempdir().expect("current");
        for (root, value) in [(baseline.path(), "before"), (current.path(), "after")] {
            masterdata_core::initialize_project(
                root,
                &masterdata_core::InitOptions {
                    project_id: "gui.compatibility".into(),
                    name: "GUI Compatibility".into(),
                    version: "1.0.0".into(),
                },
            )
            .expect("initialize project");
            std::fs::write(
                root.join("sources/schema.yaml"),
                "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: value\n    type: string\nprimaryKey:\n  fields: [id]\n",
            )
            .expect("schema");
            std::fs::write(
                root.join("sources/data.yaml"),
                format!("kind: data\ntable: item\nrecords:\n  - id: 1\n    value: {value}\n"),
            )
            .expect("data");
        }

        let report = super::compatibility_report(
            baseline.path().to_string_lossy().into_owned(),
            current.path().to_string_lossy().into_owned(),
        )
        .expect("compatibility report");
        let json = serde_json::to_value(&report).expect("report JSON");
        assert_eq!(json["summary"]["changeCount"], 1);
        assert_eq!(json["changes"][0]["artifactBinary"], "rebuild_required");
    }

    struct NativeTableFixture;
    impl NativeTableFixture {
        fn schema_path(root: &Path) -> String {
            let project = masterdata_core::Project::discover(Some(root), root).unwrap();
            let files = project.load_documents().unwrap();
            let schema = files
                .files
                .iter()
                .find(|file| matches!(file.document, masterdata_core::SourceDocument::Schema(_)))
                .unwrap();
            schema
                .path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        }
    }

    fn minimal_project() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/minimal")
            .canonicalize()
            .expect("minimal fixture path")
    }

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
    fn authoring_commands_delegate_to_shared_service() {
        let project = minimal_project();
        let project_path = Some(project.to_string_lossy().into_owned());
        let workspace = super::authoring_workspace(project_path.clone()).expect("workspace");
        assert!(workspace.files.iter().any(|file| file.kind == "data"));

        let snapshot = super::open_data_file(project_path, "sources/items-a.yaml".to_owned())
            .expect("data snapshot");
        assert_eq!(snapshot.table, "item");
        assert!(!snapshot.rows.is_empty());
    }

    #[test]
    fn validate_command_uses_shared_validation_service() {
        let project = minimal_project();
        let report = super::validate(Some(project.to_string_lossy().into_owned()))
            .expect("validation command succeeds");
        assert!(report.valid, "{report:?}");
        assert!(report.tables.iter().any(|table| table == "item"));
    }

    #[test]
    fn build_command_uses_shared_build_service() {
        let project = minimal_project();
        let response = super::build(Some(project.to_string_lossy().into_owned()), true, None)
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

#[cfg(test)]
mod desktop_workflow_tests {
    use super::*;
    use masterdata_app::{
        AuthoringRecordField, ConfigSaveStatus, CreationStatus, ProjectInitStatus, SourceSaveStatus,
    };
    use masterdata_core::{AuthoringValue, PublishTargetKind};
    use std::fs;

    #[test]
    fn desktop_workflow_reaches_build_publish_and_stale_recovery() {
        let temp = tempfile::tempdir().expect("desktop scenario");
        #[cfg(windows)]
        let root = temp.path().join("project");
        #[cfg(not(windows))]
        let root = temp
            .path()
            .canonicalize()
            .expect("canonical desktop scenario root")
            .join("project");
        let init = create_project(ProjectInitRequest {
            destination: root.clone(),
            project_id: "desktop.scenario".into(),
            name: "Desktop Scenario".into(),
            version: "0.1.0".into(),
        })
        .expect("Create Project command");
        assert_eq!(init.status, ProjectInitStatus::Success);
        let project_path = root.to_string_lossy().into_owned();

        let context = creation_context(Some(project_path.clone())).expect("creation context");
        let source_root = context.roots.first().expect("source root").label.clone();
        let table = create_source(
            Some(project_path.clone()),
            serde_json::json!({
                "sourceRoot": source_root,
                "destination": "schemas/item-schema.yaml",
                "artifact": {
                    "category": "table",
                    "table": "item",
                    "csharpName": "ItemMaster",
                    "fields": [
                        {"key": 0, "name": "id", "type": "int"},
                        {"key": 1, "name": "name", "type": "string"}
                    ],
                    "primaryKey": {"fields": ["id"]},
                    "secondaryKeys": []
                }
            }),
        )
        .expect("Create Table source");
        assert_eq!(table.status, CreationStatus::Success);
        let data_creation = create_source(
            Some(project_path.clone()),
            serde_json::json!({
                "sourceRoot": source_root,
                "destination": "data/items.yaml",
                "artifact": {"category": "data", "table": "item"}
            }),
        )
        .expect("Create Data source");
        assert_eq!(data_creation.status, CreationStatus::Success);

        let config = open_project_config(Some(project_path.clone())).expect("open settings");
        let config_report = save_project_config_edit(
            Some(project_path.clone()),
            config.base_source,
            config.base_content_identity,
            vec![
                ProjectConfigEditRequest::AddProfile {
                    name: "prod".into(),
                    include_tags: Vec::new(),
                    exclude_tags: Vec::new(),
                },
                ProjectConfigEditRequest::AddPublishTarget {
                    kind: PublishTargetKind::CSharp,
                    path: "delivery".into(),
                },
            ],
        )
        .expect("save settings");
        assert_eq!(config_report.status, ConfigSaveStatus::Success);

        let data = open_data_file(Some(project_path.clone()), data_creation.path.clone())
            .expect("open created data");
        let source_report = save_data_file(
            Some(project_path.clone()),
            data_creation.path.clone(),
            data.base_source,
            data.base_content_identity,
            Vec::new(),
            Some(vec![AuthoringRecordDraft {
                fields: vec![
                    AuthoringRecordField {
                        field: "id".into(),
                        value: AuthoringValue::Number {
                            value: "1001".into(),
                        },
                    },
                    AuthoringRecordField {
                        field: "name".into(),
                        value: AuthoringValue::String {
                            value: "Mega Potion".into(),
                        },
                    },
                ],
                tags: Vec::new(),
            }]),
            None,
            None,
            None,
        )
        .expect("save new record");
        assert_eq!(source_report.status, SourceSaveStatus::Success);
        assert!(
            fs::read_to_string(root.join(&data_creation.path))
                .expect("saved data")
                .contains("Mega Potion")
        );

        let build =
            build(Some(project_path.clone()), false, Some("prod".into())).expect("desktop build");
        assert!(!build.generated_files.is_empty());
        assert!(build.artifact_root.join("masterdata.bytes").is_file());

        let stale_preview = publish_preview(Some(project_path.clone())).expect("publish preview");
        let delivery = root.join("delivery");
        fs::create_dir_all(&delivery).expect("delivery");
        fs::write(delivery.join("External.g.cs"), b"external").expect("external file");
        fs::write(
            delivery.join(masterdata_app::PUBLISH_MANIFEST_FILENAME),
            br#"{"version":1,"files":["External.g.cs"]}"#,
        )
        .expect("external manifest");

        let stale = publish_from_preview(
            Some(project_path.clone()),
            stale_preview.artifact_set_identity,
            stale_preview.config_content_identity,
            stale_preview.publish_plan_identity,
        )
        .expect("structured stale result");
        assert_eq!(stale.status, "failure");
        assert_eq!(
            stale.outcome,
            masterdata_app::PublishAggregateStatus::Failed
        );
        assert_eq!(stale.unity_verification, "not_observed");
        assert_eq!(
            stale
                .diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.code.as_str()),
            Some("E-PUBLISH-PREVIEW-STALE-DESTINATION")
        );
        assert_eq!(
            fs::read(delivery.join("External.g.cs")).expect("stale preview is non-mutating"),
            b"external"
        );

        let fresh = publish_preview(Some(project_path.clone())).expect("fresh preview");
        assert!(
            fresh
                .targets
                .iter()
                .flat_map(|target| target.removals.iter())
                .any(|path| path == "External.g.cs")
        );
        let published = publish_from_preview(
            Some(project_path),
            fresh.artifact_set_identity,
            fresh.config_content_identity,
            fresh.publish_plan_identity,
        )
        .expect("publish result");
        assert_eq!(published.status, "success");
        assert_eq!(
            published.outcome,
            masterdata_app::PublishAggregateStatus::Succeeded
        );
        assert_eq!(published.unity_verification, "not_observed");
        assert!(
            published
                .report
                .targets
                .iter()
                .all(|target| target.status == masterdata_app::PublishTargetStatus::Succeeded)
        );
        assert!(!delivery.join("External.g.cs").exists());
        assert!(
            fs::read_dir(&delivery)
                .expect("delivery entries")
                .filter_map(|entry| entry.ok())
                .any(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == std::ffi::OsStr::new("cs"))
                })
        );
    }
}
