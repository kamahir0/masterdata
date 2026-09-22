//! Application boundary for persisted Computed View authoring.
//!
//! The core owns parsing, type checking, evaluation, and source spans.  This
//! module owns project discovery, stale checks, and the source-file write
//! lifecycle used by Desktop and future adapters.

use std::path::Path;

use masterdata_core::{
    Diagnostic, Project, ProjectDocuments, SourceDocument, ViewColumnDefinition, ViewDocument,
    dry_run_view_edit, source_content_identity, validate_documents,
};
use serde::{Deserialize, Serialize};

use crate::NativeApplicationService;
use crate::authoring::{
    SourceContentState, SourceSaveStatus, install_source_candidate, load_authoring_documents,
    project_relative_string, read_source_state, remove_source_candidate, resolve_source_file,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedViewSnapshot {
    pub path: String,
    pub base_source: String,
    pub base_content_identity: String,
    pub name: String,
    pub table: String,
    pub columns: Vec<ViewColumnDefinition>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputedViewEditRequest {
    pub name: String,
    pub table: String,
    pub columns: Vec<ViewColumnDefinition>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedViewEditPreview {
    pub path: String,
    pub base_content_identity: String,
    pub candidate_source: String,
    pub candidate_content_identity: String,
    pub changed: bool,
    pub validation: masterdata_core::ValidationReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedViewSaveReport {
    pub status: SourceSaveStatus,
    pub path: String,
    pub snapshot: Option<ComputedViewSnapshot>,
    pub current: Option<SourceContentState>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedViewRemoveReport {
    pub status: SourceSaveStatus,
    pub path: String,
    pub current: Option<SourceContentState>,
    pub diagnostic: Option<Diagnostic>,
}

impl NativeApplicationService {
    pub fn open_computed_view(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
    ) -> masterdata_core::Result<ComputedViewSnapshot> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        let (documents, parse_diagnostics) = load_authoring_documents(&project, None)?;
        let mut diagnostics = parse_diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.source.as_ref() == Some(&target))
            .collect::<Vec<_>>();
        diagnostics.extend(
            masterdata_core::validate_computed_views(&documents)
                .into_iter()
                .filter(|diagnostic| diagnostic.source.as_ref() == Some(&target)),
        );
        view_snapshot(&project, &documents, diagnostics, &target)
    }

    pub fn preview_computed_view(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        request: &ComputedViewEditRequest,
    ) -> masterdata_core::Result<ComputedViewEditPreview> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        ensure_base_identity(base_source, &target, None)?;
        let (documents, parse_diagnostics) =
            load_authoring_documents(&project, Some((&target, base_source)))?;
        let dry_run = dry_run_view_edit(&documents, &target, &requested_document(request))?;
        let mut validation = validate_documents(&dry_run.transformed_documents);
        merge_parse_diagnostics(&mut validation, parse_diagnostics);
        Ok(ComputedViewEditPreview {
            path: relative_path.to_owned(),
            base_content_identity: dry_run.plan.base_content_identity,
            candidate_source: dry_run.plan.candidate_source,
            candidate_content_identity: dry_run.plan.candidate_content_identity,
            changed: dry_run.plan.changed,
            validation,
        })
    }

    pub fn save_computed_view(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        base_content_identity: &str,
        request: &ComputedViewEditRequest,
    ) -> masterdata_core::Result<ComputedViewSaveReport> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        ensure_base_identity(base_source, &target, Some(base_content_identity))?;
        let (documents, _) = load_authoring_documents(&project, Some((&target, base_source)))?;
        let dry_run = dry_run_view_edit(&documents, &target, &requested_document(request))?;
        let current = read_source_state(&project, &target)?;
        if current.content_identity != base_content_identity {
            return Ok(ComputedViewSaveReport {
                status: SourceSaveStatus::Conflict,
                path: relative_path.to_owned(),
                snapshot: None,
                current: Some(current),
                diagnostic: Some(
                    view_error(
                        "E-VIEW-EDIT-CONFLICT",
                        "View source changed after the editor base snapshot",
                        &target,
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }
        if current.source == dry_run.plan.candidate_source {
            return Ok(ComputedViewSaveReport {
                status: SourceSaveStatus::Success,
                path: relative_path.to_owned(),
                snapshot: Some(self.open_computed_view(
                    Some(project.root()),
                    project.root(),
                    relative_path,
                )?),
                current: None,
                diagnostic: None,
            });
        }
        if let Err(error) = install_source_candidate(
            &target,
            current.source.as_bytes(),
            dry_run.plan.candidate_source.as_bytes(),
        ) {
            let after = read_source_state(&project, &target).ok();
            let status = if after.as_ref().is_some_and(|state| {
                state.content_identity == dry_run.plan.candidate_content_identity
            }) {
                SourceSaveStatus::Success
            } else if after
                .as_ref()
                .is_some_and(|state| state.content_identity == current.content_identity)
            {
                SourceSaveStatus::Failure
            } else {
                SourceSaveStatus::OutcomeUnknown
            };
            return Ok(ComputedViewSaveReport {
                status,
                path: relative_path.to_owned(),
                snapshot: if status == SourceSaveStatus::Success {
                    Some(self.open_computed_view(
                        Some(project.root()),
                        project.root(),
                        relative_path,
                    )?)
                } else {
                    None
                },
                current: after,
                diagnostic: Some(error.diagnostic().clone()),
            });
        }
        let after = read_source_state(&project, &target)?;
        if after.content_identity != dry_run.plan.candidate_content_identity {
            return Ok(ComputedViewSaveReport {
                status: SourceSaveStatus::OutcomeUnknown,
                path: relative_path.to_owned(),
                snapshot: None,
                current: Some(after),
                diagnostic: Some(
                    view_error(
                        "E-VIEW-EDIT-WRITE-VERIFY",
                        "saved View source could not be verified as the complete candidate",
                        &target,
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }
        Ok(ComputedViewSaveReport {
            status: SourceSaveStatus::Success,
            path: relative_path.to_owned(),
            snapshot: Some(self.open_computed_view(
                Some(project.root()),
                project.root(),
                relative_path,
            )?),
            current: None,
            diagnostic: None,
        })
    }

    pub fn remove_computed_view(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        base_content_identity: &str,
        confirmed: bool,
    ) -> masterdata_core::Result<ComputedViewRemoveReport> {
        let project = Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, relative_path)?;
        ensure_base_identity(base_source, &target, Some(base_content_identity))?;
        if !confirmed {
            return Err(view_error(
                "E-VIEW-REMOVE-CONFIRMATION",
                "removing a Computed View requires explicit confirmation",
                &target,
            ));
        }
        let current = read_source_state(&project, &target)?;
        if current.content_identity != base_content_identity {
            return Ok(ComputedViewRemoveReport {
                status: SourceSaveStatus::Conflict,
                path: relative_path.to_owned(),
                current: Some(current),
                diagnostic: Some(
                    view_error(
                        "E-VIEW-EDIT-CONFLICT",
                        "View source changed after the editor base snapshot",
                        &target,
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }
        if let Err(error) = remove_source_candidate(&target, current.source.as_bytes()) {
            let after = read_source_state(&project, &target).ok();
            let status = match error.diagnostic().code.as_str() {
                "E-SOURCE-EDIT-CONFLICT" => SourceSaveStatus::Conflict,
                "E-SOURCE-EDIT-OUTCOME-UNKNOWN" => SourceSaveStatus::OutcomeUnknown,
                _ => SourceSaveStatus::Failure,
            };
            return Ok(ComputedViewRemoveReport {
                status,
                path: relative_path.to_owned(),
                current: after,
                diagnostic: Some(error.diagnostic().clone()),
            });
        }
        Ok(ComputedViewRemoveReport {
            status: SourceSaveStatus::Success,
            path: relative_path.to_owned(),
            current: None,
            diagnostic: None,
        })
    }
}

fn requested_document(request: &ComputedViewEditRequest) -> ViewDocument {
    ViewDocument {
        kind: "view".to_owned(),
        name: request.name.clone(),
        table: request.table.clone(),
        columns: request.columns.clone(),
    }
}

fn view_snapshot(
    project: &Project,
    documents: &ProjectDocuments,
    parse_diagnostics: Vec<Diagnostic>,
    target: &Path,
) -> masterdata_core::Result<ComputedViewSnapshot> {
    let loaded = documents
        .files
        .iter()
        .find(|file| file.path == target)
        .ok_or_else(|| {
            view_error(
                "E-VIEW-NOT-FOUND",
                "View source could not be loaded",
                target,
            )
        })?;
    let SourceDocument::View(view) = &loaded.document else {
        return Err(view_error(
            "E-VIEW-NOT-VIEW",
            "source is not a Computed View",
            target,
        ));
    };
    Ok(ComputedViewSnapshot {
        path: project_relative_string(project.root(), target),
        base_source: loaded.source.clone(),
        base_content_identity: source_content_identity(&loaded.source),
        name: view.name.clone(),
        table: view.table.clone(),
        columns: view.columns.clone(),
        diagnostics: parse_diagnostics,
    })
}

fn ensure_base_identity(
    base_source: &str,
    target: &Path,
    expected: Option<&str>,
) -> masterdata_core::Result<()> {
    let actual = source_content_identity(base_source);
    if expected.is_some_and(|expected| expected != actual) {
        return Err(view_error(
            "E-VIEW-EDIT-BASE-IDENTITY",
            "provided View base source does not match its content identity",
            target,
        ));
    }
    Ok(())
}

fn merge_parse_diagnostics(
    report: &mut masterdata_core::ValidationReport,
    diagnostics: Vec<Diagnostic>,
) {
    if diagnostics.is_empty() {
        return;
    }
    report.valid = false;
    report.diagnostics.extend(diagnostics);
}

fn view_error(
    code: &str,
    message: impl Into<String>,
    path: &Path,
) -> masterdata_core::MasterdataError {
    masterdata_core::MasterdataError::new(code, masterdata_core::ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement("ADV-VIEW-005")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn project() -> TempDir {
        let temp = tempfile::tempdir().expect("temporary project");
        fs::create_dir_all(temp.path().join("sources")).expect("sources");
        fs::write(
            temp.path().join("masterdata.toml"),
            "[project]\nid = \"view.authoring\"\nname = \"View Authoring\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
        )
        .expect("config");
        fs::write(
            temp.path().join("sources/item.yaml"),
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: name\n    type: string\nprimaryKey:\n  fields: [name]\n",
        )
        .expect("schema");
        fs::write(
            temp.path().join("sources/view.yaml"),
            "kind: view\nname: display\ntable: item\ncolumns:\n  - name: label\n    expression: 'name + \"!\"'\n# preserve\n",
        )
        .expect("view");
        temp
    }

    #[test]
    fn view_authoring_preview_save_and_stale_conflict_are_source_preserving() {
        let temp = project();
        let service = NativeApplicationService::new();
        let opened = service
            .open_computed_view(Some(temp.path()), temp.path(), "sources/view.yaml")
            .expect("open");
        let preview = service
            .preview_computed_view(
                Some(temp.path()),
                temp.path(),
                "sources/view.yaml",
                &opened.base_source,
                &ComputedViewEditRequest {
                    name: opened.name.clone(),
                    table: opened.table.clone(),
                    columns: vec![ViewColumnDefinition {
                        name: "label".into(),
                        expression: "name + \"?\"".into(),
                    }],
                },
            )
            .expect("preview");
        assert!(preview.changed);
        assert!(preview.validation.valid);
        assert!(
            fs::read_to_string(temp.path().join("sources/view.yaml"))
                .expect("source")
                .contains("name + \"!\"")
        );
        let saved = service
            .save_computed_view(
                Some(temp.path()),
                temp.path(),
                "sources/view.yaml",
                &opened.base_source,
                &opened.base_content_identity,
                &ComputedViewEditRequest {
                    name: opened.name.clone(),
                    table: opened.table.clone(),
                    columns: vec![ViewColumnDefinition {
                        name: "label".into(),
                        expression: "name + \"?\"".into(),
                    }],
                },
            )
            .expect("save");
        assert_eq!(saved.status, SourceSaveStatus::Success);
        assert!(
            fs::read_to_string(temp.path().join("sources/view.yaml"))
                .expect("source")
                .contains("# preserve")
        );
        fs::write(
            temp.path().join("sources/view.yaml"),
            "kind: view\nname: display\ntable: item\ncolumns:\n  - name: label\n    expression: 'name + \"raced\"'\n",
        )
        .expect("race");
        let conflict = service
            .save_computed_view(
                Some(temp.path()),
                temp.path(),
                "sources/view.yaml",
                &opened.base_source,
                &opened.base_content_identity,
                &ComputedViewEditRequest {
                    name: opened.name,
                    table: opened.table,
                    columns: vec![ViewColumnDefinition {
                        name: "label".into(),
                        expression: "name".into(),
                    }],
                },
            )
            .expect("conflict report");
        assert_eq!(conflict.status, SourceSaveStatus::Conflict);

        let remove_conflict = service
            .remove_computed_view(
                Some(temp.path()),
                temp.path(),
                "sources/view.yaml",
                &opened.base_source,
                &opened.base_content_identity,
                true,
            )
            .expect("remove conflict report");
        assert_eq!(remove_conflict.status, SourceSaveStatus::Conflict);
        assert!(temp.path().join("sources/view.yaml").exists());
    }

    #[test]
    fn view_remove_requires_confirmation_and_removes_only_exact_source() {
        let temp = project();
        let service = NativeApplicationService::new();
        let opened = service
            .open_computed_view(Some(temp.path()), temp.path(), "sources/view.yaml")
            .expect("open");
        let error = service
            .remove_computed_view(
                Some(temp.path()),
                temp.path(),
                "sources/view.yaml",
                &opened.base_source,
                &opened.base_content_identity,
                false,
            )
            .expect_err("confirmation");
        assert_eq!(error.diagnostic().code, "E-VIEW-REMOVE-CONFIRMATION");
        let report = service
            .remove_computed_view(
                Some(temp.path()),
                temp.path(),
                "sources/view.yaml",
                &opened.base_source,
                &opened.base_content_identity,
                true,
            )
            .expect("remove");
        assert_eq!(report.status, SourceSaveStatus::Success);
        assert!(!temp.path().join("sources/view.yaml").exists());
    }

    #[test]
    fn opening_a_view_exposes_shared_expression_diagnostics() {
        let temp = project();
        fs::write(
            temp.path().join("sources/view.yaml"),
            "kind: view\nname: display\ntable: item\ncolumns:\n  - name: label\n    expression: missing + 1\n",
        )
        .expect("invalid view");
        let snapshot = NativeApplicationService::new()
            .open_computed_view(Some(temp.path()), temp.path(), "sources/view.yaml")
            .expect("invalid view remains openable");
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-VIEW-UNKNOWN-FIELD")
        );
    }
}
