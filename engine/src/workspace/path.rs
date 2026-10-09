use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_MOVE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveReview {
    pub token: String,
    pub source: String,
    pub destination: String,
}
impl Workspace {
    pub(super) fn check_pending_move(&self, path: &str) -> Result<()> {
        if self
            .pending_moves
            .values()
            .any(|p| p.source == path || p.destination == path)
        {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "Recheck both old and new paths before another mutation",
            ));
        }
        Ok(())
    }
    fn path_gate(&self, source: &str) -> Result<()> {
        self.check_config()?;
        self.check_pending_move(source)?;
        if self.recovery_required || native::has_pending_recovery(&self.read.root)? {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "path mutations are gated",
            ));
        }
        if self.drafts.get(source).is_some_and(Draft::dirty) {
            return Err(Error::new(
                "E-PATH-DIRTY",
                "Save / Don't Save / Cancel required for this source",
            ));
        }
        if self.uncertain_paths().iter().any(|p| p == source) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "source outcome must be rechecked",
            ));
        }
        if !self.read.sources.contains_key(source) {
            return Err(Error::new("E-SOURCE-MISSING", source));
        }
        Ok(())
    }
    pub fn path_move_choices(&self, source: &str) -> Result<serde_json::Value> {
        self.check_config()?;
        let snapshot = native::capture(&self.read.root, &self.read.roots, source)?;
        let (index, root) = self
            .read
            .roots
            .iter()
            .enumerate()
            .find(|(_, r)| snapshot.physical.starts_with(r))
            .ok_or_else(|| Error::new("E-PATH-SCOPE", "source root required"))?;
        let folders = self
            .read
            .folders
            .iter()
            .filter(|p| self.read.root.join(p).starts_with(root))
            .collect::<Vec<_>>();
        Ok(
            serde_json::json!({"root":self.read.config.sources.roots[index],"folders":folders,"filename":std::path::Path::new(source).file_name().unwrap().to_string_lossy()}),
        )
    }
    pub fn prepare_path_move(&mut self, source: &str, destination: &str) -> Result<MoveReview> {
        self.path_gate(source)?;
        self.check_pending_move(destination)?;
        let actual = native::capture(&self.read.root, &self.read.roots, source)?;
        let expected = self.snapshots.get(source).unwrap_or(&actual);
        if actual.bytes != self.read.sources[source].bytes {
            return Err(Error::new(
                "E-SOURCE-CONFLICT",
                "source changed; refresh before requesting Rename / Move",
            ));
        }
        let plan = native::prepare_move(
            &self.read.root,
            &self.read.roots,
            source,
            destination,
            expected,
        )?;
        let token = NEXT_MOVE.fetch_add(1, Ordering::Relaxed);
        let review = MoveReview {
            token: token.to_string(),
            source: source.into(),
            destination: destination.into(),
        };
        self.move_plans.insert(token, plan);
        while self.move_plans.len() > 16 {
            self.move_plans.pop_first();
        }
        Ok(review)
    }
    pub fn apply_path_move(&mut self, token: &str) -> Result<WriteResult> {
        self.apply_path_move_with_fault(token, Fault::None)
    }
    pub fn apply_path_move_with_fault(&mut self, token: &str, fault: Fault) -> Result<WriteResult> {
        let token = token
            .parse::<u64>()
            .map_err(|_| Error::new("E-PATH-PLAN", "invalid move token"))?;
        let plan = self.move_plans.get(&token).cloned().ok_or_else(|| {
            Error::new(
                "E-PATH-PLAN",
                "move review is unavailable; request a new review explicitly",
            )
        })?;
        self.path_gate(&plan.source)?;
        self.check_pending_move(&plan.destination)?;
        let mut result = native::commit_move(&plan, fault, || self.check_config());
        self.move_plans.remove(&token);
        // Native completion and client acknowledgement are separate. Retain a
        // bounded observation witness so a lost IPC reply can be explicitly
        // rechecked without repeating the write or inventing its outcome.
        self.move_observations.insert(token, plan.clone());
        while self.move_observations.len() > 16 {
            self.move_observations.pop_first();
        }
        if result.outcome == Outcome::Success
            && let Err(e) = self.accept_move(&plan)
        {
            result = WriteResult::new(&plan.source, Outcome::OutcomeUnknown, e.to_string());
        }
        if result.outcome == Outcome::OutcomeUnknown {
            self.pending_moves.insert(token, plan);
        }
        Ok(result)
    }
    pub fn recheck_path_move(&mut self, token: &str) -> Result<WriteResult> {
        let token = token
            .parse::<u64>()
            .map_err(|_| Error::new("E-PATH-PLAN", "invalid move token"))?;
        let plan = self
            .pending_moves
            .get(&token)
            .or_else(|| self.move_observations.get(&token))
            .cloned()
            .ok_or_else(|| Error::new("E-PATH-NOT-PENDING", "no uncertain path mutation"))?;
        let mut result = native::observe_move(&plan);
        if result.outcome == Outcome::Success
            && let Err(e) = self.accept_move(&plan)
        {
            result = WriteResult::new(&plan.source, Outcome::OutcomeUnknown, e.to_string());
        }
        if result.outcome != Outcome::OutcomeUnknown {
            self.pending_moves.remove(&token);
        } else {
            self.pending_moves.insert(token, plan);
        }
        Ok(result)
    }
    fn accept_move(&mut self, plan: &native::MovePlan) -> Result<()> {
        let actual = native::capture(&self.read.root, &self.read.roots, &plan.destination)?;
        if !plan.destination_matches(&actual) {
            return Err(Error::new(
                "E-PATH-OBSERVATION",
                "destination changed after rename",
            ));
        }
        let previous = self.read.sources.get(&plan.source).cloned();
        let document = previous
            .as_ref()
            .and_then(|s| s.document.clone())
            .filter(|doc| doc.bytes == actual.bytes);
        let parsed = if let Some(doc) = document {
            Ok(doc)
        } else {
            Document::parse(actual.bytes.clone()).map(Arc::new)
        };
        let mut project = (*self.read).clone();
        project.sources.remove(&plan.source);
        project.sources.insert(
            plan.destination.clone(),
            Arc::new(Source {
                path: plan.destination.clone(),
                physical: actual.physical.clone(),
                bytes: actual.bytes.clone(),
                identity: actual.content.clone(),
                kind: parsed
                    .as_ref()
                    .ok()
                    .and_then(|d| d.root.get("kind"))
                    .and_then(|n| n.text().ok())
                    .map(str::to_owned),
                binding: parsed
                    .as_ref()
                    .ok()
                    .and_then(|d| d.root.get("table"))
                    .and_then(|n| n.text().ok())
                    .map(str::to_owned),
                document: parsed.as_ref().ok().cloned(),
                error: parsed.err().map(|e| e.to_string()),
            }),
        );
        project.rebuild_declarations();
        project.generation += 1;
        self.read = Arc::new(project);
        self.snapshots.remove(&plan.source);
        self.snapshots
            .insert(plan.destination.clone(), actual.clone());
        if let Some(mut draft) = self.drafts.remove(&plan.source) {
            draft.base = actual;
            draft.revision += 1;
            draft.outcome = None;
            draft.external = None;
            self.drafts.insert(plan.destination.clone(), draft);
        }
        if let Some(view) = self.views.remove(&plan.source) {
            self.views.insert(plan.destination.clone(), view);
        }
        self.search_indexes.remove(&plan.source);
        self.authoring_views.remove(&plan.source);
        self.authoring_views.remove(&plan.destination);
        self.unavailable.remove(&plan.source);
        self.unavailable.remove(&plan.destination);
        for source in self.last_source.values_mut() {
            if *source == plan.source {
                *source = plan.destination.clone();
            }
        }
        self.generation += 1;
        self.external_version += 1;
        self.diagnostics_pending = true;
        Ok(())
    }
    pub fn discard_source(&mut self, path: &str) -> Result<()> {
        self.check_config()?;
        self.check_pending_move(path)?;
        if self.uncertain_paths().iter().any(|source| source == path) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "Recheck source outcome before discard",
            ));
        }
        if self.recovery_required || native::has_pending_recovery(&self.read.root)? {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "authoring mutations are gated",
            ));
        }
        let actual = native::capture(&self.read.root, &self.read.roots, path)?;
        let parsed = Document::parse(actual.bytes.clone()).map(Arc::new);
        self.drafts.remove(path);
        self.authoring_views.remove(path);
        self.search_indexes.remove(path);
        self.replace_read(
            path,
            &actual,
            parsed.as_ref().ok().cloned(),
            parsed.err().map(|e| e.to_string()),
        );
        self.snapshots.insert(path.into(), actual);
        self.external_version += 1;
        Ok(())
    }
}
