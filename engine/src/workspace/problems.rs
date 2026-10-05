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
        // accept_diagnostics maps the validation candidate's active ordinal to
        // the source presentation occurrence (including Pending Delete slots).
        // Search can have a different visible ordinal; the row identity cannot
        // be inferred from that ordinal or from a possibly duplicated key.
        let row = self.drafts[source]
            .row_ids
            .get(occurrence - 1)
            .cloned()
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "diagnostic occurrence missing"))?;
        self.target_for_row(source, &projection, row, field_path, false)
    }

    /// Build diagnostics describe saved inputs. Matching the physical base
    /// identifies an occurrence even after a draft changes its key or order.
    /// Current validation and dirty values are never replaced by this result.
    pub fn saved_problem_target(
        &mut self,
        diagnostic: &Diagnostic,
        config_identity: &str,
        inputs: &BTreeMap<String, String>,
    ) -> Result<Option<ProblemTarget>> {
        self.check_config()?;
        if self.read.config_identity != config_identity {
            return Err(Error::new(
                "E-BUILD-PROBLEM-STALE",
                "Build configuration differs; run Build again before locating a record",
            ));
        }
        let source = &diagnostic.source;
        let view = self.select_view(source, 0, 0)?;
        let SelectionProjection::Table(projection) = view else {
            return Ok(None);
        };
        // Selection refreshes only this source and its required schema/types.
        // Compare their observed bases, not dirty overlays or PK values.
        let dependencies = self.authoring_view(source)?;
        for (path, _, _) in dependencies.documents {
            let base = self
                .drafts
                .get(&path)
                .map(|d| &d.base)
                .or_else(|| self.snapshots.get(&path))
                .ok_or_else(|| Error::new("E-BUILD-PROBLEM-STALE", "editor base is unavailable"))?;
            if inputs.get(&path) != Some(&base.content) {
                return Err(Error::new(
                    "E-BUILD-PROBLEM-STALE",
                    "Build snapshot differs from the editor base; run Build again before locating a record",
                ));
            }
        }
        let Some(occurrence) = diagnostic.occurrence.filter(|n| *n > 0) else {
            return Ok(None);
        };
        if projection.source.as_deref() != Some(source) {
            return Ok(None);
        }
        let row = self.drafts[source]
            .saved_rows
            .get(occurrence - 1)
            .cloned()
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "saved occurrence is unavailable"))?;
        self.target_for_row(source, &projection, row, &diagnostic.field_path, true)
    }

    fn target_for_row(
        &mut self,
        source: &str,
        projection: &Projection,
        row: String,
        field_path: &[String],
        allow_pending: bool,
    ) -> Result<Option<ProblemTarget>> {
        let (table, types) =
            self.authoring_context(source, projection.revision, projection.generation, false)?;
        let d = self.drafts.get_mut(source).unwrap();
        let pending = allow_pending && d.pending.contains_key(&row);
        if !pending {
            d.active_index(&row)?;
        }
        let field = field_path
            .first()
            .filter(|name| table.fields.iter().any(|f| &f.name == *name))
            .cloned();
        let (focus_path, editor_start) = if field.is_some() && !pending {
            complex::diagnostic_path(d, &table, &types, &row, field_path)?
        } else {
            (vec![], 0)
        };
        let editor_path = field.as_ref().filter(|_| !pending).and_then(|name| {
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
