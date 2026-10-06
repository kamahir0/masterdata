use super::*;
use crate::table_declaration::{self, Command, Detail};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDeclarationDetail {
    pub detail: Detail,
    pub dirty_sources: Vec<String>,
    pub config_dirty: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDeclarationReview {
    pub plan: MigrationReview,
    pub before: Detail,
    pub after: Detail,
    pub dirty_dependencies: Vec<String>,
    pub config_dirty: bool,
}
impl Workspace {
    fn declaration_dirty(&self, detail: &Detail) -> Vec<String> {
        detail
            .dependency_sources
            .iter()
            .cloned()
            .chain(self.read.record_sources(&detail.table))
            .filter(|p| self.drafts.get(p).is_some_and(Draft::dirty))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn table_declaration_detail(
        &mut self,
        source: &str,
        table: &str,
    ) -> Result<TableDeclarationDetail> {
        self.check_config()?;
        self.refresh_source(source)?;
        if self
            .read
            .tables
            .get(table)
            .is_none_or(|t| t.source != source)
        {
            return Err(Error::new(
                "E-SOURCE-BINDING",
                "Table schema binding changed; select its current source",
            ));
        }
        let mut refreshed = BTreeSet::from([source.to_owned()]);
        loop {
            let detail = table_declaration::detail(&self.read, table)?;
            let required = detail
                .dependency_sources
                .into_iter()
                .filter(|p| !refreshed.contains(p))
                .collect::<Vec<_>>();
            if required.is_empty() {
                break;
            }
            for path in required {
                refreshed.insert(path.clone());
                // Failed refresh invalidates that read source. The detail shows
                // an unresolved declaration rather than an obsolete target key.
                let _ = self.refresh_source(&path);
            }
        }
        let mut saved = (*self.read).clone();
        saved.rebuild_declarations();
        let detail = table_declaration::detail(&saved, table)?;
        Ok(TableDeclarationDetail {
            dirty_sources: self.declaration_dirty(&detail),
            config_dirty: self.config_dirty(),
            detail,
        })
    }
    pub fn prepare_table_declaration(
        &mut self,
        command: Command,
    ) -> Result<TableDeclarationReview> {
        self.migration_gate(std::iter::empty())?;
        // Derive from the immutable saved read generation. Native preparation
        // witnesses every actual source/config byte and namespace afterwards;
        // a cache mismatch rejects the Plan, never authorizes its commit.
        let mut saved = (*self.read).clone();
        saved.rebuild_declarations();
        let before = table_declaration::detail(&saved, &command.table)?;
        let plan = table_declaration::derive(&saved, command)?;
        let mut transformed = saved.clone();
        for (path, candidate) in &plan.candidates {
            let mut changed = (**transformed.sources.get(path).unwrap()).clone();
            changed.bytes = candidate.after.bytes.clone();
            changed.identity = candidate.after.identity.clone();
            changed.document = Some(candidate.after.clone());
            transformed.sources.insert(path.clone(), Arc::new(changed));
        }
        transformed.rebuild_declarations();
        let after = table_declaration::detail(&transformed, &before.table)?;
        let dirty_dependencies = self
            .declaration_dirty(&before)
            .into_iter()
            .chain(self.declaration_dirty(&after))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let plan = self.register_migration(native::SourceSetPlan::prepare(&saved, plan)?);
        Ok(TableDeclarationReview {
            plan,
            before,
            after,
            dirty_dependencies,
            config_dirty: self.config_dirty(),
        })
    }
}
