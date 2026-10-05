//! Long-lived Desktop authoring workspace. Selection never reopens a Project.
use crate::{
    Error, Result,
    instrument::{self, Measurement},
    native::{self, Fault, Outcome, Snapshot, WriteResult},
    project::{self, Diagnostic, Project, Source},
    semantic::{self, Field, Shape, Table},
    source::{Document, Patch, Raw, Value},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Arc,
};

mod batch;
pub mod complex;
mod creation;
pub struct FieldShapeEdit<'a> {
    pub nullable: bool,
    pub array: bool,
    pub type_name: Option<&'a str>,
}
mod external;
mod problems;
mod records;
mod search;

#[derive(Clone, Debug)]
struct State {
    document: Arc<Document>,
    rows: Arc<Vec<String>>,
    pending: Arc<BTreeMap<String, PendingRecord>>,
    added: Arc<BTreeSet<String>>,
    origin: Option<Arc<AdditionOrigin>>,
    arrays: Arc<BTreeMap<Vec<String>, Vec<String>>>,
}
#[derive(Clone, Debug)]
struct PendingRecord {
    node: Arc<crate::source::Node>,
    bytes: Arc<str>,
    position: usize,
}
#[derive(Clone, Debug)]
struct AdditionOrigin {
    document: Arc<Document>,
    rows: Arc<Vec<String>>,
}
#[derive(Clone, Debug)]
pub struct Draft {
    pub base: Snapshot,
    pub document: Arc<Document>,
    pub revision: u64,
    pub row_ids: Arc<Vec<String>>,
    pending: Arc<BTreeMap<String, PendingRecord>>,
    added: Arc<BTreeSet<String>>,
    origin: Option<Arc<AdditionOrigin>>,
    arrays: Arc<BTreeMap<Vec<String>, Vec<String>>>,
    undo: Vec<State>,
    redo: Vec<State>,
    pub outcome: Option<Outcome>,
    pub external: Option<Snapshot>,
}
impl Draft {
    pub fn dirty(&self) -> bool {
        self.document.bytes != self.base.bytes
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    fn state(&self) -> State {
        State {
            document: self.document.clone(),
            rows: self.row_ids.clone(),
            pending: self.pending.clone(),
            added: self.added.clone(),
            origin: self.origin.clone(),
            arrays: self.arrays.clone(),
        }
    }
    fn active_index(&self, id: &str) -> Result<usize> {
        if self.pending.contains_key(id) {
            return Err(Error::new(
                "E-ROW-PENDING-DELETE",
                "pending delete is not editable",
            ));
        }
        self.row_ids
            .iter()
            .filter(|row| !self.pending.contains_key(*row))
            .position(|row| row == id)
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "record occurrence missing"))
    }
    fn saved(&mut self) {
        self.row_ids = Arc::new(
            self.row_ids
                .iter()
                .filter(|id| !self.pending.contains_key(*id))
                .cloned()
                .collect(),
        );
        self.pending = Arc::new(BTreeMap::new());
        self.added = Arc::new(BTreeSet::new());
        self.origin = None;
    }
    fn apply(&mut self, mut document: Document, rows: Arc<Vec<String>>) -> bool {
        if self.document.bytes == document.bytes && self.row_ids == rows {
            return false;
        }
        self.undo.push(self.state());
        self.redo.clear();
        for pending in Arc::make_mut(&mut self.pending).values_mut() {
            pending.position = document.map_anchor(pending.position);
        }
        // Positions are now relative to this accepted draft. Do not retain prior
        // patch traces after a history boundary or grow an implicit document chain.
        document.edits.clear();
        self.document = Arc::new(document);
        self.row_ids = rows;
        self.revision += 1;
        true
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceViewState {
    pub search: String,
    pub selected_row: Option<String>,
    pub selected_field: Option<String>,
    pub scroll_top: f64,
    pub scroll_left: f64,
}
impl Default for SourceViewState {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected_row: None,
            selected_field: None,
            scroll_top: 0.0,
            scroll_left: 0.0,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub value: Option<Value>,
    pub display: String,
    pub valid: bool,
    pub editable: bool,
    pub reason: Option<String>,
    pub problem: Option<semantic::ValueProblem>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewRow {
    pub id: String,
    pub occurrence: usize,
    pub view_index: usize,
    pub cells: Vec<Cell>,
    pub pending_delete: bool,
    pub added: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub field: Field,
    pub shape: Option<Shape>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteState {
    pub source: String,
    pub outcome: Outcome,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    pub clicked: String,
    pub source: Option<String>,
    pub table: Arc<Table>,
    pub sources: Vec<String>,
    pub columns: Vec<Column>,
    pub rows: Vec<ViewRow>,
    pub total_rows: usize,
    pub source_total_rows: usize,
    pub row_start: usize,
    pub revision: u64,
    pub schema_revision: u64,
    pub generation: u64,
    pub dirty: bool,
    pub schema_dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub schema_can_undo: bool,
    pub schema_can_redo: bool,
    pub conflict: bool,
    pub write_states: Vec<WriteState>,
    pub view_state: SourceViewState,
    pub can_add: bool,
    pub add_reason: Option<String>,
    pub measurement: Measurement,
}
#[derive(Clone, Debug)]
pub struct Workspace {
    pub read: Arc<Project>,
    pub drafts: BTreeMap<String, Draft>,
    pub views: BTreeMap<String, SourceViewState>,
    search_indexes: BTreeMap<String, search::SearchIndex>,
    snapshots: BTreeMap<String, Snapshot>,
    last_source: BTreeMap<String, String>,
    pub unavailable: BTreeMap<String, Error>,
    pub environment_error: Option<Error>,
    pub external_version: u64,
    authoring_views: BTreeMap<String, Vec<(u64, u64, AuthoringView)>>,
    creations: BTreeMap<String, creation::PendingCreation>,
    pub recovery_required: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub diagnostics_generation: u64,
    pub diagnostics_pending: bool,
    pub generation: u64,
}
#[derive(Clone, Debug)]
struct AuthoringView {
    documents: Vec<(String, String, native::ObservedIdentity)>,
}
impl AuthoringView {
    fn matches(&self, other: &Self) -> bool {
        self.documents.len() == other.documents.len()
            && self.documents.iter().zip(&other.documents).all(
                |((path, bytes, base), (other_path, other_bytes, other_base))| {
                    path == other_path && bytes == other_bytes && base == other_base
                },
            )
    }
}
impl Workspace {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Ok(Self {
            read: Arc::new(Project::open(path)?),
            drafts: BTreeMap::new(),
            views: BTreeMap::new(),
            search_indexes: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            last_source: BTreeMap::new(),
            unavailable: BTreeMap::new(),
            environment_error: None,
            external_version: 0,
            authoring_views: BTreeMap::new(),
            creations: BTreeMap::new(),
            recovery_required: false,
            diagnostics: vec![],
            diagnostics_generation: 0,
            diagnostics_pending: true,
            generation: 1,
        })
    }
    pub fn check_config(&self) -> Result<()> {
        if let Some(error) = &self.environment_error {
            return Err(error.clone());
        }
        let bytes = fs::read_to_string(self.read.root.join("masterdata.toml"))
            .map_err(project::io_error)?;
        instrument::count(instrument::Kind::Bytes(bytes.len() as u64));
        if bytes.as_str() != self.read.config_bytes.as_ref() {
            return Err(Error::new(
                "E-CONFIG-CONFLICT",
                "project configuration changed; explicit Project reload required",
            ));
        }
        for (binding, root) in self.read.config.sources.roots.iter().zip(&self.read.roots) {
            if self
                .read
                .root
                .join(binding)
                .canonicalize()
                .map_err(project::io_error)?
                != *root
            {
                return Err(Error::new(
                    "E-CONFIG-BINDING",
                    "source root binding changed",
                ));
            }
        }
        Ok(())
    }
    fn replace_read(
        &mut self,
        path: &str,
        snapshot: &Snapshot,
        document: Option<Arc<Document>>,
        error: Option<String>,
    ) {
        let kind = document
            .as_ref()
            .and_then(|d| d.root.get("kind"))
            .and_then(|n| n.text().ok())
            .map(str::to_owned);
        let binding = document
            .as_ref()
            .and_then(|d| d.root.get("table"))
            .and_then(|n| n.text().ok())
            .map(str::to_owned);
        let s = Source {
            path: path.into(),
            physical: snapshot.physical.clone(),
            bytes: snapshot.bytes.clone(),
            identity: snapshot.content.clone(),
            document: document.clone(),
            error,
            kind: kind.clone(),
            binding,
        };
        self.install_read(s);
        self.unavailable.remove(path);
    }
    fn install_read(&mut self, source: Source) {
        let path = &source.path;
        let mut p = (*self.read).clone();
        p.tables.retain(|_, table| table.source != *path);
        let removed = p
            .type_sources
            .iter()
            .filter(|(_, source)| *source == path)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in removed {
            Arc::make_mut(&mut p.types).remove(&name);
            // Keep an unavailable type's physical dependency locator. Its
            // semantics are absent, but a later repair must remain discoverable
            // without project-wide discovery during selection.
            if source.document.is_some() {
                p.type_sources.remove(&name);
            }
        }
        if let Some(doc) = &source.document {
            match source.kind.as_deref() {
                Some("schema") => {
                    if let Ok(t) = semantic::parse_table(doc, path) {
                        p.tables.insert(t.name.clone(), Arc::new(t));
                    }
                }
                Some("type") => {
                    if let Ok((name, t)) = semantic::parse_type(doc) {
                        p.type_sources.insert(name.clone(), path.into());
                        Arc::make_mut(&mut p.types).insert(name, t);
                    }
                }
                _ => {}
            }
        }
        p.sources.insert(path.clone(), Arc::new(source));
        p.generation += 1;
        self.read = Arc::new(p);
        self.generation += 1;
        self.diagnostics_pending = true;
    }
    fn unavailable_source(&mut self, path: &str, actual: Option<Snapshot>, error: Error) -> Error {
        if let Some(d) = self.drafts.get_mut(path) {
            if d.dirty() && d.outcome != Some(Outcome::OutcomeUnknown) {
                d.outcome = Some(Outcome::Conflict);
            }
            if let Some(actual) = &actual {
                d.external = Some(actual.clone());
            }
        }
        let same = self
            .unavailable
            .get(path)
            .is_some_and(|old| old.code == error.code && old.message == error.message)
            && actual.as_ref().is_none_or(|actual| {
                self.snapshots
                    .get(path)
                    .is_some_and(|old| old.matches(actual))
            });
        if !same {
            let mut source = (*self.read.sources[path]).clone();
            if let Some(actual) = actual {
                source.bytes = actual.bytes.clone();
                source.identity = actual.content.clone();
                source.physical = actual.physical.clone();
                self.snapshots.insert(path.into(), actual);
            }
            source.document = None;
            source.error = Some(error.to_string());
            self.install_read(source);
            self.unavailable.insert(path.into(), error.clone());
            self.external_version += 1;
        }
        error
    }
    pub fn refresh_source(&mut self, path: &str) -> Result<()> {
        let previous = self
            .read
            .sources
            .get(path)
            .ok_or_else(|| Error::new("E-SOURCE-NOT-FOUND", path))?
            .clone();
        let snapshot = match native::capture(&self.read.root, &self.read.roots, path) {
            Ok(snapshot) => snapshot,
            Err(error) => return Err(self.unavailable_source(path, None, error)),
        };
        let protected = self
            .drafts
            .get(path)
            .is_some_and(|d| d.dirty() || d.outcome == Some(Outcome::OutcomeUnknown));
        if let Some(d) = self.drafts.get_mut(path)
            && !snapshot.matches(&d.base)
        {
            if protected && d.outcome != Some(Outcome::OutcomeUnknown) {
                d.outcome = Some(Outcome::Conflict);
            }
            d.external = Some(snapshot.clone());
        }
        let unchanged = self
            .snapshots
            .get(path)
            .is_some_and(|old| old.matches(&snapshot));
        if unchanged {
            if let Some(error) = self.unavailable.get(path) {
                return Err(error.clone());
            }
            if !protected
                && self
                    .drafts
                    .get(path)
                    .is_some_and(|draft| !snapshot.matches(&draft.base))
            {
                self.drafts.remove(path);
                self.generation += 1;
                self.external_version += 1;
                self.diagnostics_pending = true;
            }
            return previous.document.as_ref().map(|_| ()).ok_or_else(|| {
                Error::new("E-YAML-PARSE", previous.error.clone().unwrap_or_default())
            });
        }
        // Failed observations invalidate the read generation, while an authoring
        // overlay and its history remain intact. They must not mask a deleted,
        // malformed or rebound dependency in the next diagnostic snapshot.
        // Initial Project reads already own the syntax tree. A fresh actual
        // capture with equal bytes can reuse it; parsing the same 2k-row source
        // again delayed first selection by hundreds of ms in native evidence.
        // The captured file/parent identity still becomes the physical base, and
        // every write independently preflights actual disk through native::commit.
        let document = if previous.bytes == snapshot.bytes
            && previous.physical == snapshot.physical
            && let Some(document) = &previous.document
        {
            document.clone()
        } else {
            match Document::parse(snapshot.bytes.clone()) {
                Ok(document) => Arc::new(document),
                Err(error) => return Err(self.unavailable_source(path, Some(snapshot), error)),
            }
        };
        let source_shape = || -> Result<()> {
            match document.root.get("kind").and_then(|n| n.text().ok()) {
                Some("schema") => {
                    semantic::parse_table(&document, path)?;
                }
                Some("type") => {
                    semantic::parse_type(&document)?;
                }
                Some("data") => {
                    document.records()?;
                }
                _ => {
                    return Err(Error::new(
                        "E-DOCUMENT-KIND",
                        "kind must be schema, data or type",
                    ));
                }
            }
            if protected {
                let base = &self.drafts[path].document;
                for field in ["kind", "table", "name"] {
                    if base.root.get(field).and_then(|n| n.text().ok())
                        != document.root.get(field).and_then(|n| n.text().ok())
                    {
                        return Err(Error::new(
                            "E-SOURCE-BINDING",
                            "external source identity/binding changed",
                        ));
                    }
                }
            }
            Ok(())
        };
        if let Err(error) = source_shape() {
            return Err(self.unavailable_source(path, Some(snapshot), error));
        }
        let changed = previous.bytes != snapshot.bytes
            || self.unavailable.contains_key(path)
            || self
                .snapshots
                .get(path)
                .is_some_and(|old| !old.matches(&snapshot));
        if changed {
            self.replace_read(path, &snapshot, Some(document), None);
            self.external_version += 1;
            if !protected {
                self.drafts.remove(path);
            }
        }
        self.snapshots.insert(path.into(), snapshot);
        Ok(())
    }
    fn ensure_draft(&mut self, path: &str) -> Result<()> {
        if let Some(error) = self.unavailable.get(path) {
            return Err(error.clone());
        }
        if self.drafts.contains_key(path) {
            return Ok(());
        }
        if !self.snapshots.contains_key(path) {
            self.refresh_source(path)?;
        }
        let source = &self.read.sources[path];
        let document = source
            .document
            .as_ref()
            .ok_or_else(|| Error::new("E-YAML-PARSE", source.error.clone().unwrap_or_default()))?
            .clone();
        let rows = document
            .root
            .get("records")
            .map(|n| n.items().map(|s| s.len()))
            .transpose()?
            .unwrap_or(0);
        let base = self.snapshots[path].clone();
        let row_ids = Arc::new(
            (0..rows)
                .map(|i| format!("{}:{}", base.content, i + 1))
                .collect(),
        );
        self.drafts.insert(
            path.into(),
            Draft {
                base,
                document,
                revision: 0,
                row_ids,
                pending: Arc::new(BTreeMap::new()),
                added: Arc::new(BTreeSet::new()),
                origin: None,
                arrays: Arc::new(BTreeMap::new()),
                undo: vec![],
                redo: vec![],
                outcome: None,
                external: None,
            },
        );
        Ok(())
    }
    pub fn current_doc(&self, path: &str) -> Result<Arc<Document>> {
        if let Some(error) = self.unavailable.get(path) {
            return Err(error.clone());
        }
        self.drafts
            .get(path)
            .map(|d| d.document.clone())
            .or_else(|| self.read.sources.get(path).and_then(|s| s.document.clone()))
            .ok_or_else(|| Error::new("E-YAML-PARSE", "no current parseable document"))
    }
    pub fn current_table(&self, name: &str) -> Result<Arc<Table>> {
        let t = self
            .read
            .tables
            .get(name)
            .ok_or_else(|| Error::new("E-TABLE-MISSING", name))?;
        if let Some(error) = self.unavailable.get(&t.source) {
            return Err(error.clone());
        }
        if let Some(d) = self.drafts.get(&t.source) {
            Ok(Arc::new(semantic::parse_table(&d.document, &t.source)?))
        } else {
            Ok(t.clone())
        }
    }
    pub fn current_types(&self) -> Result<semantic::Types> {
        let mut types = (*self.read.types).clone();
        for (path, d) in &self.drafts {
            if !self.unavailable.contains_key(path)
                && self.read.sources[path].kind.as_deref() == Some("type")
            {
                let (name, t) = semantic::parse_type(&d.document)?;
                types.insert(name, t);
            }
        }
        Ok(types)
    }
    fn authoring_view(&self, path: &str) -> Result<AuthoringView> {
        let binding = self
            .read
            .sources
            .get(path)
            .and_then(|source| source.binding.as_ref())
            .ok_or_else(|| Error::new("E-TABLE-MISSING", path))?;
        let table = self.current_table(binding)?;
        let mut paths = self.read.dependencies(&table);
        paths.push(path.into());
        paths.sort();
        paths.dedup();
        let documents = paths
            .into_iter()
            .map(|path| {
                let doc = self.current_doc(&path)?;
                let snapshot = self
                    .drafts
                    .get(&path)
                    .map(|draft| &draft.base)
                    .or_else(|| self.snapshots.get(&path))
                    .ok_or_else(|| Error::new("E-DRAFT-STALE", "unobserved dependency"))?;
                Ok((path, doc.identity.clone(), snapshot.observed_identity()))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(AuthoringView { documents })
    }
    fn remember_authoring_view(&mut self, path: &str) -> Result<()> {
        let view = self.authoring_view(path)?;
        let entries = self.authoring_views.entry(path.into()).or_default();
        if let Some((_, through, previous)) = entries.last_mut()
            && previous.matches(&view)
        {
            *through = self.generation;
            return Ok(());
        }
        entries.push((self.generation, self.generation, view));
        if entries.len() > 32 {
            entries.remove(0);
        }
        Ok(())
    }
    pub fn check_authoring_generation(&self, path: &str, generation: u64) -> Result<()> {
        // Global diagnostic generations also advance for unrelated sources.
        // An unfinished input may retain its observed generation when the
        // physical authoring base and every required document are unchanged.
        // The mutation path checks this again after fresh source/dependency I/O;
        // this bounded read witness never authorizes Save or structural writes.
        if generation == self.generation {
            return Ok(());
        }
        let current = self.authoring_view(path)?;
        if self.authoring_views.get(path).is_some_and(|entries| {
            entries.iter().any(|(observed, through, view)| {
                generation >= *observed && generation <= *through && view.matches(&current)
            })
        }) {
            return Ok(());
        }
        Err(Error::new(
            "E-DRAFT-STALE",
            "source or required semantic context changed",
        ))
    }
    pub fn select(&mut self, clicked: &str, start: usize, count: usize) -> Result<Projection> {
        let (result, measurement) =
            instrument::measure(|| self.select_inner(clicked, start, count));
        result.map(|mut v| {
            v.measurement = measurement;
            v
        })
    }
    fn select_inner(&mut self, clicked: &str, start: usize, count: usize) -> Result<Projection> {
        let freshness = instrument::span("freshness");
        self.check_config()?;
        self.refresh_source(clicked)?;
        let selected = self.read.sources[clicked].clone();
        if !matches!(selected.kind.as_deref(), Some("schema" | "data")) {
            return Err(Error::new("E-EDITOR-KIND", "Table source required"));
        }
        let table_name = selected
            .binding
            .as_ref()
            .ok_or_else(|| Error::new("E-TABLE-MISSING", "source has no Table"))?;
        if !self.read.tables.contains_key(table_name) {
            let schema = self
                .read
                .sources
                .values()
                .find(|source| {
                    source.kind.as_deref() == Some("schema")
                        && source.binding.as_ref() == Some(table_name)
                })
                .map(|source| source.path.clone());
            if let Some(schema) = schema
                && schema != clicked
            {
                self.refresh_source(&schema)?;
            }
        }
        let table = self.current_table(table_name)?;
        for path in self.read.dependencies(&table) {
            if path != clicked {
                self.refresh_source(&path)?;
            }
        }
        let table = self.current_table(table_name)?;
        let types = self.current_types()?;
        let sources = self.read.record_sources(table_name);
        let record_source = if selected.kind.as_deref() == Some("data")
            || selected
                .document
                .as_ref()
                .is_some_and(|d| d.root.get("records").is_some())
        {
            Some(clicked.into())
        } else {
            self.last_source
                .get(table_name)
                .filter(|s| sources.contains(s))
                .cloned()
                .or_else(|| sources.first().cloned())
        };
        if let Some(source) = &record_source {
            if source != clicked {
                self.refresh_source(source)?;
            }
            self.ensure_draft(source)?;
            self.last_source.insert(table_name.clone(), source.clone());
        }
        self.ensure_draft(&table.source)?;
        self.remember_authoring_view(&table.source)?;
        if let Some(source) = &record_source {
            self.remember_authoring_view(source)?;
        }
        drop(freshness);
        let _projection = instrument::span("projection");
        let table = self.current_table(table_name)?;
        let columns = table
            .fields
            .iter()
            .map(|f| match semantic::shape(f, &types) {
                Ok(shape) => Column {
                    field: f.clone(),
                    shape: Some(shape),
                    reason: None,
                },
                Err(e) => Column {
                    field: f.clone(),
                    shape: None,
                    reason: Some(e.to_string()),
                },
            })
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        let mut total_rows = 0;
        let mut source_total_rows = 0;
        let mut revision = 0;
        let mut dirty = false;
        let mut can_undo = false;
        let mut can_redo = false;
        let mut conflict = false;
        if let Some(source) = &record_source {
            let indices = self.query_rows(source, &table, &types)?;
            let d = &self.drafts[source];
            total_rows = indices.len();
            source_total_rows = d.row_ids.len();
            revision = d.revision;
            dirty = d.dirty();
            can_undo = d.can_undo();
            can_redo = d.can_redo();
            conflict = d.outcome == Some(Outcome::Conflict);
            let records = d.document.records()?;
            for (view_index, (i, active)) in
                indices.iter().enumerate().skip(start).take(count.min(128))
            {
                let id = &d.row_ids[*i];
                let pending = d.pending.get(id);
                let raw_row = if let Some(pending) = pending {
                    pending.node.as_ref()
                } else {
                    records[active.unwrap()].value.as_ref()
                };
                let cells = columns
                    .iter()
                    .map(|c| {
                        let Some(raw) = raw_row.get(&c.field.name) else {
                            return Cell {
                                value: None,
                                display: "(missing)".into(),
                                valid: pending.is_some(),
                                editable: false,
                                reason: Some("field entry missing".into()),
                                problem: None,
                            };
                        };
                        let interpreted = semantic::interpret(&c.field, raw, &types);
                        let editable = pending.is_none()
                            && c.shape.is_some()
                            && raw.safe
                            && d.outcome != Some(Outcome::OutcomeUnknown)
                            && !self.recovery_required;
                        let display = display(raw);
                        Cell {
                            value: match raw.raw {
                                Raw::Sequence(_) | Raw::Mapping(_) => None,
                                _ => Some(raw.value()),
                            },
                            display,
                            valid: pending.is_some() || interpreted.is_ok(),
                            editable,
                            reason: if editable {
                                None
                            } else {
                                Some(
                                    if pending.is_some() {
                                        Some("Pending delete".into())
                                    } else {
                                        c.reason.clone()
                                    }
                                    .unwrap_or_else(|| "unsafe source representation".into()),
                                )
                            },
                            problem: if pending.is_none() {
                                interpreted.err()
                            } else {
                                None
                            },
                        }
                    })
                    .collect();
                rows.push(ViewRow {
                    id: id.clone(),
                    occurrence: i + 1,
                    view_index,
                    cells,
                    pending_delete: pending.is_some(),
                    added: d.added.contains(id),
                });
            }
        }
        let schema = &self.drafts[&table.source];
        let view_state = record_source
            .as_ref()
            .and_then(|s| self.views.get(s))
            .cloned()
            .unwrap_or_default();
        let add_reason = if record_source.is_none() {
            Some("record source required; create/select a physical source".into())
        } else if columns.iter().any(|c| c.shape.is_none()) {
            Some("all field shapes must resolve before Add Row".into())
        } else if self.recovery_required {
            Some("Recovery Required".into())
        } else if let Some(source) = &record_source
            && self.drafts[source].outcome == Some(Outcome::OutcomeUnknown)
        {
            Some("Outcome Unknown; inspect and re-read the actual source before editing".into())
        } else if let Some(source) = &record_source
            && let Ok(records) = self.drafts[source].document.root.required("records")
            && records.style != crate::source::Style::Block
            && !records.items()?.is_empty()
        {
            Some("new record mapping requires a safely owned block sequence".into())
        } else {
            None
        };
        let mut write_states = Vec::new();
        for source in std::iter::once(&table.source)
            .chain(record_source.as_ref())
            .collect::<std::collections::BTreeSet<_>>()
        {
            if let Some(outcome) = self.drafts[source].outcome.clone() {
                write_states.push(WriteState {
                    source: source.clone(),
                    outcome,
                });
            }
        }
        Ok(Projection {
            clicked: clicked.into(),
            source: record_source,
            table,
            sources,
            columns,
            rows,
            total_rows,
            source_total_rows,
            row_start: start,
            revision,
            schema_revision: schema.revision,
            generation: self.generation,
            dirty,
            schema_dirty: schema.dirty(),
            can_undo,
            can_redo,
            schema_can_undo: schema.can_undo(),
            schema_can_redo: schema.can_redo(),
            conflict: conflict || schema.outcome == Some(Outcome::Conflict),
            write_states,
            view_state,
            can_add: add_reason.is_none(),
            add_reason,
            measurement: Measurement {
                elapsed_ms: 0.0,
                work: Default::default(),
                stages_ms: Default::default(),
            },
        })
    }
    pub fn edit(
        &mut self,
        path: &str,
        revision: u64,
        row_id: &str,
        value_path: &[String],
        value: &Value,
    ) -> Result<bool> {
        self.edit_at(path, revision, self.generation, row_id, value_path, value)
    }
    pub fn edit_at(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row_id: &str,
        value_path: &[String],
        value: &Value,
    ) -> Result<bool> {
        let (table, types) = self.batch_context(path, revision, generation)?;
        let field = value_path
            .first()
            .and_then(|name| table.fields.iter().find(|f| &f.name == name))
            .ok_or_else(|| Error::new("E-FIELD-MISSING", "resolved field required"))?;
        semantic::shape(field, &types)?;
        self.apply_value(path, revision, row_id, value_path, value)
    }
    fn apply_value(
        &mut self,
        path: &str,
        revision: u64,
        row_id: &str,
        value_path: &[String],
        value: &Value,
    ) -> Result<bool> {
        if self.recovery_required {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source mutation is gated",
            ));
        }
        self.ensure_draft(path)?;
        let d = self.drafts.get_mut(path).unwrap();
        if d.revision != revision {
            return Err(Error::new("E-DRAFT-STALE", "draft revision changed"));
        }
        if d.outcome == Some(Outcome::OutcomeUnknown) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "fresh observation and recovery required",
            ));
        }
        let i = d.active_index(row_id)?;
        let candidate = d.document.edit_occurrence(i + 1, value_path, value)?;
        let changed = d.apply(candidate, d.row_ids.clone());
        if changed {
            self.diagnostics_pending = true;
            self.generation += 1;
        }
        Ok(changed)
    }
    pub fn edit_text(
        &mut self,
        path: &str,
        revision: u64,
        row_id: &str,
        field: &str,
        text: &str,
    ) -> Result<bool> {
        self.edit_text_at(path, revision, self.generation, row_id, field, text)
    }
    pub fn edit_text_at(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row_id: &str,
        field: &str,
        text: &str,
    ) -> Result<bool> {
        let (table, types) = self.batch_context(path, revision, generation)?;
        let f = table
            .fields
            .iter()
            .find(|f| f.name == field)
            .ok_or_else(|| Error::new("E-FIELD-MISSING", field))?;
        let shape = semantic::shape(f, &types)?;
        self.apply_value(
            path,
            revision,
            row_id,
            &[field.into()],
            &semantic::authoring_input(&shape, text),
        )
    }
    pub fn schema_modifier(
        &mut self,
        path: &str,
        revision: u64,
        field_name: &str,
        nullable: bool,
        array: bool,
        type_name: Option<&str>,
    ) -> Result<bool> {
        self.schema_modifier_at(
            path,
            revision,
            self.generation,
            field_name,
            FieldShapeEdit {
                nullable,
                array,
                type_name,
            },
        )
    }
    pub fn schema_modifier_at(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        field_name: &str,
        declaration: FieldShapeEdit<'_>,
    ) -> Result<bool> {
        let FieldShapeEdit {
            nullable,
            array,
            type_name,
        } = declaration;
        self.check_authoring_generation(path, generation)?;
        if self.recovery_required {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source mutation is gated",
            ));
        }
        if nullable && array {
            return Err(Error::new(
                "E-SCHEMA-MODIFIER",
                "Nullable and Array are mutually exclusive",
            ));
        }
        self.check_config()?;
        self.refresh_source(path)?;
        self.ensure_draft(path)?;
        self.check_authoring_generation(path, generation)?;
        let d = self.drafts.get_mut(path).unwrap();
        if d.revision != revision {
            return Err(Error::new("E-DRAFT-STALE", "schema revision changed"));
        }
        if d.outcome == Some(Outcome::OutcomeUnknown) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "fresh observation required",
            ));
        }
        let fields = d.document.root.required("fields")?.items()?;
        let field = fields
            .iter()
            .find(|f| f.value.get("name").and_then(|n| n.text().ok()) == Some(field_name))
            .ok_or_else(|| Error::new("E-FIELD-MISSING", field_name))?;
        let mut patches = Vec::new();
        let mut additions = String::new();
        if let Some(t) = type_name {
            semantic::derive_field_type_patch(&d.document, &field.value, t, &mut patches)?;
        }
        for (name, value) in [("nullable", nullable), ("array", array)] {
            if let Some(n) = field.value.get(name) {
                crate::source::derive_patch(
                    &d.document,
                    n,
                    &Value::Literal(value.to_string()),
                    &mut patches,
                )?;
            } else if value {
                let indent = d.document.column(field.value.span.start);
                additions.push_str(&format!(
                    "{}{}: true{}",
                    " ".repeat(indent),
                    name,
                    d.document.newline()
                ));
            }
        }
        if !additions.is_empty() {
            if field.value.style == crate::source::Style::Flow {
                return Err(Error::new(
                    "E-SOURCE-UNSAFE",
                    "flow field membership needs structural localization",
                ));
            }
            let pair_end = field.value.members()?.last().unwrap().span.end;
            let end = d.document.line_end(pair_end);
            if !d.document.bytes[..end].ends_with('\n') {
                additions.insert_str(0, d.document.newline());
            }
            patches.push(Patch {
                span: end..end,
                text: additions,
            });
        }
        let candidate = d.document.patched(patches)?;
        semantic::parse_table(&candidate, path)?;
        let changed = d.apply(candidate, d.row_ids.clone());
        if changed {
            self.diagnostics_pending = true;
            self.generation += 1;
        }
        Ok(changed)
    }
    pub fn undo(&mut self, path: &str, redo: bool) -> Result<bool> {
        let d = self
            .drafts
            .get_mut(path)
            .ok_or_else(|| Error::new("E-DRAFT-MISSING", path))?;
        if d.outcome == Some(Outcome::OutcomeUnknown) {
            return Err(Error::new("E-OUTCOME-UNKNOWN", "observation required"));
        }
        let state = if redo { d.redo.pop() } else { d.undo.pop() };
        let Some(state) = state else {
            return Ok(false);
        };
        let current = d.state();
        if redo {
            d.undo.push(current);
        } else {
            d.redo.push(current);
        }
        d.document = state.document;
        d.row_ids = state.rows;
        d.pending = state.pending;
        d.added = state.added;
        d.origin = state.origin;
        d.arrays = state.arrays;
        d.revision += 1;
        self.diagnostics_pending = true;
        self.generation += 1;
        Ok(true)
    }
    pub fn dirty_paths(&self) -> Vec<String> {
        self.drafts
            .iter()
            .filter(|(_, d)| d.dirty())
            .map(|(p, _)| p.clone())
            .collect()
    }
    pub fn save_table(&mut self, table: &str, selected: Option<&str>) -> Result<Vec<WriteResult>> {
        let schema = self.current_table(table)?.source.clone();
        if let Some(path) = selected {
            let source = self
                .read
                .sources
                .get(path)
                .ok_or_else(|| Error::new("E-SOURCE-MISSING", path))?;
            if source.binding.as_deref() != Some(table)
                || self.current_doc(path)?.root.get("records").is_none()
            {
                return Err(Error::new(
                    "E-SAVE-SCOPE",
                    "selected record source does not belong to current Table",
                ));
            }
        }
        let targets =
            BTreeSet::from_iter(std::iter::once(schema).chain(selected.map(str::to_owned)));
        self.save_paths(
            targets
                .into_iter()
                .filter(|p| self.drafts.get(p).is_some_and(Draft::dirty))
                .collect(),
            Fault::None,
        )
    }
    pub fn save_all(&mut self) -> Result<Vec<WriteResult>> {
        self.save_paths(self.dirty_paths(), Fault::None)
    }
    pub fn save_paths(&mut self, paths: Vec<String>, fault: Fault) -> Result<Vec<WriteResult>> {
        if self.recovery_required {
            return Err(Error::new("E-RECOVERY-REQUIRED", "writes are gated"));
        }
        self.check_config()?;
        let mut preflight = BTreeMap::new();
        for path in &paths {
            let d = self
                .drafts
                .get(path)
                .ok_or_else(|| Error::new("E-DRAFT-MISSING", path))?;
            if d.outcome == Some(Outcome::OutcomeUnknown) {
                return Err(Error::new(
                    "E-OUTCOME-UNKNOWN",
                    "no automatic retry; observe actual source first",
                ));
            }
            if let Err(e) = native::preflight(&self.read.root, &self.read.roots, path, &d.base) {
                preflight.insert(path.clone(), e.to_string());
            }
        }
        if !preflight.is_empty() {
            let mut results = Vec::new();
            for path in paths {
                if let Some(message) = preflight.get(&path) {
                    self.drafts.get_mut(&path).unwrap().outcome = Some(Outcome::Conflict);
                    results.push(WriteResult::new(&path, Outcome::Conflict, message));
                } else {
                    results.push(WriteResult::new(
                        &path,
                        Outcome::NotAttempted,
                        "another target failed preflight",
                    ));
                }
            }
            return Ok(results);
        }
        let mut results = Vec::new();
        for path in paths {
            let d = &self.drafts[&path];
            let (result, snapshot) = native::commit(
                &self.read.root,
                &self.read.roots,
                &path,
                &d.base,
                &d.document.bytes,
                fault,
            );
            if result.outcome == Outcome::Success {
                let snapshot = snapshot.unwrap();
                let doc = d.document.clone();
                let d = self.drafts.get_mut(&path).unwrap();
                d.base = snapshot.clone();
                d.saved();
                d.undo.clear();
                d.redo.clear();
                d.outcome = None;
                d.external = None;
                d.revision += 1;
                self.snapshots.insert(path.clone(), snapshot.clone());
                self.replace_read(&path, &snapshot, Some(doc), None);
            } else {
                self.drafts.get_mut(&path).unwrap().outcome = Some(result.outcome.clone());
            }
            results.push(result);
        }
        Ok(results)
    }
    pub fn compare(&mut self, path: &str) -> Result<(String, String, String)> {
        let actual = native::capture(&self.read.root, &self.read.roots, path)?;
        let d = self
            .drafts
            .get_mut(path)
            .ok_or_else(|| Error::new("E-DRAFT-MISSING", path))?;
        d.external = Some(actual.clone());
        Ok((
            actual.content,
            actual.bytes.to_string(),
            d.document.bytes.to_string(),
        ))
    }
    pub fn overwrite(&mut self, path: &str, reviewed_identity: &str) -> Result<WriteResult> {
        if self.recovery_required {
            return Err(Error::new("E-RECOVERY-REQUIRED", "writes are gated"));
        }
        self.check_config()?;
        let d = self
            .drafts
            .get(path)
            .ok_or_else(|| Error::new("E-DRAFT-MISSING", path))?;
        let reviewed = d
            .external
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    "E-OVERWRITE-AUTHORITY",
                    "external identity must be captured",
                )
            })?
            .clone();
        if reviewed.content != reviewed_identity {
            return Err(Error::new("E-OVERWRITE-STALE", "reviewed identity differs"));
        }
        let (result, snapshot) = native::commit(
            &self.read.root,
            &self.read.roots,
            path,
            &reviewed,
            &d.document.bytes,
            Fault::None,
        );
        if result.outcome == Outcome::Success {
            let snapshot = snapshot.unwrap();
            let doc = d.document.clone();
            let d = self.drafts.get_mut(path).unwrap();
            d.base = snapshot.clone();
            d.saved();
            d.undo.clear();
            d.redo.clear();
            d.outcome = None;
            d.external = None;
            d.revision += 1;
            self.snapshots.insert(path.into(), snapshot.clone());
            self.replace_read(path, &snapshot, Some(doc), None);
        } else {
            self.drafts.get_mut(path).unwrap().outcome = Some(result.outcome.clone());
        }
        Ok(result)
    }
    pub fn reload_source(&mut self, path: &str) -> Result<()> {
        let snapshot = native::capture(&self.read.root, &self.read.roots, path)?;
        let doc = Document::parse(snapshot.bytes.clone())?;
        self.drafts.remove(path);
        self.replace_read(path, &snapshot, Some(Arc::new(doc)), None);
        self.snapshots.insert(path.into(), snapshot);
        self.ensure_draft(path)
    }
    pub fn uncertain_paths(&self) -> Vec<String> {
        self.drafts
            .iter()
            .filter(|(_, draft)| {
                matches!(
                    draft.outcome,
                    Some(Outcome::OutcomeUnknown | Outcome::RecoveryRequired)
                )
            })
            .map(|(path, _)| path.clone())
            .chain(self.creations.keys().cloned())
            .collect()
    }
    pub fn diagnostic_input(&self) -> Project {
        let mut p = (*self.read).clone();
        p.generation = self.generation;
        for (path, d) in &self.drafts {
            if self.unavailable.contains_key(path) {
                continue;
            }
            let old = &p.sources[path];
            let mut source = (**old).clone();
            source.bytes = d.document.bytes.clone();
            source.document = Some(d.document.clone());
            source.identity = d.document.identity.clone();
            p.sources.insert(path.clone(), Arc::new(source));
        }
        p
    }
    pub fn validation_snapshot(&self) -> Project {
        let mut p = self.diagnostic_input();
        p.rebuild_declarations();
        p
    }
    pub fn accept_diagnostics(&mut self, generation: u64, mut problems: Vec<Diagnostic>) -> bool {
        if generation != self.generation || problems.iter().any(|d| d.generation != generation) {
            return false;
        }
        problems.retain(|d| !self.unavailable.contains_key(&d.source));
        problems.extend(
            self.unavailable
                .iter()
                .map(|(path, error)| Diagnostic::error(path, error, generation)),
        );
        if let Some(error) = &self.environment_error {
            problems.push(Diagnostic::error("masterdata.toml", error, generation));
        }
        for problem in &mut problems {
            if let Some(d) = self.drafts.get(&problem.source)
                && let Some(occurrence) = problem.occurrence
                && let Some(id) = d
                    .row_ids
                    .iter()
                    .filter(|id| !d.pending.contains_key(*id))
                    .nth(occurrence.saturating_sub(1))
            {
                problem.occurrence = d.row_ids.iter().position(|row| row == id).map(|i| i + 1);
            }
        }
        self.diagnostics = problems;
        self.diagnostics_generation = generation;
        self.diagnostics_pending = false;
        true
    }
}
fn display(raw: &crate::source::Node) -> String {
    match &raw.raw {
        Raw::Null => "null".into(),
        Raw::Scalar(s) => s.clone(),
        Raw::Sequence(s) => format!("[{} items]", s.len()),
        Raw::Mapping(m) => format!("{{{} fields}}", m.len()),
    }
}
