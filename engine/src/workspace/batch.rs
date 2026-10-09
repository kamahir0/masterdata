use super::*;
use crate::{
    clipboard,
    semantic::Typed,
    source::{derive_patch, matches_value},
};

impl Workspace {
    pub fn check_authoring_input(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
    ) -> Result<()> {
        self.authoring_context(path, revision, generation, false)
            .map(|_| ())
    }
    pub(super) fn batch_context(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
    ) -> Result<(Arc<Table>, semantic::Types)> {
        self.authoring_context(path, revision, generation, true)
    }
    pub(super) fn authoring_context(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        mutation: bool,
    ) -> Result<(Arc<Table>, semantic::Types)> {
        if mutation && (self.recovery_required || native::has_pending_recovery(&self.read.root)?) {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source mutation is gated",
            ));
        }
        self.check_authoring_generation(path, generation)?;
        self.check_config()?;
        self.refresh_source(path)?;
        let binding = self
            .read
            .sources
            .get(path)
            .and_then(|s| s.binding.clone())
            .ok_or_else(|| Error::new("E-TABLE-MISSING", path))?;
        let table = self.current_table(&binding)?;
        for dependency in self.read.dependencies(&table) {
            if dependency != path {
                self.refresh_source(&dependency)?;
            }
        }
        self.ensure_draft(path)?;
        self.check_authoring_generation(path, generation)?;
        let draft = &self.drafts[path];
        if draft.revision != revision {
            return Err(Error::new(
                "E-DRAFT-STALE",
                "source/schema/type changed during preflight",
            ));
        }
        if mutation && draft.outcome == Some(Outcome::OutcomeUnknown) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "fresh observation required",
            ));
        }
        Ok((self.current_table(&binding)?, self.current_types()?))
    }

    pub fn paste_at(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        anchor: &str,
        field: &str,
        text: &str,
    ) -> Result<bool> {
        let decoded = clipboard::decode(text)?;
        let (table, types) = self.batch_context(path, revision, generation)?;
        let indices = self.query_rows(path, &table, &types)?;
        let d = &self.drafts[path];
        let first_row = indices
            .iter()
            .position(|(p, _)| d.row_ids[*p] == anchor)
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "paste anchor missing"))?;
        let first_field = table
            .fields
            .iter()
            .position(|f| f.name == field)
            .ok_or_else(|| Error::new("E-FIELD-MISSING", field))?;
        let rows = indices
            .get(first_row..first_row + decoded.len())
            .ok_or_else(|| Error::new("E-BATCH-RANGE", "paste exceeds source rows"))?
            .iter()
            .map(|(p, _)| d.row_ids[*p].clone())
            .collect::<Vec<_>>();
        let fields = table
            .fields
            .get(first_field..first_field + decoded[0].len())
            .ok_or_else(|| Error::new("E-BATCH-RANGE", "paste exceeds schema fields"))?
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>();
        self.paste(path, revision, generation, &rows, &fields, text)
    }

    pub fn paste(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        rows: &[String],
        fields: &[String],
        text: &str,
    ) -> Result<bool> {
        let values = clipboard::decode(text)?;
        if rows.is_empty()
            || fields.is_empty()
            || rows.len() != values.len()
            || fields.len() != values[0].len()
            || rows.iter().collect::<BTreeSet<_>>().len() != rows.len()
            || fields.iter().collect::<BTreeSet<_>>().len() != fields.len()
        {
            return Err(Error::new(
                "E-BATCH-RANGE",
                "exact rectangle with unique targets required",
            ));
        }
        let (table, types) = self.batch_context(path, revision, generation)?;
        let draft = &self.drafts[path];
        let preflight_stage = crate::instrument::span("pastePreflight");
        let mut patches = vec![];
        let mut expected = vec![];
        for (row, values) in rows.iter().zip(values) {
            let index = draft.active_index(row)?;
            for (field, text) in fields.iter().zip(values) {
                let f = table
                    .fields
                    .iter()
                    .find(|f| &f.name == field)
                    .ok_or_else(|| Error::new("E-FIELD-MISSING", field))?;
                let shape = semantic::shape(f, &types)?;
                scalar_capability(&shape)?;
                if !draft.added.contains(row)
                    && (table.primary.fields.contains(field)
                        || table.secondary.iter().any(|k| k.fields.contains(field)))
                {
                    return Err(Error::new(
                        "E-BATCH-READONLY",
                        "existing key is not a paste target",
                    ));
                }
                let node = draft
                    .document
                    .occurrence_value(index + 1, std::slice::from_ref(field))?;
                let value = semantic::authoring_input(&shape, &text);
                derive_patch(&draft.document, node, &value, &mut patches)?;
                expected.push((index + 1, field.clone(), value));
            }
        }
        drop(preflight_stage);
        // All localization and codec checks finish before touching the physical draft.
        let candidate = draft.document.patched(patches)?;
        let postcondition_stage = crate::instrument::span("pastePostcondition");
        for (index, field, value) in expected {
            if !matches_value(candidate.occurrence_value(index, &[field])?, &value) {
                return Err(Error::new(
                    "E-SOURCE-POSTCONDITION",
                    "paste candidate differs from exact targets",
                ));
            }
        }
        drop(postcondition_stage);
        let draft = self.drafts.get_mut(path).unwrap();
        let changed = draft.apply(candidate, draft.row_ids.clone());
        if changed {
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }

    pub fn copy(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        rows: &[String],
        fields: &[String],
    ) -> Result<String> {
        let (table, types) = self.authoring_context(path, revision, generation, false)?;
        let d = &self.drafts[path];
        let mut result = vec![];
        for row in rows {
            let index = d.active_index(row)?;
            let mut output = vec![];
            for field in fields {
                let f = table
                    .fields
                    .iter()
                    .find(|f| &f.name == field)
                    .ok_or_else(|| Error::new("E-FIELD-MISSING", field))?;
                scalar_capability(&semantic::shape(f, &types)?)?;
                let node = d
                    .document
                    .occurrence_value(index + 1, std::slice::from_ref(field))?;
                let typed = semantic::interpret(f, node, &types)
                    .map_err(|e| Error::new("E-COPY-VALUE", e.message))?;
                output.push(match typed {
                    Typed::Boolean(b) => b.to_string(),
                    Typed::Integer(s) | Typed::Float(s) | Typed::Text(s) | Typed::Enum(s) => s,
                    _ => {
                        return Err(Error::new(
                            "E-COPY-VALUE",
                            "null or complex value cannot be copied as scalar TSV",
                        ));
                    }
                });
            }
            result.push(output);
        }
        clipboard::encode(&result)
    }
}
fn scalar_capability(shape: &Shape) -> Result<()> {
    if shape.array || matches!(shape.category.as_str(), "custom" | "flags") {
        return Err(Error::new(
            "E-BATCH-SHAPE",
            "range authoring requires scalar leaf fields",
        ));
    }
    Ok(())
}
