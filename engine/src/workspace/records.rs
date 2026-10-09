use super::*;
use crate::source::{Node, Style, matches_value};

impl Workspace {
    pub fn next_row(&mut self, path: &str, row: &str) -> Result<Option<String>> {
        self.ensure_draft(path)?;
        let d = &self.drafts[path];
        let at = d.active_index(row)?;
        Ok(d.row_ids
            .iter()
            .filter(|id| !d.pending.contains_key(*id))
            .nth(at + 1)
            .cloned())
    }
    pub fn nudge_row(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
        delta: isize,
    ) -> Result<bool> {
        self.batch_context(path, revision, generation)?;
        let d = &self.drafts[path];
        let at = d.active_index(row)?;
        let mut order = d
            .row_ids
            .iter()
            .filter(|id| !d.pending.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        if !matches!(delta, -1 | 1) {
            return Err(Error::new(
                "E-REORDER-POSITION",
                "one neighbouring position required",
            ));
        }
        let target = at as isize + delta;
        if target < 0 || target >= order.len() as isize {
            return Ok(false);
        }
        order.swap(at, target as usize);
        self.reorder_rows(path, revision, generation, &order)
    }
    pub fn move_row(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
        before: Option<&str>,
    ) -> Result<bool> {
        self.batch_context(path, revision, generation)?;
        let d = &self.drafts[path];
        d.active_index(row)?;
        if let Some(before) = before {
            d.active_index(before)?;
            if before == row {
                return Ok(false);
            }
        }
        let mut order = d
            .row_ids
            .iter()
            .filter(|id| !d.pending.contains_key(*id) && *id != row)
            .cloned()
            .collect::<Vec<_>>();
        let at = before
            .map(|before| order.iter().position(|id| id == before).unwrap())
            .unwrap_or(order.len());
        order.insert(at, row.into());
        self.reorder_rows(path, revision, generation, &order)
    }
    pub fn add_row(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        before: Option<&str>,
    ) -> Result<String> {
        let (table, types) = self.batch_context(path, revision, generation)?;
        for field in &table.fields {
            semantic::shape(field, &types)?;
        }
        let d = &self.drafts[path];
        let records = record_sequence(&d.document)?;
        if records.style != Style::Block && !records.items()?.is_empty() {
            return Err(Error::new(
                "E-SOURCE-UNSAFE",
                "new record mapping requires a block sequence",
            ));
        }
        let position = before
            .map(|id| {
                d.row_ids
                    .iter()
                    .position(|r| r == id)
                    .ok_or_else(|| Error::new("E-LOCATOR-STALE", "insert occurrence missing"))
            })
            .transpose()?
            .unwrap_or(d.row_ids.len());
        let at = d
            .row_ids
            .iter()
            .take(position)
            .filter(|id| !d.pending.contains_key(*id))
            .count();
        let value = Value::Mapping(
            table
                .fields
                .iter()
                .map(|f| (f.name.clone(), Value::Null))
                .collect(),
        );
        let candidate = d.document.insert_sequence(records, at, &value)?;
        verify_insert(records, record_sequence(&candidate)?, at, &value)?;
        let origin = d.origin.clone().unwrap_or_else(|| {
            Arc::new(AdditionOrigin {
                document: d.document.clone(),
                rows: d.row_ids.clone(),
            })
        });
        let id = format!(
            "draft:{}:{}:{}",
            d.base.content,
            self.generation,
            d.revision + 1
        );
        let mut rows = (*d.row_ids).clone();
        rows.insert(position, id.clone());
        let d = self.drafts.get_mut(path).unwrap();
        d.apply(candidate, Arc::new(rows));
        Arc::make_mut(&mut d.added).insert(id.clone());
        d.origin = Some(origin);
        self.generation += 1;
        self.diagnostics_pending = true;
        Ok(id)
    }

    pub fn delete_row(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
    ) -> Result<bool> {
        self.batch_context(path, revision, generation)?;
        let d = &self.drafts[path];
        let at = d.active_index(row)?;
        let records = record_sequence(&d.document)?;
        // A pending record retains its exact block bytes for an explicit Undo Delete.
        // Flow-record removal is rejected rather than storing a lossy reconstruction.
        if records.style != Style::Block {
            return Err(Error::new(
                "E-SOURCE-UNSAFE",
                "Pending Delete requires safely owned block item bytes",
            ));
        }
        let range = d.document.item_range(records, at, true)?;
        let mut pending = PendingRecord {
            node: records.items()?[at].value.clone(),
            bytes: Arc::from(&d.document.bytes[range.clone()]),
            position: range.start,
        };
        let mut candidate = d.document.remove_sequence(records, at)?;
        pending.position = candidate.map_anchor(pending.position);
        let expected = records
            .items()?
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != at)
            .map(|(_, i)| i.value.value())
            .collect::<Vec<_>>();
        if !matches_value(record_sequence(&candidate)?, &Value::Sequence(expected)) {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "Delete altered another occurrence",
            ));
        }
        let added = d.added.contains(row);
        let mut rows = (*d.row_ids).clone();
        if added {
            rows.retain(|id| id != row);
            if d.added.len() == 1
                && let Some(origin) = &d.origin
            {
                candidate =
                    restore_cancelled_addition(&candidate, origin, &rows, d.pending.is_empty())?;
            }
        }
        let d = self.drafts.get_mut(path).unwrap();
        let changed = d.apply(candidate, Arc::new(rows));
        if added {
            Arc::make_mut(&mut d.added).remove(row);
            Arc::make_mut(&mut d.tag_origins).remove(row);
            if d.added.is_empty() {
                d.origin = None;
            }
        } else {
            Arc::make_mut(&mut d.pending).insert(row.into(), pending);
        }
        if changed {
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }

    pub fn undo_delete(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
    ) -> Result<bool> {
        self.batch_context(path, revision, generation)?;
        let d = &self.drafts[path];
        let saved = d
            .pending
            .get(row)
            .ok_or_else(|| Error::new("E-ROW-NOT-DELETED", "Pending Delete occurrence required"))?;
        let at = d
            .row_ids
            .iter()
            .take_while(|id| *id != row)
            .filter(|id| !d.pending.contains_key(*id))
            .count();
        let records = record_sequence(&d.document)?;
        let candidate = d
            .document
            .insert_owned_item(records, at, saved.position, &saved.bytes)?;
        verify_insert(
            records,
            record_sequence(&candidate)?,
            at,
            &saved.node.value(),
        )?;
        let d = self.drafts.get_mut(path).unwrap();
        let changed = d.apply(candidate, d.row_ids.clone());
        Arc::make_mut(&mut d.pending).remove(row);
        if changed {
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }

    pub fn reorder_rows(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        order: &[String],
    ) -> Result<bool> {
        self.batch_context(path, revision, generation)?;
        if self.views.get(path).is_some_and(|v| !v.search.is_empty()) {
            return Err(Error::new(
                "E-REORDER-VIEW",
                "clear Search before source-position operations",
            ));
        }
        let d = &self.drafts[path];
        let old = d
            .row_ids
            .iter()
            .filter(|id| !d.pending.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        let positions = permutation(&old, order)?;
        let records = record_sequence(&d.document)?;
        let candidate = d.document.reorder_sequence(records, &positions)?;
        let expected = positions
            .iter()
            .map(|i| records.items().unwrap()[*i].value.value())
            .collect();
        if !matches_value(record_sequence(&candidate)?, &Value::Sequence(expected)) {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "reorder differs from exact occurrence permutation",
            ));
        }
        let mut next = order.iter();
        let rows = d
            .row_ids
            .iter()
            .map(|id| {
                if d.pending.contains_key(id) {
                    id.clone()
                } else {
                    next.next().unwrap().clone()
                }
            })
            .collect();
        let d = self.drafts.get_mut(path).unwrap();
        let changed = d.apply(candidate, Arc::new(rows));
        if changed {
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }

    pub fn reorder_columns(&mut self, path: &str, revision: u64, order: &[String]) -> Result<bool> {
        self.reorder_columns_at(path, revision, self.generation, order)
    }
    pub fn reorder_columns_at(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        order: &[String],
    ) -> Result<bool> {
        self.check_authoring_generation(path, generation)?;
        if self.recovery_required || native::has_pending_recovery(&self.read.root)? {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source mutation is gated",
            ));
        }
        self.check_config()?;
        self.refresh_source(path)?;
        self.ensure_draft(path)?;
        self.check_authoring_generation(path, generation)?;
        let d = &self.drafts[path];
        if d.revision != revision {
            return Err(Error::new("E-DRAFT-STALE", "schema revision changed"));
        }
        if d.outcome == Some(Outcome::OutcomeUnknown) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "fresh observation required",
            ));
        }
        let table = semantic::parse_table(&d.document, path)?;
        let positions = permutation(
            &table
                .fields
                .iter()
                .map(|f| f.name.clone())
                .collect::<Vec<_>>(),
            order,
        )?;
        let fields = d
            .document
            .root
            .get("fields")
            .ok_or_else(|| Error::new("E-SCHEMA-FIELDS", "fields sequence required"))?;
        let candidate = d.document.reorder_sequence(fields, &positions)?;
        let after = semantic::parse_table(&candidate, path)?;
        if after.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
            != order.iter().collect::<Vec<_>>()
        {
            return Err(Error::new("E-SOURCE-POSTCONDITION", "column order differs"));
        }
        let d = self.drafts.get_mut(path).unwrap();
        let changed = d.apply(candidate, d.row_ids.clone());
        if changed {
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }
}

