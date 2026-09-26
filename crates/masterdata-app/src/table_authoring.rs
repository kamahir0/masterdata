//! Session-owned plans bind Apply to reviewed bytes, never a frontend-supplied patch.
use crate::authoring::project_relative_string;
use masterdata_core::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum TableOperationInput {
    Add {
        table: String,
        field: FieldDefinition,
        initializer: Option<String>,
    },
    #[serde(rename = "add_direct")]
    AddDirect {
        table: String,
        #[serde(rename = "typeName")]
        type_name: String,
        #[serde(default)]
        name: Option<String>,
    },
    Rename {
        table: String,
        field: String,
        #[serde(rename = "newName")]
        new_name: String,
    },
    #[serde(rename = "change_type", alias = "change_field_type")]
    ChangeType {
        table: String,
        field: String,
        #[serde(rename = "newType")]
        new_type: String,
    },
    Drop {
        table: String,
        field: String,
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
            Self::AddDirect {
                table,
                type_name,
                name,
            } => {
                let schema = documents
                    .schemas()
                    .find(|(_, schema)| schema.table == table)
                    .map(|(_, schema)| schema)
                    .ok_or_else(|| {
                        error("E-MIGRATION-TABLE-NOT-FOUND", "target Table does not exist")
                    })?;
                let name = name
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| next_direct_field_name(schema));
                if schema.fields.iter().any(|field| field.name == name) {
                    return Err(error(
                        "E-MIGRATION-FIELD-NAME-CONFLICT",
                        "direct Add Field name already exists",
                    ));
                }
                let key = schema
                    .fields
                    .iter()
                    .map(|field| field.key)
                    .max()
                    .map_or(0, |key| key.saturating_add(1));
                let field = FieldDefinition {
                    key,
                    name,
                    type_name,
                    nullable: false,
                    array: false,
                };
                let initializer = default_field_value_for_table(documents, &table, &field)?;
                MigrationCommand::AddField(AddFieldCommand {
                    table,
                    field,
                    initializer: Some(initializer),
                })
            }
            Self::Rename {
                table,
                field,
                new_name,
            } => MigrationCommand::RenameField(RenameFieldCommand {
                table,
                field,
                new_name,
            }),
            Self::ChangeType {
                table,
                field,
                new_type,
            } => MigrationCommand::ChangeFieldType(ChangeFieldTypeCommand {
                table,
                field,
                new_type,
            }),
            Self::Drop { table, field } => {
                MigrationCommand::DropField(DropFieldCommand { table, field })
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

fn next_direct_field_name(schema: &SchemaDocument) -> String {
    let names = schema
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut index = 1;
    loop {
        let candidate = format!("field{index}");
        if !names.contains(candidate.as_str()) {
            return candidate;
        }
        index += 1;
    }
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
    pub fn open_table(&self, root: &Path, path: &str) -> Result<TableSnapshot> {
        let project = Project::discover(Some(root), root)?;
        let documents = project.load_documents()?;
        table_snapshot(&documents, &project.root().join(path), path)
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
                MigrationOperation::ChangeFieldType => "ChangeFieldType",
                MigrationOperation::DropField => "DropField",
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
