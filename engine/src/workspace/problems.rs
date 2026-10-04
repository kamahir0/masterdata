use super::*;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemTarget {
    pub source: String,
    pub row: String,
    pub field: Option<String>,
    pub view_index: Option<usize>,
    pub editor_path: Option<Vec<String>>,
    pub focus_path: Vec<String>,
    pub editor_start: usize,
}

impl Workspace {
    pub fn problem_target(
        &mut self,
        source: &str,
        generation: u64,
        occurrence: Option<usize>,
        field_path: &[String],
    ) -> Result<Option<ProblemTarget>> {
        if generation != self.generation
            || self.diagnostics_generation != generation
            || self.diagnostics_pending
            || !self.diagnostics.iter().any(|d| {
                d.source == source && d.occurrence == occurrence && d.field_path == field_path
            })
        {
            return Err(Error::new("E-DRAFT-STALE", "diagnostic generation changed"));
        }
        let projection = self.select(source, 0, 0)?;
        if generation != self.generation {
            return Err(Error::new(
                "E-DRAFT-STALE",
                "source changed since diagnostic",
            ));
        }
        let Some(occurrence) = occurrence.filter(|n| *n > 0) else {
            return Ok(None);
        };
        if projection.source.as_deref() != Some(source) {
            return Ok(None);
        }
        let (table, types) =
            self.authoring_context(source, projection.revision, generation, false)?;
        let d = self.drafts.get_mut(source).unwrap();
        // accept_diagnostics maps the validation candidate's active ordinal to
        // the source presentation occurrence (including Pending Delete slots).
        // Search can have a different visible ordinal; the row identity cannot
        // be inferred from that ordinal or from a possibly duplicated key.
        let row = d
            .row_ids
            .get(occurrence - 1)
            .cloned()
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "diagnostic occurrence missing"))?;
        d.active_index(&row)?;
        let field = field_path
            .first()
            .filter(|name| table.fields.iter().any(|f| &f.name == *name))
            .cloned();
        let (focus_path, editor_start) = if field.is_some() {
            complex::diagnostic_path(d, &table, &types, &row, field_path)?
        } else {
            (vec![], 0)
        };
        let editor_path = field.as_ref().and_then(|name| {
            if focus_path.len() > 1 {
                Some(focus_path[..focus_path.len() - 1].to_vec())
            } else {
                let f = table.fields.iter().find(|f| &f.name == name)?;
                let s = semantic::shape(f, &types).ok()?;
                (s.array
                    || s.nullable
                    || matches!(s.category.as_str(), "custom" | "flags" | "enum"))
                .then(|| vec![name.clone()])
            }
        });
        let view_index = self.locate_row(source, &row)?;
        Ok(Some(ProblemTarget {
            source: source.into(),
            row,
            field,
            view_index,
            editor_path,
            focus_path,
            editor_start,
        }))
    }
}
