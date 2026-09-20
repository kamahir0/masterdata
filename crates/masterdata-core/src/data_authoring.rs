use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    AuthoringValue, BuildProfileInfo, Diagnostic, ErrorKind, MasterdataError, ProjectDocuments,
    ResolvedAuthoringField, SchemaDocument, SourceDocument, ValidationReport, project_source_value,
    project_typed_source_value, resolve_authoring_field_shape, source_content_identity,
    validate_documents,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorColumn {
    pub name: String,
    pub type_name: String,
    pub editable: bool,
    pub key_field: bool,
    pub shape: Option<ResolvedAuthoringField>,
    pub read_only_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorCell {
    pub field: String,
    pub text: String,
    pub value: AuthoringValue,
    pub editable: bool,
    pub read_only_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorRow {
    pub record_index: usize,
    pub cells: Vec<DataEditorCell>,
    /// `$tags` is record metadata, not a domain column.  Keep it alongside
    /// the row so adapters can offer a separate tag editor without teaching
    /// the domain-field grid about the reserved metadata key.
    pub tags: Vec<String>,
    pub tags_editable: bool,
    pub tags_read_only_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEditorAddCapability {
    pub supported: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFileSnapshot {
    pub path: String,
    pub table: String,
    pub base_source: String,
    pub base_content_identity: String,
    pub columns: Vec<DataEditorColumn>,
    pub rows: Vec<DataEditorRow>,
    pub tag_candidates: Vec<String>,
    pub tag_candidates_complete: bool,
    pub add_row: DataEditorAddCapability,
    pub validation: ValidationReport,
}

pub fn data_file_snapshot(
    project_root: &Path,
    profiles: &[BuildProfileInfo],
    documents: &ProjectDocuments,
    mut parse_diagnostics: Vec<Diagnostic>,
    target: &Path,
) -> crate::Result<DataFileSnapshot> {
    let loaded = documents
        .files
        .iter()
        .find(|loaded| loaded.path == target)
        .ok_or_else(|| {
            snapshot_error(
                "E-GUI-DATA-FILE-UNAVAILABLE",
                "selected data file is not a parseable Masterdata document",
                Some(target.to_path_buf()),
                "GUI-DATA-LAYOUT-001",
            )
        })?;
    let SourceDocument::Data(data) = &loaded.document else {
        return Err(snapshot_error(
            "E-GUI-DATA-FILE-KIND",
            "selected source is not a Data document",
            Some(target.to_path_buf()),
            "GUI-EXPLORER-003",
        ));
    };
    let schema = unique_schema(documents, &data.table, target)?;
    let key_fields = schema_key_fields(schema);
    let columns = schema
        .fields
        .iter()
        .map(|field| {
            let key_field = key_fields.contains(field.name.as_str());
            let shape = resolve_authoring_field_shape(documents, field);
            let read_only_reason = if key_field {
                None
            } else if shape.is_none() {
                Some(format!(
                    "The shared Type System could not resolve `{}` as a supported authoring shape.",
                    field.type_name
                ))
            } else {
                None
            };
            DataEditorColumn {
                name: field.name.clone(),
                type_name: field.type_name.clone(),
                editable: shape.is_some(),
                key_field,
                shape,
                read_only_reason,
            }
        })
        .collect::<Vec<_>>();
    let rows = data
        .records
        .iter()
        .enumerate()
        .map(|(record_index, record)| {
            let mut tag_diagnostics = Vec::new();
            let tags = crate::record_tags(record, target, record_index, &mut tag_diagnostics);
            let tags_read_only_reason = tags.is_none().then(|| {
                tag_diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_else(|| {
                        "The source `$tags` value is invalid or ambiguous.".to_owned()
                    })
            });
            let source_tags = record
                .get("$tags")
                .and_then(serde_yaml::Value::as_sequence)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_yaml::Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            DataEditorRow {
                record_index,
                tags: source_tags,
                tags_editable: tags_read_only_reason.is_none(),
                tags_read_only_reason,
                cells: columns
                    .iter()
                    .map(|column| DataEditorCell {
                        field: column.name.clone(),
                        text: record
                            .get(&column.name)
                            .map(display_value)
                            .unwrap_or_default(),
                        value: match record.get(&column.name) {
                            Some(value) => column
                                .shape
                                .as_ref()
                                .and_then(|shape| project_typed_source_value(shape, value).ok())
                                .or_else(|| project_source_value(value).ok())
                                .unwrap_or_else(|| AuthoringValue::String {
                                    value: display_value(value),
                                }),
                            None => AuthoringValue::Null,
                        },
                        editable: column.editable
                            && record.get(&column.name).is_some_and(|value| {
                                column.shape.as_ref().is_some_and(|shape| {
                                    project_typed_source_value(shape, value).is_ok()
                                })
                            }),
                        read_only_reason: if !record.contains_key(&column.name) {
                            Some(format!("The source record is missing `{}`.", column.name))
                        } else if let Some(shape) = &column.shape {
                            record
                                .get(&column.name)
                                .and_then(|value| project_typed_source_value(shape, value).err())
                                .map(|error| error.diagnostic().message.clone())
                                .or_else(|| column.read_only_reason.clone())
                        } else {
                            column.read_only_reason.clone()
                        },
                    })
                    .collect(),
            }
        })
        .collect();
    let add_row = data_editor_add_capability(documents, schema);
    let tag_candidates = profiles
        .iter()
        .flat_map(|profile| profile.include_tags.iter().chain(&profile.exclude_tags))
        .cloned()
        .chain(documents.data().flat_map(|(_, data)| {
            data.records.iter().flat_map(|record| {
                record
                    .get("$tags")
                    .and_then(serde_yaml::Value::as_sequence)
                    .into_iter()
                    .flat_map(|items| {
                        items
                            .iter()
                            .filter_map(serde_yaml::Value::as_str)
                            .map(str::to_owned)
                    })
            })
        }))
        .collect::<BTreeSet<_>>();
    let parse_complete = parse_diagnostics.is_empty();
    let mut validation = validate_documents(documents);
    let tag_candidates_complete = parse_complete
        && !validation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("E-BUILD-RECORD-TAGS"));
    merge_parse_diagnostics(&mut validation, &mut parse_diagnostics);
    Ok(DataFileSnapshot {
        path: relative_string(project_root, target),
        table: data.table.clone(),
        base_source: loaded.source.clone(),
        base_content_identity: source_content_identity(&loaded.source),
        columns,
        rows,
        tag_candidates: tag_candidates.into_iter().collect(),
        tag_candidates_complete,
        add_row,
        validation,
    })
}

fn data_editor_add_capability(
    documents: &ProjectDocuments,
    schema: &SchemaDocument,
) -> DataEditorAddCapability {
    if schema.fields.is_empty() {
        return DataEditorAddCapability {
            supported: false,
            reason: Some("Add Row requires a Table with at least one field.".to_owned()),
        };
    }
    let unsupported = schema
        .fields
        .iter()
        .find(|field| resolve_authoring_field_shape(documents, field).is_none());
    match unsupported {
        Some(field) => DataEditorAddCapability {
            supported: false,
            reason: Some(format!(
                "Field `{}` does not have a supported resolved authoring shape.",
                field.name
            )),
        },
        None => DataEditorAddCapability {
            supported: true,
            reason: None,
        },
    }
}

fn unique_schema<'a>(
    documents: &'a ProjectDocuments,
    table: &str,
    target: &Path,
) -> crate::Result<&'a SchemaDocument> {
    let schemas = documents
        .files
        .iter()
        .filter_map(|loaded| match &loaded.document {
            SourceDocument::Schema(schema) if schema.table == table => Some(schema),
            _ => None,
        })
        .collect::<Vec<_>>();
    match schemas.as_slice() {
        [schema] => Ok(*schema),
        [] => Err(snapshot_error(
            "E-GUI-DATA-SCHEMA-NOT-FOUND",
            format!("table `{table}` has no parseable schema document"),
            Some(target.to_path_buf()),
            "GUI-DATA-LAYOUT-001",
        )),
        _ => Err(snapshot_error(
            "E-GUI-DATA-SCHEMA-AMBIGUOUS",
            format!("table `{table}` has multiple schema documents"),
            Some(target.to_path_buf()),
            "GUI-DATA-LAYOUT-001",
        )),
    }
}

fn schema_key_fields(schema: &SchemaDocument) -> BTreeSet<&str> {
    let mut result = BTreeSet::new();
    if let Some(primary) = &schema.primary_key {
        result.extend(primary.fields.iter().map(String::as_str));
    }
    for secondary in &schema.secondary_keys {
        result.extend(secondary.fields.iter().map(String::as_str));
    }
    result
}

fn display_value(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null => "null".to_owned(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::String(value) => value.clone(),
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => serde_yaml::to_string(value)
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|_| "<unsupported>".to_owned()),
    }
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => part.to_str().map(str::to_owned),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn merge_parse_diagnostics(report: &mut ValidationReport, diagnostics: &mut Vec<Diagnostic>) {
    if !diagnostics.is_empty() {
        report.valid = false;
        report.diagnostics.append(diagnostics);
    }
}

fn snapshot_error(
    code: &str,
    message: impl Into<String>,
    source: Option<PathBuf>,
    requirement: &str,
) -> MasterdataError {
    let mut error = MasterdataError::new(code, ErrorKind::Validation, message)
        .with_related_requirement(requirement);
    if let Some(source) = source {
        error = error.with_source(source);
    }
    error
}
