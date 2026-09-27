//! Session-owned plans bind Apply to reviewed bytes, never a frontend-supplied patch.
use crate::authoring::{
    SourceContentState, SourceSaveStatus, install_source_candidate,
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
            Self::AddDefault { table } => {
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
        let declarations = schema
            .fields
            .iter()
            .zip(fields)
            .map(|(old, draft)| {
                if old.name != draft.name {
                    return Err(error(
                        "E-FIELD-DECL-IDENTITY",
                        "schema draft field identity changed",
                    ));
                }
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
