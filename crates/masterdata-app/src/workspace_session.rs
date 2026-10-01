//! Interactive read ownership; writes still use the fresh Native application preflight.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
    sync::{Arc, Mutex, OnceLock, RwLock},
    time::SystemTime,
};

use masterdata_core::{
    AuthoringValidation, DataFileSnapshot, Diagnostic, ErrorKind, MasterdataError, Project,
    ProjectDocuments, Result, SourceDocument, TableSnapshot, TypeSnapshot, ValidationReport,
    apply_reference_resolution, authoring_tag_candidates, authoring_tag_candidates_complete,
    data_file_view, parse_yaml_document, source_content_identity, table_definition_snapshot,
    type_snapshot, validate_authoring_documents,
};
use serde::Serialize;

use crate::authoring::project_relative_string;
use crate::table_authoring::table_context_from_documents;
use crate::{
    AuthoringWorkspace, SourceContentState, TableContext, WorkspaceFolder, WorkspaceSourceFile,
};

#[derive(Clone, PartialEq, Eq)]
struct FileStamp {
    length: u64,
    modified: Option<SystemTime>,
}
fn stamp(path: &Path) -> Option<FileStamp> {
    fs::metadata(path).ok().map(|value| FileStamp {
        length: value.len(),
        modified: value.modified().ok(),
    })
}
struct CapturedSource {
    source: Result<String>,
    stamp: Option<FileStamp>,
}
#[derive(Clone)]
struct SourceEntry {
    identity: Option<String>,
    stamp: Option<FileStamp>,
    diagnostic: Option<Diagnostic>,
}
struct ReadGeneration {
    number: u64,
    documents: ProjectDocuments,
    sources: BTreeMap<String, SourceEntry>,
    dependencies: BTreeSet<String>,
    tags: Vec<String>,
    validation: OnceLock<AuthoringValidation>,
    contexts: Mutex<BTreeMap<String, Arc<TableContext>>>,
}