fn record_sequence(document: &Document) -> Result<&Node> {
    let records = document
        .root
        .get("records")
        .ok_or_else(|| Error::new("E-RECORD-SOURCE", "explicit record-bearing source required"))?;
    if !records.safe {
        return Err(Error::new("E-SOURCE-UNSAFE", "unsafe records region"));
    }
    records.items()?;
    Ok(records)
}
pub(super) fn permutation(old: &[String], order: &[String]) -> Result<Vec<usize>> {
    if old.len() != order.len() || order.iter().collect::<BTreeSet<_>>().len() != old.len() {
        return Err(Error::new(
            "E-REORDER-IDENTITY",
            "exact occurrence permutation required",
        ));
    }
    order
        .iter()
        .map(|id| {
            old.iter()
                .position(|s| s == id)
                .ok_or_else(|| Error::new("E-REORDER-IDENTITY", "unknown occurrence"))
        })
        .collect()
}
fn verify_insert(before: &Node, after: &Node, at: usize, value: &Value) -> Result<()> {
    let mut expected = before
        .items()?
        .iter()
        .map(|i| i.value.value())
        .collect::<Vec<_>>();
    expected.insert(at, value.clone());
    if !matches_value(after, &Value::Sequence(expected)) {
        return Err(Error::new(
            "E-SOURCE-POSTCONDITION",
            "insert changed unrelated items",
        ));
    }
    Ok(())
}
fn restore_cancelled_addition(
    current: &Document,
    origin: &AdditionOrigin,
    rows: &[String],
    no_pending: bool,
) -> Result<Document> {
    let original = &origin.document;
    let old = record_sequence(original)?;
    let now = record_sequence(current)?;
    if old.items()?.is_empty() && now.items()?.is_empty() {
        let old_start = original.line_start(old.span.start);
        let now_start = current.line_start(now.span.start);
        if original.bytes[old_start..old.span.start].trim().is_empty() {
            return current.patched(vec![Patch {
                span: now_start..current.line_end(now.span.end),
                text: original.bytes[old_start..original.line_end(old.span.end)].to_owned(),
            }]);
        }
        let key = current
            .root
            .members()?
            .iter()
            .find(|m| m.name == "records")
            .unwrap();
        let header = current.line_start(key.key.start)..current.line_end(key.key.end);
        if !current.bytes[now_start..now.span.start].trim().is_empty() || now_start < header.end {
            return Err(Error::new(
                "E-SOURCE-UNSAFE",
                "empty container restoration ambiguous",
            ));
        }
        return current.patched(vec![
            Patch {
                span: header,
                text: original.bytes[old_start..original.line_end(old.span.end)].to_owned(),
            },
            Patch {
                span: now_start..current.line_end(now.span.end),
                text: String::new(),
            },
        ]);
    }
    // Appending to an unterminated final item creates a separating newline. Remove
    // that newline only when the same final physical occurrence is still at EOF.
    if no_pending
        && rows == origin.rows.as_slice()
        && !original.bytes.ends_with('\n')
        && !old.items()?.is_empty()
        && !now.items()?.is_empty()
        && original.item_range(old, old.items()?.len() - 1, true)?.end == original.bytes.len()
        && current.item_range(now, now.items()?.len() - 1, true)?.end == current.bytes.len()
        && current.bytes.ends_with('\n')
    {
        let length = current.newline().len();
        return current.patched(vec![Patch {
            span: current.bytes.len() - length..current.bytes.len(),
            text: String::new(),
        }]);
    }
    Ok(current.clone())
}
