use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use masterdata_core::{
    AddedRecordDraft, AddedRecordField, Diagnostic, ErrorKind, MasterdataError, PrimitiveType,
    Project, ProjectDocuments, ProjectInfo, RecordValueEdit, SchemaDocument, SourceDocument,
    SourceRecordMutation, ValidationReport, dry_run_source_record_mutation, parse_yaml_document,
    source_content_identity, validate_documents,
};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::NativeApplicationService;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringCapabilities {
    pub workspace_read: bool,
    pub workspace_write: bool,
    pub validate: bool,
    pub build: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSourceFile {
    pub path: String,
    pub source_root: String,
    pub kind: String,
    pub table: Option<String>,
    pub type_name: Option<String>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringWorkspace {
    pub project: ProjectInfo,
    pub source_roots: Vec<String>,
    pub files: Vec<WorkspaceSourceFile>,
    pub folders: Vec<WorkspaceFolder>,
    pub capabilities: AuthoringCapabilities,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolder {
    pub path: String,
    pub source_root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorColumn {
    pub name: String,
    pub type_name: String,
    pub editable: bool,
    pub key_field: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorCell {
    pub field: String,
    pub text: String,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorRow {
    pub record_index: usize,
    pub cells: Vec<DataEditorCell>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorAddCapability {
    pub supported: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFileSnapshot {
    pub path: String,
    pub table: String,
    pub base_source: String,
    pub base_content_identity: String,
    pub columns: Vec<DataEditorColumn>,
    pub rows: Vec<DataEditorRow>,
    pub add_row: DataEditorAddCapability,
    pub validation: ValidationReport,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringEdit {
    pub record_index: usize,
    pub field: String,
    pub value: String,
}

impl From<&AuthoringEdit> for RecordValueEdit {
    fn from(value: &AuthoringEdit) -> Self {
        Self {
            record_index: value.record_index,
            field: value.field.clone(),
            value: value.value.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringRecordField {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringRecordDraft {
    pub fields: Vec<AuthoringRecordField>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringRecordMutation {
    #[serde(default)]
    pub edits: Vec<AuthoringEdit>,
    #[serde(default)]
    pub added_records: Vec<AuthoringRecordDraft>,
    #[serde(default)]
    pub deleted_record_indices: Vec<usize>,
}

impl From<&AuthoringRecordMutation> for SourceRecordMutation {
    fn from(value: &AuthoringRecordMutation) -> Self {
        Self {
            edits: value.edits.iter().map(RecordValueEdit::from).collect(),
            additions: value
                .added_records
                .iter()
                .map(|draft| AddedRecordDraft {
                    fields: draft
                        .fields
                        .iter()
                        .map(|field| AddedRecordField {
                            field: field.field.clone(),
                            value: field.value.clone(),
                        })
                        .collect(),
                })
                .collect(),
            deletions: value.deleted_record_indices.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEditPreview {
    pub candidate_source: String,
    pub candidate_content_identity: String,
    pub changed: bool,
    pub validation: ValidationReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceContentState {
    pub path: String,
    pub content_identity: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceSaveStatus {
    Success,
    Conflict,
    Failure,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSaveReport {
    pub status: SourceSaveStatus,
    pub path: String,
    pub snapshot: Option<DataFileSnapshot>,
    pub current: Option<SourceContentState>,
    pub diagnostic: Option<Diagnostic>,
}

impl NativeApplicationService {
    pub fn authoring_workspace(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> masterdata_core::Result<AuthoringWorkspace> {
        let project = Project::discover(explicit_project, current_dir)?;
        let info = project.info();
        let files = project.source_files()?;
        let source_roots = info
            .source_roots
            .iter()
            .map(|root| project_relative_string(project.root(), root))
            .collect::<Vec<_>>();
        let mut entries = Vec::with_capacity(files.len());
        for path in files {
            let relative = project_relative_string(project.root(), &path);
            let source_root = info
                .source_roots
                .iter()
                .find(|root| path.starts_with(root))
                .map(|root| project_relative_string(project.root(), root))
                .unwrap_or_else(|| ".".to_owned());
            match fs::read_to_string(&path) {
                Ok(source) => match parse_yaml_document(path.clone(), &source) {
                    Ok(loaded) => entries.push(WorkspaceSourceFile {
                        path: relative,
                        source_root,
                        kind: loaded.document.kind().to_owned(),
                        table: loaded.document.table_identity().map(str::to_owned),
                        type_name: loaded.document.type_name().map(str::to_owned),
                        diagnostic: None,
                    }),
                    Err(error) => entries.push(WorkspaceSourceFile {
                        path: relative,
                        source_root,
                        kind: "invalid".to_owned(),
                        table: None,
                        type_name: None,
                        diagnostic: Some(error.diagnostic().clone()),
                    }),
                },
                Err(error) => entries.push(WorkspaceSourceFile {
                    path: relative,
                    source_root,
                    kind: "unavailable".to_owned(),
                    table: None,
                    type_name: None,
                    diagnostic: Some(
                        MasterdataError::new(
                            "E-IO-ACCESS",
                            ErrorKind::Io,
                            format!("could not read source file: {error}"),
                        )
                        .with_source(path.clone())
                        .diagnostic()
                        .clone(),
                    ),
                }),
            }
        }
        let mut folders = Vec::new();
        for root in &info.source_roots {
            // Folder enumeration must preserve the existing read/discovery policy.
            // Creation applies its stricter no-symlink mutation gate separately.
            if !root.is_dir() {
                continue;
            }
            let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())
                .map_err(|error| io_authoring_error(root, error.to_string()))?;
            let mut paths = Vec::new();
            crate::creation::collect_folders(&dir, "", &mut paths)
                .map_err(|error| io_authoring_error(root, error.to_string()))?;
            for path in paths {
                folders.push(WorkspaceFolder {
                    path: project_relative_string(project.root(), &root.join(path)),
                    source_root: project_relative_string(project.root(), root),
                });
            }
        }
        folders.sort_by(|a, b| a.path.cmp(&b.path));
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(AuthoringWorkspace {
            project: info,
            source_roots,
            files: entries,
            folders,
            capabilities: AuthoringCapabilities {
                workspace_read: true,
                workspace_write: true,
                validate: true,
                build: true,
            },
        })
    }

    pub fn open_data_file(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
    ) -> masterdata_core::Result<DataFileSnapshot> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        let (documents, parse_diagnostics) = load_authoring_documents(&project, None)?;
        data_file_snapshot(&project, &documents, parse_diagnostics, &target)
    }

    pub fn preview_data_file(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        edits: &[AuthoringEdit],
    ) -> masterdata_core::Result<SourceEditPreview> {
        self.preview_data_file_mutation(
            explicit_project,
            current_dir,
            relative_path,
            base_source,
            &AuthoringRecordMutation {
                edits: edits.to_vec(),
                ..AuthoringRecordMutation::default()
            },
        )
    }

    pub fn preview_data_file_mutation(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        mutation: &AuthoringRecordMutation,
    ) -> masterdata_core::Result<SourceEditPreview> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        let (documents, mut parse_diagnostics) =
            load_authoring_documents(&project, Some((&target, base_source)))?;
        let mutation = SourceRecordMutation::from(mutation);
        let dry_run = dry_run_source_record_mutation(&documents, &target, &mutation)?;
        let mut validation = validate_documents(&dry_run.transformed_documents);
        merge_parse_diagnostics(&mut validation, &mut parse_diagnostics);
        Ok(SourceEditPreview {
            candidate_source: dry_run.plan.candidate_source,
            candidate_content_identity: dry_run.plan.candidate_content_identity,
            changed: dry_run.plan.changed,
            validation,
        })
    }

    pub fn source_content(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
    ) -> masterdata_core::Result<SourceContentState> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        read_source_state(&project, &target)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_data_file(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        base_content_identity: &str,
        edits: &[AuthoringEdit],
        overwrite_expected_identity: Option<&str>,
    ) -> masterdata_core::Result<SourceSaveReport> {
        self.save_data_file_mutation(
            explicit_project,
            current_dir,
            relative_path,
            base_source,
            base_content_identity,
            &AuthoringRecordMutation {
                edits: edits.to_vec(),
                ..AuthoringRecordMutation::default()
            },
            overwrite_expected_identity,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_data_file_mutation(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        base_content_identity: &str,
        mutation: &AuthoringRecordMutation,
        overwrite_expected_identity: Option<&str>,
    ) -> masterdata_core::Result<SourceSaveReport> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        if source_content_identity(base_source) != base_content_identity {
            return Err(authoring_error(
                "E-SOURCE-EDIT-BASE-IDENTITY",
                "provided base source does not match its content identity",
                Some(target),
                "SOURCE-EDIT-001",
            ));
        }

        let (documents, _) = load_authoring_documents(&project, Some((&target, base_source)))?;
        let mutation = SourceRecordMutation::from(mutation);
        let dry_run = dry_run_source_record_mutation(&documents, &target, &mutation)?;
        let current = read_source_state(&project, &target)?;
        let expected = overwrite_expected_identity.unwrap_or(base_content_identity);
        if current.content_identity != expected {
            return Ok(SourceSaveReport {
                status: SourceSaveStatus::Conflict,
                path: relative_path.to_owned(),
                snapshot: None,
                current: Some(current),
                diagnostic: Some(
                    authoring_error(
                        "E-SOURCE-EDIT-CONFLICT",
                        "source file changed after the editor base snapshot",
                        Some(target),
                        "SOURCE-EDIT-008",
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }

        if current.source == dry_run.plan.candidate_source {
            let snapshot =
                self.open_data_file(Some(project.root()), project.root(), relative_path)?;
            return Ok(SourceSaveReport {
                status: SourceSaveStatus::Success,
                path: relative_path.to_owned(),
                snapshot: Some(snapshot),
                current: None,
                diagnostic: None,
            });
        }

        if let Err(error) = install_source_candidate(
            &target,
            current.source.as_bytes(),
            dry_run.plan.candidate_source.as_bytes(),
        ) {
            let after = read_source_state(&project, &target).ok();
            if error.diagnostic().code == "E-SOURCE-EDIT-CONFLICT" {
                return Ok(SourceSaveReport {
                    status: SourceSaveStatus::Conflict,
                    path: relative_path.to_owned(),
                    snapshot: None,
                    current: after,
                    diagnostic: Some(error.diagnostic().clone()),
                });
            }
            if after.as_ref().is_some_and(|state| {
                state.content_identity == dry_run.plan.candidate_content_identity
            }) {
                let snapshot =
                    self.open_data_file(Some(project.root()), project.root(), relative_path)?;
                return Ok(SourceSaveReport {
                    status: SourceSaveStatus::Success,
                    path: relative_path.to_owned(),
                    snapshot: Some(snapshot),
                    current: None,
                    diagnostic: None,
                });
            }
            let status = if after
                .as_ref()
                .is_some_and(|state| state.content_identity == current.content_identity)
            {
                SourceSaveStatus::Failure
            } else {
                SourceSaveStatus::OutcomeUnknown
            };
            return Ok(SourceSaveReport {
                status,
                path: relative_path.to_owned(),
                snapshot: None,
                current: after,
                diagnostic: Some(error.diagnostic().clone()),
            });
        }

        let after = match read_source_state(&project, &target) {
            Ok(after) => after,
            Err(error) => {
                return Ok(SourceSaveReport {
                    status: SourceSaveStatus::OutcomeUnknown,
                    path: relative_path.to_owned(),
                    snapshot: None,
                    current: None,
                    diagnostic: Some(error.diagnostic().clone()),
                });
            }
        };
        if after.content_identity != dry_run.plan.candidate_content_identity {
            return Ok(SourceSaveReport {
                status: SourceSaveStatus::OutcomeUnknown,
                path: relative_path.to_owned(),
                snapshot: None,
                current: Some(after),
                diagnostic: Some(
                    authoring_error(
                        "E-SOURCE-EDIT-WRITE-VERIFY",
                        "saved source could not be verified as the complete candidate content",
                        Some(target),
                        "SOURCE-EDIT-010",
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }

        let snapshot = self.open_data_file(Some(project.root()), project.root(), relative_path)?;
        Ok(SourceSaveReport {
            status: SourceSaveStatus::Success,
            path: relative_path.to_owned(),
            snapshot: Some(snapshot),
            current: None,
            diagnostic: None,
        })
    }
}

fn data_file_snapshot(
    project: &Project,
    documents: &ProjectDocuments,
    mut parse_diagnostics: Vec<Diagnostic>,
    target: &Path,
) -> masterdata_core::Result<DataFileSnapshot> {
    let loaded = documents
        .files
        .iter()
        .find(|loaded| loaded.path == target)
        .ok_or_else(|| {
            authoring_error(
                "E-GUI-DATA-FILE-UNAVAILABLE",
                "selected data file is not a parseable Masterdata document",
                Some(target.to_path_buf()),
                "GUI-DATA-LAYOUT-001",
            )
        })?;
    let SourceDocument::Data(data) = &loaded.document else {
        return Err(authoring_error(
            "E-GUI-DATA-FILE-KIND",
            "selected source is not a Data document",
            Some(target.to_path_buf()),
            "GUI-EXPLORER-003",
        ));
    };
    let schema = unique_schema(documents, &data.table, target)?;
    let key_fields = schema_key_fields(schema);
    let columns = schema
        .fields
        .iter()
        .map(|field| {
            let key_field = key_fields.contains(field.name.as_str());
            DataEditorColumn {
                name: field.name.clone(),
                type_name: field.type_name.clone(),
                editable: !key_field
                    && !field.nullable
                    && !field.array
                    && PrimitiveType::parse(&field.type_name).is_some(),
                key_field,
            }
        })
        .collect::<Vec<_>>();
    let rows = data
        .records
        .iter()
        .enumerate()
        .map(|(record_index, record)| DataEditorRow {
            record_index,
            cells: columns
                .iter()
                .map(|column| DataEditorCell {
                    field: column.name.clone(),
                    text: record
                        .get(&column.name)
                        .map(display_value)
                        .unwrap_or_default(),
                    editable: column.editable,
                })
                .collect(),
        })
        .collect();
    let add_row = data_editor_add_capability(schema);
    let mut validation = validate_documents(documents);
    merge_parse_diagnostics(&mut validation, &mut parse_diagnostics);
    Ok(DataFileSnapshot {
        path: project_relative_string(project.root(), target),
        table: data.table.clone(),
        base_source: loaded.source.clone(),
        base_content_identity: source_content_identity(&loaded.source),
        columns,
        rows,
        add_row,
        validation,
    })
}

fn data_editor_add_capability(schema: &SchemaDocument) -> DataEditorAddCapability {
    if schema.fields.is_empty() {
        return DataEditorAddCapability {
            supported: false,
            reason: Some(
                "Add Row requires a Table with at least one Required Primitive field.".to_owned(),
            ),
        };
    }
    let unsupported = schema.fields.iter().find(|field| {
        field.nullable || field.array || PrimitiveType::parse(&field.type_name).is_none()
    });
    match unsupported {
        Some(field) => DataEditorAddCapability {
            supported: false,
            reason: Some(format!(
                "Add Row currently supports Required Primitive fields only; `{}` is outside this scope.",
                field.name
            )),
        },
        None => DataEditorAddCapability {
            supported: true,
            reason: None,
        },
    }
}

pub(super) fn load_authoring_documents(
    project: &Project,
    override_source: Option<(&Path, &str)>,
) -> masterdata_core::Result<(ProjectDocuments, Vec<Diagnostic>)> {
    let mut documents = ProjectDocuments::default();
    let mut diagnostics = Vec::new();
    for path in project.source_files()? {
        let source = if let Some((target, source)) = override_source {
            if target == path {
                source.to_owned()
            } else {
                match fs::read_to_string(&path) {
                    Ok(source) => source,
                    Err(error) => {
                        diagnostics.push(
                            io_authoring_error(
                                &path,
                                format!("could not read source file: {error}"),
                            )
                            .diagnostic()
                            .clone(),
                        );
                        continue;
                    }
                }
            }
        } else {
            match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    diagnostics.push(
                        io_authoring_error(&path, format!("could not read source file: {error}"))
                            .diagnostic()
                            .clone(),
                    );
                    continue;
                }
            }
        };
        match parse_yaml_document(path.clone(), &source) {
            Ok(loaded) => documents.files.push(loaded),
            Err(error) => diagnostics.push(error.diagnostic().clone()),
        }
    }
    Ok((documents, diagnostics))
}

fn merge_parse_diagnostics(report: &mut ValidationReport, diagnostics: &mut Vec<Diagnostic>) {
    if !diagnostics.is_empty() {
        report.valid = false;
        report.diagnostics.append(diagnostics);
    }
}

fn resolve_source_file(project: &Project, relative_path: &str) -> masterdata_core::Result<PathBuf> {
    let requested = normalized_logical_path(relative_path).ok_or_else(|| {
        authoring_error(
            "E-GUI-SOURCE-PATH-UNSAFE",
            "source path must be a project-relative path without parent traversal",
            Some(project.root().to_path_buf()),
            "SOURCE-EDIT-008",
        )
    })?;
    project
        .source_files()?
        .into_iter()
        .find(|path| project_relative_string(project.root(), path) == requested)
        .ok_or_else(|| {
            authoring_error(
                "E-GUI-SOURCE-NOT-FOUND",
                format!("source file `{relative_path}` is not in a configured source root"),
                Some(project.root().join(relative_path)),
                "GUI-EXPLORER-001",
            )
        })
}

fn normalized_logical_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    if path.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

pub(super) fn project_relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str().map(str::to_owned),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn unique_schema<'a>(
    documents: &'a ProjectDocuments,
    table: &str,
    target: &Path,
) -> masterdata_core::Result<&'a SchemaDocument> {
    let schemas = documents
        .files
        .iter()
        .filter_map(|loaded| match &loaded.document {
            SourceDocument::Schema(schema) if schema.table == table => Some(schema),
            _ => None,
        })
        .collect::<Vec<_>>();
    match schemas.as_slice() {
        [schema] => Ok(*schema),
        [] => Err(authoring_error(
            "E-GUI-DATA-SCHEMA-NOT-FOUND",
            format!("table `{table}` has no parseable schema document"),
            Some(target.to_path_buf()),
            "GUI-DATA-LAYOUT-001",
        )),
        _ => Err(authoring_error(
            "E-GUI-DATA-SCHEMA-AMBIGUOUS",
            format!("table `{table}` has multiple schema documents"),
            Some(target.to_path_buf()),
            "GUI-DATA-LAYOUT-001",
        )),
    }
}

fn schema_key_fields(schema: &SchemaDocument) -> BTreeSet<&str> {
    let mut result = BTreeSet::new();
    if let Some(primary) = &schema.primary_key {
        result.extend(primary.fields.iter().map(String::as_str));
    }
    for secondary in &schema.secondary_keys {
        result.extend(secondary.fields.iter().map(String::as_str));
    }
    result
}

fn display_value(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null => "null".to_owned(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::String(value) => value.clone(),
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => serde_yaml::to_string(value)
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|_| "<unsupported>".to_owned()),
    }
}

fn read_source_state(
    project: &Project,
    target: &Path,
) -> masterdata_core::Result<SourceContentState> {
    let metadata = fs::symlink_metadata(target).map_err(|error| {
        io_authoring_error(target, format!("could not inspect source file: {error}"))
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(authoring_error(
            "E-SOURCE-EDIT-PATH-UNSAFE",
            "source edit target is no longer the same regular file",
            Some(target.to_path_buf()),
            "SOURCE-EDIT-008",
        ));
    }
    let source = fs::read_to_string(target).map_err(|error| {
        io_authoring_error(target, format!("could not read source file: {error}"))
    })?;
    Ok(SourceContentState {
        path: project_relative_string(project.root(), target),
        content_identity: source_content_identity(&source),
        source,
    })
}

fn install_source_candidate(
    target: &Path,
    expected_current: &[u8],
    candidate: &[u8],
) -> masterdata_core::Result<()> {
    install_source_candidate_with_pre_replace_hook(target, expected_current, candidate, |_| {})
}

fn install_source_candidate_with_pre_replace_hook<F>(
    target: &Path,
    expected_current: &[u8],
    candidate: &[u8],
    before_replace: F,
) -> masterdata_core::Result<()>
where
    F: FnOnce(&Path),
{
    let parent = target.parent().ok_or_else(|| {
        authoring_error(
            "E-SOURCE-EDIT-PATH-UNSAFE",
            "source edit target has no parent directory",
            Some(target.to_path_buf()),
            "SOURCE-EDIT-012",
        )
    })?;
    let metadata = fs::symlink_metadata(target).map_err(|error| {
        io_authoring_error(target, format!("could not inspect source file: {error}"))
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(authoring_error(
            "E-SOURCE-EDIT-PATH-UNSAFE",
            "source edit target is no longer a regular non-symlink file",
            Some(target.to_path_buf()),
            "SOURCE-EDIT-008",
        ));
    }

    // WHY: A complete candidate is staged before the source is replaced, and
    // the bytes actually moved to backup are rechecked before installation.
    // IF REMOVED: an external edit racing preflight can become a lost update.
    // EVIDENCE: docs/specs/source-edit.md SOURCE-EDIT-008, SOURCE-EDIT-010, SOURCE-EDIT-012.
    let transaction = TempDir::new_in(parent).map_err(|error| {
        io_authoring_error(
            target,
            format!("could not create source edit staging directory: {error}"),
        )
    })?;
    let staged = transaction.path().join("candidate");
    let backup = transaction.path().join("base");
    {
        let mut file = fs::File::create(&staged).map_err(|error| {
            io_authoring_error(
                target,
                format!("could not stage source edit candidate: {error}"),
            )
        })?;
        file.write_all(candidate).map_err(|error| {
            io_authoring_error(
                target,
                format!("could not write source edit candidate: {error}"),
            )
        })?;
        file.sync_all().map_err(|error| {
            io_authoring_error(
                target,
                format!("could not sync source edit candidate: {error}"),
            )
        })?;
    }
    fs::set_permissions(&staged, metadata.permissions()).map_err(|error| {
        io_authoring_error(
            target,
            format!("could not preserve source file permissions: {error}"),
        )
    })?;
    let current = fs::read(target).map_err(|error| {
        io_authoring_error(
            target,
            format!("could not recheck source file before save: {error}"),
        )
    })?;
    if current != expected_current {
        return Err(source_conflict(
            target,
            "source file changed during save preflight",
        ));
    }

    before_replace(target);

    fs::rename(target, &backup).map_err(|error| {
        io_authoring_error(
            target,
            format!("could not stage existing source for replacement: {error}"),
        )
    })?;
    let actual_base = match fs::read(&backup) {
        Ok(actual_base) => actual_base,
        Err(error) => {
            rollback_source_backup(
                target,
                &backup,
                format!("could not verify staged source before replacement: {error}"),
            )?;
            return Err(io_authoring_error(
                target,
                format!("could not verify staged source before replacement: {error}"),
            ));
        }
    };
    if actual_base != expected_current {
        rollback_source_backup(
            target,
            &backup,
            "source file changed during the save replacement window",
        )?;
        return Err(source_conflict(
            target,
            "source file changed during the save replacement window",
        ));
    }

    if let Err(error) = fs::rename(&staged, target) {
        rollback_source_backup(
            target,
            &backup,
            format!("could not install source edit candidate: {error}"),
        )?;
        return Err(io_authoring_error(
            target,
            format!("could not install source edit candidate: {error}"),
        ));
    }
    Ok(())
}

fn rollback_source_backup(
    target: &Path,
    backup: &Path,
    context: impl Into<String>,
) -> masterdata_core::Result<()> {
    let context = context.into();
    if target.exists() {
        return Err(authoring_error(
            "E-SOURCE-EDIT-OUTCOME-UNKNOWN",
            format!(
                "{context}; source path was recreated before rollback could restore staged content"
            ),
            Some(target.to_path_buf()),
            "SOURCE-EDIT-010",
        ));
    }
    fs::rename(backup, target).map_err(|rollback_error| {
        authoring_error(
            "E-SOURCE-EDIT-OUTCOME-UNKNOWN",
            format!("{context}; rollback also failed ({rollback_error})"),
            Some(target.to_path_buf()),
            "SOURCE-EDIT-010",
        )
    })
}

fn source_conflict(target: &Path, message: impl Into<String>) -> MasterdataError {
    authoring_error(
        "E-SOURCE-EDIT-CONFLICT",
        message,
        Some(target.to_path_buf()),
        "SOURCE-EDIT-008",
    )
}

fn io_authoring_error(path: &Path, message: impl Into<String>) -> MasterdataError {
    let mut error = MasterdataError::new("E-IO-ACCESS", ErrorKind::Io, message);
    error.diagnostic.source = Some(path.to_path_buf());
    error
        .diagnostic
        .related_requirements
        .push("SOURCE-EDIT-010".to_owned());
    error
}

fn authoring_error(
    code: &str,
    message: impl Into<String>,
    source: Option<PathBuf>,
    requirement: &str,
) -> MasterdataError {
    let mut error = MasterdataError::new(code, ErrorKind::Validation, message);
    if let Some(source) = source {
        error.diagnostic.source = Some(source);
    }
    error
        .diagnostic
        .related_requirements
        .push(requirement.to_owned());
    error
}

#[cfg(test)]
mod tests {
    use super::{
        AuthoringEdit, AuthoringRecordDraft, AuthoringRecordField, AuthoringRecordMutation,
        SourceSaveStatus, install_source_candidate_with_pre_replace_hook,
    };
    use crate::NativeApplicationService;
    use std::fs;
    use tempfile::TempDir;

    fn project() -> TempDir {
        let temp = tempfile::tempdir().expect("temp project");
        fs::create_dir_all(temp.path().join("sources/schemas")).expect("schemas");
        fs::create_dir_all(temp.path().join("sources/data")).expect("data");
        fs::create_dir_all(temp.path().join("sources/types")).expect("types");
        fs::write(
            temp.path().join("masterdata.toml"),
            r#"[project]
id = "authoring.test"
name = "Authoring"
version = "0.1.0"

[sources]
roots = ["sources"]

[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"
"#,
        )
        .expect("config");
        fs::write(
            temp.path().join("sources/schemas/item.yaml"),
            r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: weight
    type: ulong
  - key: 2
    name: note
    type: string
primaryKey:
  fields: [id]
secondaryKeys: []
"#,
        )
        .expect("schema");
        fs::write(temp.path().join("sources/data/items.yaml"), "kind: data\ntable: item\nrecords:\n  - id: 18446744073709551615\n    weight: 10 # keep\n    note: 'hello'\n").expect("data");
        temp
    }

    #[test]
    fn workspace_and_snapshot_keep_provenance_and_editability() {
        let temp = project();
        let service = NativeApplicationService::new();
        let workspace = service
            .authoring_workspace(Some(temp.path()), temp.path())
            .expect("workspace");
        assert_eq!(workspace.source_roots, vec!["sources"]);
        assert!(
            workspace
                .files
                .iter()
                .any(|file| file.path == "sources/data/items.yaml" && file.kind == "data")
        );
        let snapshot = service
            .open_data_file(Some(temp.path()), temp.path(), "sources/data/items.yaml")
            .expect("snapshot");
        assert_eq!(snapshot.rows[0].cells[0].text, "18446744073709551615");
        assert!(!snapshot.columns[0].editable);
        assert!(snapshot.columns[1].editable);
        assert!(snapshot.columns[2].editable);
    }

    #[test]
    fn save_preserves_unrelated_source_and_detects_conflict() {
        let temp = project();
        let service = NativeApplicationService::new();
        let snapshot = service
            .open_data_file(Some(temp.path()), temp.path(), "sources/data/items.yaml")
            .expect("snapshot");
        let report = service
            .save_data_file(
                Some(temp.path()),
                temp.path(),
                "sources/data/items.yaml",
                &snapshot.base_source,
                &snapshot.base_content_identity,
                &[AuthoringEdit {
                    record_index: 0,
                    field: "weight".to_owned(),
                    value: "20".to_owned(),
                }],
                None,
            )
            .expect("save");
        assert_eq!(report.status, SourceSaveStatus::Success);
        let saved = fs::read_to_string(temp.path().join("sources/data/items.yaml")).expect("saved");
        assert!(saved.contains("weight: 20 # keep"));
        assert!(saved.contains("note: 'hello'"));
        let old = report.snapshot.expect("success snapshot");
        fs::write(
            temp.path().join("sources/data/items.yaml"),
            saved.replace("note: 'hello'", "note: external"),
        )
        .expect("external edit");
        let conflict = service
            .save_data_file(
                Some(temp.path()),
                temp.path(),
                "sources/data/items.yaml",
                &old.base_source,
                &old.base_content_identity,
                &[AuthoringEdit {
                    record_index: 0,
                    field: "weight".to_owned(),
                    value: "30".to_owned(),
                }],
                None,
            )
            .expect("conflict result");
        assert_eq!(conflict.status, SourceSaveStatus::Conflict);
        assert!(
            conflict
                .current
                .expect("current source")
                .source
                .contains("note: external")
        );
    }

    #[test]
    fn save_rechecks_actual_backup_bytes_before_installing_candidate() {
        let temp = tempfile::tempdir().expect("temp dir");
        let target = temp.path().join("data.yaml");
        fs::write(&target, b"base").expect("base source");

        let error = install_source_candidate_with_pre_replace_hook(
            &target,
            b"base",
            b"candidate",
            |path| fs::write(path, b"external").expect("racing external edit"),
        )
        .expect_err("racing external edit must become a conflict");

        assert_eq!(error.diagnostic().code, "E-SOURCE-EDIT-CONFLICT");
        assert_eq!(
            fs::read(&target).expect("restored external source"),
            b"external"
        );
    }

    #[test]
    fn preview_keeps_validation_non_blocking() {
        let temp = project();
        let service = NativeApplicationService::new();
        let snapshot = service
            .open_data_file(Some(temp.path()), temp.path(), "sources/data/items.yaml")
            .expect("snapshot");
        let preview = service
            .preview_data_file(
                Some(temp.path()),
                temp.path(),
                "sources/data/items.yaml",
                &snapshot.base_source,
                &[AuthoringEdit {
                    record_index: 0,
                    field: "weight".to_owned(),
                    value: "invalid".to_owned(),
                }],
            )
            .expect("preview");
        assert!(preview.changed);
        assert!(!preview.validation.valid);
    }

    #[test]
    fn record_mutation_preview_and_save_share_the_source_edit_lifecycle() {
        let temp = project();
        let service = NativeApplicationService::new();
        let snapshot = service
            .open_data_file(Some(temp.path()), temp.path(), "sources/data/items.yaml")
            .expect("snapshot");
        assert!(snapshot.add_row.supported);

        let mutation = AuthoringRecordMutation {
            edits: vec![AuthoringEdit {
                record_index: 0,
                field: "weight".to_owned(),
                value: "11".to_owned(),
            }],
            added_records: vec![AuthoringRecordDraft {
                fields: vec![
                    AuthoringRecordField {
                        field: "note".to_owned(),
                        value: "new".to_owned(),
                    },
                    AuthoringRecordField {
                        field: "id".to_owned(),
                        value: "18446744073709551614".to_owned(),
                    },
                    AuthoringRecordField {
                        field: "weight".to_owned(),
                        value: "12".to_owned(),
                    },
                ],
            }],
            deleted_record_indices: Vec::new(),
        };
        let preview = service
            .preview_data_file_mutation(
                Some(temp.path()),
                temp.path(),
                "sources/data/items.yaml",
                &snapshot.base_source,
                &mutation,
            )
            .expect("mutation preview");
        assert!(preview.changed);
        assert!(preview.candidate_source.contains("weight: 11 # keep"));
        assert!(
            preview
                .candidate_source
                .contains("id: 18446744073709551614")
        );

        let report = service
            .save_data_file_mutation(
                Some(temp.path()),
                temp.path(),
                "sources/data/items.yaml",
                &snapshot.base_source,
                &snapshot.base_content_identity,
                &mutation,
                None,
            )
            .expect("mutation save");
        assert_eq!(report.status, SourceSaveStatus::Success);
        let saved = report.snapshot.expect("saved snapshot");
        assert_eq!(saved.rows.len(), 2);
        assert!(
            !saved.columns[0].editable,
            "saved added key becomes existing read-only"
        );
    }

    #[test]
    fn record_mutation_save_deletes_selected_occurrence_and_keeps_duplicate_key_record() {
        let temp = project();
        fs::write(
            temp.path().join("sources/data/items.yaml"),
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n  - id: 1\n    weight: 20\n    note: second\n",
        )
        .expect("duplicate-key source");
        let service = NativeApplicationService::new();
        let snapshot = service
            .open_data_file(Some(temp.path()), temp.path(), "sources/data/items.yaml")
            .expect("snapshot");
        let report = service
            .save_data_file_mutation(
                Some(temp.path()),
                temp.path(),
                "sources/data/items.yaml",
                &snapshot.base_source,
                &snapshot.base_content_identity,
                &AuthoringRecordMutation {
                    deleted_record_indices: vec![1],
                    ..AuthoringRecordMutation::default()
                },
                None,
            )
            .expect("delete save");
        assert_eq!(report.status, SourceSaveStatus::Success);
        let source =
            fs::read_to_string(temp.path().join("sources/data/items.yaml")).expect("source");
        assert!(source.contains("note: first"));
        assert!(!source.contains("note: second"));
    }

    #[test]
    fn snapshot_reports_add_row_scope_without_disabling_existing_data_access() {
        let temp = project();
        let schema =
            fs::read_to_string(temp.path().join("sources/schemas/item.yaml")).expect("schema");
        fs::write(
            temp.path().join("sources/schemas/item.yaml"),
            schema.replace(
                "    type: ulong\n  - key: 2",
                "    type: ulong\n    nullable: true\n  - key: 2",
            ),
        )
        .expect("complex schema");
        let snapshot = NativeApplicationService::new()
            .open_data_file(Some(temp.path()), temp.path(), "sources/data/items.yaml")
            .expect("snapshot");

        assert!(!snapshot.add_row.supported);
        assert!(snapshot.add_row.reason.expect("reason").contains("weight"));
        assert_eq!(snapshot.rows.len(), 1);
    }
}
