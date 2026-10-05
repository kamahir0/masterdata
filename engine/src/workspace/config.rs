use super::*;
use crate::config;

impl Workspace {
    pub fn config_dirty(&self) -> bool {
        self.configuration.dirty()
    }
    pub fn config_uncertain(&self) -> bool {
        self.configuration.uncertain()
    }
    pub fn protected(&self) -> bool {
        self.config_dirty()
            || self.config_uncertain()
            || !self.dirty_paths().is_empty()
            || self.recovery_required
            || !self.uncertain_paths().is_empty()
    }
    pub fn config_view(&mut self, selected: Option<&str>, starts: [usize; 4]) -> config::View {
        match self.configuration.observe() {
            Ok(true) => self.reconcile_config(),
            Ok(false) => (),
            Err(error) => self.invalidate_environment(error),
        }
        if self.configuration.outcome == Some(Outcome::Conflict) {
            self.invalidate_environment(Error::new(
                "E-CONFIG-CONFLICT",
                "external config changed; Compare / Reload required",
            ));
        }
        let mut v = self.configuration.view(selected, starts);
        if self.recovery_required {
            v.editable = false;
            v.reason = Some("Recovery Required: config edits and Save are gated".into());
            v.can_add_profile = false;
            v.can_add_target = false;
        }
        v
    }
    pub fn edit_config(&mut self, revision: u64, operation: config::Operation) -> Result<()> {
        self.config_write_gate()?;
        self.configuration.edit(revision, operation)
    }
    fn config_write_gate(&self) -> Result<()> {
        if self.recovery_required || native::has_pending_recovery(&self.read.root)? {
            return Err(Error::new("E-RECOVERY-REQUIRED", "config writes are gated"));
        }
        Ok(())
    }
    pub fn save_config(&mut self, revision: u64, fault: Fault) -> Result<WriteResult> {
        self.config_write_gate()?;
        let result = self.configuration.save(revision, fault)?;
        if result.outcome == Outcome::Success {
            self.reconcile_config();
        }
        Ok(result)
    }
    pub fn recheck_config(&mut self) -> Result<WriteResult> {
        let result = self.configuration.recheck()?;
        if result.outcome == Outcome::Success {
            self.reconcile_config();
        }
        Ok(result)
    }
    pub fn reload_config(&mut self, revision: u64, discard_authorized: bool) -> Result<()> {
        self.configuration.reload(revision, discard_authorized)?;
        self.reconcile_config();
        Ok(())
    }
    fn reconcile_config(&mut self) {
        let saved = &self.configuration.base;
        let config = match project::config(&saved.bytes) {
            Ok(c) => c,
            Err(error) => {
                self.invalidate_environment(error);
                return;
            }
        };
        let old = &self.read.config;
        let binding_unchanged = self.config_binding_available
            && old.project.id == config.project.id
            && old.sources.roots == config.sources.roots
            && old.build.artifact_dir == config.build.artifact_dir
            && old.build.cache == config.build.cache
            && config
                .sources
                .roots
                .iter()
                .zip(&self.read.roots)
                .all(|(binding, root)| {
                    self.read
                        .root
                        .join(binding)
                        .canonicalize()
                        .is_ok_and(|p| p == *root)
                });
        if !binding_unchanged {
            self.invalidate_environment(Error::new(
                "E-CONFIG-BINDING",
                "saved config requires explicit Project Reload; existing drafts are retained",
            ));
            return;
        }
        let changed = self.read.config_bytes != saved.bytes;
        let project = Arc::make_mut(&mut self.read);
        project.config = config;
        project.config_bytes = saved.bytes.clone();
        project.config_identity = saved.content.clone();
        // Config repair cannot clear an unrelated watcher/permission failure.
        let repaired = self
            .environment_error
            .as_ref()
            .is_some_and(|e| e.code.starts_with("E-CONFIG"));
        if repaired {
            self.environment_error = None;
        }
        if changed || repaired {
            self.generation += 1;
            self.external_version += 1;
            self.diagnostics_pending = true;
        }
    }
    pub fn save_all_with_config_fault(
        &mut self,
        config_fault: Fault,
        yaml_fault: Fault,
    ) -> Result<Vec<WriteResult>> {
        self.config_write_gate()?;
        let paths = self.dirty_paths();
        let mut results = vec![];
        if self.config_dirty() || self.config_uncertain() {
            let result = match self.save_config(self.configuration.revision, config_fault) {
                Ok(r) => r,
                Err(e) => WriteResult::new(
                    config::PATH,
                    if self.config_uncertain() {
                        Outcome::OutcomeUnknown
                    } else {
                        Outcome::Failure
                    },
                    e.to_string(),
                ),
            };
            let success = result.outcome == Outcome::Success;
            results.push(result);
            if !success {
                results.extend(paths.iter().map(|p| {
                    WriteResult::new(p, Outcome::NotAttempted, "config Save did not succeed")
                }));
                return Ok(results);
            }
        }
        if let Err(error) = self.check_config() {
            results.extend(
                paths
                    .iter()
                    .map(|p| WriteResult::new(p, Outcome::NotAttempted, error.to_string())),
            );
            if paths.is_empty() {
                results.push(WriteResult::new(
                    config::PATH,
                    Outcome::NotAttempted,
                    error.to_string(),
                ));
            }
            return Ok(results);
        }
        results.extend(self.save_paths(paths, yaml_fault)?);
        Ok(results)
    }
}
