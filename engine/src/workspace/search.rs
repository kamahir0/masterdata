use super::*;
use crate::semantic::Typed;

pub(super) type OccurrenceWindow = Arc<Vec<(usize, Option<usize>)>>;

#[derive(Clone, Debug)]
pub(super) struct SearchIndex {
    revision: u64,
    generation: u64,
    query: String,
    pub rows: OccurrenceWindow,
}
impl Workspace {
    pub fn set_search(&mut self, path: &str, query: &str) -> Result<()> {
        if !self.read.sources.contains_key(path) {
            return Err(Error::new("E-SOURCE-MISSING", path));
        }
        self.views.entry(path.into()).or_default().search = query.into();
        Ok(())
    }
    pub(super) fn query_rows(
        &mut self,
        path: &str,
        table: &Table,
        types: &semantic::Types,
    ) -> Result<OccurrenceWindow> {
        let query = self
            .views
            .get(path)
            .map(|s| s.search.as_str())
            .unwrap_or("");
        let d = &self.drafts[path];
        if let Some(index) = self.search_indexes.get(path)
            && index.revision == d.revision
            && index.generation == self.generation
            && index.query == query
        {
            return Ok(index.rows.clone());
        }
        let records = d.document.records()?;
        let scalar = table
            .fields
            .iter()
            .filter(|f| {
                semantic::shape(f, types)
                    .is_ok_and(|s| !s.array && s.category != "custom" && s.category != "flags")
            })
            .collect::<Vec<_>>();
        let mut active = 0;
        let mut rows = vec![];
        for (position, id) in d.row_ids.iter().enumerate() {
            if d.pending.contains_key(id) {
                if query.is_empty() {
                    rows.push((position, None));
                }
                continue;
            }
            let record = &records[active].value;
            let index = active;
            active += 1;
            let matched = query.is_empty()
                || scalar.iter().any(|f| {
                    record
                        .get(&f.name)
                        .and_then(|n| semantic::interpret(f, n, types).ok())
                        .and_then(copy_text)
                        .is_some_and(|s| s.contains(query))
                });
            if matched {
                rows.push((position, Some(index)));
            }
        }
        let rows = Arc::new(rows);
        self.search_indexes.insert(
            path.into(),
            SearchIndex {
                revision: d.revision,
                generation: self.generation,
                query: query.into(),
                rows: rows.clone(),
            },
        );
        Ok(rows)
    }
    pub fn locate_row(&mut self, path: &str, id: &str) -> Result<Option<usize>> {
        let binding = self
            .read
            .sources
            .get(path)
            .and_then(|s| s.binding.clone())
            .ok_or_else(|| Error::new("E-TABLE-MISSING", path))?;
        self.ensure_draft(path)?;
        let table = self.current_table(&binding)?;
        let types = self.current_types()?;
        let rows = self.query_rows(path, &table, &types)?;
        Ok(rows
            .iter()
            .position(|(position, _)| self.drafts[path].row_ids[*position] == id))
    }
    pub fn copy_between(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        rows: [&str; 2],
        fields: [&str; 2],
    ) -> Result<String> {
        let [anchor, focus] = rows;
        let [first, last] = fields;
        let (table, types) = self.authoring_context(path, revision, generation, false)?;
        let indices = self.query_rows(path, &table, &types)?;
        let d = &self.drafts[path];
        let a = indices
            .iter()
            .position(|(p, _)| d.row_ids[*p] == anchor)
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "copy anchor missing"))?;
        let b = indices
            .iter()
            .position(|(p, _)| d.row_ids[*p] == focus)
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "copy focus missing"))?;
        let c = table
            .fields
            .iter()
            .position(|f| f.name == first)
            .ok_or_else(|| Error::new("E-FIELD-MISSING", first))?;
        let e = table
            .fields
            .iter()
            .position(|f| f.name == last)
            .ok_or_else(|| Error::new("E-FIELD-MISSING", last))?;
        let rows = indices[a.min(b)..=a.max(b)]
            .iter()
            .map(|(p, _)| d.row_ids[*p].clone())
            .collect::<Vec<_>>();
        let fields = table.fields[c.min(e)..=c.max(e)]
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>();
        self.copy(path, revision, generation, &rows, &fields)
    }
}
fn copy_text(value: Typed) -> Option<String> {
    match value {
        Typed::Text(s) | Typed::Integer(s) | Typed::Float(s) | Typed::Enum(s) => Some(s),
        Typed::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}
