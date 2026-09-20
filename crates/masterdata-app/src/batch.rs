//! Application boundary for scalar range paste/fill.
//!
//! The clipboard grammar and typed input conversion live in `masterdata-core`.
//! This module only binds that shared behavior to one source-file snapshot and
//! composes the resulting edits with the existing source-record mutation.

use std::collections::BTreeSet;
use std::path::Path;

use masterdata_core::{
    AuthoringValue, MasterdataError, Result, authoring_value_from_clipboard,
    authoring_value_to_clipboard, decode_clipboard_tsv, encode_clipboard_tsv,
    is_scalar_batch_field,
};
use serde::{Deserialize, Serialize};

use crate::authoring::{
    AuthoringEdit, AuthoringRecordMutation, DataFileSnapshot, SourceEditPreview,
};
use crate::{NativeApplicationService, load_authoring_documents};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTarget {
    #[serde(default)]
    pub record_index: Option<usize>,
    #[serde(default)]
    pub added_record_index: Option<usize>,
    pub field: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringBatchRequest {
    pub targets: Vec<BatchTarget>,
    pub clipboard_text: String,
    #[serde(default)]
    pub fill: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCellChange {
    pub record_index: Option<usize>,
    pub added_record_index: Option<usize>,
    pub field: String,
    pub before: AuthoringValue,
    pub after: AuthoringValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringBatchPreview {
    pub source: SourceEditPreview,
    pub target_count: usize,
    pub changed_cell_count: usize,
    pub changes: Vec<BatchCellChange>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringBatchCopyRequest {
    pub targets: Vec<BatchTarget>,
    #[serde(default)]
    pub current_mutation: AuthoringRecordMutation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringBatchCopyResult {
    pub clipboard_text: String,
    pub target_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringClipboardShape {
    pub rows: usize,
    pub columns: usize,
}

impl NativeApplicationService {
    pub fn authoring_clipboard_shape(
        &self,
        clipboard_text: &str,
    ) -> Result<AuthoringClipboardShape> {
        let decoded = decode_clipboard_tsv(clipboard_text).map_err(|error| {
            batch_error(
                "E-AUTHORING-BATCH-CODEC",
                format!(
                    "clipboard text is not valid rectangular TSV: {}",
                    error.diagnostic().message
                ),
            )
        })?;
        let columns = decoded.first().map_or(0, Vec::len);
        if decoded.is_empty() || columns == 0 || decoded.iter().any(|row| row.len() != columns) {
            return Err(batch_error(
                "E-AUTHORING-BATCH-SHAPE",
                "clipboard must contain a non-empty rectangular TSV matrix",
            ));
        }
        Ok(AuthoringClipboardShape {
            rows: decoded.len(),
            columns,
        })
    }

    /// Prepare a paste/fill candidate against the caller's current local
    /// mutation. The method never writes the source file.
    pub fn preview_data_file_batch(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        current_mutation: &AuthoringRecordMutation,
        request: &AuthoringBatchRequest,
    ) -> Result<AuthoringBatchPreview> {
        if request.targets.is_empty() {
            return Err(batch_error(
                "E-AUTHORING-BATCH-TARGETS",
                "batch operation has no targets",
            ));
        }
        let project = masterdata_core::Project::discover(explicit_project, current_dir)?;
        let target = super::authoring::resolve_source_file(&project, relative_path)?;
        let (documents, parse_diagnostics) =
            load_authoring_documents(&project, Some((&target, base_source)))?;
        let snapshot =
            super::authoring::data_file_snapshot(&project, &documents, parse_diagnostics, &target)?;
        let (mut mutation, changes) = build_batch_mutation(&snapshot, current_mutation, request)?;
        let preview = self.preview_data_file_mutation(
            Some(project.root()),
            project.root(),
            relative_path,
            base_source,
            &mutation,
        )?;
        mutation.edits.shrink_to_fit();
        let changed_cell_count = changes
            .iter()
            .filter(|change| change.before != change.after)
            .count();
        Ok(AuthoringBatchPreview {
            source: preview,
            target_count: request.targets.len(),
            changed_cell_count,
            changes,
        })
    }

    /// Copy a rectangular range through the same shared scalar validation and
    /// TSV codec used by paste. The operation reads the current local buffer
    /// only and never writes the source file or the system clipboard.
    pub fn copy_data_file_batch(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        relative_path: &str,
        base_source: &str,
        request: &AuthoringBatchCopyRequest,
    ) -> Result<AuthoringBatchCopyResult> {
        if request.targets.is_empty() {
            return Err(batch_error(
                "E-AUTHORING-BATCH-COPY-TARGETS",
                "copy operation has no targets",
            ));
        }
        let project = masterdata_core::Project::discover(explicit_project, current_dir)?;
        let target = super::authoring::resolve_source_file(&project, relative_path)?;
        let (documents, parse_diagnostics) =
            load_authoring_documents(&project, Some((&target, base_source)))?;
        let base_snapshot = super::authoring::data_file_snapshot(
            &project,
            &documents,
            parse_diagnostics.clone(),
            &target,
        )?;
        let base_record_count = base_snapshot.rows.len();
        let mut query_mutation = request.current_mutation.clone();
        query_mutation.deleted_record_indices.clear();
        let documents = if query_mutation.edits.is_empty()
            && query_mutation.added_records.is_empty()
            && query_mutation.tag_edits.is_empty()
        {
            documents
        } else {
            masterdata_core::dry_run_source_record_mutation(
                &documents,
                &target,
                &masterdata_core::SourceRecordMutation::from(&query_mutation),
            )?
            .transformed_documents
        };
        let snapshot =
            super::authoring::data_file_snapshot(&project, &documents, parse_diagnostics, &target)?;
        let _ = validate_target_layout(
            &snapshot,
            &AuthoringBatchRequest {
                targets: request.targets.clone(),
                clipboard_text: String::new(),
                fill: false,
            },
            base_record_count,
        )?;

        let mut rows = Vec::<Vec<String>>::new();
        let mut current_row = None;
        for batch_target in &request.targets {
            let (row_index, column_index) =
                target_position(&snapshot, batch_target, base_record_count)?;
            if request
                .current_mutation
                .deleted_record_indices
                .contains(&row_index)
            {
                return Err(batch_error(
                    "E-AUTHORING-BATCH-COPY-INVALID",
                    format!("record[{row_index}] is pending delete and cannot be copied"),
                ));
            }
            if current_row != Some(row_index) {
                current_row = Some(row_index);
                rows.push(Vec::new());
            }
            let column = snapshot.columns.get(column_index).ok_or_else(|| {
                batch_error(
                    "E-AUTHORING-BATCH-COPY-FIELD",
                    "copy target column is outside the captured snapshot",
                )
            })?;
            let value = snapshot
                .rows
                .iter()
                .find(|row| row.record_index == row_index)
                .and_then(|row| row.cells.get(column_index))
                .map(|cell| &cell.value)
                .ok_or_else(|| {
                    batch_error(
                        "E-AUTHORING-BATCH-COPY-TARGETS",
                        format!("record[{row_index}] target does not exist"),
                    )
                })?;
            let text = authoring_value_to_clipboard(
                column.shape.as_ref().ok_or_else(|| {
                    batch_error(
                        "E-AUTHORING-BATCH-COPY-UNSUPPORTED",
                        format!("field `{}` has no resolved scalar shape", column.name),
                    )
                })?,
                value,
            )
            .map_err(|error| {
                batch_error(
                    "E-AUTHORING-BATCH-COPY-INVALID",
                    error.diagnostic().message.clone(),
                )
            })?;
            rows.last_mut().expect("row created above").push(text);
        }
        Ok(AuthoringBatchCopyResult {
            clipboard_text: encode_clipboard_tsv(&rows)?,
            target_count: request.targets.len(),
        })
    }
}

fn build_batch_mutation(
    snapshot: &DataFileSnapshot,
    current: &AuthoringRecordMutation,
    request: &AuthoringBatchRequest,
) -> Result<(AuthoringRecordMutation, Vec<BatchCellChange>)> {
    let (target_height, target_width) =
        validate_target_layout(snapshot, request, snapshot.rows.len())?;
    let decoded = decode_clipboard_tsv(&request.clipboard_text).map_err(|error| {
        batch_error(
            "E-AUTHORING-BATCH-CODEC",
            format!(
                "clipboard text is not valid rectangular TSV: {}",
                error.diagnostic().message
            ),
        )
    })?;
    let values = if request.fill {
        if decoded.len() != 1 || decoded[0].len() != 1 {
            return Err(batch_error(
                "E-AUTHORING-BATCH-SHAPE",
                "fill requires exactly one clipboard field",
            ));
        }
        vec![decoded[0][0].clone(); request.targets.len()]
    } else {
        let width = decoded.first().map_or(0, Vec::len);
        if decoded.iter().any(|row| row.len() != width) {
            return Err(batch_error(
                "E-AUTHORING-BATCH-SHAPE",
                "clipboard rows do not form a rectangle",
            ));
        }
        if decoded.len() != target_height || width != target_width {
            return Err(batch_error(
                "E-AUTHORING-BATCH-SHAPE",
                format!(
                    "clipboard rectangle is {}x{width}, but the target rectangle is {target_height}x{target_width}",
                    decoded.len()
                ),
            ));
        }
        decoded.into_iter().flatten().collect()
    };

    let mut mutation = current.clone();
    let mut changes = Vec::with_capacity(request.targets.len());
    for (target, text) in request.targets.iter().zip(values) {
        let column = snapshot
            .columns
            .iter()
            .find(|column| column.name == target.field)
            .ok_or_else(|| {
                batch_error(
                    "E-AUTHORING-BATCH-FIELD",
                    format!("unknown batch field `{}`", target.field),
                )
            })?;
        let shape = column.shape.as_ref().ok_or_else(|| {
            batch_error(
                "E-AUTHORING-BATCH-UNSUPPORTED",
                format!(
                    "field `{}` has no resolved scalar batch shape",
                    target.field
                ),
            )
        })?;
        if !is_scalar_batch_field(shape) {
            return Err(batch_error(
                "E-AUTHORING-BATCH-UNSUPPORTED",
                format!("field `{}` is outside scalar range editing", target.field),
            ));
        }
        let after = authoring_value_from_clipboard(shape, &text).map_err(|error| {
            batch_error(
                "E-AUTHORING-BATCH-CODEC",
                format!(
                    "could not convert clipboard value for `{}`: {}",
                    target.field,
                    error.diagnostic().message
                ),
            )
        })?;
        if let Some(record_index) = target.record_index {
            let cell = snapshot
                .rows
                .iter()
                .find(|row| row.record_index == record_index)
                .and_then(|row| row.cells.iter().find(|cell| cell.field == target.field))
                .ok_or_else(|| {
                    batch_error(
                        "E-AUTHORING-BATCH-TARGETS",
                        format!("record[{record_index}] target does not exist"),
                    )
                })?;
            if !column.editable || !cell.editable {
                return Err(batch_error(
                    "E-AUTHORING-BATCH-READ-ONLY",
                    format!("record[{record_index}].{} is read-only", target.field),
                ));
            }
            if column.key_field {
                return Err(batch_error(
                    "E-AUTHORING-BATCH-READ-ONLY",
                    format!(
                        "record[{record_index}].{} is an existing key field and cannot be changed by a batch operation",
                        target.field
                    ),
                ));
            }
            if mutation.deleted_record_indices.contains(&record_index) {
                return Err(batch_error(
                    "E-AUTHORING-BATCH-READ-ONLY",
                    format!("record[{record_index}] is pending delete"),
                ));
            }
            let before = current_edit_value(snapshot, &mutation, record_index, &target.field);
            replace_existing_edit(
                &mut mutation.edits,
                record_index,
                &target.field,
                after.clone(),
            );
            changes.push(BatchCellChange {
                record_index: Some(record_index),
                added_record_index: None,
                field: target.field.clone(),
                before,
                after,
            });
        } else if let Some(added_record_index) = target.added_record_index {
            let draft = mutation
                .added_records
                .get_mut(added_record_index)
                .ok_or_else(|| {
                    batch_error(
                        "E-AUTHORING-BATCH-TARGETS",
                        format!("added record draft[{added_record_index}] does not exist"),
                    )
                })?;
            let field = draft
                .fields
                .iter_mut()
                .find(|field| field.field == target.field)
                .ok_or_else(|| {
                    batch_error(
                        "E-AUTHORING-BATCH-TARGETS",
                        format!(
                            "added draft[{added_record_index}] has no field `{}`",
                            target.field
                        ),
                    )
                })?;
            let before = field.value.clone();
            field.value = after.clone();
            changes.push(BatchCellChange {
                record_index: None,
                added_record_index: Some(added_record_index),
                field: target.field.clone(),
                before,
                after,
            });
        } else {
            return Err(batch_error(
                "E-AUTHORING-BATCH-TARGETS",
                "each batch target must identify an existing or added record",
            ));
        }
    }
    Ok((mutation, changes))
}

fn validate_target_layout(
    snapshot: &DataFileSnapshot,
    request: &AuthoringBatchRequest,
    added_record_base: usize,
) -> Result<(usize, usize)> {
    let mut seen = BTreeSet::new();
    let mut positions = Vec::with_capacity(request.targets.len());
    for target in &request.targets {
        let (row_position, column_index) = target_position(snapshot, target, added_record_base)?;
        let position = (row_position, column_index);
        if !seen.insert(position) {
            return Err(batch_error(
                "E-AUTHORING-BATCH-DUPLICATE-TARGET",
                format!("batch target `{}` is duplicated", target.field),
            ));
        }
        positions.push(position);
    }
    let Some(&(first_row, first_column)) = positions.first() else {
        return Err(batch_error(
            "E-AUTHORING-BATCH-TARGETS",
            "batch operation has no targets",
        ));
    };
    let (min_row, max_row) = positions
        .iter()
        .fold((first_row, first_row), |(min, max), &(row, _)| {
            (min.min(row), max.max(row))
        });
    let (min_column, max_column) = positions
        .iter()
        .fold((first_column, first_column), |(min, max), &(_, column)| {
            (min.min(column), max.max(column))
        });
    let height = max_row - min_row + 1;
    let width = max_column - min_column + 1;
    if positions.len() != height * width
        || (min_row..=max_row)
            .any(|row| (min_column..=max_column).any(|column| !seen.contains(&(row, column))))
        || positions.windows(2).any(|window| window[0] >= window[1])
    {
        return Err(batch_error(
            "E-AUTHORING-BATCH-SHAPE",
            "batch targets must be a contiguous row-major rectangle",
        ));
    }
    Ok((height, width))
}

fn target_position(
    snapshot: &DataFileSnapshot,
    target: &BatchTarget,
    added_record_base: usize,
) -> Result<(usize, usize)> {
    let row_position = match (target.record_index, target.added_record_index) {
        (Some(record), None) => snapshot
            .rows
            .iter()
            .position(|row| row.record_index == record)
            .ok_or_else(|| {
                batch_error(
                    "E-AUTHORING-BATCH-TARGETS",
                    format!("record[{record}] target does not exist"),
                )
            })?,
        (None, Some(draft)) => added_record_base + draft,
        _ => {
            return Err(batch_error(
                "E-AUTHORING-BATCH-TARGETS",
                "batch target must identify exactly one record kind",
            ));
        }
    };
    let column_index = snapshot
        .columns
        .iter()
        .position(|column| column.name == target.field)
        .ok_or_else(|| {
            batch_error(
                "E-AUTHORING-BATCH-FIELD",
                format!("unknown batch field `{}`", target.field),
            )
        })?;
    Ok((row_position, column_index))
}

fn current_edit_value(
    snapshot: &DataFileSnapshot,
    mutation: &AuthoringRecordMutation,
    record_index: usize,
    field: &str,
) -> AuthoringValue {
    mutation
        .edits
        .iter()
        .find(|edit| edit.record_index == record_index && edit.field == field)
        .map(|edit| edit.value.clone())
        .or_else(|| {
            snapshot
                .rows
                .iter()
                .find(|row| row.record_index == record_index)
                .and_then(|row| row.cells.iter().find(|cell| cell.field == field))
                .map(|cell| cell.value.clone())
        })
        .unwrap_or(AuthoringValue::Null)
}

fn replace_existing_edit(
    edits: &mut Vec<AuthoringEdit>,
    record_index: usize,
    field: &str,
    value: AuthoringValue,
) {
    if let Some(edit) = edits
        .iter_mut()
        .find(|edit| edit.record_index == record_index && edit.field == field)
    {
        edit.value = value;
    } else {
        edits.push(AuthoringEdit {
            record_index,
            field: field.to_owned(),
            value,
        });
    }
}

fn batch_error(code: &str, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new(code, masterdata_core::ErrorKind::Validation, message)
        .with_related_requirement("AUTHORING-BATCH-001")
}

#[cfg(test)]
mod clipboard_shape_tests {
    use super::*;
    use crate::{DataEditorAddCapability, DataEditorCell, DataEditorColumn, DataEditorRow};
    use masterdata_core::{
        FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType,
        ValidationReport,
    };

    #[test]
    fn clipboard_shape_preserves_two_dimensional_geometry() {
        let shape = NativeApplicationService::new()
            .authoring_clipboard_shape("a\tb\nc\td")
            .expect("2x2 clipboard");
        assert_eq!(shape.rows, 2);
        assert_eq!(shape.columns, 2);
    }

    #[test]
    fn clipboard_shape_rejects_ragged_tsv() {
        let error = NativeApplicationService::new()
            .authoring_clipboard_shape("a\tb\nc")
            .expect_err("ragged clipboard");
        assert_eq!(error.diagnostic().code, "E-AUTHORING-BATCH-SHAPE");
    }

    #[test]
    fn existing_key_field_is_rejected_by_batch_mutation_even_when_directly_editable() {
        let key_shape = ResolvedAuthoringField {
            name: "id".to_owned(),
            type_name: "ulong".to_owned(),
            modifier: FieldModifier::Required,
            shape: ResolvedAuthoringType::Primitive {
                primitive: PrimitiveType::ULong,
            },
        };
        let snapshot = DataFileSnapshot {
            path: "sources/data/item.yaml".to_owned(),
            table: "item".to_owned(),
            base_source: "id: 1\n".to_owned(),
            base_content_identity: "base".to_owned(),
            columns: vec![DataEditorColumn {
                name: "id".to_owned(),
                type_name: "ulong".to_owned(),
                editable: true,
                key_field: true,
                shape: Some(key_shape.clone()),
                read_only_reason: None,
            }],
            rows: vec![DataEditorRow {
                record_index: 0,
                cells: vec![DataEditorCell {
                    field: "id".to_owned(),
                    text: "1".to_owned(),
                    value: AuthoringValue::Number {
                        value: "1".to_owned(),
                    },
                    editable: true,
                    read_only_reason: None,
                }],
                tags: Vec::new(),
                tags_editable: true,
                tags_read_only_reason: None,
            }],
            tag_candidates: Vec::new(),
            tag_candidates_complete: true,
            add_row: DataEditorAddCapability {
                supported: true,
                reason: None,
            },
            validation: ValidationReport {
                valid: true,
                diagnostics: Vec::new(),
                files_scanned: 1,
                schema_documents: 1,
                data_documents: 1,
                type_documents: 0,
                tables: vec!["item".to_owned()],
                types: Vec::new(),
            },
        };
        let error = build_batch_mutation(
            &snapshot,
            &AuthoringRecordMutation::default(),
            &AuthoringBatchRequest {
                targets: vec![BatchTarget {
                    record_index: Some(0),
                    added_record_index: None,
                    field: "id".to_owned(),
                }],
                clipboard_text: "2".to_owned(),
                fill: false,
            },
        )
        .expect_err("existing key batch mutation must remain restricted");
        assert_eq!(error.diagnostic().code, "E-AUTHORING-BATCH-READ-ONLY");
    }
}
