use super::*;
use crate::migration::{self, Command};
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_PLAN: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FieldOperation {
    Rename {
        field: String,
        new_name: String,
    },
    Add {
        neighbor: Option<String>,
        after: bool,
    },
    Drop {
        field: String,
    },
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldIntent {
    pub token: String,
    pub operation: FieldOperation,
    pub authorize_destructive: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFile {
    pub source: String,
    pub before_identity: String,
    pub after_identity: String,
    pub before_bytes: usize,
    pub after_bytes: usize,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReview {
    pub token: String,
    pub command: Command,
    pub destructive: bool,
    pub affected_records: usize,
    pub files: Vec<MigrationFile>,
    pub dirty_sources: Vec<String>,
}
fn plan_token(token: &str) -> Result<u64> {
    token
        .parse()
        .map_err(|_| Error::new("E-MIGRATION-PLAN", "invalid Plan token"))
}
fn migration_review(token: u64, plan: &native::SourceSetPlan) -> MigrationReview {
    MigrationReview {
        token: token.to_string(),
        command: plan.plan.command.clone(),
        destructive: plan.plan.destructive,
        affected_records: plan.plan.affected_records,
        files: plan
            .plan
            .candidates
            .iter()
            .map(|(path, candidate)| MigrationFile {
                source: path.clone(),
                before_identity: candidate.before.identity.clone(),
                after_identity: candidate.after.identity.clone(),
                before_bytes: candidate.before.bytes.len(),
                after_bytes: candidate.after.bytes.len(),
            })
            .collect(),
        dirty_sources: vec![],
    }
}
impl Workspace {
    fn field_command(
        &mut self,
        source: &str,
        revision: u64,
        generation: u64,
        operation: FieldOperation,
    ) -> Result<Command> {
        self.check_authoring_generation(source, generation)?;
        self.check_config()?;
        self.refresh_source(source)?;
        self.ensure_draft(source)?;
        self.check_authoring_generation(source, generation)?;
        let draft = &self.drafts[source];
        if draft.revision != revision {
            return Err(Error::new(
                "E-DRAFT-STALE",
                "schema changed before structural authoring",
            ));
        }
        let binding = self
            .read
            .sources
            .get(source)
            .and_then(|s| s.binding.as_ref())
            .ok_or_else(|| Error::new("E-TABLE-MISSING", "schema binding required"))?;
        let table = self.current_table(binding)?;
        if table.source != source {
            return Err(Error::new(
                "E-MIGRATION-TARGET",
                "the current schema source is required",
            ));
        }
        Ok(match operation {
            FieldOperation::Rename { field, new_name } => Command::RenameField {
                table: table.name.clone(),
                field,
                new_name,
            },
            FieldOperation::Drop { field } => Command::DropField {
                table: table.name.clone(),
                field,
            },
            FieldOperation::Add { neighbor, after } => {
                let mut fields = table
                    .fields
                    .iter()
                    .map(|f| crate::creation::Declaration {
                        key: f.key.to_string(),
                        name: f.name.clone(),
                        type_name: f.type_name.clone(),
                        nullable: f.nullable,
                        array: f.array,
                    })
                    .collect::<Vec<_>>();
                let mut declaration = loop {
                    let candidate = crate::creation::suggest_field(&fields)?;
                    let generated = semantic::public_name(&candidate.name);
                    if generated != table.csharp_name
                        && !table.references.iter().any(|r| r.csharp_name == generated)
                    {
                        break candidate;
                    }
                    fields.push(crate::creation::Declaration {
                        key: fields[0].key.clone(),
                        ..candidate
                    });
                };
                declaration.type_name = "string".into();
                declaration.nullable = true;
                declaration.array = false;
                let position = neighbor
                    .map(|name| {
                        table
                            .fields
                            .iter()
                            .position(|f| f.name == name)
                            .map(|i| i + usize::from(after))
                            .ok_or_else(|| {
                                Error::new("E-FIELD-MISSING", "insertion neighbor is unavailable")
                            })
                    })
                    .transpose()?;
                Command::AddField {
                    table: table.name.clone(),
                    declaration,
                    initializer: Some(Value::Null),
                    position,
                }
            }
        })
    }
    pub fn field_operation_scope(
        &mut self,
        source: &str,
        revision: u64,
        generation: u64,
        operation: FieldOperation,
    ) -> Result<serde_json::Value> {
        self.migration_gate(std::iter::empty())?;
        let command = self.field_command(source, revision, generation, operation)?;
        let project = Project::open(&self.read.root)?;
        let plan = migration::derive(&project, command)?;
        let sources = plan.candidates.keys().cloned().collect::<Vec<_>>();
        let dirty = sources
            .iter()
            .filter(|p| self.drafts.get(*p).is_some_and(Draft::dirty))
            .cloned()
            .collect::<Vec<_>>();
        let review = self.register_migration(native::SourceSetPlan::prepare(&project, plan)?);
        Ok(
            serde_json::json!({"token":review.token,"sources":sources,"dirty":dirty,"affectedRecords":review.affected_records,"destructive":review.destructive}),
        )
    }
    pub fn direct_field_operation(
        &mut self,
        source: &str,
        revision: u64,
        generation: u64,
        request: FieldIntent,
    ) -> Result<(MigrationReview, native::SetResult)> {
        let command = self.field_command(source, revision, generation, request.operation)?;
        let token = plan_token(&request.token)?;
        let plan = self.migration_plans.get(&token).ok_or_else(|| {
            Error::new(
                "E-MIGRATION-PLAN",
                "scope is unavailable; confirm the command again",
            )
        })?;
        let encoded = |command: &Command| {
            serde_json::to_value(command)
                .map_err(|e| Error::new("E-MIGRATION-COMMAND", e.to_string()))
        };
        if encoded(&command)? != encoded(&plan.plan.command)? {
            return Err(Error::new(
                "E-MIGRATION-STALE",
                "the requested command differs from the captured scope",
            ));
        }
        let review = migration_review(token, plan);
        // Enter / Add is the explicit semantic command. Native owns derivation,
        // authorization and commit; the frontend never auto-approves a Plan DTO.
        let result = self.apply_migration(&review.token, request.authorize_destructive)?;
        Ok((review, result))
    }
    pub(super) fn migration_gate(&self, paths: impl Iterator<Item = String>) -> Result<()> {
        if self.recovery_required || native::has_pending_recovery(&self.read.root)? {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "structural writes are gated",
            ));
        }
        self.migration_local_gate(paths)
    }
    fn migration_local_gate(&self, paths: impl Iterator<Item = String>) -> Result<()> {
        self.check_config()?;
        if self.recovery_required {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "structural writes are gated",
            ));
        }
        let uncertain = self.uncertain_paths();
        for path in paths {
            self.check_pending_move(&path)?;
            if self.drafts.get(&path).is_some_and(Draft::dirty) {
                return Err(Error::new(
                    "E-MIGRATION-DIRTY",
                    format!("{path}: Save / Don't Save / Cancel required for affected sources"),
                ));
            }
            if uncertain.contains(&path) {
                return Err(Error::new(
                    "E-OUTCOME-UNKNOWN",
                    format!("{path}: source outcome must be rechecked"),
                ));
            }
        }
        Ok(())
    }
    pub fn prepare_migration(&mut self, command: Command) -> Result<MigrationReview> {
        self.migration_gate(std::iter::empty())?;
        let project = Project::open(&self.read.root)?;
        let plan = migration::derive(&project, command)?;
        self.migration_gate(plan.candidates.keys().cloned())?;
        let plan = native::SourceSetPlan::prepare(&project, plan)?;
        Ok(self.register_migration(plan))
    }
    pub(super) fn register_migration(&mut self, plan: native::SourceSetPlan) -> MigrationReview {
        let token = NEXT_PLAN.fetch_add(1, Ordering::Relaxed);
        let mut review = migration_review(token, &plan);
        review.dirty_sources = plan
            .plan
            .candidates
            .keys()
            .filter(|path| self.drafts.get(*path).is_some_and(Draft::dirty))
            .cloned()
            .collect();
        self.migration_plans.insert(token, plan);
        while self.migration_plans.len() > 4 {
            self.migration_plans.pop_first();
        }
        review
    }
    pub fn migration_compare(&self, token: &str, source: &str) -> Result<(String, String)> {
        let plan = self
            .migration_plans
            .get(&plan_token(token)?)
            .ok_or_else(|| {
                Error::new(
                    "E-MIGRATION-PLAN",
                    "Plan is unavailable; request a new Plan explicitly",
                )
            })?;
        let candidate = plan.plan.candidates.get(source).ok_or_else(|| {
            Error::new("E-MIGRATION-SOURCE", "source is not affected by this Plan")
        })?;
        Ok((
            candidate.before.bytes.to_string(),
            candidate.after.bytes.to_string(),
        ))
    }
    pub fn apply_migration(&mut self, token: &str, destructive: bool) -> Result<native::SetResult> {
        self.apply_migration_with_fault(token, destructive, native::SetFault::None)
    }
    pub fn apply_migration_with_fault(
        &mut self,
        token: &str,
        destructive: bool,
        fault: native::SetFault,
    ) -> Result<native::SetResult> {
        let token = plan_token(token)?;
        if self.migration_results.contains_key(&token) {
            return Err(Error::new(
                "E-MIGRATION-APPLIED",
                "operation was already attempted; Recheck its result without repeating the write",
            ));
        }
        let plan = self.migration_plans.get(&token).cloned().ok_or_else(|| {
            Error::new(
                "E-MIGRATION-PLAN",
                "Plan is unavailable; request a new Plan explicitly",
            )
        })?;
        self.migration_gate(plan.plan.candidates.keys().cloned())?;
        // Entry rejects other pending journals. The per-file callback checks
        // local authoring authority without rejecting this commit's own journal.
        let result = plan.commit_authorized(destructive, fault, || {
            self.migration_local_gate(plan.plan.candidates.keys().cloned())
        })?;
        self.accept_migration(&plan, &result)?;
        self.migration_results.insert(token, result.clone());
        while self.migration_results.len() > 4 {
            self.migration_results.pop_first();
        }
        Ok(result)
    }
    fn accept_migration(
        &mut self,
        plan: &native::SourceSetPlan,
        result: &native::SetResult,
    ) -> Result<()> {
        if result.outcome == Outcome::RecoveryRequired {
            self.recovery_required = true;
            if let Some(info) = &result.recovery {
                self.recovery_information.push(info.clone());
            }
        }
        for file in &result.files {
            let Some(actual) = result.snapshots.get(&file.source) else {
                self.unavailable_source(
                    &file.source,
                    None,
                    Error::new("E-RECOVERY-SOURCE", "actual source could not be observed"),
                );
                continue;
            };
            let candidate = &plan.plan.candidates[&file.source];
            let parsed = if actual.bytes == candidate.after.bytes {
                Ok(candidate.after.clone())
            } else if actual.bytes == candidate.before.bytes {
                Ok(candidate.before.clone())
            } else {
                Document::parse(actual.bytes.clone()).map(Arc::new)
            };
            match parsed {
                Ok(document) => {
                    if let Some(draft) = self.drafts.get_mut(&file.source) {
                        let changed = draft.document.bytes != document.bytes;
                        draft.document = document.clone();
                        draft.base = actual.clone();
                        draft.revision += 1;
                        draft.outcome = None;
                        draft.external = None;
                        if changed {
                            draft.undo.clear();
                            draft.redo.clear();
                            draft.pending = Arc::new(BTreeMap::new());
                            draft.added = Arc::new(BTreeSet::new());
                            draft.origin = None;
                            draft.arrays = Arc::new(BTreeMap::new());
                        }
                    }
                    self.snapshots.insert(file.source.clone(), actual.clone());
                    self.replace_read(&file.source, actual, Some(document), None);
                }
                Err(error) => {
                    self.unavailable_source(&file.source, Some(actual.clone()), error);
                }
            }
        }
        if result.outcome == Outcome::Success
            && let Command::RenameField {
                table,
                field,
                new_name,
            } = &plan.plan.command
        {
            for (path, view) in &mut self.views {
                if self
                    .read
                    .sources
                    .get(path)
                    .is_some_and(|s| s.binding.as_ref() == Some(table))
                    && view.selected_field.as_ref() == Some(field)
                {
                    view.selected_field = Some(new_name.clone());
                }
            }
        }
        self.authoring_views.clear();
        self.search_indexes.clear();
        self.diagnostics_pending = true;
        Ok(())
    }
    pub fn recheck_migration_result(&self, token: &str) -> Result<native::SetResult> {
        let token = plan_token(token)?;
        if !self.migration_results.contains_key(&token) {
            return self
                .migration_plans
                .get(&token)
                .ok_or_else(|| {
                    Error::new("E-MIGRATION-RESULT", "attempt information is unavailable")
                })?
                .confirm_unattempted();
        }
        let result = self
            .migration_results
            .get(&token)
            .ok_or_else(|| Error::new("E-MIGRATION-RESULT", "no completed attempt is available"))?;
        for (path, observed) in &result.snapshots {
            native::preflight(&self.read.root, &self.read.roots, path, observed)?;
        }
        Ok(result.clone())
    }
    pub fn recheck_migration_recovery(
        &mut self,
        id: &str,
        restore_old: bool,
        authorized: bool,
    ) -> Result<native::RecoveryInfo> {
        if let Some((info, reconciled)) = self.completed_recoveries.get(id).cloned() {
            for (path, observed) in &info.snapshots {
                native::preflight(&self.read.root, &self.read.roots, path, observed)?;
            }
            if !reconciled {
                self.accept_recovery(&info)?;
                self.completed_recoveries.get_mut(id).unwrap().1 = true;
            }
            return Ok(info);
        }
        let info = if restore_old {
            native::restore_recovery(&self.read.root, id, authorized)?
        } else {
            native::recheck_recovery(&self.read.root, id)?
        };
        // Keep the observed completion before read publication. A lost IPC reply
        // must permit a fresh read-only Recheck without repeating Restore OLD.
        self.completed_recoveries
            .insert(id.into(), (info.clone(), false));
        while self.completed_recoveries.len() > 4 {
            let oldest = self
                .completed_recoveries
                .keys()
                .find(|key| *key != id)
                .cloned();
            if let Some(oldest) = oldest {
                self.completed_recoveries.remove(&oldest);
            }
        }
        self.accept_recovery(&info)?;
        self.completed_recoveries.get_mut(id).unwrap().1 = true;
        Ok(info)
    }
    fn accept_recovery(&mut self, info: &native::RecoveryInfo) -> Result<()> {
        let fresh = Project::open(&self.read.root)?;
        for (path, observed) in &info.snapshots {
            native::preflight(&fresh.root, &fresh.roots, path, observed)?;
        }
        for file in &info.files {
            let actual = native::capture(&fresh.root, &fresh.roots, &file.source)?;
            let document = fresh
                .sources
                .get(&file.source)
                .and_then(|source| source.document.clone());
            self.drafts.remove(&file.source);
            self.snapshots.insert(file.source.clone(), actual.clone());
            self.replace_read(&file.source, &actual, document, None);
        }
        self.recovery_information = native::pending_recovery(&self.read.root)?;
        self.recovery_required = !self.recovery_information.is_empty();
        self.authoring_views.clear();
        self.search_indexes.clear();
        self.external_version += 1;
        self.diagnostics_pending = true;
        Ok(())
    }
    pub fn detect_recovery(&mut self) {
        if self.recovery_required {
            return;
        }
        match native::pending_recovery(&self.read.root) {
            Ok(information) if information.is_empty() => {}
            Ok(information) => {
                self.recovery_required = true;
                self.recovery_information = information;
            }
            Err(error) => {
                self.recovery_required = true;
                self.recovery_information = vec![native::RecoveryInfo {
                    snapshots: BTreeMap::new(),
                    id: String::new(),
                    directory: self
                        .read
                        .root
                        .join(".masterdata/migrations")
                        .to_string_lossy()
                        .into(),
                    message: error.to_string(),
                    files: vec![],
                }];
            }
        }
    }
}