/// An immutable captured generation is shared by navigation and background validation.
/// WHY: holding the migration-plan mutex or rebuilding the Project for selection freezes
/// Desktop and duplicates parsing. Publication is short; Core derivation runs off-lock.
/// EVIDENCE: docs/adr/0008-interactive-workspace-read-session.md
pub struct WorkspaceAuthoringSession {
    project: Project,
    current: RwLock<Arc<ReadGeneration>>,
    update: Mutex<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSelection {
    pub requested_path: String,
    pub path: String,
    pub generation: u64,
    pub files: Vec<WorkspaceSourceFile>,
    pub current_source: Option<SourceContentState>,
    pub context: Option<TableContext>,
    pub data: Option<DataFileSnapshot>,
    pub type_snapshot: Option<TypeSnapshot>,
    pub validation_pending: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceReadStatus {
    pub generation: u64,
    pub workspace: AuthoringWorkspace,
    pub validation: Option<ValidationReport>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceValidation {
    pub generation: u64,
    pub tag_candidates_complete: bool,
    pub tables: BTreeMap<String, TableSnapshot>,
    pub validation: ValidationReport,
}

impl WorkspaceAuthoringSession {
    pub fn open(explicit_project: Option<&Path>, current_dir: &Path) -> Result<Self> {
        let project = Project::discover(explicit_project, current_dir)?;
        let mut documents = ProjectDocuments::default();
        let mut sources = BTreeMap::new();
        for path in project.source_files()? {
            let relative = project_relative_string(project.root(), &path);
            let (entry, loaded) = read_entry(project.root(), &path);
            sources.insert(relative, entry);
            if let Some(loaded) = loaded {
                documents.files.push(loaded);
            }
        }
        let current = Arc::new(generation(&project, 1, documents, sources));
        Ok(Self {
            project,
            current: RwLock::new(current),
            update: Mutex::new(()),
        })
    }

    pub fn root(&self) -> &Path {
        self.project.root()
    }
    pub fn generation(&self) -> u64 {
        self.captured().number
    }
    fn captured(&self) -> Arc<ReadGeneration> {
        self.current
            .read()
            .expect("workspace publication lock poisoned")
            .clone()
    }
    fn check_binding(&self) -> Result<()> {
        let source = read_text(self.project.root(), self.project.config_path())?;
        if source_content_identity(&source) != self.project.config_content_identity() {
            return Err(error(
                "E-WORKSPACE-BINDING-CHANGED",
                "Project binding changed. Reload the Project before editing; local buffers are retained.",
            ));
        }
        Ok(())
    }

    /// Exact bytes, not filesystem timestamps, authorize reuse of an editable read view.
    /// Writes independently repeat their authoritative identity/patch preflight.
    pub fn select_source(&self, path: &str) -> Result<WorkspaceSelection> {
        self.check_binding()?;
        self.check_path(path)?;
        let mut captured = self.refresh_exact(path)?;
        let mut context = context_for(&self.project, &captured, path)?;
        let resolved = context
            .as_ref()
            .and_then(|value| value.selected_record_source.clone())
            .unwrap_or_else(|| path.to_owned());
        if resolved != path {
            let refreshed = self.refresh_exact(&resolved)?;
            if refreshed.number != captured.number {
                context = context_for(&self.project, &refreshed, path)?;
            }
            captured = refreshed;
        }
        let target = self.project.root().join(&resolved);
        ensure_available(&captured, &resolved)?;
        let validation = captured.validation.get().map(|model| model.report.clone());
        let pending = validation.is_none();
        let complete = validation.as_ref().is_some_and(|report| {
            authoring_tag_candidates_complete(
                report,
                captured
                    .sources
                    .values()
                    .all(|source| source.diagnostic.is_none()),
            )
        });
        let data = captured
            .documents
            .files
            .iter()
            .find(|file| file.path == target)
            .filter(|file| file.document.record_data().is_some())
            .map(|_| {
                data_file_view(
                    self.project.root(),
                    &captured.documents,
                    &target,
                    captured.tags.clone(),
                    complete,
                    validation.unwrap_or_else(pending_validation),
                )
            })
            .transpose()?;
        let type_view = captured
            .documents
            .files
            .iter()
            .find(|file| file.path == target)
            .filter(|file| matches!(file.document, SourceDocument::Type(_)))
            .map(|_| type_snapshot(&captured.documents, &target, &resolved))
            .transpose()?;
        let current_source = if data.is_none() {
            captured
                .documents
                .files
                .iter()
                .find(|file| file.path == target)
                .map(|file| SourceContentState {
                    path: resolved.clone(),
                    content_identity: source_content_identity(&file.source),
                    source: file.source.clone(),
                })
        } else {
            None
        };
        Ok(WorkspaceSelection {
            requested_path: path.to_owned(),
            path: resolved,
            generation: captured.number,
            files: self.files_from(&captured),
            current_source,
            context,
            data,
            type_snapshot: type_view,
            validation_pending: pending,
        })
    }

    pub fn source_content(&self, path: &str) -> Result<SourceContentState> {
        self.check_binding()?;
        self.check_path(path)?;
        let source = read_text(self.project.root(), &self.project.root().join(path))?;
        Ok(SourceContentState {
            path: path.to_owned(),
            content_identity: source_content_identity(&source),
            source,
        })
    }

    fn overlay_documents(
        &self,
        overrides: &[(&str, &str)],
    ) -> Result<(ProjectDocuments, Vec<Diagnostic>)> {
        self.check_binding()?;
        let mut captured = self.captured();
        for (path, _) in overrides {
            self.check_path(path)?;
            captured = self.refresh_exact(path)?;
        }
        let mut documents = captured.documents.clone();
        let mut diagnostics = captured
            .sources
            .iter()
            .filter(|(path, _)| !overrides.iter().any(|(target, _)| target == &path.as_str()))
            .filter_map(|(_, entry)| entry.diagnostic.clone())
            .collect::<Vec<_>>();
        for (path, text) in overrides {
            let target = self.project.root().join(path);
            if documents
                .files
                .iter()
                .any(|file| file.path == target && file.source == *text)
            {
                continue;
            }
            documents.files.retain(|file| file.path != target);
            match parse_yaml_document(target, text) {
                Ok(loaded) => documents.files.push(loaded),
                Err(cause) => diagnostics.push(cause.diagnostic().clone()),
            }
        }
        Ok((documents, diagnostics))
    }

    pub fn preview_data_file(
        &self,
        path: &str,
        base_source: &str,
        mutation: &crate::AuthoringRecordMutation,
    ) -> Result<crate::SourceEditPreview> {
        let (documents, diagnostics) = self.overlay_documents(&[(path, base_source)])?;
        crate::authoring::preview_records(
            &documents,
            diagnostics,
            &self.project.root().join(path),
            mutation,
        )
    }
    pub fn query_data_file(
        &self,
        request: &crate::DataFileQueryRequest,
    ) -> Result<crate::DataFileQueryResult> {
        let (documents, diagnostics) =
            self.overlay_documents(&[(&request.relative_path, &request.base_source)])?;
        masterdata_core::query_data_file(
            self.project.root(),
            &self.project.info().profiles,
            &documents,
            diagnostics,
            &self.project.root().join(&request.relative_path),
            &masterdata_core::SourceRecordMutation::from(&request.mutation),
            &request.query,
        )
    }
    pub fn preview_data_file_batch(
        &self,
        path: &str,
        base_source: &str,
        mutation: &crate::AuthoringRecordMutation,
        request: &crate::AuthoringBatchRequest,
    ) -> Result<crate::AuthoringBatchPreview> {
        if request.targets.is_empty() {
            return Err(crate::batch::batch_error(
                "E-AUTHORING-BATCH-TARGETS",
                "batch operation has no targets",
            ));
        }
        let (documents, diagnostics) = self.overlay_documents(&[(path, base_source)])?;
        let target = self.project.root().join(path);
        let snapshot = data_file_view(
            self.project.root(),
            &documents,
            &target,
            Vec::new(),
            false,
            pending_validation(),
        )?;
        crate::batch::batch_preview(
            &snapshot,
            &documents,
            diagnostics,
            &target,
            mutation,
            request,
        )
    }
    pub fn copy_data_file_batch(
        &self,
        path: &str,
        base_source: &str,
        request: &crate::AuthoringBatchCopyRequest,
    ) -> Result<crate::AuthoringBatchCopyResult> {
        if request.targets.is_empty() {
            return Err(crate::batch::batch_error(
                "E-AUTHORING-BATCH-COPY-TARGETS",
                "copy operation has no targets",
            ));
        }
        let (documents, _) = self.overlay_documents(&[(path, base_source)])?;
        let target = self.project.root().join(path);
        let snapshot = data_file_view(
            self.project.root(),
            &documents,
            &target,
            Vec::new(),
            false,
            pending_validation(),
        )?;
        let base_record_count = snapshot.rows.len();
        let mut mutation = masterdata_core::SourceRecordMutation::from(&request.current_mutation);
        mutation.deletions.clear();
        let transformed =
            masterdata_core::dry_run_source_record_mutation(&documents, &target, &mutation)?
                .transformed_documents;
        let snapshot = data_file_view(
            self.project.root(),
            &transformed,
            &target,
            Vec::new(),
            false,
            pending_validation(),
        )?;
        crate::batch::batch_copy(&snapshot, base_record_count, request)
    }
    pub fn table_overview(
        &self,
        request: &crate::TableOverviewRequest,
    ) -> Result<crate::TableOverviewSnapshot> {
        self.refresh_inventory()?;
        let paths = self.project.source_files()?;
        let before = crate::overview::source_identities(&self.project, &paths)?;
        let captured = self.refresh_paths(
            paths
                .iter()
                .map(|path| project_relative_string(self.project.root(), path))
                .collect(),
        )?;
        let diagnostics = captured
            .sources
            .values()
            .filter_map(|entry| entry.diagnostic.clone())
            .collect();
        let validation = validation_for(&captured).report.clone();
        crate::overview::overview_from_documents(
            &self.project,
            request,
            &captured.documents,
            diagnostics,
            before,
            validation,
        )
    }
    pub fn preview_schema_draft(
        &self,
        schema_path: &str,
        base_source: &str,
        fields: &[crate::SchemaDraftField],
        record_drafts: &[crate::SchemaDraftRecordSource],
        selected_record_path: Option<&str>,
    ) -> Result<crate::SchemaDraftPreview> {
        if let Some(path) = selected_record_path {
            self.check_path(path)?;
            self.refresh_exact(path)?;
        }
        let mut overrides = vec![(schema_path, base_source)];
        overrides.extend(
            record_drafts
                .iter()
                .map(|draft| (draft.path.as_str(), draft.candidate_source.as_str())),
        );
        if let Some(inline) = overrides.iter().rfind(|(path, _)| *path == schema_path) {
            overrides[0] = *inline;
        }
        let (documents, diagnostics) = self.overlay_documents(&overrides)?;
        crate::table_authoring::schema_draft_preview(
            &self.project,
            &documents,
            diagnostics,
            &self.project.root().join(schema_path),
            base_source,
            fields,
            selected_record_path,
        )
    }

    fn check_path(&self, path: &str) -> Result<()> {
        let valid = !path.is_empty()
            && Path::new(path)
                .components()
                .all(|part| matches!(part, Component::Normal(_)));
        if !valid || !self.captured().sources.contains_key(path) {
            return Err(error(
                "E-GUI-SOURCE-PATH",
                "Selected path is not a current Project source",
            ));
        }
        Ok(())
    }

    fn refresh_exact(&self, target: &str) -> Result<Arc<ReadGeneration>> {
        self.refresh_paths(BTreeSet::from([target.to_owned()]))
    }
    fn refresh_paths(&self, paths: BTreeSet<String>) -> Result<Arc<ReadGeneration>> {
        let before = self.captured();
        let mut checked_paths = before.dependencies.clone();
        checked_paths.extend(paths);
        let mut changed = BTreeMap::new();
        for path in checked_paths {
            let physical = self.project.root().join(&path);
            let source = capture_source(self.project.root(), &physical);
            let identity = source
                .source
                .as_ref()
                .ok()
                .map(|text| source_content_identity(text));
            if before
                .sources
                .get(&path)
                .is_none_or(|entry| entry.identity != identity)
            {
                changed.insert(path, source);
            }
        }
        if changed.is_empty() {
            return Ok(before);
        }
        let _update = self.update.lock().expect("workspace update lock poisoned");
        let current = self.captured();
        // Re-read after waiting for another publisher; never install an older read over it.
        let changes = changed
            .into_keys()
            .map(|path| {
                let result = capture_source(self.project.root(), &self.project.root().join(&path));
                (path, result)
            })
            .collect();
        Ok(self.publish_changes(&current, changes, None))
    }

    fn publish_changes(
        &self,
        before: &ReadGeneration,
        changes: BTreeMap<String, CapturedSource>,
        inventory: Option<BTreeSet<String>>,
    ) -> Arc<ReadGeneration> {
        let mut documents = before.documents.clone();
        let mut sources = before.sources.clone();
        if let Some(inventory) = inventory {
            sources.retain(|path, _| inventory.contains(path));
            documents.files.retain(|file| {
                inventory.contains(&project_relative_string(self.project.root(), &file.path))
            });
        }
        for (path, source) in changes {
            let physical = self.project.root().join(&path);
            documents.files.retain(|file| file.path != physical);
            let (entry, loaded) = entry_from_source(&physical, source);
            sources.insert(path, entry);
            if let Some(loaded) = loaded {
                documents.files.push(loaded);
            }
        }
        documents.files.sort_by(|a, b| a.path.cmp(&b.path));
        let next = Arc::new(generation(
            &self.project,
            before.number + 1,
            documents,
            sources,
        ));
        *self
            .current
            .write()
            .expect("workspace publication lock poisoned") = next.clone();
        next
    }

    /// Inventory polling is separate from selection. Metadata is only an invalidation hint;
    /// navigation and Save never use it as content identity or commit authorization.
    pub fn refresh_inventory(&self) -> Result<WorkspaceReadStatus> {
        self.check_binding()?;
        let _update = self.update.lock().expect("workspace update lock poisoned");
        let before = self.captured();
        let inventory = self
            .project
            .source_files()?
            .iter()
            .map(|path| project_relative_string(self.project.root(), path))
            .collect::<BTreeSet<_>>();
        let changes = inventory
            .iter()
            .filter(|path| {
                before
                    .sources
                    .get(*path)
                    .is_none_or(|entry| entry.stamp != stamp(&self.project.root().join(path)))
            })
            .map(|path| {
                (
                    path.clone(),
                    capture_source(self.project.root(), &self.project.root().join(path)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let captured = if !changes.is_empty()
            || inventory.len() != before.sources.len()
            || inventory
                .iter()
                .any(|path| !before.sources.contains_key(path))
        {
            self.publish_changes(&before, changes, Some(inventory))
        } else {
            before
        };
        Ok(WorkspaceReadStatus {
            generation: captured.number,
            workspace: self.workspace_from(&captured)?,
            validation: captured.validation.get().map(|model| model.report.clone()),
        })
    }

    pub fn workspace(&self) -> Result<AuthoringWorkspace> {
        self.workspace_from(&self.captured())
    }
    fn workspace_from(&self, captured: &ReadGeneration) -> Result<AuthoringWorkspace> {
        let info = self.project.info();
        let source_roots = info
            .source_roots
            .iter()
            .map(|root| project_relative_string(self.project.root(), root))
            .collect();
        let files = self.files_from(captured);
        let mut folders = Vec::new();
        for root in &info.source_roots {
            if !root.is_dir() {
                continue;
            }
            let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())
                .map_err(|cause| io_error(root, cause))?;
            let mut paths = Vec::new();
            crate::creation::collect_folders(&dir, "", &mut paths)
                .map_err(|cause| io_error(root, cause))?;
            folders.extend(paths.into_iter().map(|path| WorkspaceFolder {
                path: project_relative_string(self.project.root(), &root.join(path)),
                source_root: project_relative_string(self.project.root(), root),
            }));
        }
        folders.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(AuthoringWorkspace {
            project: info,
            source_roots,
            files,
            folders,
        })
    }

    fn files_from(&self, captured: &ReadGeneration) -> Vec<WorkspaceSourceFile> {
        let info = self.project.info();
        captured.sources.iter().map(|(relative, entry)| {
            let physical = self.project.root().join(relative);
            let loaded = captured.documents.files.iter().find(|file| file.path == physical);
            WorkspaceSourceFile { path: relative.clone(), source_root: info.source_roots.iter().find(|root| physical.starts_with(root)).map(|root| project_relative_string(self.project.root(), root)).unwrap_or_else(|| ".".into()),
                kind: loaded.map(|file| file.document.kind()).unwrap_or(if entry.identity.is_some() { "invalid" } else { "unavailable" }).into(),
                table: loaded.and_then(|file| file.document.table_identity()).map(str::to_owned),
                type_name: loaded.and_then(|file| file.document.type_name()).map(str::to_owned),
                has_inline_records: loaded.is_some_and(|file| matches!(&file.document, SourceDocument::Schema(schema) if schema.records.is_some())),
                diagnostic: entry.diagnostic.clone() }
        }).collect()
    }

    /// Computes once per immutable generation, outside publication and migration locks.
    pub fn validate(&self) -> WorkspaceValidation {
        let captured = self.captured();
        let model = validation_for(&captured);
        let tables = captured
            .documents
            .schemas()
            .filter_map(|(path, schema)| {
                let mut snapshot = table_definition_snapshot(
                    &captured.documents,
                    path,
                    &project_relative_string(self.project.root(), path),
                )
                .ok()?;
                snapshot.schema.records = None;
                if let Some(references) = model.references.get(&schema.table) {
                    apply_reference_resolution(&mut snapshot, references);
                }
                snapshot.reference_diagnostics = model.reference_diagnostics.clone();
                Some((schema.table.clone(), snapshot))
            })
            .collect();
        WorkspaceValidation {
            generation: captured.number,
            tag_candidates_complete: authoring_tag_candidates_complete(
                &model.report,
                captured
                    .sources
                    .values()
                    .all(|entry| entry.diagnostic.is_none()),
            ),
            tables,
            validation: model.report.clone(),
        }
    }
}
fn validation_for(captured: &ReadGeneration) -> &AuthoringValidation {
    captured.validation.get_or_init(|| {
        let mut model = validate_authoring_documents(&captured.documents);
        model.report.diagnostics.extend(
            captured
                .sources
                .values()
                .filter_map(|entry| entry.diagnostic.clone()),
        );
        model.report.valid = model.report.diagnostics.is_empty();
        model
    })
}

fn generation(
    project: &Project,
    number: u64,
    documents: ProjectDocuments,
    sources: BTreeMap<String, SourceEntry>,
) -> ReadGeneration {
    let _span = masterdata_core::read_trace::read_span("workspaceIndex");
    // Schema/Type dependencies are intentionally conservative. Record sources outside
    // the selected Table are not needed to grant local primitive editing capability.
    let dependencies = documents
        .files
        .iter()
        .filter(|file| !matches!(file.document, SourceDocument::Data(_)))
        .map(|file| project_relative_string(project.root(), &file.path))
        .chain(
            sources
                .iter()
                .filter(|(_, entry)| entry.diagnostic.is_some())
                .map(|(path, _)| path.clone()),
        )
        .collect();
    let tags = authoring_tag_candidates(&project.info().profiles, &documents);
    ReadGeneration {
        number,
        documents,
        sources,
        dependencies,
        tags,
        validation: OnceLock::new(),
        contexts: Mutex::new(BTreeMap::new()),
    }
}
fn read_text(root: &Path, path: &Path) -> Result<String> {
    let _span = masterdata_core::read_trace::read_span("fileIo");
    let relative = path
        .strip_prefix(root)
        .map_err(|_| error("E-WORKSPACE-SOURCE", "Source is outside the bound Project"))?;
    let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())
        .map_err(|cause| io_error(root, cause))?;
    let mut checked = std::path::PathBuf::new();
    for component in relative.components() {
        checked.push(component);
        if dir
            .symlink_metadata(&checked)
            .map_err(|cause| io_error(path, cause))?
            .file_type()
            .is_symlink()
        {
            return Err(error(
                "E-WORKSPACE-SOURCE-SYMLINK",
                "Source must not traverse a symlink",
            ));
        }
    }
    // Capability access also closes the check/open race for an escaping symlink.
    dir.read_to_string(relative)
        .map_err(|cause| io_error(path, cause))
}
fn read_entry(root: &Path, path: &Path) -> (SourceEntry, Option<masterdata_core::LoadedDocument>) {
    entry_from_source(path, capture_source(root, path))
}
fn entry_from_source(
    path: &Path,
    captured: CapturedSource,
) -> (SourceEntry, Option<masterdata_core::LoadedDocument>) {
    let identity = captured
        .source
        .as_ref()
        .ok()
        .map(|text| source_content_identity(text));
    let loaded = captured
        .source
        .and_then(|text| parse_yaml_document(path.to_path_buf(), &text));
    let diagnostic = loaded
        .as_ref()
        .err()
        .map(|cause| cause.diagnostic().clone());
    (
        SourceEntry {
            identity,
            stamp: captured.stamp,
            diagnostic,
        },
        loaded.ok(),
    )
}
fn context_for(
    project: &Project,
    captured: &ReadGeneration,
    path: &str,
) -> Result<Option<TableContext>> {
    ensure_available(captured, path)?;
    let file = captured
        .documents
        .files
        .iter()
        .find(|file| file.path == project.root().join(path))
        .expect("available parsed source");
    let Some(table) = file.document.table_identity() else {
        return Ok(None);
    };
    let cached = captured
        .contexts
        .lock()
        .expect("context cache poisoned")
        .get(table)
        .cloned();
    let base = if let Some(context) = cached {
        context
    } else {
        let mut context = table_context_from_documents(project, &captured.documents, path)?;
        context.schema.schema.records = None;
        let context = Arc::new(context);
        captured
            .contexts
            .lock()
            .expect("context cache poisoned")
            .insert(table.to_owned(), context.clone());
        context
    };
    let mut context = (*base).clone();
    context.schema.schema.records = None;
    if let Some(model) = captured.validation.get() {
        if let Some(references) = model.references.get(table) {
            apply_reference_resolution(&mut context.schema, references);
        }
        context.schema.reference_diagnostics = model.reference_diagnostics.clone();
    }
    context.selected_record_source = context
        .record_sources
        .iter()
        .find(|source| source.path == path)
        .or_else(|| context.record_sources.first())
        .map(|source| source.path.clone());
    Ok(Some(context))
}
fn ensure_available(captured: &ReadGeneration, path: &str) -> Result<()> {
    let entry = captured
        .sources
        .get(path)
        .ok_or_else(|| error("E-GUI-SOURCE-PATH", "Selected source no longer exists"))?;
    if let Some(diagnostic) = &entry.diagnostic {
        return Err(MasterdataError {
            diagnostic: Box::new(diagnostic.clone()),
        });
    }
    Ok(())
}
fn pending_validation() -> ValidationReport {
    ValidationReport {
        valid: false,
        files_scanned: 0,
        schema_documents: 0,
        data_documents: 0,
        type_documents: 0,
        tables: vec![],
        types: vec![],
        diagnostics: vec![],
    }
}
fn error(code: &str, message: &str) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
}

fn io_error(path: &Path, cause: impl std::fmt::Display) -> MasterdataError {
    MasterdataError::new(
        "E-IO-ACCESS",
        ErrorKind::Io,
        format!("could not access source: {cause}"),
    )
    .with_source(path)
}

fn capture_source(root: &Path, path: &Path) -> CapturedSource {
    // Capture the invalidation hint before reading/parsing. A concurrent later
    // change must not lend its metadata to older captured bytes.
    let stamp = stamp(path);
    CapturedSource {
        source: read_text(root, path),
        stamp,
    }
}
