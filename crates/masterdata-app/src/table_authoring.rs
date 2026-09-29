//! Session-owned plans bind Apply to reviewed bytes, never a frontend-supplied patch.
use crate::authoring::{
    AuthoringRecordMutation, SourceContentState, SourceSaveStatus, install_source_candidate,
    load_authoring_documents_with_overrides, project_relative_string, read_source_state,
    resolve_source_file,
};
use masterdata_core::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum TableOperationInput {
    AddDefault {
        table: String,
        #[serde(rename = "beforeField")]
        before_field: Option<String>,
    },
    Add {
        table: String,
        field: FieldDefinition,
        initializer: Option<String>,
    },
    Rename {
        table: String,
        field: String,
        #[serde(rename = "newName")]
        new_name: String,
    },
    Drop {
        table: String,
        field: String,
    },
    ChangeDeclaration {
        table: String,
        field: String,
        #[serde(rename = "type")]
        type_name: String,
        nullable: bool,
        array: bool,
    },
    AddReference {
        table: String,
        reference: ReferenceDefinition,
    },
    EditReference {
        table: String,
        name: String,
        reference: ReferenceDefinition,
    },
    RemoveReference {
        table: String,
        name: String,
    },
}
impl TableOperationInput {
    fn command(self, documents: &ProjectDocuments) -> Result<MigrationCommand> {
        Ok(match self {
            Self::AddDefault {
                table,
                before_field,
            } => {
                let schema = documents
                    .schemas()
                    .find(|(_, schema)| schema.table == table)
                    .map(|(_, schema)| schema)
                    .ok_or_else(|| error("E-TABLE-EDITOR-SOURCE", "Table schema not found"))?;
                let mut key = 0u32;
                for used in schema
                    .fields
                    .iter()
                    .map(|field| field.key)
                    .collect::<BTreeSet<_>>()
                {
                    if used != key {
                        break;
                    }
                    key = key.checked_add(1).ok_or_else(|| {
                        error("E-TABLE-KEY-EXHAUSTED", "No MessagePack key remains")
                    })?;
                }
                let mut serial = schema.fields.len() + 1;
                let name = loop {
                    let name = format!("field{serial}");
                    if !schema.fields.iter().any(|field| field.name == name) {
                        break name;
                    }
                    serial += 1;
                };
                let position = before_field
                    .map(|before| {
                        schema
                            .fields
                            .iter()
                            .position(|field| field.name == before)
                            .ok_or_else(|| {
                                error(
                                    "E-TABLE-FIELD-POSITION",
                                    "Insert target column is unavailable",
                                )
                            })
                    })
                    .transpose()?;
                MigrationCommand::AddField(AddFieldCommand {
                    table,
                    field: FieldDefinition {
                        key,
                        name,
                        type_name: "string".into(),
                        nullable: true,
                        array: false,
                    },
                    initializer: Some(serde_yaml::Value::Null),
                    position,
                })
            }
            Self::Add {
                table,
                field,
                initializer,
            } => MigrationCommand::AddField(AddFieldCommand {
                table,
                field,
                initializer: initializer
                    .map(|text| crate::type_authoring::parse_constant(&text))
                    .transpose()?,
                position: None,
            }),
            Self::Rename {
                table,
                field,
                new_name,
            } => MigrationCommand::RenameField(RenameFieldCommand {
                table,
                field,
                new_name,
            }),
            Self::Drop { table, field } => {
                MigrationCommand::DropField(DropFieldCommand { table, field })
            }
            Self::ChangeDeclaration {
                table,
                field,
                type_name,
                nullable,
                array,
            } => {
                let old = documents
                    .schemas()
                    .find(|(_, schema)| schema.table == table)
                    .and_then(|(_, schema)| {
                        schema
                            .fields
                            .iter()
                            .find(|candidate| candidate.name == field)
                    })
                    .ok_or_else(|| error("E-FIELD-DECL-FIELD", "Table field not found"))?;
                let mut declaration = old.clone();
                declaration.type_name = type_name;
                declaration.nullable = nullable;
                declaration.array = array;
                MigrationCommand::ChangeFieldDeclaration(ChangeFieldDeclarationCommand {
                    table,
                    field,
                    declaration,
                })
            }
            Self::AddReference { table, reference } => {
                MigrationCommand::AddReference(AddReferenceCommand { table, reference })
            }
            Self::EditReference {
                table,
                name,
                reference,
            } => MigrationCommand::EditReference(EditReferenceCommand {
                table,
                name,
                reference,
            }),
            Self::RemoveReference { table, name } => {
                MigrationCommand::RemoveReference(RemoveReferenceCommand { table, name })
            }
        })
    }
}
pub use masterdata_core::TableSnapshot;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableContext {
    pub table: String,
    pub schema_path: String,
    pub schema_content_identity: String,
    pub schema_source: String,
    pub record_sources: Vec<TableRecordSource>,
    pub selected_record_source: Option<String>,
    pub schema: TableSnapshot,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDraftField {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub nullable: bool,
    pub array: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDraftRecordSource {
    pub path: String,
    pub candidate_source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDraftPreview {
    pub candidate_source: String,
    pub candidate_content_identity: String,
    pub changed: bool,
    pub validation: ValidationReport,
    pub selected_snapshot: Option<DataFileSnapshot>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDraftSaveReport {
    pub status: SourceSaveStatus,
    pub path: String,
    pub candidate_content_identity: String,
    pub current: Option<SourceContentState>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSchemaSaveDraft {
    pub base_source: String,
    pub base_content_identity: String,
    pub fields: Vec<SchemaDraftField>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRecordSaveDraft {
    pub base_source: String,
    pub base_content_identity: String,
    pub mutation: AuthoringRecordMutation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableContextSaveRequest {
    pub schema_path: String,
    pub selected_record_source: Option<String>,
    pub schema_draft: Option<TableSchemaSaveDraft>,
    pub inline_record_draft: Option<TableRecordSaveDraft>,
    pub record_draft: Option<TableRecordSaveDraft>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableContextFileSaveStatus {
    Success,
    Unchanged,
    Conflict,
    Failure,
    OutcomeUnknown,
    NotAttempted,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableContextFileSaveResult {
    pub path: String,
    pub status: TableContextFileSaveStatus,
    pub candidate_content_identity: String,
    pub current: Option<SourceContentState>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableContextSaveReport {
    pub files: Vec<TableContextFileSaveResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRecordSource {
    pub path: String,
    pub inline: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSourceIdentity {
    pub path: String,
    pub content_identity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableFileDiff {
    pub path: String,
    pub before: String,
    pub after: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TablePlanView {
    pub token: String,
    pub table: String,
    pub operation: String,
    pub field: String,
    pub destructive: bool,
    pub affected_record_count: usize,
    pub files: Vec<TableFileDiff>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableApplyView {
    pub file_states: Vec<TableFileState>,
    pub state: String,
    pub files: Vec<String>,
    pub diagnostic: Option<Diagnostic>,
    pub recovery_workspace: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TableFileState {
    pub path: String,
    pub state: String,
}
pub(crate) struct Prepared {
    pub(crate) project: Project,
    pub(crate) before: ProjectDocuments,
    pub(crate) dry_run: SourceCommitCandidate,
}
#[derive(Default)]
pub struct TableAuthoringSession {
    pub(crate) sequence: u64,
    pub(crate) plans: BTreeMap<String, Prepared>,
    recovery: BTreeMap<PathBuf, TableApplyView>,
}
fn error(code: &str, message: &str) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
}
impl TableAuthoringSession {
    pub fn open_context(&self, root: &Path, selected_path: &str) -> Result<TableContext> {
        let project = Project::discover(Some(root), root)?;
        let documents = project.load_documents()?;
        let selected = documents
            .files
            .iter()
            .find(|file| file.path == project.root().join(selected_path))
            .ok_or_else(|| error("E-TABLE-EDITOR-SOURCE", "Selected source not found"))?;
        let table = selected
            .document
            .table_identity()
            .ok_or_else(|| {
                error(
                    "E-TABLE-EDITOR-KIND",
                    "Selected source is not a Table source",
                )
            })?
            .to_owned();
        let schemas = documents.files.iter().filter(|file| {
            matches!(&file.document, SourceDocument::Schema(schema) if schema.table == table)
        }).collect::<Vec<_>>();
        if schemas.len() != 1 {
            return Err(error(
                "E-TABLE-DUPLICATE-SCHEMA",
                "Table must have exactly one schema source",
            ));
        }
        let schema_file = schemas[0];
        let schema_path = project_relative_string(project.root(), &schema_file.path);
        let schema = table_snapshot(&documents, &schema_file.path, &schema_path)?;
        let mut record_sources = documents
            .files
            .iter()
            .filter_map(|file| match &file.document {
                SourceDocument::Schema(schema)
                    if schema.table == table && schema.records.is_some() =>
                {
                    Some(TableRecordSource {
                        path: project_relative_string(project.root(), &file.path),
                        inline: true,
                    })
                }
                SourceDocument::Data(data) if data.table == table => Some(TableRecordSource {
                    path: project_relative_string(project.root(), &file.path),
                    inline: false,
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        record_sources.sort_by(|a, b| b.inline.cmp(&a.inline).then_with(|| a.path.cmp(&b.path)));
        let selected_record_source = record_sources
            .iter()
            .find(|source| source.path == selected_path)
            .or_else(|| record_sources.first())
            .map(|source| source.path.clone());
        Ok(TableContext {
            table,
            schema_path,
            schema_content_identity: source_content_identity(&schema_file.source),
            schema_source: schema_file.source.clone(),
            record_sources,
            selected_record_source,
            schema,
        })
    }

    pub fn open_table(&self, root: &Path, path: &str) -> Result<TableSnapshot> {
        let project = Project::discover(Some(root), root)?;
        let documents = project.load_documents()?;
        table_snapshot(&documents, &project.root().join(path), path)
    }

    pub fn preview_schema_draft(
        &self,
        root: &Path,
        schema_path: &str,
        base_source: &str,
        fields: &[SchemaDraftField],
        record_drafts: &[SchemaDraftRecordSource],
        selected_record_path: Option<&str>,
    ) -> Result<SchemaDraftPreview> {
        let project = Project::discover(Some(root), root)?;
        let target = resolve_source_file(&project, schema_path)?;
        let record_paths = record_drafts
            .iter()
            .map(|draft| resolve_source_file(&project, &draft.path))
            .collect::<Result<Vec<_>>>()?;
        let mut overrides = vec![(target.as_path(), base_source)];
        overrides.extend(
            record_paths
                .iter()
                .zip(record_drafts)
                .map(|(path, draft)| (path.as_path(), draft.candidate_source.as_str())),
        );
        if let Some(inline) = overrides.iter().rfind(|(path, _)| *path == target) {
            overrides[0] = *inline;
        }
        let (documents, mut parse_diagnostics) =
            load_authoring_documents_with_overrides(&project, &overrides)?;
        let schema = documents
            .files
            .iter()
            .find(|file| file.path == target)
            .and_then(|file| match &file.document {
                SourceDocument::Schema(schema) => Some(schema),
                _ => None,
            })
            .ok_or_else(|| error("E-FIELD-DECL-SOURCE", "schema draft base is unavailable"))?;
        if fields.len() != schema.fields.len() {
            return Err(error(
                "E-FIELD-DECL-IDENTITY",
                "schema draft field count changed",
            ));
        }
        let declarations = fields
            .iter()
            .map(|draft| {
                let old = schema
                    .fields
                    .iter()
                    .find(|old| old.name == draft.name)
                    .ok_or_else(|| {
                        error(
                            "E-FIELD-DECL-IDENTITY",
                            "schema draft field identity changed",
                        )
                    })?;
                let mut next = old.clone();
                next.type_name = draft.type_name.clone();
                next.nullable = draft.nullable;
                next.array = draft.array;
                Ok(next)
            })
            .collect::<Result<Vec<_>>>()?;
        let transformed = dry_run_schema_declaration_draft(&documents, &target, &declarations)?;
        let candidate_source = transformed
            .files
            .iter()
            .find(|file| file.path == target)
            .expect("schema candidate")
            .source
            .clone();
        let mut validation = validate_documents(&transformed);
        if !parse_diagnostics.is_empty() {
            validation.valid = false;
            validation.diagnostics.append(&mut parse_diagnostics);
        }
        let selected_snapshot = selected_record_path
            .map(|path| {
                let selected = resolve_source_file(&project, path)?;
                if !transformed.files.iter().any(|file| file.path == selected) {
                    return Ok(None);
                }
                crate::authoring::data_file_snapshot(&project, &transformed, Vec::new(), &selected)
                    .map(Some)
            })
            .transpose()?
            .flatten();
        Ok(SchemaDraftPreview {
            candidate_content_identity: source_content_identity(&candidate_source),
            changed: candidate_source != base_source,
            candidate_source,
            validation,
            selected_snapshot,
        })
    }

    pub fn save_schema_draft(
        &self,
        root: &Path,
        schema_path: &str,
        base_source: &str,
        base_content_identity: &str,
        fields: &[SchemaDraftField],
    ) -> Result<SchemaDraftSaveReport> {
        self.ensure_mutation_allowed(root)?;
        let project = Project::discover(Some(root), root)?;
        let target = resolve_source_file(&project, schema_path)?;
        if source_content_identity(base_source) != base_content_identity {
            return Err(error(
                "E-FIELD-DECL-BASE-IDENTITY",
                "schema draft base identity does not match its source",
            ));
        }
        let preview =
            self.preview_schema_draft(root, schema_path, base_source, fields, &[], None)?;
        let current = read_source_state(&project, &target)?;
        if current.content_identity != base_content_identity {
            return Ok(SchemaDraftSaveReport {
                status: SourceSaveStatus::Conflict,
                path: schema_path.into(),
                candidate_content_identity: preview.candidate_content_identity,
                current: Some(current),
                diagnostic: Some(
                    error(
                        "E-FIELD-DECL-CONFLICT",
                        "schema source changed since this Table was opened",
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }
        if current.source == preview.candidate_source {
            return Ok(SchemaDraftSaveReport {
                status: SourceSaveStatus::Success,
                path: schema_path.into(),
                candidate_content_identity: preview.candidate_content_identity,
                current: None,
                diagnostic: None,
            });
        }
        if let Err(failure) = install_source_candidate(
            &target,
            current.source.as_bytes(),
            preview.candidate_source.as_bytes(),
        ) {
            let after = read_source_state(&project, &target).ok();
            let status = if failure.diagnostic().code == "E-SOURCE-EDIT-CONFLICT" {
                SourceSaveStatus::Conflict
            } else if after
                .as_ref()
                .is_some_and(|state| state.content_identity == preview.candidate_content_identity)
            {
                SourceSaveStatus::Success
            } else if after
                .as_ref()
                .is_some_and(|state| state.content_identity == current.content_identity)
            {
                SourceSaveStatus::Failure
            } else {
                SourceSaveStatus::OutcomeUnknown
            };
            return Ok(SchemaDraftSaveReport {
                status,
                path: schema_path.into(),
                candidate_content_identity: preview.candidate_content_identity,
                current: after,
                diagnostic: (status != SourceSaveStatus::Success)
                    .then(|| failure.diagnostic().clone()),
            });
        }
        let after = read_source_state(&project, &target).ok();
        let status = if after
            .as_ref()
            .is_some_and(|state| state.content_identity == preview.candidate_content_identity)
        {
            SourceSaveStatus::Success
        } else {
            SourceSaveStatus::OutcomeUnknown
        };
        Ok(SchemaDraftSaveReport {
            status,
            path: schema_path.into(),
            candidate_content_identity: preview.candidate_content_identity,
            current: (status != SourceSaveStatus::Success)
                .then_some(after)
                .flatten(),
            diagnostic: (status != SourceSaveStatus::Success).then(|| {
                error(
                    "E-FIELD-DECL-WRITE-VERIFY",
                    "saved schema could not be verified",
                )
                .diagnostic()
                .clone()
            }),
        })
    }

    pub fn save_current_table_context(
        &self,
        root: &Path,
        request: &TableContextSaveRequest,
    ) -> Result<TableContextSaveReport> {
        self.save_current_table_context_with_installer(root, request, install_source_candidate)
    }

    fn save_current_table_context_with_installer(
        &self,
        root: &Path,
        request: &TableContextSaveRequest,
        mut install: impl FnMut(&Path, &[u8], &[u8]) -> Result<()>,
    ) -> Result<TableContextSaveReport> {
        self.ensure_mutation_allowed(root)?;
        let project = Project::discover(Some(root), root)?;
        let schema_path = resolve_source_file(&project, &request.schema_path)?;
        let schema_relative = project_relative_string(project.root(), &schema_path);
        let context = self.open_context(root, &schema_relative)?;
        if context.schema_path != schema_relative {
            return Err(error(
                "E-TABLE-SAVE-CONTEXT",
                "selected source is not the Table schema",
            ));
        }
        let record_path = request
            .selected_record_source
            .as_deref()
            .map(|path| resolve_source_file(&project, path))
            .transpose()?;
        if let Some(path) = &record_path {
            let relative = project_relative_string(project.root(), path);
            if !context
                .record_sources
                .iter()
                .any(|source| source.path == relative)
            {
                return Err(error(
                    "E-TABLE-SAVE-CONTEXT",
                    "selected record source does not belong to this Table",
                ));
            }
        }
        if request.record_draft.is_some() && record_path.is_none() {
            return Err(error(
                "E-TABLE-SAVE-CONTEXT",
                "record draft has no selected record source",
            ));
        }
        if request.inline_record_draft.is_some()
            && !context
                .record_sources
                .iter()
                .any(|source| source.path == schema_relative)
        {
            return Err(error(
                "E-TABLE-SAVE-CONTEXT",
                "schema has no inline record source",
            ));
        }
        if request.inline_record_draft.is_some()
            && request.record_draft.is_some()
            && record_path.as_ref() == Some(&schema_path)
        {
            return Err(error(
                "E-TABLE-SAVE-CONTEXT",
                "inline record draft is duplicated",
            ));
        }

        struct Candidate {
            path: PathBuf,
            relative: String,
            base_source: String,
            base_identity: String,
            source: String,
        }
        let mut candidates = Vec::<Candidate>::new();
        let schema_source = if let Some(draft) = &request.schema_draft {
            if source_content_identity(&draft.base_source) != draft.base_content_identity {
                return Err(error(
                    "E-FIELD-DECL-BASE-IDENTITY",
                    "schema draft base identity does not match its source",
                ));
            }
            let preview = self.preview_schema_draft(
                root,
                &schema_relative,
                &draft.base_source,
                &draft.fields,
                &[],
                None,
            )?;
            candidates.push(Candidate {
                path: schema_path.clone(),
                relative: schema_relative.clone(),
                base_source: draft.base_source.clone(),
                base_identity: draft.base_content_identity.clone(),
                source: preview.candidate_source.clone(),
            });
            preview.candidate_source
        } else {
            context.schema_source
        };

        for (path, draft) in [
            request
                .inline_record_draft
                .as_ref()
                .map(|draft| (&schema_path, draft)),
            record_path.as_ref().zip(request.record_draft.as_ref()),
        ]
        .into_iter()
        .flatten()
        {
            if source_content_identity(&draft.base_source) != draft.base_content_identity {
                return Err(error(
                    "E-SOURCE-EDIT-BASE-IDENTITY",
                    "record draft base identity does not match its source",
                ));
            }
            if *path == schema_path
                && request.schema_draft.as_ref().is_some_and(|schema| {
                    schema.base_content_identity != draft.base_content_identity
                        || schema.base_source != draft.base_source
                })
            {
                return Err(error(
                    "E-TABLE-SAVE-BASE-IDENTITY",
                    "schema and inline records have different physical source bases",
                ));
            }
            let current_schema_candidate = candidates
                .iter()
                .find(|item| item.path == schema_path)
                .map(|item| item.source.as_str())
                .unwrap_or(schema_source.as_str());
            let mut overrides = vec![(schema_path.as_path(), current_schema_candidate)];
            if *path != schema_path {
                overrides.push((path.as_path(), draft.base_source.as_str()));
            }
            let (documents, _) = load_authoring_documents_with_overrides(&project, &overrides)?;
            let dry_run = dry_run_source_record_mutation(
                &documents,
                path,
                &SourceRecordMutation::from(&draft.mutation),
            )?;
            if let Some(schema_candidate) = candidates.iter_mut().find(|item| item.path == *path) {
                schema_candidate.source = dry_run.plan.candidate_source;
            } else {
                candidates.push(Candidate {
                    path: path.clone(),
                    relative: project_relative_string(project.root(), path),
                    base_source: draft.base_source.clone(),
                    base_identity: draft.base_content_identity.clone(),
                    source: dry_run.plan.candidate_source,
                });
            }
        }

        candidates.sort_by(|left, right| left.relative.cmp(&right.relative));
        let mut files = candidates
            .iter()
            .map(|candidate| TableContextFileSaveResult {
                path: candidate.relative.clone(),
                status: if candidate.source == candidate.base_source {
                    TableContextFileSaveStatus::Unchanged
                } else {
                    TableContextFileSaveStatus::NotAttempted
                },
                candidate_content_identity: source_content_identity(&candidate.source),
                current: None,
                diagnostic: None,
            })
            .collect::<Vec<_>>();

        // All known conflicts are found before the first write. The installer
        // still rechecks exact bytes immediately before each individual commit.
        let mut preflight = Vec::with_capacity(candidates.len());
        let mut blocked = false;
        for (index, candidate) in candidates.iter().enumerate() {
            match read_source_state(&project, &candidate.path) {
                Ok(current)
                    if current.content_identity == candidate.base_identity
                        && current.source == candidate.base_source =>
                {
                    if files[index].status != TableContextFileSaveStatus::Unchanged {
                        let eligible = candidate
                            .path
                            .parent()
                            .and_then(|parent| tempfile::TempDir::new_in(parent).ok());
                        if eligible.is_none() {
                            blocked = true;
                            files[index].status = TableContextFileSaveStatus::Failure;
                            files[index].diagnostic = Some(
                                error(
                                    "E-TABLE-SAVE-PREFLIGHT",
                                    "could not stage a source candidate beside the target",
                                )
                                .diagnostic()
                                .clone(),
                            );
                            preflight.push(None);
                            continue;
                        }
                    }
                    preflight.push(
                        (files[index].status != TableContextFileSaveStatus::Unchanged)
                            .then_some(current),
                    )
                }
                Ok(current) => {
                    blocked = true;
                    files[index].status = TableContextFileSaveStatus::Conflict;
                    files[index].current = Some(current);
                    files[index].diagnostic = Some(
                        error(
                            "E-TABLE-SAVE-CONFLICT",
                            "source changed since this Table was opened",
                        )
                        .diagnostic()
                        .clone(),
                    );
                    preflight.push(None);
                }
                Err(failure) => {
                    blocked = true;
                    files[index].status = TableContextFileSaveStatus::Failure;
                    files[index].diagnostic = Some(failure.diagnostic().clone());
                    preflight.push(None);
                }
            }
        }
        if blocked {
            return Ok(TableContextSaveReport { files });
        }

        for (index, candidate) in candidates.iter().enumerate() {
            let Some(current) = &preflight[index] else {
                continue;
            };
            let installed = install(
                &candidate.path,
                current.source.as_bytes(),
                candidate.source.as_bytes(),
            );
            let after = read_source_state(&project, &candidate.path).ok();
            files[index].status = if installed
                .as_ref()
                .err()
                .is_some_and(|failure| failure.diagnostic().code == "E-SOURCE-EDIT-CONFLICT")
            {
                TableContextFileSaveStatus::Conflict
            } else if installed.is_ok()
                && after.as_ref().is_some_and(|state| {
                    state.content_identity == files[index].candidate_content_identity
                        && state.source == candidate.source
                })
            {
                TableContextFileSaveStatus::Success
            } else if installed.is_ok() || after.is_none() {
                TableContextFileSaveStatus::OutcomeUnknown
            } else if after
                .as_ref()
                .is_some_and(|state| state.content_identity == current.content_identity)
            {
                TableContextFileSaveStatus::Failure
            } else {
                TableContextFileSaveStatus::OutcomeUnknown
            };
            files[index].current = after;
            if files[index].status != TableContextFileSaveStatus::Success {
                files[index].diagnostic = Some(match installed {
                    Err(failure) => failure.diagnostic().clone(),
                    Ok(()) => error(
                        "E-TABLE-SAVE-WRITE-VERIFY",
                        "saved source could not be verified",
                    )
                    .diagnostic()
                    .clone(),
                });
                break;
            }
        }
        Ok(TableContextSaveReport { files })
    }
    pub fn plan(&mut self, root: &Path, input: TableOperationInput) -> Result<TablePlanView> {
        self.ensure_mutation_allowed(root)?;
        let project = Project::discover(Some(root), root)?;
        let before = project.load_documents()?;
        let command = input.command(&before)?;
        let dry_run = dry_run_migration(&before, &command)?;
        self.sequence += 1;
        let token = self.sequence.to_string();
        let files = dry_run
            .plan
            .affected_files
            .iter()
            .map(|file| TableFileDiff {
                path: project_relative_string(project.root(), &file.path),
                before: before
                    .files
                    .iter()
                    .find(|doc| doc.path == file.path)
                    .expect("plan source")
                    .source
                    .clone(),
                after: dry_run
                    .transformed_documents
                    .files
                    .iter()
                    .find(|doc| doc.path == file.path)
                    .expect("plan target")
                    .source
                    .clone(),
            })
            .collect();
        let view = TablePlanView {
            token: token.clone(),
            table: dry_run.plan.target_table.clone(),
            field: dry_run.plan.field.name.clone(),
            operation: match dry_run.plan.operation {
                MigrationOperation::AddField => "AddField",
                MigrationOperation::RenameField => "RenameField",
                MigrationOperation::DropField => "DropField",
                MigrationOperation::ChangeFieldDeclaration => "ChangeFieldDeclaration",
                MigrationOperation::AddReference => "AddReference",
                MigrationOperation::EditReference => "EditReference",
                MigrationOperation::RemoveReference => "RemoveReference",
            }
            .into(),
            destructive: dry_run.plan.destructive,
            affected_record_count: dry_run.plan.affected_record_count,
            files,
            diagnostics: dry_run.plan.validation.diagnostics.clone(),
        };
        self.plans
            .retain(|_, plan| plan.project.root() != project.root());
        self.plans.insert(
            token,
            Prepared {
                project,
                before,
                dry_run: dry_run.into(),
            },
        );
        Ok(view)
    }
    pub fn apply(
        &mut self,
        root: &Path,
        token: &str,
        allow_destructive: bool,
    ) -> Result<TableApplyView> {
        self.apply_with_failures(root, token, allow_destructive, &[])
    }
    pub fn apply_intent(
        &mut self,
        root: &Path,
        input: TableOperationInput,
        expected_sources: &[TableSourceIdentity],
        dirty_paths: &[String],
    ) -> Result<TableApplyView> {
        let plan = self.plan(root, input)?;
        if plan.destructive {
            return Err(error(
                "E-MIGRATION-AUTHORIZATION",
                "Destructive schema changes require explicit authorization",
            ));
        }
        let project = Project::discover(Some(root), root)?;
        let documents = project.load_documents()?;
        let schema_path = documents
            .files
            .iter()
            .find_map(|file| match &file.document {
                SourceDocument::Schema(schema) if schema.table == plan.table => {
                    Some(project_relative_string(project.root(), &file.path))
                }
                _ => None,
            })
            .ok_or_else(|| error("E-TABLE-EDITOR-SOURCE", "Table schema not found"))?;
        if !expected_sources
            .iter()
            .any(|source| source.path == schema_path)
        {
            return Err(error(
                "E-TABLE-STALE-SOURCE",
                "Table schema identity is required to change columns",
            ));
        }
        for expected in expected_sources {
            let current = documents
                .files
                .iter()
                .find(|file| project_relative_string(project.root(), &file.path) == expected.path);
            if current.is_none_or(|file| {
                source_content_identity(&file.source) != expected.content_identity
            }) {
                return Err(error(
                    "E-TABLE-STALE-SOURCE",
                    format!("Source {} changed since this Table was opened; reload before changing columns", expected.path).as_str(),
                ));
            }
        }
        let mut affected = plan
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        affected.extend(documents.files.iter().filter_map(|file| {
            let contains_records = match &file.document {
                SourceDocument::Schema(schema) => {
                    schema.table == plan.table && schema.records.is_some()
                }
                SourceDocument::Data(data) => data.table == plan.table,
                _ => false,
            };
            contains_records.then(|| project_relative_string(project.root(), &file.path))
        }));
        affected.sort();
        affected.dedup();
        let blocked = affected
            .iter()
            .filter(|path| dirty_paths.contains(path))
            .map(String::as_str)
            .collect::<Vec<_>>();
        if !blocked.is_empty() {
            return Err(error(
                "E-TABLE-AFFECTED-DIRTY",
                format!(
                    "Save or resolve unsaved changes in {} before changing columns",
                    blocked.join(", ")
                )
                .as_str(),
            ));
        }
        self.apply(root, &plan.token, false)
    }
    #[doc(hidden)]
    pub fn apply_with_failures(
        &mut self,
        root: &Path,
        token: &str,
        allow_destructive: bool,
        injections: &[MigrationCommitFailureInjection],
    ) -> Result<TableApplyView> {
        self.ensure_mutation_allowed(root)?;
        let project = Project::discover(Some(root), root)?;
        let plan = self
            .plans
            .get(token)
            .filter(|plan| plan.project.root() == project.root())
            .ok_or_else(|| {
                error(
                    "E-MIGRATION-PLAN-STALE",
                    "Plan is no longer available; re-plan",
                )
            })?;
        let (report, diagnostic) = match commit_source_candidate_with_failures(
            &plan.project,
            &plan.before,
            &plan.dry_run,
            allow_destructive,
            injections,
        ) {
            Ok(report) => (report, None),
            Err(failure) => (failure.report, Some(failure.error.diagnostic().clone())),
        };
        let state = match report.state {
            MigrationCommitState::NotStarted => "not_started",
            MigrationCommitState::Success => "success",
            MigrationCommitState::RolledBack => "rolled_back",
            MigrationCommitState::RecoveryRequired => "recovery_required",
        }
        .to_owned();
        let file_states = report
            .files
            .iter()
            .map(|file| TableFileState {
                path: project_relative_string(project.root(), &file.path),
                state: match file.state {
                    MigrationFileCommitState::Unchanged => "unchanged",
                    MigrationFileCommitState::New => "new",
                    MigrationFileCommitState::Old => "old",
                    MigrationFileCommitState::RecoveryRequired => "recovery_required",
                }
                .into(),
            })
            .collect();
        let result = TableApplyView {
            file_states,
            state,
            files: report
                .files
                .iter()
                .map(|file| project_relative_string(project.root(), &file.path))
                .collect(),
            diagnostic,
            recovery_workspace: report.recovery_workspace,
        };
        if report.state == MigrationCommitState::RecoveryRequired {
            self.recovery
                .insert(project.root().to_path_buf(), result.clone());
        }
        Ok(result)
    }
    pub fn recovery_status(&self, root: &Path) -> Result<Option<TableApplyView>> {
        let project = Project::discover(Some(root), root)?;
        Ok(self.recovery.get(project.root()).cloned())
    }
    pub fn ensure_mutation_allowed(&self, root: &Path) -> Result<()> {
        if self.recovery_status(root)?.is_some() {
            Err(recovery_required_error())
        } else {
            Ok(())
        }
    }

    /// Config repair must remain possible while TOML is domain-invalid.  This
    /// raw-root gate checks only the in-memory migration recovery marker and
    /// therefore does not rediscover/parse the project configuration.
    pub fn ensure_mutation_allowed_at_root(&self, root: &Path) -> Result<()> {
        if self.recovery.contains_key(root) {
            Err(recovery_required_error())
        } else {
            Ok(())
        }
    }
    pub fn recheck(&mut self, root: &Path) -> Result<Option<TableApplyView>> {
        let project = Project::discover(Some(root), root)?;
        // Re-enable only after host reads a complete known OLD or NEW snapshot.
        // A successful parse alone cannot distinguish a mixed transaction set.
        // EVIDENCE: MIGRATION-010; GUI-SHELL-CAPABILITY-001.
        if let Some(plan) = self
            .plans
            .values()
            .find(|plan| plan.project.root() == project.root())
        {
            let current = project.load_documents()?;
            let same = |expected: &ProjectDocuments| {
                current.files.len() == expected.files.len()
                    && expected.files.iter().all(|old| {
                        current
                            .files
                            .iter()
                            .any(|new| new.path == old.path && new.source == old.source)
                    })
            };
            if same(&plan.before) || same(&plan.dry_run.transformed_documents) {
                self.recovery.remove(project.root());
            } else if !self.recovery.contains_key(project.root()) {
                self.recovery.insert(
                    project.root().into(),
                    TableApplyView {
                        file_states: Vec::new(),
                        state: "recovery_required".into(),
                        files: plan
                            .dry_run
                            .affected_files
                            .iter()
                            .map(|file| project_relative_string(project.root(), &file.path))
                            .collect(),
                        diagnostic: Some(
                            error(
                                "E-MIGRATION-RECOVERY-REQUIRED",
                                "Workspace differs from both reviewed source sets",
                            )
                            .diagnostic()
                            .clone(),
                        ),
                        recovery_workspace: None,
                    },
                );
            }
        }
        self.recovery_status(root)
    }
}

fn recovery_required_error() -> MasterdataError {
    error(
        "E-MIGRATION-RECOVERY-REQUIRED",
        "Project requires source recovery before further mutation or Build",
    )
}

#[cfg(test)]
mod context_save_tests {
    use super::*;
    use crate::authoring::{AuthoringEdit, AuthoringRecordMutation};
    use std::fs;

    #[test]
    fn runtime_second_write_failure_reports_committed_subset() {
        let dir = tempfile::tempdir().unwrap();
        initialize_project(
            dir.path(),
            &InitOptions {
                project_id: "test.partial".into(),
                name: "Partial".into(),
                version: "0.1.0".into(),
            },
        )
        .unwrap();
        fs::write(dir.path().join("sources/schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: note\n    type: string\nprimaryKey:\n  fields: [id]\n").unwrap();
        fs::write(
            dir.path().join("sources/data.yaml"),
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: old\n",
        )
        .unwrap();
        let session = TableAuthoringSession::default();
        let context = session
            .open_context(dir.path(), "sources/data.yaml")
            .unwrap();
        let snapshot = crate::NativeApplicationService::new()
            .open_data_file(Some(dir.path()), dir.path(), "sources/data.yaml")
            .unwrap();
        let schema_before = fs::read(dir.path().join("sources/schema.yaml")).unwrap();
        let request = TableContextSaveRequest {
            schema_path: context.schema_path,
            selected_record_source: context.selected_record_source,
            schema_draft: Some(TableSchemaSaveDraft {
                base_source: context.schema_source,
                base_content_identity: context.schema_content_identity,
                fields: vec![
                    SchemaDraftField {
                        name: "id".into(),
                        type_name: "int".into(),
                        nullable: false,
                        array: false,
                    },
                    SchemaDraftField {
                        name: "note".into(),
                        type_name: "string".into(),
                        nullable: true,
                        array: false,
                    },
                ],
            }),
            inline_record_draft: None,
            record_draft: Some(TableRecordSaveDraft {
                base_source: snapshot.base_source,
                base_content_identity: snapshot.base_content_identity,
                mutation: AuthoringRecordMutation {
                    edits: vec![AuthoringEdit {
                        record_index: 0,
                        field: "note".into(),
                        value: AuthoringValue::String {
                            value: "new".into(),
                        },
                    }],
                    ..Default::default()
                },
            }),
        };
        let mut attempts = 0;
        let report = session
            .save_current_table_context_with_installer(
                dir.path(),
                &request,
                |path, base, candidate| {
                    attempts += 1;
                    if attempts == 2 {
                        Err(error("E-IO-TEST", "injected second write failure"))
                    } else {
                        install_source_candidate(path, base, candidate)
                    }
                },
            )
            .unwrap();
        assert_eq!(attempts, 2);
        assert_eq!(report.files[0].status, TableContextFileSaveStatus::Success);
        assert_eq!(report.files[1].status, TableContextFileSaveStatus::Failure);
        assert!(
            fs::read_to_string(dir.path().join("sources/data.yaml"))
                .unwrap()
                .contains("note: new")
        );
        assert_eq!(
            fs::read(dir.path().join("sources/schema.yaml")).unwrap(),
            schema_before
        );
    }
}
