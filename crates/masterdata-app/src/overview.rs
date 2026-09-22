//! Saved-snapshot Table Overview composition.
//!
//! Filesystem discovery, YAML parsing, tag selection, and query evaluation
//! remain in the shared Rust layers.  The GUI receives rows with provenance
//! and never has to infer identity from a primary key or a path.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{
    AuthoringQuery, AuthoringValue, BuildSelection, Diagnostic, ErrorKind, MasterdataError,
    ProjectDocuments, QueryRow, ResolvedAuthoringField, Result, apply_authoring_query,
    computed_views_for_table, evaluate_computed_column, project_source_value,
    project_typed_source_value, record_tags, resolve_authoring_field_shape, resolve_computed_views,
    source_content_identity, validate_documents,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::NativeApplicationService;
use crate::authoring::{load_authoring_documents, project_relative_string};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableOverviewRequest {
    pub table: String,
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub query: AuthoringQuery,
    #[serde(default)]
    pub selected_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewColumn {
    pub name: String,
    pub type_name: String,
    pub key_field: bool,
    pub shape: Option<ResolvedAuthoringField>,
    #[serde(default)]
    pub computed: bool,
    #[serde(default)]
    pub computed_view: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSourceSnapshot {
    pub path: String,
    pub content_identity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverviewStatus {
    Complete,
    Partial,
    Unavailable,
    Stale,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSelection {
    pub profile: Option<String>,
    pub include_tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableOverviewRow {
    pub table: String,
    pub source_path: String,
    pub record_index: usize,
    pub values: Vec<AuthoringValue>,
    pub selected: Option<bool>,
    pub matched_include_tags: Vec<String>,
    pub matched_exclude_tags: Vec<String>,
    pub selection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableOverviewSnapshot {
    pub status: OverviewStatus,
    pub table: String,
    pub view: Option<String>,
    pub result_identity: String,
    pub config_content_identity: String,
    pub sources: Vec<OverviewSourceSnapshot>,
    pub columns: Vec<OverviewColumn>,
    pub rows: Vec<TableOverviewRow>,
    pub total_count: usize,
    pub selected_count: Option<usize>,
    pub displayed_count: usize,
    pub selection: OverviewSelection,
    pub query: AuthoringQuery,
    pub validation: masterdata_core::ValidationReport,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct OverviewRowBuffer {
    path: String,
    record_index: usize,
    values: Vec<AuthoringValue>,
    query_values: Vec<AuthoringValue>,
    tags: Option<BTreeSet<String>>,
}

impl NativeApplicationService {
    pub fn table_overview(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        request: &TableOverviewRequest,
    ) -> Result<TableOverviewSnapshot> {
        let project = masterdata_core::Project::discover(explicit_project, current_dir)?;
        let source_paths = project.source_files()?;
        let before_sources = source_identities(&project, &source_paths)?;
        let config_identity = project.config_content_identity();
        let selection = match project.build_selection(request.profile.as_deref()) {
            Ok(selection) => selection,
            Err(error) => {
                return Ok(incomplete_snapshot(
                    &project,
                    request,
                    config_identity,
                    Vec::new(),
                    vec![error.diagnostic().clone()],
                    OverviewStatus::Unavailable,
                ));
            }
        };
        let (documents, parse_diagnostics) = load_authoring_documents(&project, None)?;
        let mut diagnostics = parse_diagnostics;
        let computed_build = resolve_computed_views(&documents);
        let selected_view_sources = request.view.as_deref().map(|name| {
            documents
                .views()
                .filter(|(_, view)| view.name == name && view.table == request.table)
                .map(|(path, _)| path.clone())
                .collect::<BTreeSet<_>>()
        });
        if let Some(selected_view_sources) = &selected_view_sources {
            diagnostics.extend(
                computed_build
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        diagnostic
                            .source
                            .as_ref()
                            .is_some_and(|source| selected_view_sources.contains(source))
                    })
                    .cloned(),
            );
        }
        let selected_view = request.view.as_deref().and_then(|name| {
            computed_views_for_table(&computed_build, &request.table).find(|view| view.name == name)
        });
        if request.view.is_some() && selected_view.is_none() {
            diagnostics.push(overview_error(
                "E-AUTHORING-OVERVIEW-VIEW-NOT-FOUND",
                format!(
                    "computed view `{}` is unavailable for Table `{}`",
                    request.view.as_deref().unwrap_or_default(),
                    request.table
                ),
            ));
            return Ok(incomplete_snapshot(
                &project,
                request,
                config_identity,
                Vec::new(),
                diagnostics,
                OverviewStatus::Unavailable,
            ));
        }
        let schemas = documents
            .schemas()
            .filter(|(_, schema)| schema.table == request.table)
            .collect::<Vec<_>>();
        let schema = match schemas.as_slice() {
            [(_, schema)] => *schema,
            [] => {
                diagnostics.push(overview_error(
                    "E-AUTHORING-OVERVIEW-TABLE-NOT-FOUND",
                    format!("table `{}` has no parseable schema", request.table),
                ));
                return Ok(incomplete_snapshot(
                    &project,
                    request,
                    config_identity,
                    Vec::new(),
                    diagnostics,
                    OverviewStatus::Unavailable,
                ));
            }
            _ => {
                diagnostics.push(overview_error(
                    "E-AUTHORING-OVERVIEW-TABLE-AMBIGUOUS",
                    format!("table `{}` has multiple schema documents", request.table),
                ));
                return Ok(incomplete_snapshot(
                    &project,
                    request,
                    config_identity,
                    Vec::new(),
                    diagnostics,
                    OverviewStatus::Unavailable,
                ));
            }
        };
        let key_fields = schema
            .primary_key
            .as_ref()
            .map(|key| key.fields.iter().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let base_columns = schema
            .fields
            .iter()
            .map(|field| OverviewColumn {
                name: field.name.clone(),
                type_name: field.type_name.clone(),
                key_field: key_fields.contains(&field.name),
                shape: resolve_authoring_field_shape(&documents, field),
                computed: false,
                computed_view: None,
            })
            .collect::<Vec<_>>();
        let selected_columns = selected_view
            .map(|view| {
                view.columns
                    .iter()
                    .map(|column| OverviewColumn {
                        name: column.name.clone(),
                        type_name: column.shape.type_name.clone(),
                        key_field: false,
                        shape: Some(column.shape.clone()),
                        computed: true,
                        computed_view: Some(view.name.clone()),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let computed_columns = selected_columns.len();
        let mut columns = base_columns;
        columns.extend(selected_columns);
        let query_shape_indices = query_shapes(&columns, &request.query)?;
        let mut rows = Vec::new();
        for (path, data) in documents
            .data()
            .filter(|(_, data)| data.table == request.table)
        {
            let absolute_path = path.clone();
            let path = project_relative_string(project.root(), &absolute_path);
            for (record_index, record) in data.records.iter().enumerate() {
                let tags = record_tags(record, &absolute_path, record_index, &mut diagnostics);
                let base_values = columns
                    .iter()
                    .take(columns.len() - computed_columns)
                    .map(|column| {
                        record
                            .get(&column.name)
                            .and_then(|value| {
                                column
                                    .shape
                                    .as_ref()
                                    .and_then(|shape| project_typed_source_value(shape, value).ok())
                                    .or_else(|| project_source_value(value).ok())
                            })
                            .unwrap_or(AuthoringValue::Null)
                    })
                    .collect::<Vec<_>>();
                let mut values = base_values;
                if let Some(view) = selected_view {
                    let typed_values = schema
                        .fields
                        .iter()
                        .map(|field| {
                            let value = record
                                .get(&field.name)
                                .map(|value| {
                                    match resolve_authoring_field_shape(&documents, field) {
                                        Some(shape) => {
                                            match project_typed_source_value(&shape, value) {
                                                Ok(value) => value,
                                                Err(error) => AuthoringValue::Invalid {
                                                    diagnostic: Box::new(
                                                        error.diagnostic().clone(),
                                                    ),
                                                },
                                            }
                                        }
                                        None => project_source_value(value)
                                            .unwrap_or(AuthoringValue::Null),
                                    }
                                })
                                .unwrap_or(AuthoringValue::Null);
                            (field.name.clone(), value)
                        })
                        .collect::<std::collections::BTreeMap<_, _>>();
                    for column in &view.columns {
                        values.push(evaluate_computed_column(
                            column,
                            &typed_values,
                            &absolute_path,
                            record_index,
                        ));
                    }
                }
                let query_values = query_shape_indices
                    .iter()
                    .map(|(_, column_index)| values[*column_index].clone())
                    .collect::<Vec<_>>();
                rows.push(OverviewRowBuffer {
                    path: path.clone(),
                    record_index,
                    values,
                    query_values,
                    tags,
                });
            }
        }
        let query_shapes = query_shape_indices
            .iter()
            .map(|(shape, _)| shape.clone())
            .collect::<Vec<_>>();
        let query_rows = rows
            .iter()
            .enumerate()
            .map(|(source_order, row)| QueryRow {
                source_order,
                values: row.query_values.clone(),
            })
            .collect::<Vec<_>>();
        let query_positions = apply_authoring_query(&query_shapes, &query_rows, &request.query)?;
        let total_count = rows.len();
        let mut selection_view = selection_view(&selection, &rows);
        selection_view.metadata.profile = request.profile.clone();
        let selected_count = selection_view.available.then(|| {
            rows.iter()
                .filter(|row| {
                    row.tags
                        .as_ref()
                        .is_some_and(|tags| selection.is_selected(tags))
                })
                .count()
        });
        let rows = query_positions
            .into_iter()
            .filter_map(|position| {
                let row = &rows[position];
                let (selected, include, exclude, reason) =
                    selection_row(&selection, row.tags.as_ref());
                if request.selected_only && selected != Some(true) {
                    return None;
                }
                Some(TableOverviewRow {
                    table: request.table.clone(),
                    source_path: row.path.clone(),
                    record_index: row.record_index,
                    values: row.values.clone(),
                    selected,
                    matched_include_tags: include,
                    matched_exclude_tags: exclude,
                    selection_reason: reason,
                })
            })
            .collect::<Vec<_>>();

        let after_paths = project.source_files()?;
        let after_sources = source_identities(&project, &after_paths)?;
        let config_changed =
            masterdata_core::Project::from_config_path(project.config_path().to_path_buf())
                .ok()
                .is_none_or(|current| current.config_content_identity() != config_identity);
        if before_sources != after_sources || config_changed {
            return Ok(stale_snapshot(
                &project,
                request,
                config_identity,
                after_sources,
                diagnostics,
            ));
        }
        let status = if diagnostics.is_empty() {
            OverviewStatus::Complete
        } else {
            OverviewStatus::Partial
        };
        let result_identity =
            overview_identity(&config_identity, &after_sources, request, &selection);
        Ok(TableOverviewSnapshot {
            status,
            table: request.table.clone(),
            view: request.view.clone(),
            result_identity,
            config_content_identity: config_identity,
            sources: after_sources,
            columns,
            total_count,
            selected_count,
            displayed_count: rows.len(),
            selection: selection_view.metadata,
            rows,
            query: request.query.clone(),
            validation: validate_documents(&documents),
            diagnostics,
        })
    }
}

struct SelectionView {
    metadata: OverviewSelection,
    available: bool,
}

fn selection_view(selection: &BuildSelection, rows: &[OverviewRowBuffer]) -> SelectionView {
    let available = rows.iter().all(|row| row.tags.is_some());
    SelectionView {
        metadata: OverviewSelection {
            profile: None,
            include_tags: selection.include_tags().iter().cloned().collect(),
            exclude_tags: selection.exclude_tags().iter().cloned().collect(),
            available,
        },
        available,
    }
}

fn selection_row(
    selection: &BuildSelection,
    tags: Option<&BTreeSet<String>>,
) -> (Option<bool>, Vec<String>, Vec<String>, Option<String>) {
    let Some(tags) = tags else {
        return (
            None,
            Vec::new(),
            Vec::new(),
            Some("selection unavailable for invalid tags".into()),
        );
    };
    let include = tags
        .intersection(selection.include_tags())
        .cloned()
        .collect::<Vec<_>>();
    let exclude = tags
        .intersection(selection.exclude_tags())
        .cloned()
        .collect::<Vec<_>>();
    let selected = selection.is_selected(tags);
    let reason = if !exclude.is_empty() {
        Some("excluded by matching tag".into())
    } else if selection.include_tags().is_empty() {
        Some("include set is empty".into())
    } else if include.is_empty() {
        Some("no include tag matched".into())
    } else {
        Some("include tag matched".into())
    };
    (Some(selected), include, exclude, reason)
}

fn query_shapes(
    columns: &[OverviewColumn],
    query: &AuthoringQuery,
) -> Result<Vec<(ResolvedAuthoringField, usize)>> {
    let mut requested = BTreeSet::new();
    requested.extend(query.filters.iter().map(|filter| filter.field.clone()));
    if let Some(sort) = &query.sort {
        requested.insert(sort.field.clone());
    }
    let mut shapes = Vec::new();
    for (index, column) in columns.iter().enumerate() {
        if !requested.is_empty() && !requested.contains(&column.name) && query.search.is_empty() {
            continue;
        }
        let Some(shape) = column.shape.clone() else {
            if requested.contains(&column.name) {
                return Err(MasterdataError::new(
                    if column.computed {
                        "E-VIEW-QUERY-UNAVAILABLE"
                    } else {
                        "E-AUTHORING-QUERY-UNAVAILABLE"
                    },
                    ErrorKind::Validation,
                    format!("query field `{}` has no resolved type shape", column.name),
                )
                .with_related_requirement("AUTHORING-OVERVIEW-001"));
            }
            continue;
        };
        shapes.push((shape, index));
    }
    Ok(shapes)
}

fn source_identities(
    project: &masterdata_core::Project,
    paths: &[PathBuf],
) -> Result<Vec<OverviewSourceSnapshot>> {
    paths
        .iter()
        .map(|path| {
            let source = fs::read_to_string(path).map_err(|error| {
                MasterdataError::new(
                    "E-AUTHORING-OVERVIEW-READ",
                    ErrorKind::Io,
                    format!("could not read overview source: {error}"),
                )
                .with_source(path.clone())
            })?;
            Ok(OverviewSourceSnapshot {
                path: project_relative_string(project.root(), path),
                content_identity: source_content_identity(&source),
            })
        })
        .collect()
}

fn overview_identity(
    config_identity: &str,
    sources: &[OverviewSourceSnapshot],
    request: &TableOverviewRequest,
    selection: &BuildSelection,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(config_identity.as_bytes());
    hasher.update(request.table.as_bytes());
    hasher.update(request.view.as_deref().unwrap_or_default().as_bytes());
    hasher.update(serde_json::to_vec(&request.query).unwrap_or_default());
    hasher.update([request.selected_only as u8]);
    for tag in selection.include_tags() {
        hasher.update(b"include:");
        hasher.update(tag.as_bytes());
    }
    for tag in selection.exclude_tags() {
        hasher.update(b"exclude:");
        hasher.update(tag.as_bytes());
    }
    for source in sources {
        hasher.update(source.path.as_bytes());
        hasher.update(source.content_identity.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn incomplete_snapshot(
    project: &masterdata_core::Project,
    request: &TableOverviewRequest,
    config_identity: String,
    rows: Vec<TableOverviewRow>,
    diagnostics: Vec<Diagnostic>,
    status: OverviewStatus,
) -> TableOverviewSnapshot {
    let sources =
        source_identities(project, &project.source_files().unwrap_or_default()).unwrap_or_default();
    TableOverviewSnapshot {
        status,
        table: request.table.clone(),
        view: request.view.clone(),
        result_identity: overview_identity(
            &config_identity,
            &sources,
            request,
            &BuildSelection::unfiltered(),
        ),
        config_content_identity: config_identity,
        sources,
        columns: Vec::new(),
        total_count: rows.len(),
        selected_count: None,
        displayed_count: rows.len(),
        selection: OverviewSelection {
            profile: request.profile.clone(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            available: false,
        },
        rows,
        query: request.query.clone(),
        validation: masterdata_core::ValidationReport {
            valid: false,
            files_scanned: 0,
            schema_documents: 0,
            data_documents: 0,
            type_documents: 0,
            tables: Vec::new(),
            types: Vec::new(),
            diagnostics: diagnostics.clone(),
        },
        diagnostics,
    }
}

fn stale_snapshot(
    _project: &masterdata_core::Project,
    request: &TableOverviewRequest,
    config_identity: String,
    sources: Vec<OverviewSourceSnapshot>,
    mut diagnostics: Vec<Diagnostic>,
) -> TableOverviewSnapshot {
    diagnostics.push(overview_error(
        "E-AUTHORING-OVERVIEW-STALE",
        "source or configuration changed while the Overview snapshot was loading; refresh is required",
    ));
    TableOverviewSnapshot {
        status: OverviewStatus::Stale,
        table: request.table.clone(),
        view: request.view.clone(),
        result_identity: overview_identity(
            &config_identity,
            &sources,
            request,
            &BuildSelection::unfiltered(),
        ),
        config_content_identity: config_identity,
        sources,
        columns: Vec::new(),
        rows: Vec::new(),
        total_count: 0,
        selected_count: None,
        displayed_count: 0,
        selection: OverviewSelection {
            profile: request.profile.clone(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            available: false,
        },
        query: request.query.clone(),
        validation: validate_documents(&ProjectDocuments::default()),
        diagnostics,
    }
}

fn overview_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(code, ErrorKind::Validation, message)
        .with_related_requirement("AUTHORING-OVERVIEW-001")
}

#[cfg(test)]
mod tests {
    use super::{OverviewStatus, TableOverviewRequest};
    use crate::NativeApplicationService;
    use masterdata_core::{
        AuthoringQuery, AuthoringValue, ColumnFilter, QueryOperator, QuerySort, SortDirection,
    };
    use std::fs;
    use tempfile::TempDir;

    fn project() -> TempDir {
        let temp = tempfile::tempdir().expect("temporary project");
        fs::create_dir_all(temp.path().join("sources/data")).expect("data root");
        fs::create_dir_all(temp.path().join("sources/schemas")).expect("schema root");
        fs::write(
            temp.path().join("masterdata.toml"),
            r#"[project]
id = "overview.test"
name = "Overview"
version = "0.1.0"

[sources]
roots = ["sources"]

[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"

[build.profiles.release]
include_tags = ["release"]
exclude_tags = ["debug"]
"#,
        )
        .expect("config");
        fs::write(
            temp.path().join("sources/schemas/item.yaml"),
            r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: int
  - key: 1
    name: label
    type: string
primaryKey:
  fields: [id]
secondaryKeys: []
"#,
        )
        .expect("schema");
        fs::write(
            temp.path().join("sources/data/a.yaml"),
            r#"kind: data
table: item
records:
  - id: 1
    label: first
    $tags: [release]
"#,
        )
        .expect("first data");
        fs::write(
            temp.path().join("sources/data/b.yaml"),
            r#"kind: data
table: item
records:
  - id: 2
    label: second
    $tags: [release]
  - id: 3
    label: debug
    $tags: [release, debug]
"#,
        )
        .expect("second data");
        temp
    }

    #[test]
    fn overview_merges_split_sources_and_exposes_profile_selection() {
        let temp = project();
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: None,
                    profile: Some("release".to_owned()),
                    query: Default::default(),
                    selected_only: false,
                },
            )
            .expect("overview");

        assert!(matches!(snapshot.status, OverviewStatus::Complete));
        assert_eq!(snapshot.total_count, 3);
        assert_eq!(snapshot.displayed_count, 3);
        assert_eq!(snapshot.selected_count, Some(2));
        assert_eq!(snapshot.selection.profile.as_deref(), Some("release"));
        assert!(snapshot.selection.available);
        assert_eq!(snapshot.sources.len(), 3);
        assert_eq!(
            snapshot
                .rows
                .iter()
                .filter(|row| row.selected == Some(true))
                .count(),
            2
        );
        assert!(snapshot.rows.iter().all(|row| row.table == "item"));
    }

    #[test]
    fn selected_only_is_applied_after_query_without_deduplicating_primary_keys() {
        let temp = project();
        fs::write(
            temp.path().join("sources/data/b.yaml"),
            r#"kind: data
table: item
records:
  - id: 1
    label: duplicate-key-in-split-source
    $tags: [release]
  - id: 3
    label: debug
    $tags: [debug]
"#,
        )
        .expect("replacement data");
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: None,
                    profile: Some("release".to_owned()),
                    query: Default::default(),
                    selected_only: true,
                },
            )
            .expect("selected overview");

        assert_eq!(snapshot.total_count, 3);
        assert_eq!(snapshot.selected_count, Some(2));
        assert_eq!(snapshot.displayed_count, 2);
        assert_eq!(
            snapshot
                .rows
                .iter()
                .filter(|row| row.values[0]
                    == masterdata_core::AuthoringValue::Number { value: "1".into() })
                .count(),
            2
        );
    }

    #[test]
    fn missing_profile_is_unavailable_without_unfiltered_fallback() {
        let temp = project();
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: None,
                    profile: Some("missing".to_owned()),
                    query: Default::default(),
                    selected_only: false,
                },
            )
            .expect("overview unavailable snapshot");

        assert!(matches!(snapshot.status, OverviewStatus::Unavailable));
        assert!(!snapshot.selection.available);
        assert!(snapshot.rows.is_empty());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-BUILD-PROFILE-NOT-FOUND")
        );
    }

    #[test]
    fn overview_evaluates_selected_computed_view_and_composes_query() {
        let temp = project();
        fs::write(
            temp.path().join("sources/schemas/item-view.yaml"),
            r#"kind: view
name: display
table: item
columns:
  - name: displayLabel
    expression: 'label + "!"'
"#,
        )
        .expect("view");
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: Some("display".to_owned()),
                    profile: None,
                    query: AuthoringQuery {
                        filters: vec![ColumnFilter {
                            field: "displayLabel".to_owned(),
                            operator: QueryOperator::Contains,
                            value: Some(AuthoringValue::String {
                                value: "first!".to_owned(),
                            }),
                        }],
                        ..Default::default()
                    },
                    selected_only: false,
                },
            )
            .expect("computed overview");
        assert_eq!(snapshot.view.as_deref(), Some("display"));
        assert!(
            snapshot
                .columns
                .last()
                .is_some_and(|column| column.computed)
        );
        assert_eq!(snapshot.displayed_count, 1);
        assert_eq!(
            snapshot.rows[0].values.last(),
            Some(&AuthoringValue::String {
                value: "first!".into()
            })
        );
    }

    #[test]
    fn invalid_computed_cell_is_visible_without_mutating_source() {
        let temp = project();
        let source_path = temp.path().join("sources/data/a.yaml");
        let before = fs::read_to_string(&source_path).expect("source");
        fs::write(&source_path, before.replace("label: first", "label: 10"))
            .expect("invalid source value");
        let invalid_before = fs::read_to_string(&source_path).expect("invalid source");
        fs::write(
            temp.path().join("sources/schemas/item-view.yaml"),
            r#"kind: view
name: display
table: item
columns:
  - name: doubled
    expression: 'label + "!"'
"#,
        )
        .expect("view");
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: Some("display".to_owned()),
                    profile: None,
                    query: Default::default(),
                    selected_only: false,
                },
            )
            .expect("computed overview");
        assert!(matches!(
            snapshot.rows[0].values.last(),
            Some(AuthoringValue::Invalid { .. })
        ));
        assert_eq!(
            fs::read_to_string(source_path).expect("source"),
            invalid_before
        );
        assert_ne!(before, invalid_before);
    }

    #[test]
    fn computed_scalar_uses_shared_search_and_sort_semantics() {
        let temp = project();
        fs::write(
            temp.path().join("sources/schemas/item-view.yaml"),
            "kind: view\nname: scores\ntable: item\ncolumns:\n  - name: score\n    expression: 'id + 1'\n",
        )
        .expect("view");
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: Some("scores".to_owned()),
                    profile: None,
                    query: AuthoringQuery {
                        search: "3".to_owned(),
                        sort: Some(QuerySort {
                            field: "score".to_owned(),
                            direction: SortDirection::Descending,
                        }),
                        ..Default::default()
                    },
                    selected_only: false,
                },
            )
            .expect("computed search/sort overview");
        assert_eq!(snapshot.displayed_count, 2);
        assert_eq!(
            snapshot.rows[0].values.last(),
            Some(&AuthoringValue::Number { value: "4".into() })
        );
    }

    #[test]
    fn invalid_selected_view_is_unavailable_without_showing_a_base_projection() {
        let temp = project();
        fs::write(
            temp.path().join("sources/schemas/item-view.yaml"),
            "kind: view\nname: broken\ntable: item\ncolumns:\n  - name: computedLabel\n    expression: missing\n",
        )
        .expect("invalid view");
        let snapshot = NativeApplicationService::new()
            .table_overview(
                Some(temp.path()),
                temp.path(),
                &TableOverviewRequest {
                    table: "item".to_owned(),
                    view: Some("broken".to_owned()),
                    profile: None,
                    query: Default::default(),
                    selected_only: false,
                },
            )
            .expect("unavailable computed view");
        assert!(matches!(snapshot.status, OverviewStatus::Unavailable));
        assert!(snapshot.rows.is_empty());
        assert!(snapshot.columns.is_empty());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-VIEW-UNKNOWN-FIELD"),
            "diagnostics: {:?}",
            snapshot.diagnostics
        );
    }
}
