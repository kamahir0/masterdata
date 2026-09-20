use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_yaml::Value;
use sha2::{Digest, Sha256};

use crate::document::{
    DataDocument, ProjectDocuments, SchemaDocument, SourceDocument, parse_yaml_document,
};
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::type_system::{
    FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType,
    resolve_authoring_field_shape,
};
use crate::{AuthoringSequenceItem, AuthoringValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordValueEdit {
    pub record_index: usize,
    pub field: String,
    pub value: AuthoringValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedRecordField {
    pub field: String,
    pub value: AuthoringValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedRecordDraft {
    pub fields: Vec<AddedRecordField>,
    /// Record Tags are source metadata and are rendered after declared fields.
    /// They never enter the resolved Table/domain field set.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordTagEdit {
    pub record_index: usize,
    /// The requested ordered tag entries. Order is preserved in source text,
    /// while Build Selection later treats the membership as a set.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceRecordMutation {
    pub edits: Vec<RecordValueEdit>,
    pub additions: Vec<AddedRecordDraft>,
    pub deletions: Vec<usize>,
    pub tag_edits: Vec<RecordTagEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEditPlan {
    pub path: PathBuf,
    pub base_content_identity: String,
    pub candidate_source: String,
    pub candidate_content_identity: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceEditDryRun {
    pub plan: SourceEditPlan,
    pub transformed_documents: ProjectDocuments,
}

pub fn source_content_identity(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn dry_run_source_edit(
    documents: &ProjectDocuments,
    path: &Path,
    edits: &[RecordValueEdit],
) -> Result<SourceEditDryRun> {
    dry_run_source_record_mutation(
        documents,
        path,
        &SourceRecordMutation {
            edits: edits.to_vec(),
            additions: Vec::new(),
            deletions: Vec::new(),
            tag_edits: Vec::new(),
        },
    )
}

pub fn dry_run_source_record_mutation(
    documents: &ProjectDocuments,
    path: &Path,
    mutation: &SourceRecordMutation,
) -> Result<SourceEditDryRun> {
    let loaded = documents
        .files
        .iter()
        .find(|loaded| loaded.path == path)
        .ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-FILE-NOT-FOUND",
                format!(
                    "source edit target `{}` is not in the loaded snapshot",
                    path.display()
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-001",
            )
        })?;
    let SourceDocument::Data(data) = &loaded.document else {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-NOT-DATA",
            format!(
                "source edit target `{}` is not a Data document",
                path.display()
            ),
            Some(path.to_path_buf()),
            "SOURCE-EDIT-002",
        ));
    };

    let schema = unique_schema_for_table(documents, &data.table, path)?;
    let editable = editable_fields(schema, documents);
    let deleted = deleted_record_indices(data, &mutation.deletions, path)?;
    let mut seen = BTreeSet::new();
    let mut expected = data.clone();
    let mut patches = Vec::new();
    let mut member_source = None;

    for edit in &mutation.edits {
        if !seen.insert((edit.record_index, edit.field.clone())) {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-DUPLICATE-TARGET",
                format!(
                    "source edit contains the same target more than once: record[{}].{}",
                    edit.record_index, edit.field
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-005",
            ));
        }
        let record = data.records.get(edit.record_index).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-RECORD-NOT-FOUND",
                format!(
                    "record[{}] does not exist in `{}`",
                    edit.record_index,
                    path.display()
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-001",
            )
        })?;
        let current = record.get(&edit.field).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-FIELD-NOT-FOUND",
                format!(
                    "record[{}] does not contain existing member `{}`",
                    edit.record_index, edit.field
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-002",
            )
        })?;
        let shape = editable.get(edit.field.as_str()).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-FIELD-READ-ONLY",
                format!("field `{}` is not an editable resolved field", edit.field),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-002",
            )
        })?;
        let desired = authoring_value_to_yaml(shape, &edit.value)?;
        let source_identity_changed = crate::project_typed_source_value(shape, current)
            .ok()
            .is_some_and(|source_value| {
                source_occurrence_identity_changed(&source_value, &edit.value)
            });
        let value_changed = *current != desired || source_identity_changed;
        expected.records[edit.record_index].insert(edit.field.clone(), desired.clone());

        // A pending delete owns the final candidate. Keep the edit in the
        // in-memory expectation so Undo can restore it, but do not create an
        // overlapping scalar patch for source text that will be removed.
        if deleted.contains(&edit.record_index) || !value_changed {
            continue;
        }
        let member_source = if let Some(member_source) = member_source.as_ref() {
            member_source
        } else {
            member_source = Some(RecordMemberSource::new(&loaded.source, data, path)?);
            member_source.as_ref().expect("member source initialized")
        };
        let span = member_source.locate(&loaded.source, edit.record_index, &edit.field, path)?;
        patches.extend(value_change_patches(
            &loaded.source,
            &span,
            current,
            &desired,
            &edit.value,
            shape,
            path,
        )?);
    }

    let mut seen_tag_targets = BTreeSet::new();
    for tag_edit in &mutation.tag_edits {
        if !seen_tag_targets.insert(tag_edit.record_index) {
            return Err(source_edit_error(
                "E-SOURCE-TAG-DUPLICATE-TARGET",
                format!(
                    "source tag edit contains record[{}] more than once",
                    tag_edit.record_index
                ),
                Some(path.to_path_buf()),
                "SOURCE-TAG-001",
            ));
        }
        let record = data.records.get(tag_edit.record_index).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-TAG-RECORD-NOT-FOUND",
                format!("record[{}] does not exist", tag_edit.record_index),
                Some(path.to_path_buf()),
                "SOURCE-TAG-001",
            )
        })?;
        if deleted.contains(&tag_edit.record_index) {
            return Err(source_edit_error(
                "E-SOURCE-TAG-PENDING-DELETE",
                format!(
                    "record[{}] is pending delete and cannot receive a tag edit",
                    tag_edit.record_index
                ),
                Some(path.to_path_buf()),
                "SOURCE-TAG-001",
            ));
        }
        let tag_shape = tag_field_shape();
        match record.get("$tags") {
            None => {
                if !tag_edit.tags.is_empty() {
                    expected.records[tag_edit.record_index]
                        .insert("$tags".to_owned(), tag_sequence_yaml(&tag_edit.tags));
                    patches.push(insert_record_tags_patch(
                        &loaded.source,
                        data,
                        tag_edit.record_index,
                        &tag_edit.tags,
                        path,
                    )?);
                }
            }
            Some(Value::Sequence(items)) => {
                if items.iter().any(|item| item.as_str().is_none()) {
                    return Err(source_edit_error(
                        "E-SOURCE-TAG-SHAPE",
                        "$tags contains a non-string entry and cannot be safely localized",
                        Some(path.to_path_buf()),
                        "SOURCE-TAG-002",
                    ));
                }
                let old = Value::Sequence(items.clone());
                let desired = tag_sequence_value(&tag_edit.tags, Some(items));
                let desired_yaml = tag_sequence_yaml(&tag_edit.tags);
                expected.records[tag_edit.record_index]
                    .insert("$tags".to_owned(), desired_yaml.clone());
                let member_source = if let Some(member_source) = member_source.as_ref() {
                    member_source
                } else {
                    member_source = Some(RecordMemberSource::new(&loaded.source, data, path)?);
                    member_source.as_ref().expect("member source initialized")
                };
                let span =
                    member_source.locate(&loaded.source, tag_edit.record_index, "$tags", path)?;
                patches.extend(value_change_patches(
                    &loaded.source,
                    &span,
                    &old,
                    &desired_yaml,
                    &desired,
                    &tag_shape,
                    path,
                )?);
            }
            Some(_) => {
                return Err(source_edit_error(
                    "E-SOURCE-TAG-SHAPE",
                    "$tags is not a string sequence and cannot be safely localized",
                    Some(path.to_path_buf()),
                    "SOURCE-TAG-002",
                ));
            }
        }
    }

    let additions = mutation
        .additions
        .iter()
        .enumerate()
        .map(|(index, draft)| added_record_plan(schema, documents, draft, index, path))
        .collect::<Result<Vec<_>>>()?;

    let existing_records = std::mem::take(&mut expected.records);
    expected.records = existing_records
        .into_iter()
        .enumerate()
        .filter_map(|(index, record)| (!deleted.contains(&index)).then_some(record))
        .collect();
    expected
        .records
        .extend(additions.iter().map(|addition| addition.values.clone()));

    if !deleted.is_empty() || !additions.is_empty() {
        let sequence = locate_record_sequence(&loaded.source, data, path)?;
        patches.extend(delete_record_patches(&loaded.source, &sequence, &deleted));
        if !additions.is_empty() {
            patches.extend(add_record_patches(
                &loaded.source,
                &sequence,
                &additions,
                path,
            )?);
        } else if expected.records.is_empty() {
            patches.push(empty_records_patch(&loaded.source, &sequence));
        }
    }

    let candidate_source = apply_patches(&loaded.source, &patches, path)?;
    let reparsed = parse_yaml_document(path.to_path_buf(), &candidate_source)
        .map_err(|error| with_requirement(error, "SOURCE-EDIT-005"))?;
    let SourceDocument::Data(reparsed_data) = &reparsed.document else {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-POSTCONDITION",
            "source edit candidate no longer parses as the target Data document",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-005",
        ));
    };
    if reparsed_data != &expected {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-POSTCONDITION",
            "source edit candidate does not match the expected record-value transformation",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-005",
        ));
    }
    let mut transformed_documents = documents.clone();
    let index = transformed_documents
        .files
        .iter()
        .position(|item| item.path == path)
        .expect("target file resolved");
    transformed_documents.files[index] = reparsed;
    Ok(SourceEditDryRun {
        plan: SourceEditPlan {
            path: path.to_path_buf(),
            base_content_identity: source_content_identity(&loaded.source),
            candidate_content_identity: source_content_identity(&candidate_source),
            changed: candidate_source != loaded.source,
            candidate_source,
        },
        transformed_documents,
    })
}

fn deleted_record_indices(
    data: &DataDocument,
    indices: &[usize],
    path: &Path,
) -> Result<BTreeSet<usize>> {
    let mut deleted = BTreeSet::new();
    for &index in indices {
        if !deleted.insert(index) {
            return Err(source_edit_error(
                "E-SOURCE-RECORD-DUPLICATE-DELETE",
                format!("record[{index}] is scheduled for deletion more than once"),
                Some(path.to_path_buf()),
                "SOURCE-RECORD-006",
            ));
        }
        if index >= data.records.len() {
            return Err(source_edit_error(
                "E-SOURCE-RECORD-RECORD-NOT-FOUND",
                format!("record[{index}] does not exist in `{}`", path.display()),
                Some(path.to_path_buf()),
                "SOURCE-RECORD-001",
            ));
        }
    }
    Ok(deleted)
}

#[derive(Debug, Clone)]
struct AddedRecordPlan {
    values: BTreeMap<String, Value>,
    rendered_fields: Vec<(String, String)>,
}

fn added_record_plan(
    schema: &SchemaDocument,
    documents: &ProjectDocuments,
    draft: &AddedRecordDraft,
    draft_index: usize,
    path: &Path,
) -> Result<AddedRecordPlan> {
    let shapes = schema
        .fields
        .iter()
        .filter_map(|field| {
            resolve_authoring_field_shape(documents, field)
                .map(|shape| (field.name.as_str(), shape))
        })
        .collect::<BTreeMap<_, _>>();
    let unsupported = schema
        .fields
        .iter()
        .find(|field| !shapes.contains_key(field.name.as_str()));
    if schema.fields.is_empty() || shapes.len() != schema.fields.len() {
        let detail = unsupported.map_or_else(
            || "the table schema contains duplicate field declarations".to_owned(),
            |field| {
                format!(
                    "field `{}` has no supported resolved value shape",
                    field.name
                )
            },
        );
        return Err(source_edit_error(
            "E-SOURCE-RECORD-ADD-UNSUPPORTED",
            format!("record addition is unsupported for this Table: {detail}"),
            Some(path.to_path_buf()),
            "SOURCE-RECORD-002",
        ));
    }

    let mut inputs = BTreeMap::new();
    for field in &draft.fields {
        if inputs
            .insert(field.field.clone(), field.value.clone())
            .is_some()
        {
            return Err(source_edit_error(
                "E-SOURCE-RECORD-DUPLICATE-FIELD",
                format!(
                    "added record draft[{draft_index}] contains field `{}` more than once",
                    field.field
                ),
                Some(path.to_path_buf()),
                "SOURCE-RECORD-003",
            ));
        }
        if !schema
            .fields
            .iter()
            .any(|declared| declared.name == field.field)
        {
            return Err(source_edit_error(
                "E-SOURCE-RECORD-UNKNOWN-FIELD",
                format!(
                    "added record draft[{draft_index}] contains unknown field `{}`",
                    field.field
                ),
                Some(path.to_path_buf()),
                "SOURCE-RECORD-003",
            ));
        }
    }
    if inputs.len() != schema.fields.len() {
        let missing = schema
            .fields
            .iter()
            .find(|field| !inputs.contains_key(&field.name))
            .map(|field| field.name.as_str())
            .unwrap_or("<unknown>");
        return Err(source_edit_error(
            "E-SOURCE-RECORD-MISSING-FIELD",
            format!("added record draft[{draft_index}] is missing field `{missing}`"),
            Some(path.to_path_buf()),
            "SOURCE-RECORD-003",
        ));
    }

    let mut values = BTreeMap::new();
    let mut rendered_fields =
        Vec::with_capacity(schema.fields.len() + usize::from(!draft.tags.is_empty()));
    for field in &schema.fields {
        let shape = shapes
            .get(field.name.as_str())
            .expect("all field shapes were checked above");
        let input = inputs.get(&field.name).expect("field shape checked above");
        let desired = authoring_value_to_yaml(shape, input)?;
        rendered_fields.push((field.name.clone(), render_source_value(&desired)?));
        values.insert(field.name.clone(), desired);
    }
    if !draft.tags.is_empty() {
        let tags = tag_sequence_yaml(&draft.tags);
        rendered_fields.push(("$tags".to_owned(), render_source_value(&tags)?));
        values.insert("$tags".to_owned(), tags);
    }
    Ok(AddedRecordPlan {
        values,
        rendered_fields,
    })
}

fn tag_sequence_yaml(tags: &[String]) -> Value {
    Value::Sequence(tags.iter().cloned().map(Value::String).collect::<Vec<_>>())
}

fn tag_sequence_value(tags: &[String], old: Option<&[Value]>) -> AuthoringValue {
    let old = old.unwrap_or_default();
    let old_strings = old.iter().map(Value::as_str).collect::<Vec<_>>();
    let mut used = BTreeSet::new();
    let items = tags
        .iter()
        .enumerate()
        .map(|(index, tag)| {
            let source_index = old_strings
                .iter()
                .enumerate()
                .find(|(old_index, old_tag)| {
                    !used.contains(old_index) && old_tag.is_some_and(|value| value == tag)
                })
                .map(|(old_index, _)| old_index)
                .or_else(|| (index < old.len() && used.insert(index)).then_some(index));
            if let Some(source_index) = source_index {
                used.insert(source_index);
            }
            AuthoringSequenceItem {
                source_index,
                value: AuthoringValue::String { value: tag.clone() },
            }
        })
        .collect();
    AuthoringValue::Sequence {
        items,
        source_identity: true,
    }
}

fn tag_field_shape() -> ResolvedAuthoringField {
    ResolvedAuthoringField {
        name: "$tags".to_owned(),
        type_name: "string".to_owned(),
        modifier: FieldModifier::Array,
        shape: ResolvedAuthoringType::Primitive {
            primitive: PrimitiveType::String,
        },
    }
}

fn insert_record_tags_patch(
    source: &str,
    data: &DataDocument,
    record_index: usize,
    tags: &[String],
    path: &Path,
) -> Result<SourcePatch> {
    let location = locate_record_sequence(source, data, path)?;
    let Some(sequence) = &location.sequence else {
        return Err(source_edit_error(
            "E-SOURCE-TAG-SOURCE-UNCLASSIFIABLE",
            "record tag property cannot be added because the records sequence is unavailable",
            Some(path.to_path_buf()),
            "SOURCE-TAG-002",
        ));
    };
    let lines = source_lines(source);
    let item_line = *sequence.items.get(record_index).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-TAG-RECORD-NOT-FOUND",
            format!("record[{record_index}] does not exist"),
            Some(path.to_path_buf()),
            "SOURCE-TAG-001",
        )
    })?;
    if sequence_item_parts(strip_yaml_comment(lines[item_line].text))
        .is_some_and(|parts| parts.trim_start().starts_with('{'))
    {
        return Err(source_edit_error(
            "E-SOURCE-TAG-SOURCE-UNCLASSIFIABLE",
            "flow record mappings are not safe for adding a missing `$tags` property",
            Some(path.to_path_buf()),
            "SOURCE-TAG-002",
        ));
    }
    let record_end = sequence
        .items
        .get(record_index + 1)
        .copied()
        .unwrap_or(location.region_end);
    let insert_at = lines
        .get(record_end)
        .map_or(source.len(), |line| line.start);
    let newline = newline_for(source);
    let prefix = if insert_at > 0 && !source[..insert_at].ends_with('\n') {
        newline
    } else {
        ""
    };
    let suffix = if insert_at < source.len() || source.ends_with('\n') {
        newline
    } else {
        ""
    };
    let rendered = render_source_value(&tag_sequence_yaml(tags))?;
    Ok(SourcePatch {
        start: insert_at,
        end: insert_at,
        replacement: format!(
            "{prefix}{}$tags: {rendered}{suffix}",
            " ".repeat(sequence.indent + 2)
        ),
    })
}

fn unique_schema_for_table<'a>(
    documents: &'a ProjectDocuments,
    table: &str,
    target_path: &Path,
) -> Result<&'a SchemaDocument> {
    let matches = documents
        .files
        .iter()
        .filter_map(|loaded| match &loaded.document {
            SourceDocument::Schema(schema) if schema.table == table => Some(schema),
            _ => None,
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [schema] => Ok(*schema),
        [] => Err(source_edit_error(
            "E-SOURCE-EDIT-SCHEMA-NOT-FOUND",
            format!("table `{table}` has no schema document in the loaded snapshot"),
            Some(target_path.to_path_buf()),
            "SOURCE-EDIT-006",
        )),
        _ => Err(source_edit_error(
            "E-SOURCE-EDIT-SCHEMA-AMBIGUOUS",
            format!("table `{table}` has multiple schema documents in the loaded snapshot"),
            Some(target_path.to_path_buf()),
            "SOURCE-EDIT-006",
        )),
    }
}

fn editable_fields<'a>(
    schema: &'a SchemaDocument,
    documents: &ProjectDocuments,
) -> BTreeMap<&'a str, ResolvedAuthoringField> {
    schema
        .fields
        .iter()
        .filter_map(|field| {
            resolve_authoring_field_shape(documents, field)
                .map(|shape| (field.name.as_str(), shape))
        })
        .collect()
}

fn authoring_value_to_yaml(
    shape: &ResolvedAuthoringField,
    value: &AuthoringValue,
) -> Result<Value> {
    if matches!(value, AuthoringValue::Null) {
        // Added rows and nested values use null until the user supplies a
        // value; validation reports whether that placeholder is domain-valid.
        return Ok(Value::Null);
    }
    match shape.modifier {
        FieldModifier::Array => {
            let AuthoringValue::Sequence { items, .. } = value else {
                return untyped_authoring_value(value);
            };
            items
                .iter()
                .map(|item| authoring_type_value_to_yaml(&shape.shape, &item.value))
                .collect::<Result<Vec<_>>>()
                .map(Value::Sequence)
        }
        FieldModifier::Required | FieldModifier::Nullable => {
            authoring_type_value_to_yaml(&shape.shape, value)
        }
    }
}

fn authoring_type_value_to_yaml(
    shape: &ResolvedAuthoringType,
    value: &AuthoringValue,
) -> Result<Value> {
    match shape {
        ResolvedAuthoringType::Primitive { primitive } => {
            primitive_authoring_value(*primitive, value)
        }
        ResolvedAuthoringType::ValueObject { underlying, .. } => {
            primitive_authoring_value(*underlying, value)
        }
        ResolvedAuthoringType::Enum { .. } => match value {
            AuthoringValue::String { value } => Ok(Value::String(value.clone())),
            _ => untyped_authoring_value(value),
        },
        ResolvedAuthoringType::Flags { .. } => match value {
            AuthoringValue::Sequence { items, .. } => items
                .iter()
                .map(|item| untyped_authoring_value(&item.value))
                .collect::<Result<Vec<_>>>()
                .map(Value::Sequence),
            _ => untyped_authoring_value(value),
        },
        ResolvedAuthoringType::Custom { fields, .. } => {
            let AuthoringValue::Mapping { entries } = value else {
                return untyped_authoring_value(value);
            };
            let mut result = serde_yaml::Mapping::new();
            let mut seen = BTreeSet::new();
            for entry in entries {
                if !seen.insert(entry.name.as_str()) {
                    return Err(source_edit_error(
                        "E-SOURCE-EDIT-DUPLICATE-CUSTOM-MEMBER",
                        format!(
                            "Custom Type value contains member `{}` more than once",
                            entry.name
                        ),
                        None,
                        "SOURCE-EDIT-016",
                    ));
                }
                let converted = match fields.iter().find(|field| field.name == entry.name) {
                    Some(field) => authoring_value_to_yaml(field, &entry.value)?,
                    None => untyped_authoring_value(&entry.value)?,
                };
                result.insert(Value::String(entry.name.clone()), converted);
            }
            Ok(Value::Mapping(result))
        }
    }
}

fn primitive_authoring_value(primitive: PrimitiveType, value: &AuthoringValue) -> Result<Value> {
    match value {
        AuthoringValue::Number { value } => {
            let desired = desired_scalar(primitive, value)?;
            Ok(desired.value)
        }
        AuthoringValue::Bool { value } if primitive == PrimitiveType::Bool => {
            Ok(Value::Bool(*value))
        }
        AuthoringValue::String { value } if primitive == PrimitiveType::String => {
            Ok(Value::String(value.clone()))
        }
        _ => untyped_authoring_value(value),
    }
}

fn untyped_authoring_value(value: &AuthoringValue) -> Result<Value> {
    match value {
        AuthoringValue::Null => Ok(Value::Null),
        AuthoringValue::Bool { value } => Ok(Value::Bool(*value)),
        AuthoringValue::Number { value } => Ok(desired_scalar(PrimitiveType::Double, value)?.value),
        AuthoringValue::String { value } => Ok(Value::String(value.clone())),
        AuthoringValue::Sequence { items, .. } => items
            .iter()
            .map(|item| untyped_authoring_value(&item.value))
            .collect::<Result<Vec<_>>>()
            .map(Value::Sequence),
        AuthoringValue::Mapping { entries } => {
            let mut result = serde_yaml::Mapping::new();
            let mut seen = BTreeSet::new();
            for entry in entries {
                if !seen.insert(entry.name.as_str()) {
                    return Err(source_edit_error(
                        "E-SOURCE-EDIT-DUPLICATE-CUSTOM-MEMBER",
                        format!("mapping contains member `{}` more than once", entry.name),
                        None,
                        "SOURCE-EDIT-016",
                    ));
                }
                result.insert(
                    Value::String(entry.name.clone()),
                    untyped_authoring_value(&entry.value)?,
                );
            }
            Ok(Value::Mapping(result))
        }
    }
}

fn requested_value_shape_error(path: &Path) -> MasterdataError {
    source_edit_error(
        "E-SOURCE-EDIT-VALUE-SHAPE",
        "edit request value does not match the resolved authoring shape",
        Some(path.to_path_buf()),
        "SOURCE-EDIT-016",
    )
}

fn render_source_value(value: &Value) -> Result<String> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => {
            if plain_string_round_trips(value) {
                Ok(value.clone())
            } else {
                json_string(value)
            }
        }
        Value::Sequence(items) => items
            .iter()
            .map(render_source_value)
            .collect::<Result<Vec<_>>>()
            .map(|items| format!("[{}]", items.join(", "))),
        Value::Mapping(entries) => {
            let mut rendered = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let key = key.as_str().ok_or_else(|| {
                    source_edit_error(
                        "E-SOURCE-EDIT-VALUE-UNSUPPORTED",
                        "Custom Type mapping keys must be strings",
                        None,
                        "SOURCE-EDIT-015",
                    )
                })?;
                let key = if plain_string_round_trips(key) {
                    key.to_owned()
                } else {
                    json_string(key)?
                };
                rendered.push(format!("{key}: {}", render_source_value(value)?));
            }
            Ok(format!("{{{}}}", rendered.join(", ")))
        }
        Value::Tagged(_) => Err(source_edit_error(
            "E-SOURCE-EDIT-VALUE-UNSUPPORTED",
            "tagged YAML values cannot be rendered as typed authoring values",
            None,
            "SOURCE-EDIT-015",
        )),
    }
}

fn value_change_patches(
    source: &str,
    span: &ValueSpan,
    old: &Value,
    new: &Value,
    requested: &AuthoringValue,
    shape: &ResolvedAuthoringField,
    path: &Path,
) -> Result<Vec<SourcePatch>> {
    let source_identity_changed = crate::project_typed_source_value(shape, old)
        .ok()
        .is_some_and(|source_value| source_occurrence_identity_changed(&source_value, requested));
    if old == new && !source_identity_changed {
        return Ok(Vec::new());
    }
    match (&shape.modifier, &shape.shape, old, new) {
        (FieldModifier::Array, _, Value::Sequence(old_items), Value::Sequence(new_items)) => {
            let AuthoringValue::Sequence {
                items: requested_items,
                source_identity,
            } = requested
            else {
                return Err(requested_value_shape_error(path));
            };
            return sequence_value_change_patches(
                source,
                span,
                SequenceEditRequest {
                    old_items,
                    new_items,
                    requested_items,
                    source_identity: *source_identity,
                    element_shape: &shape.shape,
                },
                path,
            );
        }
        (
            FieldModifier::Required | FieldModifier::Nullable,
            ResolvedAuthoringType::Custom { fields, .. },
            Value::Mapping(old_entries),
            Value::Mapping(new_entries),
        ) => {
            let _ = (old_entries, new_entries);
            return mapping_value_change_patches(source, span, old, new, requested, fields, path);
        }
        (
            FieldModifier::Required | FieldModifier::Nullable,
            ResolvedAuthoringType::Flags { .. },
            Value::Sequence(old_items),
            Value::Sequence(new_items),
        ) => {
            let AuthoringValue::Sequence {
                items: requested_items,
                source_identity,
            } = requested
            else {
                return Err(requested_value_shape_error(path));
            };
            return sequence_value_change_patches(
                source,
                span,
                SequenceEditRequest {
                    old_items,
                    new_items,
                    requested_items,
                    source_identity: *source_identity,
                    element_shape: &ResolvedAuthoringType::Primitive {
                        primitive: PrimitiveType::String,
                    },
                },
                path,
            );
        }
        _ => {}
    }
    if is_scalar_value(old) && is_scalar_value(new) {
        return scalar_value_patch(source, span, new, shape);
    }
    replace_value_patches(source, span, new, shape)
}

fn source_occurrence_identity_changed(source: &AuthoringValue, requested: &AuthoringValue) -> bool {
    match (source, requested) {
        (
            AuthoringValue::Sequence {
                items: source_items,
                source_identity: source_tracks_identity,
            },
            AuthoringValue::Sequence {
                items: requested_items,
                source_identity: request_tracks_identity,
            },
        ) => {
            if !request_tracks_identity {
                return false;
            }
            if !source_tracks_identity || source_items.len() != requested_items.len() {
                return true;
            }
            requested_items.iter().enumerate().any(|(index, item)| {
                item.source_index != Some(index)
                    || item.source_index.is_some_and(|source_index| {
                        source_items.get(source_index).is_some_and(|source_item| {
                            source_occurrence_identity_changed(&source_item.value, &item.value)
                        })
                    })
            })
        }
        (
            AuthoringValue::Mapping {
                entries: source_entries,
            },
            AuthoringValue::Mapping {
                entries: requested_entries,
            },
        ) => requested_entries.iter().any(|requested_entry| {
            source_entries
                .iter()
                .find(|source_entry| source_entry.name == requested_entry.name)
                .is_some_and(|source_entry| {
                    source_occurrence_identity_changed(&source_entry.value, &requested_entry.value)
                })
        }),
        _ => false,
    }
}

fn scalar_value_patch(
    source: &str,
    span: &ValueSpan,
    new: &Value,
    shape: &ResolvedAuthoringField,
) -> Result<Vec<SourcePatch>> {
    if new.is_null() {
        if let Some(block) = &span.block_header {
            return Ok(vec![
                SourcePatch {
                    start: block.child_start,
                    end: block.child_end,
                    replacement: String::new(),
                },
                SourcePatch {
                    start: block.insert_at,
                    end: block.insert_at,
                    replacement: "null".to_owned(),
                },
            ]);
        }
        return Ok(vec![SourcePatch {
            start: span.start,
            end: span.end,
            replacement: "null".to_owned(),
        }]);
    }
    let primitive = scalar_primitive(shape).unwrap_or(PrimitiveType::String);
    let input = scalar_text(new);
    let desired = desired_scalar(primitive, &input)?;
    let replacement = if let Some(literal) = &span.literal {
        if matches!(new, Value::String(_)) {
            render_replacement(source, span, PrimitiveType::String, &input, &desired)?
        } else {
            format!(
                "{}{}",
                render_source_value(new)?,
                &source[literal.header_end..literal.body_start]
            )
        }
    } else if let Some(block) = &span.block_header {
        return Ok(vec![
            SourcePatch {
                start: block.child_start,
                end: block.child_end,
                replacement: String::new(),
            },
            SourcePatch {
                start: block.insert_at,
                end: block.insert_at,
                replacement: render_source_value(new)?,
            },
        ]);
    } else {
        render_replacement(source, span, primitive, &input, &desired)?
    };
    Ok(vec![SourcePatch {
        start: span.start,
        end: span.end,
        replacement,
    }])
}

fn replace_value_patches(
    source: &str,
    span: &ValueSpan,
    new: &Value,
    shape: &ResolvedAuthoringField,
) -> Result<Vec<SourcePatch>> {
    if is_scalar_value(new) {
        return scalar_value_patch(source, span, new, shape);
    }
    let rendered = render_source_value(new)?;
    if let Some(block) = &span.block_header {
        return Ok(vec![
            SourcePatch {
                start: block.child_start,
                end: block.child_end,
                replacement: String::new(),
            },
            SourcePatch {
                start: block.insert_at,
                end: block.insert_at,
                replacement: rendered,
            },
        ]);
    }
    if let Some(literal) = &span.literal {
        return Ok(vec![SourcePatch {
            start: span.start,
            end: span.end,
            replacement: format!(
                "{rendered}{}",
                &source[literal.header_end..literal.body_start]
            ),
        }]);
    }
    Ok(vec![SourcePatch {
        start: span.start,
        end: span.end,
        replacement: rendered,
    }])
}

#[derive(Clone, Copy)]
struct SequenceEditRequest<'a> {
    old_items: &'a [Value],
    new_items: &'a [Value],
    requested_items: &'a [AuthoringSequenceItem],
    source_identity: bool,
    element_shape: &'a ResolvedAuthoringType,
}

fn sequence_value_change_patches(
    source: &str,
    span: &ValueSpan,
    request: SequenceEditRequest<'_>,
    path: &Path,
) -> Result<Vec<SourcePatch>> {
    let SequenceEditRequest {
        old_items,
        new_items,
        requested_items,
        source_identity: requested_source_identity,
        element_shape,
    } = request;
    if requested_items.len() != new_items.len() {
        return Err(requested_value_shape_error(path));
    }
    let (layout, source_items) = sequence_source_layout(source, span, path)?;
    if source_items.len() != old_items.len() {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "sequence source item count does not match the parsed value",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        ));
    }
    let has_source_identity = requested_source_identity;
    let mut seen_source_indices = BTreeSet::new();
    for source_index in requested_items.iter().filter_map(|item| item.source_index) {
        if source_index >= old_items.len() || !seen_source_indices.insert(source_index) {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "sequence source item identity is stale or duplicated",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            ));
        }
    }
    let source_by_new = has_source_identity.then(|| {
        requested_items
            .iter()
            .map(|item| item.source_index)
            .collect::<Vec<_>>()
    });
    let structure_changed = if let Some(source_by_new) = &source_by_new {
        old_items.len() != new_items.len()
            || source_by_new
                .iter()
                .enumerate()
                .any(|(index, source_index)| *source_index != Some(index))
    } else {
        let permutation = old_items.len() == new_items.len()
            && sequence_match_indices(old_items, new_items)
                .iter()
                .all(Option::is_some);
        old_items.len() != new_items.len() || (permutation && old_items != new_items)
    };
    if structure_changed {
        return sequence_structure_patch(
            source,
            layout,
            &source_items,
            request,
            source_by_new,
            path,
        );
    }

    let field_shape = ResolvedAuthoringField {
        name: String::new(),
        type_name: String::new(),
        modifier: FieldModifier::Required,
        shape: element_shape.clone(),
    };
    let mut patches = Vec::new();
    for (index, (old, new)) in old_items.iter().zip(new_items).enumerate() {
        patches.extend(value_change_patches(
            source,
            &source_items[index].value,
            old,
            new,
            &requested_items[index].value,
            &field_shape,
            path,
        )?);
    }
    Ok(patches)
}

fn sequence_match_indices(old_items: &[Value], new_items: &[Value]) -> Vec<Option<usize>> {
    let mut used = vec![false; old_items.len()];
    new_items
        .iter()
        .map(|new| {
            old_items
                .iter()
                .enumerate()
                .find(|(index, old)| !used[*index] && *old == new)
                .map(|(index, _)| {
                    used[index] = true;
                    index
                })
        })
        .collect()
}

fn sequence_structure_patch(
    source: &str,
    layout: SequenceSourceLayout,
    source_items: &[SequenceItemSource],
    request: SequenceEditRequest<'_>,
    source_by_new: Option<Vec<Option<usize>>>,
    path: &Path,
) -> Result<Vec<SourcePatch>> {
    let SequenceEditRequest {
        old_items,
        new_items,
        requested_items,
        element_shape,
        ..
    } = request;
    if source_items.len() != old_items.len() {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "sequence source item count does not match the parsed value",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        ));
    }

    let has_source_identity = source_by_new.is_some();
    let simple_append = if let Some(source_by_new) = &source_by_new {
        new_items.len() >= old_items.len()
            && old_items
                .iter()
                .enumerate()
                .all(|(index, old)| source_by_new[index] == Some(index) && *old == new_items[index])
            && source_by_new[old_items.len()..].iter().all(Option::is_none)
    } else {
        new_items.len() >= old_items.len()
            && old_items.iter().zip(new_items).all(|(old, new)| old == new)
    };
    let mut source_by_new = source_by_new.unwrap_or_else(|| {
        if simple_append {
            (0..new_items.len())
                .map(|index| (index < old_items.len()).then_some(index))
                .collect::<Vec<_>>()
        } else {
            sequence_match_indices(old_items, new_items)
        }
    });
    let mut used_old = vec![false; old_items.len()];
    for index in source_by_new.iter().flatten() {
        used_old[*index] = true;
    }
    let unmatched_old = used_old
        .iter()
        .enumerate()
        .filter_map(|(index, used)| (!used).then_some(index))
        .collect::<Vec<_>>();
    let unmatched_new = source_by_new
        .iter()
        .enumerate()
        .filter_map(|(index, matched)| matched.is_none().then_some(index))
        .collect::<Vec<_>>();
    if !has_source_identity {
        for (old_index, new_index) in unmatched_old.iter().zip(unmatched_new) {
            source_by_new[new_index] = Some(*old_index);
        }
    }

    let old_positions = {
        let mut positions = vec![None; old_items.len()];
        for (new_index, old_index) in source_by_new.iter().enumerate() {
            if let Some(old_index) = old_index {
                positions[*old_index] = Some(new_index);
            }
        }
        positions
    };
    let existing_items_stay_in_place = old_positions
        .iter()
        .enumerate()
        .all(|(index, position)| *position == Some(index));

    // WHY: A removed/reordered block item can make separator comments or blank lines
    // ambiguous between neighboring values; guessing would move or erase unrelated bytes.
    // SOURCE-EDIT-006 requires failing closed when the source location is not unique.
    // Regression: block_array_structure_rejects_ambiguous_inter_item_source.
    if !existing_items_stay_in_place
        && let SequenceSourceLayout::Block { separators, .. } = &layout
        && separators.iter().any(|separator| !separator.is_empty())
    {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "block sequence separators contain comments or blank lines that cannot be preserved safely during this structural edit",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        ));
    }

    // Equal semantic items may carry different quotes or inline comments. When
    // an older caller omits occurrence identities, a structural edit cannot tell
    // which duplicate was moved or removed. Stable UI identities disambiguate it.
    let duplicate_source_is_ambiguous = (0..old_items.len()).any(|right| {
        (0..right).any(|left| {
            old_items[left] == old_items[right] && source_items[left].raw != source_items[right].raw
        })
    });
    if duplicate_source_is_ambiguous && !simple_append && !has_source_identity {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "duplicate sequence values have distinct source text and cannot be identified safely",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        ));
    }

    let item_shape = ResolvedAuthoringField {
        name: String::new(),
        type_name: String::new(),
        modifier: FieldModifier::Required,
        shape: element_shape.clone(),
    };
    let chunks = source_by_new
        .iter()
        .copied()
        .zip(new_items)
        .enumerate()
        .map(|(new_index, (matched, value))| match matched {
            Some(index) if old_items[index] == *value => Ok(source_items[index].raw.clone()),
            Some(index) => {
                let item = &source_items[index];
                let patches = value_change_patches(
                    source,
                    &item.value,
                    &old_items[index],
                    value,
                    &requested_items[new_index].value,
                    &item_shape,
                    path,
                )?;
                apply_local_patches(&item.raw, item.start, &patches, path)
            }
            None => render_source_value(value),
        })
        .collect::<Result<Vec<_>>>()?;
    let (start, end, replacement) = match layout {
        SequenceSourceLayout::Flow {
            start,
            end,
            prefix,
            separators,
            suffix,
        } => {
            let mut replacement = prefix;
            if chunks.is_empty() {
                let mut suffix = suffix;
                if let Some(comma) = suffix.find(',') {
                    suffix.remove(comma);
                }
                replacement.push_str(&suffix);
            } else {
                for (index, chunk) in chunks.iter().enumerate() {
                    if index > 0 {
                        replacement.push_str(
                            separators
                                .get(index - 1)
                                .map(String::as_str)
                                .unwrap_or(", "),
                        );
                    }
                    replacement.push_str(chunk);
                }
                replacement.push_str(&suffix);
            }
            (start, end, replacement)
        }
        SequenceSourceLayout::Block {
            start,
            end,
            indent,
            empty_replacement,
            prefix,
            separators,
            suffix,
        } => {
            let mut replacement = prefix;
            if chunks.is_empty() {
                replacement.push_str(&empty_replacement);
            } else {
                for (index, chunk) in chunks.iter().enumerate() {
                    if index > 0 {
                        replacement
                            .push_str(separators.get(index - 1).map(String::as_str).unwrap_or(""));
                    }
                    if source_by_new[index].is_none() {
                        replacement.push_str(&" ".repeat(indent));
                        replacement.push_str("- ");
                        replacement.push_str(chunk);
                        replacement.push_str(newline_for(source));
                    } else {
                        replacement.push_str(chunk);
                    }
                }
                replacement.push_str(&suffix);
            }
            (start, end, replacement)
        }
    };
    if start > end {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-PATCH-INVALID",
            "sequence patch range is invalid",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        ));
    }
    Ok(vec![SourcePatch {
        start,
        end,
        replacement,
    }])
}

fn apply_local_patches(
    item_source: &str,
    item_start: usize,
    patches: &[SourcePatch],
    path: &Path,
) -> Result<String> {
    let mut local = Vec::with_capacity(patches.len());
    for patch in patches {
        if patch.start < item_start || patch.end < item_start {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-PATCH-INVALID",
                "nested sequence patch escapes its source item",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            ));
        }
        local.push(SourcePatch {
            start: patch.start - item_start,
            end: patch.end - item_start,
            replacement: patch.replacement.clone(),
        });
    }
    apply_patches(item_source, &local, path)
}

fn mapping_value_change_patches(
    source: &str,
    span: &ValueSpan,
    old: &Value,
    new: &Value,
    requested: &AuthoringValue,
    fields: &[ResolvedAuthoringField],
    path: &Path,
) -> Result<Vec<SourcePatch>> {
    let old_mapping = old.as_mapping().ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-EDIT-VALUE-SHAPE",
            "existing Custom Type value is not a mapping",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-015",
        )
    })?;
    let new_mapping = new.as_mapping().ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-EDIT-VALUE-SHAPE",
            "edited Custom Type value is not a mapping",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-016",
        )
    })?;
    let AuthoringValue::Mapping {
        entries: requested_entries,
    } = requested
    else {
        return Err(requested_value_shape_error(path));
    };
    let source_members = mapping_member_spans(source, span, path)?;
    let mut patches = Vec::new();
    let mut additions = Vec::<(String, Value)>::new();

    for field in fields {
        let key = Value::String(field.name.clone());
        let old_value = old_mapping.get(&key);
        let new_value = new_mapping.get(&key);
        match (old_value, new_value) {
            (Some(old_value), Some(new_value)) => {
                let requested_value = requested_entries
                    .iter()
                    .find(|entry| entry.name == field.name)
                    .map(|entry| &entry.value)
                    .ok_or_else(|| requested_value_shape_error(path))?;
                let source_identity_changed = crate::project_typed_source_value(field, old_value)
                    .ok()
                    .is_some_and(|source_value| {
                        source_occurrence_identity_changed(&source_value, requested_value)
                    });
                if old_value == new_value && !source_identity_changed {
                    continue;
                }
                let matches = source_members
                    .iter()
                    .filter(|(name, _)| name == &field.name)
                    .map(|(_, span)| span)
                    .collect::<Vec<_>>();
                let [source_span] = matches.as_slice() else {
                    return Err(source_edit_error(
                        "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                        format!(
                            "Custom Type member `{}` could not be located exactly once",
                            field.name
                        ),
                        Some(path.to_path_buf()),
                        "SOURCE-EDIT-006",
                    ));
                };
                patches.extend(value_change_patches(
                    source,
                    source_span,
                    old_value,
                    new_value,
                    requested_value,
                    field,
                    path,
                )?);
            }
            (Some(_), None) => {
                return Err(source_edit_error(
                    "E-SOURCE-EDIT-CUSTOM-MEMBER-DROPPED",
                    format!("Custom Type edit omitted declared member `{}`", field.name),
                    Some(path.to_path_buf()),
                    "SOURCE-EDIT-016",
                ));
            }
            (None, Some(new_value)) => additions.push((field.name.clone(), new_value.clone())),
            (None, None) => {}
        }
    }

    for (key, old_value) in old_mapping {
        let Some(name) = key.as_str() else {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-VALUE-SHAPE",
                "Custom Type mapping contains a non-string key",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-015",
            ));
        };
        if fields.iter().any(|field| field.name == name) {
            continue;
        }
        let new_value = new_mapping.get(key);
        if new_value != Some(old_value) {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-UNKNOWN-MEMBER-CHANGED",
                format!(
                    "unknown Custom Type member `{name}` cannot be changed through this editor"
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-015",
            ));
        }
    }
    for key in new_mapping.keys() {
        if !old_mapping.contains_key(key)
            && key
                .as_str()
                .is_none_or(|name| !fields.iter().any(|field| field.name == name))
        {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-UNKNOWN-MEMBER-ADDED",
                "only declared Custom Type members can be authored",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-016",
            ));
        }
    }
    if !additions.is_empty() {
        patches.extend(insert_mapping_members(source, span, &additions, path)?);
    }
    Ok(patches)
}

fn insert_mapping_members(
    source: &str,
    span: &ValueSpan,
    additions: &[(String, Value)],
    path: &Path,
) -> Result<Vec<SourcePatch>> {
    if additions.is_empty() {
        return Ok(Vec::new());
    }
    let trimmed = span.raw.trim_start();
    if trimmed.starts_with('{') {
        // WHY: Inserting at the closing brace retains exact sibling tokens and
        // spacing; rebuilding the flow map would normalize unrelated source bytes.
        // EVIDENCE: SOURCE-EDIT-005; Regression: flow_custom_member_insertion_preserves_existing_mapping_bytes.
        let flow_start = span.start + span.raw.len() - trimmed.len();
        let flow_end = flow_collection_end(source, flow_start).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "Custom Type flow mapping end could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        if flow_has_comment(source, flow_start, flow_end) {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow mapping comments prevent a safe member insertion",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            ));
        }
        let mut chunks = Vec::with_capacity(additions.len());
        for (name, value) in additions {
            let key = if plain_string_round_trips(name) {
                name.clone()
            } else {
                json_string(name)?
            };
            chunks.push(format!("{key}: {}", render_source_value(value)?));
        }
        let content_start = flow_start + 1;
        let content_end = flow_end - 1;
        let content = &source[content_start..content_end];
        let is_empty = content.trim().is_empty();
        let has_trailing_comma = content.trim_end().ends_with(',');
        let insertion = if is_empty || has_trailing_comma {
            chunks.join(", ")
        } else {
            format!(", {}", chunks.join(", "))
        };
        let insert_at = if is_empty {
            content_start + content.len() - content.trim_start().len()
        } else {
            content_end
        };
        return Ok(vec![SourcePatch {
            start: insert_at,
            end: insert_at,
            replacement: insertion,
        }]);
    }

    let lines = source_lines(source);
    let first_line = line_index_for_offset(&lines, span.start).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "Custom Type block mapping start could not be located safely",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        )
    })?;
    let mapping_indent = if is_sequence_item(lines[first_line].text)
        && mapping_span(lines[first_line].text).is_some()
    {
        yaml_indent(lines[first_line].text) + 2
    } else {
        yaml_indent(lines[first_line].text)
    };
    let mut replacement = String::new();
    if span.end > 0 && !source[..span.end].ends_with('\n') {
        replacement.push_str(newline_for(source));
    }
    for (name, value) in additions {
        let key = if plain_string_round_trips(name) {
            name.clone()
        } else {
            json_string(name)?
        };
        replacement.push_str(&" ".repeat(mapping_indent));
        replacement.push_str(&key);
        replacement.push_str(": ");
        replacement.push_str(&render_source_value(value)?);
        replacement.push_str(newline_for(source));
    }
    Ok(vec![SourcePatch {
        start: span.end,
        end: span.end,
        replacement,
    }])
}

fn is_scalar_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        _ => String::new(),
    }
}

fn scalar_primitive(shape: &ResolvedAuthoringField) -> Option<PrimitiveType> {
    match &shape.shape {
        ResolvedAuthoringType::Primitive { primitive } => Some(*primitive),
        ResolvedAuthoringType::ValueObject { underlying, .. } => Some(*underlying),
        ResolvedAuthoringType::Enum { .. } => Some(PrimitiveType::String),
        ResolvedAuthoringType::Flags { .. } | ResolvedAuthoringType::Custom { .. } => None,
    }
}

struct DesiredScalar {
    value: Value,
    source_text: Option<String>,
}

fn desired_scalar(primitive: PrimitiveType, input: &str) -> Result<DesiredScalar> {
    if primitive == PrimitiveType::String {
        return Ok(DesiredScalar {
            value: Value::String(input.to_owned()),
            source_text: None,
        });
    }
    if !input.contains(['\n', '\r']) {
        let probe = format!("kind: data\ntable: edit-probe\nrecords:\n  - value: {input}\n");
        if let Ok(parsed) = parse_yaml_document(PathBuf::from("<source-edit-input>"), &probe)
            && let SourceDocument::Data(data) = parsed.document
            && let Some(record) = data.records.first()
            && let Some(value) = record.get("value")
            && matches!(
                value,
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
            )
        {
            return Ok(DesiredScalar {
                value: value.clone(),
                source_text: Some(input.to_owned()),
            });
        }
    }
    Ok(DesiredScalar {
        value: Value::String(input.to_owned()),
        source_text: None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourcePatch {
    start: usize,
    end: usize,
    replacement: String,
}
#[derive(Debug, Clone)]
struct ValueSpan {
    start: usize,
    end: usize,
    raw: String,
    literal: Option<LiteralSpan>,
    block_header: Option<BlockHeader>,
}
#[derive(Debug, Clone)]
struct BlockHeader {
    insert_at: usize,
    child_start: usize,
    child_end: usize,
}
#[derive(Debug, Clone)]
struct LiteralSpan {
    header_start: usize,
    header_end: usize,
    body_start: usize,
    body_end: usize,
    content_indent: usize,
    newline: String,
}
#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    start: usize,
    next_start: usize,
    text: &'a str,
}
#[derive(Debug, Clone)]
struct MappingSpan {
    key: String,
    value_start: usize,
    value_end: usize,
    raw_value: String,
}
#[derive(Debug, Clone)]
struct SequenceRegion {
    items: Vec<usize>,
    indent: usize,
}

// WHY: Reuse the source line/literal/record boundaries for every scalar or tag
// cell in one preview; reparsing the same YAML for each pasted cell made the
// fixed Desktop authoring scenario scale quadratically with the edit count.
// EVIDENCE: docs/evidence/desktop-v1-performance.md
#[derive(Debug, Clone)]
struct RecordMemberSource<'source> {
    lines: Vec<SourceLine<'source>>,
    literal_content: Vec<bool>,
    region_end: usize,
    sequence: SequenceRegion,
}

#[derive(Debug, Clone)]
struct RecordSequenceLocation {
    records_line: usize,
    region_end: usize,
    mapping: MappingSpan,
    sequence: Option<SequenceRegion>,
}

fn locate_record_sequence(
    source: &str,
    data: &DataDocument,
    path: &Path,
) -> Result<RecordSequenceLocation> {
    let lines = source_lines(source);
    let records_line = find_top_level_key(&lines, "records").ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-RECORD-SOURCE-UNCLASSIFIABLE",
            "data `records` source shape could not be located safely",
            Some(path.to_path_buf()),
            "SOURCE-RECORD-010",
        )
    })?;
    let mapping = mapping_span(lines[records_line].text).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-RECORD-SOURCE-UNCLASSIFIABLE",
            "data `records` source mapping could not be located safely",
            Some(path.to_path_buf()),
            "SOURCE-RECORD-010",
        )
    })?;
    let region_end = block_region_end(&lines, records_line);
    let raw_value = mapping.raw_value.trim();
    if raw_value == "[]" {
        if !data.records.is_empty() {
            return Err(source_edit_error(
                "E-SOURCE-RECORD-SOURCE-UNCLASSIFIABLE",
                "data `records` source declares an empty sequence but parsed records exist",
                Some(path.to_path_buf()),
                "SOURCE-RECORD-010",
            ));
        }
        return Ok(RecordSequenceLocation {
            records_line,
            region_end,
            mapping,
            sequence: None,
        });
    }

    let sequence = find_block_sequence(&lines, records_line, region_end).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-RECORD-SOURCE-UNCLASSIFIABLE",
            "data `records` block sequence could not be located safely",
            Some(path.to_path_buf()),
            "SOURCE-RECORD-010",
        )
    })?;
    if sequence.items.len() != data.records.len() {
        return Err(source_edit_error(
            "E-SOURCE-RECORD-SOURCE-UNCLASSIFIABLE",
            format!(
                "data `records` source has {} sequence item(s), but semantic parsing found {}",
                sequence.items.len(),
                data.records.len()
            ),
            Some(path.to_path_buf()),
            "SOURCE-RECORD-010",
        ));
    }
    Ok(RecordSequenceLocation {
        records_line,
        region_end,
        mapping,
        sequence: Some(sequence),
    })
}

fn delete_record_patches(
    source: &str,
    location: &RecordSequenceLocation,
    deleted: &BTreeSet<usize>,
) -> Vec<SourcePatch> {
    let Some(sequence) = &location.sequence else {
        return Vec::new();
    };
    let lines = source_lines(source);
    let literal_content = literal_block_scalar_content_lines(&lines);
    let mut patches = Vec::new();
    for &record_index in deleted {
        let Some(&item_line) = sequence.items.get(record_index) else {
            continue;
        };
        let end_line = sequence
            .items
            .get(record_index + 1)
            .copied()
            .unwrap_or(location.region_end);
        for (line_index, line) in lines.iter().enumerate().take(end_line).skip(item_line) {
            // Preserve separator comments and blank lines outside the target
            // mapping. Literal block content is structural source owned by
            // the deleted member and therefore must leave with its record.
            if line_index == item_line
                || literal_content.get(line_index).copied().unwrap_or(false)
                || !is_ignorable_line(line.text)
            {
                patches.push(SourcePatch {
                    start: line.start,
                    end: line.next_start,
                    replacement: String::new(),
                });
            }
        }
    }
    patches
}

fn add_record_patches(
    source: &str,
    location: &RecordSequenceLocation,
    additions: &[AddedRecordPlan],
    path: &Path,
) -> Result<Vec<SourcePatch>> {
    let lines = source_lines(source);
    let (insert_at, indent, clear_value) = match &location.sequence {
        Some(sequence) => {
            let insert_at = lines
                .get(location.region_end)
                .map_or(source.len(), |line| line.start);
            (insert_at, sequence.indent, None)
        }
        None => {
            if location.mapping.raw_value.trim() != "[]" {
                return Err(source_edit_error(
                    "E-SOURCE-RECORD-SOURCE-UNCLASSIFIABLE",
                    "empty `records` sequence is not represented by a supported source shape",
                    Some(path.to_path_buf()),
                    "SOURCE-RECORD-010",
                ));
            }
            let line = lines[location.records_line];
            let value_start = line.start + location.mapping.value_start;
            let mut clear_start = value_start;
            while clear_start > line.start
                && source
                    .as_bytes()
                    .get(clear_start - 1)
                    .is_some_and(|byte| matches!(*byte, b' ' | b'\t'))
            {
                clear_start -= 1;
            }
            (
                line.next_start,
                yaml_indent(line.text) + 2,
                Some((clear_start, line.start + location.mapping.value_end)),
            )
        }
    };
    let newline = newline_for(source);
    let body = render_added_records(additions, indent, newline);
    let prefix = if insert_at > 0 && !source[..insert_at].ends_with('\n') {
        newline
    } else {
        ""
    };
    let suffix = if insert_at < source.len() || source.ends_with('\n') {
        newline
    } else {
        ""
    };
    let mut patches = Vec::with_capacity(2);
    if let Some((start, end)) = clear_value {
        patches.push(SourcePatch {
            start,
            end,
            replacement: String::new(),
        });
    }
    patches.push(SourcePatch {
        start: insert_at,
        end: insert_at,
        replacement: format!("{prefix}{body}{suffix}"),
    });
    Ok(patches)
}

fn empty_records_patch(source: &str, location: &RecordSequenceLocation) -> SourcePatch {
    let line = source_lines(source)[location.records_line];
    let start = line.start + location.mapping.value_start;
    let replacement = if source.as_bytes().get(start) == Some(&b'#') {
        "[] ".to_owned()
    } else {
        " []".to_owned()
    };
    SourcePatch {
        start,
        end: line.start + location.mapping.value_end,
        replacement,
    }
}

fn render_added_records(additions: &[AddedRecordPlan], indent: usize, newline: &str) -> String {
    let mut lines = Vec::new();
    for addition in additions {
        for (field_index, (field, value)) in addition.rendered_fields.iter().enumerate() {
            let field_indent = if field_index == 0 { indent } else { indent + 2 };
            let item_prefix = if field_index == 0 { "- " } else { "" };
            lines.push(format!(
                "{}{item_prefix}{field}: {value}",
                " ".repeat(field_indent)
            ));
        }
    }
    lines.join(newline)
}

impl<'source> RecordMemberSource<'source> {
    fn new(source: &'source str, data: &DataDocument, path: &Path) -> Result<Self> {
        let lines = source_lines(source);
        let literal_content = literal_block_scalar_content_lines(&lines);
        let records_line = find_top_level_key(&lines, "records").ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "data `records` source shape could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        let region_end = block_region_end(&lines, records_line);
        let sequence = find_block_sequence(&lines, records_line, region_end).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "data `records` block sequence could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        if sequence.items.len() != data.records.len() {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                format!(
                    "data `records` source has {} sequence item(s), but semantic parsing found {}",
                    sequence.items.len(),
                    data.records.len()
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            ));
        }
        Ok(Self {
            lines,
            literal_content,
            region_end,
            sequence,
        })
    }

    fn locate(
        &self,
        source: &str,
        record_index: usize,
        field: &str,
        path: &Path,
    ) -> Result<ValueSpan> {
        let item_line = *self.sequence.items.get(record_index).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-RECORD-NOT-FOUND",
                format!("record[{record_index}] does not exist"),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-001",
            )
        })?;
        let record_end = self
            .sequence
            .items
            .get(record_index + 1)
            .copied()
            .unwrap_or(self.region_end);
        let map_indent = self.sequence.indent + 2;
        let mut found = Vec::new();
        for (line_index, line) in self
            .lines
            .iter()
            .enumerate()
            .take(record_end)
            .skip(item_line)
        {
            if self
                .literal_content
                .get(line_index)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            let direct_entry = line_index == item_line || yaml_indent(line.text) == map_indent;
            if !direct_entry {
                continue;
            }
            if line_index == item_line
                && let Some(parts) = sequence_item_parts(strip_yaml_comment(line.text))
                && parts.trim_start().starts_with('{')
            {
                let flow_start = line.start + line.text.find('{').unwrap_or(0);
                if let Some(flow_end) = flow_collection_end(source, flow_start) {
                    for (key, span) in flow_mapping_entries(source, flow_start, flow_end)? {
                        if key == field {
                            found.push((span, None));
                        }
                    }
                }
                continue;
            }
            if let Some(mapping) = mapping_span(line.text)
                && mapping.key == field
            {
                let span = mapping_value_span(
                    source,
                    &self.lines,
                    line_index,
                    &mapping,
                    record_end,
                    &self.literal_content,
                    path,
                )?;
                found.push((span, Some((line_index, mapping))));
            }
        }
        let [(span, _)] = found.as_slice() else {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                format!(
                    "record[{record_index}] member `{field}` could not be located exactly once in source"
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            ));
        };
        Ok(span.clone())
    }
}

fn mapping_value_span(
    source: &str,
    lines: &[SourceLine<'_>],
    line_index: usize,
    mapping: &MappingSpan,
    limit_line: usize,
    literal_content: &[bool],
    path: &Path,
) -> Result<ValueSpan> {
    let line = lines[line_index];
    let mapping_indent = yaml_indent(line.text) + usize::from(is_sequence_item(line.text)) * 2;
    let start = line.start + mapping.value_start;
    let raw = mapping.raw_value.trim_end();
    if raw == "|" || raw == ">" {
        let literal = literal_span(source, lines, line_index, limit_line, literal_content)?;
        return Ok(ValueSpan {
            start: literal.header_start,
            end: literal.body_end,
            raw: mapping.raw_value.clone(),
            literal: Some(literal),
            block_header: None,
        });
    }
    if raw.starts_with(['[', '{']) {
        let flow_end = flow_collection_end(source, start).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow collection end could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        return Ok(ValueSpan {
            start,
            end: flow_end,
            raw: source[start..flow_end].to_owned(),
            literal: None,
            block_header: None,
        });
    }
    if !raw.is_empty() {
        return Ok(ValueSpan {
            start,
            end: line.start + mapping.value_end,
            raw: mapping.raw_value.clone(),
            literal: None,
            block_header: None,
        });
    }

    if let Some((child_start, child_end)) =
        block_child_region(source, lines, line_index, mapping_indent, limit_line)
    {
        return Ok(ValueSpan {
            start: child_start,
            end: child_end,
            raw: source[child_start..child_end].to_owned(),
            literal: None,
            block_header: Some(BlockHeader {
                insert_at: start,
                child_start,
                child_end,
            }),
        });
    }
    Ok(ValueSpan {
        start,
        end: start,
        raw: String::new(),
        literal: None,
        block_header: None,
    })
}

fn block_child_region(
    source: &str,
    lines: &[SourceLine<'_>],
    header_line: usize,
    parent_indent: usize,
    limit_line: usize,
) -> Option<(usize, usize)> {
    let first =
        (header_line + 1..limit_line).find(|index| !is_ignorable_line(lines[*index].text))?;
    if yaml_indent(lines[first].text) <= parent_indent {
        return None;
    }
    let boundary = (first + 1..limit_line)
        .find(|index| {
            !is_ignorable_line(lines[*index].text)
                && yaml_indent(lines[*index].text) <= parent_indent
        })
        .unwrap_or(limit_line);
    let mut last = boundary;
    while last > first && is_ignorable_line(lines[last - 1].text) {
        last -= 1;
    }
    let end = if last > first {
        lines[last - 1].next_start
    } else {
        lines[first].start
    };
    Some((lines[first].start, end.min(source.len())))
}

fn flow_collection_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let first = *bytes.get(start)?;
    if !matches!(first, b'[' | b'{') {
        return None;
    }
    let mut stack = vec![if first == b'[' { b']' } else { b'}' }];
    let mut quote = None;
    let mut index = start + 1;
    while index < bytes.len() {
        match quote {
            Some(b'\'') => {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else if bytes[index] == b'\'' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            None if bytes[index] == b'#'
                && (index == 0 || bytes[index - 1].is_ascii_whitespace()) =>
            {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            None => match bytes[index] {
                b'\'' | b'"' => {
                    quote = Some(bytes[index]);
                    index += 1;
                }
                b'[' => {
                    stack.push(b']');
                    index += 1;
                }
                b'{' => {
                    stack.push(b'}');
                    index += 1;
                }
                closing @ (b']' | b'}') => {
                    if stack.pop() != Some(closing) {
                        return None;
                    }
                    index += 1;
                    if stack.is_empty() {
                        return Some(index);
                    }
                }
                _ => index += 1,
            },
            _ => unreachable!(),
        }
    }
    None
}

fn flow_segments(source: &str, start: usize, end: usize, delimiter: u8) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut segments = Vec::new();
    let mut segment_start = start;
    let mut stack = Vec::new();
    let mut quote = None;
    let mut index = start;
    while index < end {
        match quote {
            Some(b'\'') => {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else if bytes[index] == b'\'' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            None if bytes[index] == b'#'
                && (index == 0 || bytes[index - 1].is_ascii_whitespace()) =>
            {
                while index < end && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            None => match bytes[index] {
                b'\'' | b'"' => {
                    quote = Some(bytes[index]);
                    index += 1;
                }
                b'[' => {
                    stack.push(b']');
                    index += 1;
                }
                b'{' => {
                    stack.push(b'}');
                    index += 1;
                }
                b']' | b'}' if stack.last() == Some(&bytes[index]) => {
                    stack.pop();
                    index += 1;
                }
                current if current == delimiter && stack.is_empty() => {
                    segments.push((segment_start, index));
                    segment_start = index + 1;
                    index += 1;
                }
                _ => index += 1,
            },
            _ => unreachable!(),
        }
    }
    segments.push((segment_start, end));
    segments
}

fn flow_top_level_colon(source: &str) -> Option<usize> {
    flow_segments(source, 0, source.len(), b':')
        .into_iter()
        .next()
        .and_then(|(_, end)| (end < source.len()).then_some(end))
}

fn flow_value_span(source: &str, start: usize, end: usize) -> Option<ValueSpan> {
    let raw = strip_yaml_comment(&source[start..end]);
    let leading = raw.len() - raw.trim_start().len();
    let value = raw.trim();
    if value.is_empty() {
        let at = start + leading;
        return Some(ValueSpan {
            start: at,
            end: at,
            raw: String::new(),
            literal: None,
            block_header: None,
        });
    }
    let value_start = start + leading;
    let value_end = value_start + value.len();
    let first = source.as_bytes().get(value_start).copied()?;
    let value_end = if matches!(first, b'[' | b'{') {
        flow_collection_end(source, value_start)?
    } else {
        value_end
    };
    Some(ValueSpan {
        start: value_start,
        end: value_end,
        raw: source[value_start..value_end].to_owned(),
        literal: None,
        block_header: None,
    })
}

fn flow_sequence_entries(source: &str, start: usize, end: usize) -> Result<Vec<ValueSpan>> {
    if source.as_bytes().get(start) != Some(&b'[') || source.as_bytes().get(end - 1) != Some(&b']')
    {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for (segment_start, segment_end) in flow_segments(source, start + 1, end - 1, b',') {
        if let Some(span) = flow_value_span(source, segment_start, segment_end)
            && !span.raw.is_empty()
        {
            entries.push(span);
        }
    }
    Ok(entries)
}

fn flow_mapping_entries(
    source: &str,
    start: usize,
    end: usize,
) -> Result<Vec<(String, ValueSpan)>> {
    if source.as_bytes().get(start) != Some(&b'{') || source.as_bytes().get(end - 1) != Some(&b'}')
    {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for (segment_start, segment_end) in flow_segments(source, start + 1, end - 1, b',') {
        let segment = &source[segment_start..segment_end];
        if strip_yaml_comment(segment).trim().is_empty() {
            continue;
        }
        let Some(colon) = flow_top_level_colon(segment) else {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow mapping member could not be located safely",
                None,
                "SOURCE-EDIT-006",
            ));
        };
        let raw_key = segment[..colon].trim();
        let key = decode_mapping_key(raw_key).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow mapping key could not be decoded safely",
                None,
                "SOURCE-EDIT-006",
            )
        })?;
        let value_start = segment_start + colon + 1;
        let value = flow_value_span(source, value_start, segment_end).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow mapping value could not be located safely",
                None,
                "SOURCE-EDIT-006",
            )
        })?;
        entries.push((key, value));
    }
    Ok(entries)
}

fn mapping_member_spans(
    source: &str,
    span: &ValueSpan,
    path: &Path,
) -> Result<Vec<(String, ValueSpan)>> {
    let trimmed = span.raw.trim_start();
    if trimmed.starts_with('{') {
        let offset = span.raw.len() - trimmed.len();
        let start = span.start + offset;
        let end = flow_collection_end(source, start).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow mapping end could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        return flow_mapping_entries(source, start, end)
            .map_err(|error| with_requirement(error, "SOURCE-EDIT-006"));
    }
    let lines = source_lines(source);
    let literal_content = literal_block_scalar_content_lines(&lines);
    let first_line = line_index_for_offset(&lines, span.start).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "mapping start could not be located safely",
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        )
    })?;
    let limit_line = lines
        .iter()
        .position(|line| line.start >= span.end)
        .unwrap_or(lines.len());
    let first = (first_line..limit_line).find(|index| !is_ignorable_line(lines[*index].text));
    let Some(first) = first else {
        return Ok(Vec::new());
    };
    let indent = yaml_indent(lines[first].text);
    let mapping_indent =
        if is_sequence_item(lines[first].text) && mapping_span(lines[first].text).is_some() {
            indent + 2
        } else {
            indent
        };
    let mut result = Vec::new();
    for index in first..limit_line {
        if literal_content.get(index).copied().unwrap_or(false) {
            continue;
        }
        let line = lines[index];
        let direct = index == first && is_sequence_item(line.text)
            || yaml_indent(line.text) == mapping_indent;
        if !direct {
            continue;
        }
        let Some(mapping) = mapping_span(line.text) else {
            continue;
        };
        let child = mapping_value_span(
            source,
            &lines,
            index,
            &mapping,
            limit_line,
            &literal_content,
            path,
        )?;
        result.push((mapping.key, child));
    }
    Ok(result)
}

fn line_index_for_offset(lines: &[SourceLine<'_>], offset: usize) -> Option<usize> {
    lines
        .iter()
        .position(|line| line.start <= offset && offset < line.next_start)
        .or_else(|| lines.iter().position(|line| line.start == offset))
}

#[derive(Debug, Clone)]
struct SequenceItemSource {
    value: ValueSpan,
    raw: String,
    start: usize,
    end: usize,
}

struct SequenceItemSpanContext<'source, 'lines> {
    source: &'source str,
    lines: &'lines [SourceLine<'source>],
    item_line: usize,
    chunk_end_line: usize,
    chunk_end: usize,
    sequence_indent: usize,
    dash_column: usize,
}

#[derive(Debug, Clone)]
enum SequenceSourceLayout {
    Flow {
        start: usize,
        end: usize,
        prefix: String,
        separators: Vec<String>,
        suffix: String,
    },
    Block {
        start: usize,
        end: usize,
        indent: usize,
        empty_replacement: String,
        prefix: String,
        separators: Vec<String>,
        suffix: String,
    },
}

fn sequence_source_layout(
    source: &str,
    span: &ValueSpan,
    path: &Path,
) -> Result<(SequenceSourceLayout, Vec<SequenceItemSource>)> {
    let trimmed = span.raw.trim_start();
    if trimmed.starts_with('[') {
        let offset = span.raw.len() - trimmed.len();
        let start = span.start + offset;
        let end = flow_collection_end(source, start).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow sequence end could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        if flow_has_comment(source, start, end) {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow sequence comments prevent a safe structural patch",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            ));
        }
        let value_spans = flow_sequence_entries(source, start, end)?;
        let items = value_spans
            .iter()
            .map(|value| SequenceItemSource {
                value: value.clone(),
                raw: source[value.start..value.end].to_owned(),
                start: value.start,
                end: value.end,
            })
            .collect::<Vec<_>>();
        let prefix_end = items.first().map_or(end - 1, |item| item.start);
        let suffix_start = items.last().map_or(end - 1, |item| item.end);
        let separators = items
            .windows(2)
            .map(|pair| source[pair[0].end..pair[1].start].to_owned())
            .collect();
        Ok((
            SequenceSourceLayout::Flow {
                start,
                end,
                prefix: source[start..prefix_end].to_owned(),
                separators,
                suffix: source[suffix_start..end].to_owned(),
            },
            items,
        ))
    } else {
        let lines = source_lines(source);
        let first_line = line_index_for_offset(&lines, span.start).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "block sequence start could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        let limit_line = lines
            .iter()
            .position(|line| line.start >= span.end)
            .unwrap_or(lines.len());
        let first_content_start = span.start + span.raw.len() - span.raw.trim_start().len();
        let first_line_indent = yaml_indent(lines[first_line].text);
        let inline_nested_sequence = first_content_start
            > lines[first_line].start + first_line_indent
            && source.as_bytes().get(first_content_start) == Some(&b'-')
            && source
                .as_bytes()
                .get(first_content_start + 1)
                .is_some_and(u8::is_ascii_whitespace);
        let indent = if inline_nested_sequence {
            first_content_start - lines[first_line].start
        } else {
            first_line_indent
        };
        let item_lines = if inline_nested_sequence {
            std::iter::once(first_line)
                .chain((first_line + 1..limit_line).filter(|index| {
                    !is_ignorable_line(lines[*index].text)
                        && is_sequence_item(lines[*index].text)
                        && yaml_indent(lines[*index].text) == indent
                }))
                .collect::<Vec<_>>()
        } else {
            (first_line..limit_line)
                .filter(|index| {
                    !is_ignorable_line(lines[*index].text)
                        && is_sequence_item(lines[*index].text)
                        && yaml_indent(lines[*index].text) == indent
                })
                .collect::<Vec<_>>()
        };
        let Some(_first_candidate) = item_lines.first().copied() else {
            return Ok((
                SequenceSourceLayout::Block {
                    start: span.start,
                    end: span.end,
                    indent,
                    empty_replacement: format!("{}[]{}", " ".repeat(indent), newline_for(source)),
                    prefix: source[span.start..span.end].to_owned(),
                    separators: Vec::new(),
                    suffix: String::new(),
                },
                Vec::new(),
            ));
        };
        let mut items = Vec::with_capacity(item_lines.len());
        for (position, item_line) in item_lines.iter().copied().enumerate() {
            let boundary = item_lines.get(position + 1).copied().unwrap_or(limit_line);
            let mut chunk_end_line = boundary;
            while chunk_end_line > item_line
                && is_ignorable_line(lines[chunk_end_line - 1].text)
                && yaml_indent(lines[chunk_end_line - 1].text) <= indent
            {
                chunk_end_line -= 1;
            }
            let chunk_end = if chunk_end_line > item_line {
                lines[chunk_end_line - 1].next_start
            } else {
                lines[item_line].start
            };
            let item_start = if inline_nested_sequence && position == 0 {
                first_content_start
            } else {
                lines[item_line].start
            };
            let dash_column = if inline_nested_sequence && position == 0 {
                first_content_start - lines[item_line].start
            } else {
                yaml_indent(lines[item_line].text)
            };
            let item_source = sequence_item_value_span(
                SequenceItemSpanContext {
                    source,
                    lines: &lines,
                    item_line,
                    chunk_end_line,
                    chunk_end,
                    sequence_indent: indent,
                    dash_column,
                },
                path,
            )?;
            items.push(SequenceItemSource {
                value: item_source,
                raw: source[item_start..chunk_end].to_owned(),
                start: item_start,
                end: chunk_end,
            });
        }
        let start = span.start;
        let first_item_start = items.first().map_or(start, |item| item.start);
        let end = items.last().map_or(start, |item| item.end);
        let prefix = source[start..first_item_start].to_owned();
        let suffix = source[end..span.end].to_owned();
        let separators = items
            .windows(2)
            .map(|pair| source[pair[0].end..pair[1].start].to_owned())
            .collect();
        Ok((
            SequenceSourceLayout::Block {
                start: span.start,
                end: span.end,
                indent,
                empty_replacement: if inline_nested_sequence {
                    format!("[]{}", newline_for(source))
                } else {
                    format!("{}[]{}", " ".repeat(indent), newline_for(source))
                },
                prefix,
                separators,
                suffix,
            },
            items,
        ))
    }
}

fn sequence_item_value_span(
    context: SequenceItemSpanContext<'_, '_>,
    path: &Path,
) -> Result<ValueSpan> {
    let SequenceItemSpanContext {
        source,
        lines,
        item_line,
        chunk_end_line,
        chunk_end,
        sequence_indent,
        dash_column,
    } = context;
    let line = lines[item_line];
    let code = strip_yaml_comment(line.text);
    let dash = dash_column;
    let after_dash = code.get(dash + 1..).unwrap_or("");
    let leading = after_dash.len() - after_dash.trim_start().len();
    let value_start = line.start + dash + 1 + leading;
    let raw = after_dash.trim();
    if raw.is_empty() {
        if let Some((child_start, child_end)) =
            block_child_region(source, lines, item_line, sequence_indent, chunk_end_line)
        {
            return Ok(ValueSpan {
                start: child_start,
                end: child_end,
                raw: source[child_start..child_end].to_owned(),
                literal: None,
                block_header: None,
            });
        }
        return Ok(ValueSpan {
            start: value_start,
            end: value_start,
            raw: String::new(),
            literal: None,
            block_header: None,
        });
    }
    if raw.starts_with(['[', '{']) {
        let end = flow_collection_end(source, value_start).ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
                "flow value inside a block sequence could not be located safely",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-006",
            )
        })?;
        return Ok(ValueSpan {
            start: value_start,
            end,
            raw: source[value_start..end].to_owned(),
            literal: None,
            block_header: None,
        });
    }
    if raw == "-" || raw.starts_with("- ") || raw.starts_with("-\t") {
        return Ok(ValueSpan {
            start: value_start,
            end: chunk_end,
            raw: source[value_start..chunk_end].to_owned(),
            literal: None,
            block_header: None,
        });
    }
    if mapping_span(&code[dash + 1..]).is_some() {
        return Ok(ValueSpan {
            start: value_start,
            end: chunk_end,
            raw: source[value_start..chunk_end].to_owned(),
            literal: None,
            block_header: None,
        });
    }
    let end = line.start + code.trim_end().len();
    Ok(ValueSpan {
        start: value_start,
        end,
        raw: source[value_start..end].to_owned(),
        literal: None,
        block_header: None,
    })
}

fn flow_has_comment(source: &str, start: usize, end: usize) -> bool {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut index = start;
    while index < end {
        match quote {
            Some(b'\'') => {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else if bytes[index] == b'\'' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            None if bytes[index] == b'#'
                && (index == 0 || bytes[index - 1].is_ascii_whitespace()) =>
            {
                return true;
            }
            None if matches!(bytes[index], b'\'' | b'"') => {
                quote = Some(bytes[index]);
                index += 1;
            }
            _ => index += 1,
        }
    }
    false
}

fn literal_span(
    source: &str,
    lines: &[SourceLine<'_>],
    header_line: usize,
    record_end: usize,
    literal_content: &[bool],
) -> Result<LiteralSpan> {
    let header = lines[header_line];
    let mapping = mapping_span(header.text).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            "literal scalar header could not be located",
            None,
            "SOURCE-EDIT-006",
        )
    })?;
    let body_start_line = header_line + 1;
    let mut body_end_line = body_start_line;
    while body_end_line < record_end && literal_content.get(body_end_line).copied().unwrap_or(false)
    {
        body_end_line += 1;
    }
    // WHY: clip-style literal scalars do not own trailing separator blank lines.
    // Replacing those bytes would erase unrelated source formatting (SOURCE-EDIT-005).
    while body_end_line > body_start_line && lines[body_end_line - 1].text.trim().is_empty() {
        body_end_line -= 1;
    }
    let content_indent = (body_start_line..body_end_line)
        .find_map(|index| {
            let text = lines[index].text;
            (!text.trim().is_empty()).then(|| yaml_indent(text))
        })
        .unwrap_or_else(|| yaml_indent(header.text) + 2);
    let body_start = lines
        .get(body_start_line)
        .map_or(header.next_start, |line| line.start);
    let body_end = lines
        .get(body_end_line)
        .map_or(source.len(), |line| line.start);
    Ok(LiteralSpan {
        header_start: header.start + mapping.value_start,
        header_end: header.start + mapping.value_end,
        body_start,
        body_end,
        content_indent,
        newline: newline_for(source).to_owned(),
    })
}

fn render_replacement(
    source: &str,
    span: &ValueSpan,
    primitive: PrimitiveType,
    input: &str,
    desired: &DesiredScalar,
) -> Result<String> {
    if let Some(literal) = &span.literal {
        if primitive == PrimitiveType::String
            && input.ends_with('\n')
            && !input.ends_with("\n\n")
            && !input.contains('\r')
        {
            let body = input.strip_suffix('\n').unwrap_or(input);
            let mut rendered = String::from("|");
            rendered.push_str(&source[literal.header_end..literal.body_start]);
            if !body.is_empty() {
                for (index, line) in body.split('\n').enumerate() {
                    if index > 0 {
                        rendered.push_str(&literal.newline);
                    }
                    rendered.push_str(&" ".repeat(literal.content_indent));
                    rendered.push_str(line);
                }
                rendered.push_str(&literal.newline);
            }
            return Ok(rendered);
        }
        let quoted = json_string(input)?;
        return Ok(format!(
            "{}{}",
            quoted,
            &source[literal.header_end..literal.body_start]
        ));
    }
    if primitive == PrimitiveType::String {
        if input.contains(['\n', '\r']) {
            return json_string(input);
        }
        if span.raw.starts_with('\'') && span.raw.ends_with('\'') {
            return Ok(format!("'{}'", input.replace('\'', "''")));
        }
        if span.raw.starts_with('"') && span.raw.ends_with('"') {
            return json_string(input);
        }
        if plain_string_round_trips(input) {
            return Ok(input.to_owned());
        }
        return json_string(input);
    }
    if let Some(source_text) = &desired.source_text {
        return Ok(source_text.clone());
    }
    json_string(input)
}

fn plain_string_round_trips(input: &str) -> bool {
    if input.is_empty() || input.trim() != input || input.contains(['#', ':', '\n', '\r']) {
        return false;
    }
    let probe = format!("kind: data\ntable: edit-probe\nrecords:\n  - value: {input}\n");
    parse_yaml_document(PathBuf::from("<source-edit-string>"), &probe)
        .ok()
        .and_then(|parsed| match parsed.document {
            SourceDocument::Data(data) => data.records.into_iter().next(),
            _ => None,
        })
        .and_then(|record| record.get("value").cloned())
        == Some(Value::String(input.to_owned()))
}

fn json_string(input: &str) -> Result<String> {
    serde_json::to_string(input).map_err(|error| {
        source_edit_error(
            "E-SOURCE-EDIT-SCALAR-RENDER",
            format!("could not render edited string scalar: {error}"),
            None,
            "SOURCE-EDIT-003",
        )
    })
}

fn apply_patches(source: &str, patches: &[SourcePatch], path: &Path) -> Result<String> {
    let mut ordered = patches.to_vec();
    ordered.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
    });
    let mut result = source.to_owned();
    let mut previous_start = usize::MAX;
    for patch in ordered {
        if patch.start > patch.end
            || patch.end > source.len()
            || patch.end > previous_start
            || !source.is_char_boundary(patch.start)
            || !source.is_char_boundary(patch.end)
        {
            return Err(source_edit_error(
                "E-SOURCE-EDIT-PATCH-INVALID",
                "source edit patch ranges overlap or are not valid UTF-8 boundaries",
                Some(path.to_path_buf()),
                "SOURCE-EDIT-005",
            ));
        }
        result.replace_range(patch.start..patch.end, &patch.replacement);
        previous_start = patch.start;
    }
    Ok(result)
}

fn source_lines(source: &str) -> Vec<SourceLine<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for segment in source.split_inclusive('\n') {
        let next_start = start + segment.len();
        let content = segment.strip_suffix('\n').unwrap_or(segment);
        let content = content.strip_suffix('\r').unwrap_or(content);
        lines.push(SourceLine {
            start,
            next_start,
            text: content,
        });
        start = next_start;
    }
    if source.is_empty() {
        lines.push(SourceLine {
            start: 0,
            next_start: 0,
            text: "",
        });
    }
    lines
}

fn mapping_span(line: &str) -> Option<MappingSpan> {
    let code = strip_yaml_comment(line);
    let leading = code.len() - code.trim_start().len();
    let mut mapping_offset = leading;
    let mut mapping = &code[leading..];
    if let Some(rest) = mapping.strip_prefix('-')
        && (rest.is_empty() || rest.as_bytes().first().is_some_and(u8::is_ascii_whitespace))
    {
        let consumed = 1 + (rest.len() - rest.trim_start().len());
        mapping_offset += consumed;
        mapping = &code[mapping_offset..];
    }
    let colon = mapping_colon(mapping)?;
    let key = decode_mapping_key(mapping[..colon].trim())?;
    let after_colon = &mapping[colon + 1..];
    let value_leading = after_colon.len() - after_colon.trim_start().len();
    let value_start = mapping_offset + colon + 1 + value_leading;
    // A block collection may be introduced on the following line while an
    // inline comment leaves only whitespace in this mapping line. Retain an
    // empty source span so structural patches can keep that comment intact.
    let value_end = code.trim_end().len().max(value_start);
    Some(MappingSpan {
        key,
        value_start,
        value_end,
        raw_value: code[value_start..value_end].to_owned(),
    })
}

fn find_top_level_key(lines: &[SourceLine<'_>], key: &str) -> Option<usize> {
    lines.iter().position(|line| {
        yaml_indent(line.text) == 0
            && !is_ignorable_line(line.text)
            && mapping_span(line.text).is_some_and(|entry| entry.key == key)
    })
}
fn block_region_end(lines: &[SourceLine<'_>], key_line: usize) -> usize {
    let key_indent = yaml_indent(lines[key_line].text);
    for (index, line_entry) in lines.iter().enumerate().skip(key_line + 1) {
        if is_ignorable_line(line_entry.text) {
            continue;
        }
        let indent = yaml_indent(line_entry.text);
        if indent <= key_indent && !is_sequence_item(line_entry.text) {
            return index;
        }
    }
    lines.len()
}
fn find_block_sequence(
    lines: &[SourceLine<'_>],
    key_line: usize,
    region_end: usize,
) -> Option<SequenceRegion> {
    let mut indent = None;
    let mut items = Vec::new();
    for (index, line_entry) in lines
        .iter()
        .enumerate()
        .skip(key_line + 1)
        .take(region_end.saturating_sub(key_line + 1))
    {
        let line = line_entry.text;
        if is_ignorable_line(line) {
            continue;
        }
        let current_indent = yaml_indent(line);
        if !is_sequence_item(line) {
            let expected_indent = indent?;
            if current_indent <= expected_indent {
                return Some(SequenceRegion {
                    items,
                    indent: expected_indent,
                });
            }
            continue;
        }
        match indent {
            Some(expected) if expected != current_indent => continue,
            None => indent = Some(current_indent),
            _ => {}
        }
        if indent == Some(current_indent) {
            items.push(index);
        }
    }
    indent.map(|indent| SequenceRegion { items, indent })
}

fn literal_block_scalar_content_lines(lines: &[SourceLine<'_>]) -> Vec<bool> {
    let mut content_lines = vec![false; lines.len()];
    let mut active_scalar: Option<(usize, Option<usize>)> = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some((header_indent, content_indent)) = active_scalar {
            if line.text.trim().is_empty() {
                content_lines[index] = true;
                continue;
            }
            let indent = yaml_indent(line.text);
            match content_indent {
                Some(content_indent) if indent >= content_indent => {
                    content_lines[index] = true;
                    continue;
                }
                None if indent > header_indent => {
                    content_lines[index] = true;
                    active_scalar = Some((header_indent, Some(indent)));
                    continue;
                }
                _ => active_scalar = None,
            }
        }
        if let Some(header_indent) = bare_literal_block_scalar_header_indent(line.text) {
            active_scalar = Some((header_indent, None));
        }
    }
    content_lines
}
fn bare_literal_block_scalar_header_indent(line: &str) -> Option<usize> {
    let code = strip_yaml_comment(line);
    if code.trim().is_empty() {
        return None;
    }
    let mapping_value = mapping_span(code).is_some_and(|entry| entry.raw_value.trim() == "|");
    let sequence_value = sequence_item_parts(code).is_some_and(|value| value.trim() == "|");
    (mapping_value || sequence_value).then(|| yaml_indent(line))
}
fn sequence_item_parts(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('-')?;
    if rest.is_empty() || rest.as_bytes().first().is_some_and(u8::is_ascii_whitespace) {
        Some(rest.trim_start())
    } else {
        None
    }
}
fn is_sequence_item(line: &str) -> bool {
    sequence_item_parts(strip_yaml_comment(line)).is_some()
}
fn is_ignorable_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}
fn decode_mapping_key(key: &str) -> Option<String> {
    if key.is_empty() {
        return None;
    }
    if key.starts_with('"') || key.starts_with('\'') {
        serde_yaml::from_str::<Value>(key)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
    } else {
        Some(key.to_owned())
    }
}

fn mapping_colon(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        match quote {
            Some(b'\'') => {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else if bytes[index] == b'\'' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            None => match bytes[index] {
                b'\'' | b'"' => {
                    quote = Some(bytes[index]);
                    index += 1;
                }
                b':' if bytes
                    .get(index + 1)
                    .is_none_or(|next| next.is_ascii_whitespace()) =>
                {
                    return Some(index);
                }
                _ => index += 1,
            },
            _ => unreachable!(),
        }
    }
    None
}
fn strip_yaml_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        match quote {
            Some(b'\'') => {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else if bytes[index] == b'\'' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            None if bytes[index] == b'#'
                && (index == 0 || bytes[index - 1].is_ascii_whitespace()) =>
            {
                return &value[..index];
            }
            None => {
                if matches!(bytes[index], b'\'' | b'"') {
                    quote = Some(bytes[index]);
                }
                index += 1;
            }
            _ => unreachable!(),
        }
    }
    value
}
fn yaml_indent(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}
fn newline_for(source: &str) -> &str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn source_edit_error(
    code: &str,
    message: impl Into<String>,
    source: Option<PathBuf>,
    requirement: &str,
) -> MasterdataError {
    let mut error = MasterdataError::new(code, ErrorKind::Validation, message);
    if let Some(source) = source {
        error.diagnostic.source = Some(source);
    }
    error
        .diagnostic
        .related_requirements
        .push(requirement.to_owned());
    error
}
fn with_requirement(error: MasterdataError, requirement: &str) -> MasterdataError {
    let mut diagnostic = error.diagnostic().clone();
    if !diagnostic
        .related_requirements
        .iter()
        .any(|existing| existing == requirement)
    {
        diagnostic.related_requirements.push(requirement.to_owned());
    }
    MasterdataError {
        diagnostic: Box::new(diagnostic),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AddedRecordDraft, AddedRecordField, RecordTagEdit, RecordValueEdit, SourceRecordMutation,
        dry_run_source_edit, dry_run_source_record_mutation,
    };
    use crate::{
        AuthoringSequenceItem, AuthoringValue, ProjectDocuments, parse_yaml_document,
        validate_documents,
    };
    use std::path::{Path, PathBuf};
    fn documents(schema: &str, data_path: &str, data: &str) -> ProjectDocuments {
        ProjectDocuments {
            files: vec![
                parse_yaml_document(PathBuf::from("schema.yaml"), schema).expect("schema parses"),
                parse_yaml_document(PathBuf::from(data_path), data).expect("data parses"),
            ],
        }
    }
    const SCHEMA: &str = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: weight
    type: ulong
  - key: 2
    name: note
    type: string
primaryKey:
  fields: [id]
secondaryKeys: []
"#;
    #[test]
    fn source_edit_preserves_unrelated_text_and_exact_ulong() {
        let data = "kind: data\r\ntable: item\r\nrecords:\r\n  # keep\r\n  - id: 18446744073709551615\r\n    weight: 10 # inline\r\n    note: 'alpha'\r\n  - id: 2\r\n    weight: 20\r\n    note: beta\r\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_edit(
            &snapshot,
            Path::new("data.yaml"),
            &[
                RecordValueEdit {
                    record_index: 0,
                    field: "weight".to_owned(),
                    value: number("18446744073709551615"),
                },
                RecordValueEdit {
                    record_index: 0,
                    field: "note".to_owned(),
                    value: text("it's ready"),
                },
            ],
        )
        .expect("edit is safe");
        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("weight: 18446744073709551615 # inline\r\n    note: 'it''s ready'\r\n")
        );
        assert!(dry_run.plan.candidate_source.contains("  # keep\r\n"));
        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("  - id: 2\r\n    weight: 20\r\n    note: beta\r\n")
        );
    }
    #[test]
    fn source_edit_allows_domain_invalid_scalar_and_validation_reports_it() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: ok\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_edit(
            &snapshot,
            Path::new("data.yaml"),
            &[RecordValueEdit {
                record_index: 0,
                field: "weight".to_owned(),
                value: number("not-a-number"),
            }],
        )
        .expect("source-safe invalid domain value can be prepared");
        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("weight: not-a-number")
        );
        let report = validate_documents(&dry_run.transformed_documents);
        assert!(!report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-TABLE-INVALID-RECORD-VALUE")
        );
    }
    #[test]
    fn source_edit_allows_existing_key_field_by_exact_occurrence() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: ok\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_edit(
            &snapshot,
            Path::new("data.yaml"),
            &[RecordValueEdit {
                record_index: 0,
                field: "id".to_owned(),
                value: number("2"),
            }],
        )
        .expect("existing key field is directly editable");
        assert!(dry_run.plan.candidate_source.contains("  - id: 2\n"));
        assert!(dry_run.plan.candidate_source.contains("weight: 10\n"));
    }

    #[test]
    fn source_edit_allows_existing_secondary_key_field_by_exact_occurrence() {
        let schema = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: note
    type: string
primaryKey:
  fields: [id]
secondaryKeys:
  - fields: [note]
"#;
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: old\n";
        let snapshot = documents(schema, "data.yaml", data);
        let dry_run = dry_run_source_edit(
            &snapshot,
            Path::new("data.yaml"),
            &[RecordValueEdit {
                record_index: 0,
                field: "note".to_owned(),
                value: text("new"),
            }],
        )
        .expect("existing secondary key field is directly editable");
        assert!(dry_run.plan.candidate_source.contains("note: new"));
    }
    #[test]
    fn source_edit_reversion_is_byte_identical() {
        let data =
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: 'same'\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_edit(
            &snapshot,
            Path::new("data.yaml"),
            &[RecordValueEdit {
                record_index: 0,
                field: "note".to_owned(),
                value: text("same"),
            }],
        )
        .expect("same semantic value needs no patch");
        assert!(!dry_run.plan.changed);
        assert_eq!(dry_run.plan.candidate_source, data);
    }

    fn number(value: &str) -> AuthoringValue {
        AuthoringValue::Number {
            value: value.to_owned(),
        }
    }

    fn text(value: &str) -> AuthoringValue {
        AuthoringValue::String {
            value: value.to_owned(),
        }
    }

    #[test]
    fn record_tag_edit_adds_metadata_without_making_it_a_domain_field() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n  - id: 2\n    weight: 20\n    note: second\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                tag_edits: vec![RecordTagEdit {
                    record_index: 0,
                    tags: vec!["debug".to_owned(), "event-summer".to_owned()],
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("missing tags can be added to a block record mapping");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("$tags: [debug, event-summer]")
        );
        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("    note: first\n    $tags: [debug, event-summer]\n")
        );
        assert!(dry_run.plan.candidate_source.contains("  - id: 2"));
        let record = &dry_run
            .transformed_documents
            .data()
            .next()
            .unwrap()
            .1
            .records[0];
        assert_eq!(
            record
                .get("$tags")
                .and_then(serde_yaml::Value::as_sequence)
                .unwrap()
                .len(),
            2
        );
        let report = validate_documents(&dry_run.transformed_documents);
        assert!(
            report.valid,
            "tag metadata should not invalidate domain fields: {report:?}"
        );
    }

    #[test]
    fn record_tag_edit_preserves_existing_sequence_spelling_and_comments() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n    $tags: [old, 'keep'] # tags comment\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                tag_edits: vec![RecordTagEdit {
                    record_index: 0,
                    tags: vec!["new".to_owned(), "keep".to_owned()],
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("existing string sequence can be patched entry by entry");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("$tags: [new, 'keep'] # tags comment")
        );
    }

    #[test]
    fn record_tag_edit_keeps_explicit_empty_sequence_and_composes_with_added_tags() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n    $tags: [old, keep]\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                tag_edits: vec![RecordTagEdit {
                    record_index: 0,
                    tags: Vec::new(),
                }],
                additions: vec![AddedRecordDraft {
                    fields: vec![
                        AddedRecordField {
                            field: "id".to_owned(),
                            value: number("2"),
                        },
                        AddedRecordField {
                            field: "weight".to_owned(),
                            value: number("20"),
                        },
                        AddedRecordField {
                            field: "note".to_owned(),
                            value: text("second"),
                        },
                    ],
                    tags: vec!["debug".to_owned()],
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("tag and record changes compose");

        assert!(dry_run.plan.candidate_source.contains("$tags: []"));
        assert!(dry_run.plan.candidate_source.contains("$tags: [debug]"));
    }

    fn sequence(items: &[AuthoringValue]) -> AuthoringValue {
        AuthoringValue::Sequence {
            items: items
                .iter()
                .cloned()
                .map(|value| AuthoringSequenceItem {
                    source_index: None,
                    value,
                })
                .collect(),
            source_identity: false,
        }
    }

    fn sequence_with_sources(items: &[(Option<usize>, AuthoringValue)]) -> AuthoringValue {
        AuthoringValue::Sequence {
            items: items
                .iter()
                .map(|(source_index, value)| AuthoringSequenceItem {
                    source_index: *source_index,
                    value: value.clone(),
                })
                .collect(),
            source_identity: true,
        }
    }

    fn mapping(entries: &[(&str, AuthoringValue)]) -> AuthoringValue {
        AuthoringValue::Mapping {
            entries: entries
                .iter()
                .map(|(name, value)| crate::AuthoringMember {
                    name: (*name).to_owned(),
                    value: value.clone(),
                })
                .collect(),
        }
    }

    fn complex_documents(data: &str) -> ProjectDocuments {
        let schema = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: profile
    type: Profile
  - key: 2
    name: tags
    type: string
    array: true
  - key: 3
    name: status
    type: Status
  - key: 4
    name: access
    type: Permissions
  - key: 5
    name: bonus
    type: Profile
    nullable: true
primaryKey:
  fields: [id]
secondaryKeys: []
"#;
        let profile = r#"kind: type
name: Profile
custom:
  fields:
    - key: 0
      name: credits
      type: ulong
    - key: 1
      name: label
      type: string
    - key: 2
      name: alias
      type: string
      nullable: true
"#;
        let status = r#"kind: type
name: Status
enum:
  underlying: int
  members:
    - name: Ready
      value: 0
    - name: Paused
      value: 1
"#;
        let permissions = r#"kind: type
name: Permissions
flags:
  underlying: int
  members:
    - name: None
      value: 0
    - name: Read
      value: 1
    - name: Write
      value: 2
    - name: Execute
      value: 4
"#;
        ProjectDocuments {
            files: vec![
                parse_yaml_document(PathBuf::from("schema.yaml"), schema)
                    .expect("complex schema parses"),
                parse_yaml_document(PathBuf::from("profile.yaml"), profile)
                    .expect("Custom Type parses"),
                parse_yaml_document(PathBuf::from("status.yaml"), status).expect("Enum parses"),
                parse_yaml_document(PathBuf::from("permissions.yaml"), permissions)
                    .expect("Flags parses"),
                parse_yaml_document(PathBuf::from("data.yaml"), data).expect("complex data parses"),
            ],
        }
    }

    const COMPLEX_DATA: &str = "kind: data\ntable: item\nrecords:\n  - id: 1\n    profile:\n      credits: 18446744073709551614 # precise sibling\n      label: 'old' # label comment\n      alias: null # alias comment\n    tags: [one, 'two', three] # tags comment\n    status: Ready\n    access: [None] # access comment\n    bonus: null # bonus comment\n";

    fn added(fields: &[(&str, AuthoringValue)]) -> AddedRecordDraft {
        AddedRecordDraft {
            fields: fields
                .iter()
                .map(|(field, value)| AddedRecordField {
                    field: (*field).to_owned(),
                    value: value.clone(),
                })
                .collect(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn record_addition_supports_complex_shapes_and_preserves_null_placeholders_for_validation() {
        let snapshot = complex_documents("kind: data\ntable: item\nrecords: []\n");
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![
                    added(&[
                        ("status", text("Paused")),
                        (
                            "profile",
                            mapping(&[
                                ("credits", number("18446744073709551615")),
                                ("label", text("draft")),
                                ("alias", AuthoringValue::Null),
                            ]),
                        ),
                        ("access", sequence(&[text("Read"), text("Write")])),
                        ("bonus", AuthoringValue::Null),
                        ("tags", sequence(&[text("one"), text("two")])),
                        ("id", number("1")),
                    ]),
                    added(&[
                        ("id", number("2")),
                        (
                            "profile",
                            mapping(&[
                                ("credits", AuthoringValue::Null),
                                ("label", AuthoringValue::Null),
                                ("alias", AuthoringValue::Null),
                            ]),
                        ),
                        ("tags", AuthoringValue::Null),
                        ("status", AuthoringValue::Null),
                        ("access", AuthoringValue::Null),
                        ("bonus", AuthoringValue::Null),
                    ]),
                ],
                ..SourceRecordMutation::default()
            },
        )
        .expect("resolved complex shapes can be added through the shared value model");

        assert!(dry_run.plan.candidate_source.contains(
            "  - id: 1\n    profile: {credits: 18446744073709551615, label: draft, alias: null}\n    tags: [one, two]\n    status: Paused\n    access: [Read, Write]\n    bonus: null\n"
        ));
        assert!(dry_run.plan.candidate_source.contains(
            "  - id: 2\n    profile: {credits: null, label: null, alias: null}\n    tags: null\n    status: null\n    access: null\n    bonus: null\n"
        ));
        let validation = validate_documents(&dry_run.transformed_documents);
        assert!(!validation.valid);
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-TABLE-INVALID-RECORD-VALUE")
        );
    }

    #[test]
    fn record_addition_expands_empty_records_and_uses_schema_order() {
        let data = "kind: data\r\ntable: item\r\nrecords: []\r\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![added(&[
                    ("note", text("Potion")),
                    ("id", number("18446744073709551615")),
                    ("weight", number("10")),
                ])],
                ..SourceRecordMutation::default()
            },
        )
        .expect("empty records can receive a first row");

        assert_eq!(
            dry_run.plan.candidate_source,
            "kind: data\r\ntable: item\r\nrecords:\r\n  - id: 18446744073709551615\r\n    weight: 10\r\n    note: Potion\r\n"
        );
        assert_eq!(
            dry_run
                .transformed_documents
                .data()
                .next()
                .unwrap()
                .1
                .records
                .len(),
            1
        );
    }

    #[test]
    fn complex_value_edits_preserve_sibling_source_and_exact_nested_integers() {
        let snapshot = complex_documents(COMPLEX_DATA);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![
                    RecordValueEdit {
                        record_index: 0,
                        field: "profile".to_owned(),
                        value: mapping(&[
                            ("credits", number("18446744073709551615")),
                            ("label", text("updated")),
                            ("alias", text("chosen")),
                        ]),
                    },
                    RecordValueEdit {
                        record_index: 0,
                        field: "tags".to_owned(),
                        value: sequence(&[text("one"), text("three")]),
                    },
                    RecordValueEdit {
                        record_index: 0,
                        field: "status".to_owned(),
                        value: text("Paused"),
                    },
                    RecordValueEdit {
                        record_index: 0,
                        field: "access".to_owned(),
                        value: sequence(&[text("Read"), text("Write")]),
                    },
                    RecordValueEdit {
                        record_index: 0,
                        field: "bonus".to_owned(),
                        value: mapping(&[
                            ("credits", AuthoringValue::Null),
                            ("label", AuthoringValue::Null),
                            ("alias", AuthoringValue::Null),
                        ]),
                    },
                ],
                ..SourceRecordMutation::default()
            },
        )
        .expect("complex typed values can be edited together");

        let source = &dry_run.plan.candidate_source;
        assert!(source.contains("credits: 18446744073709551615 # precise sibling"));
        assert!(source.contains("label: 'updated' # label comment"));
        assert!(source.contains("alias: chosen # alias comment"));
        assert!(source.contains("tags: [one, three] # tags comment"));
        assert!(source.contains("status: Paused"));
        assert!(source.contains("access: [Read, Write] # access comment"));
        assert!(
            source.contains("bonus: {credits: null, label: null, alias: null} # bonus comment")
        );
        assert!(!source.contains("'two'"));
    }

    #[test]
    fn flow_custom_member_insertion_preserves_existing_mapping_bytes() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    profile: { credits : 18446744073709551614,  label : 'old' } # profile comment\n    tags: [one]\n    status: Ready\n    access: [None]\n    bonus: null\n";
        let snapshot = complex_documents(data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "profile".to_owned(),
                    value: mapping(&[
                        ("credits", number("18446744073709551614")),
                        ("label", text("old")),
                        ("alias", text("new")),
                    ]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("a missing declared Custom Type member can be inserted locally");

        assert!(
            dry_run.plan.candidate_source.contains(
                "profile: { credits : 18446744073709551614,  label : 'old' , alias: new} # profile comment"
            ),
            "candidate source: {:?}",
            dry_run.plan.candidate_source
        );
    }

    #[test]
    fn array_reordering_retains_each_item_source_spelling() {
        let snapshot = complex_documents(COMPLEX_DATA);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[text("three"), text("one"), text("two")]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("array reordering is a local sequence patch");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: [three, one, 'two'] # tags comment")
        );
    }

    #[test]
    fn array_element_addition_preserves_existing_item_source_and_comment() {
        let snapshot = complex_documents(COMPLEX_DATA);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[text("one"), text("two"), text("third")]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("an Array element can be appended as a local sequence patch");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: [one, 'two', third] # tags comment")
        );
    }

    #[test]
    fn array_edit_and_append_preserve_block_item_comments_and_quotes() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags:\n      - 'one' # first item\n      - two # second item\n      - three # third item",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[text("updated"), text("two"), text("three"), text("four")]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("editing and appending block sequence items is source preserving");

        assert!(dry_run.plan.candidate_source.contains(
            "      - 'updated' # first item\n      - two # second item\n      - three # third item\n      - four\n"
        ));
    }

    #[test]
    fn editing_and_reordering_uses_stable_source_occurrences() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags:\n      - \"first\" # first item\n      - 'second' # second item",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence_with_sources(&[
                        (Some(1), text("second")),
                        (Some(0), text("updated")),
                    ]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("source occurrence identity keeps edits attached to the moved item");

        assert!(dry_run.plan.candidate_source.contains(
            "    tags:\n      - 'second' # second item\n      - \"updated\" # first item\n"
        ));
    }

    #[test]
    fn sequence_edit_rejects_stale_or_reused_source_occurrences() {
        let snapshot = complex_documents(COMPLEX_DATA);
        for value in [
            sequence_with_sources(&[(Some(9), text("one"))]),
            sequence_with_sources(&[(Some(0), text("one")), (Some(0), text("one"))]),
        ] {
            let error = dry_run_source_record_mutation(
                &snapshot,
                Path::new("data.yaml"),
                &SourceRecordMutation {
                    edits: vec![RecordValueEdit {
                        record_index: 0,
                        field: "tags".to_owned(),
                        value,
                    }],
                    ..SourceRecordMutation::default()
                },
            )
            .expect_err("invalid sequence occurrence identity cannot select source text");

            assert_eq!(
                error.diagnostic().code,
                "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE"
            );
            assert!(error.diagnostic().message.contains("source item identity"));
        }
    }

    #[test]
    fn removing_the_last_block_array_item_keeps_an_empty_array_value() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags:\n      - one",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("the last block Array item can be removed");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("    tags:\n      []\n")
        );
    }

    #[test]
    fn removing_the_last_flow_array_item_handles_a_trailing_comma() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags: [one, 'two', three,] # tags comment",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("the last flow Array item can be removed");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: [] # tags comment")
        );
    }

    #[test]
    fn block_array_structure_rejects_ambiguous_inter_item_source() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags:\n      - one\n      # keep between items\n      - two",
        );
        let snapshot = complex_documents(&data);
        let error = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[text("two")]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect_err("a standalone inter-item comment cannot be reassigned safely");

        assert_eq!(
            error.diagnostic().code,
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE"
        );
        assert!(error.diagnostic().message.contains("separators"));
    }

    #[test]
    fn duplicate_array_removal_without_source_identity_fails_closed() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags: [same, 'same'] # tags comment",
        );
        let snapshot = complex_documents(&data);
        let error = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence(&[text("same")]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect_err("duplicate item source cannot be identified without occurrence metadata");

        assert_eq!(
            error.diagnostic().code,
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE"
        );
        assert!(
            error
                .diagnostic()
                .message
                .contains("duplicate sequence values")
        );
    }

    #[test]
    fn duplicate_array_removal_with_source_identity_preserves_the_selected_occurrence() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags: [same, 'same'] # tags comment",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence_with_sources(&[(Some(1), text("same"))]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("the request identifies the selected duplicate source occurrence");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: ['same'] # tags comment")
        );
    }

    #[test]
    fn reordering_semantically_equal_items_still_moves_their_source_occurrences() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags: [same, 'same'] # tags comment",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence_with_sources(&[
                        (Some(1), text("same")),
                        (Some(0), text("same")),
                    ]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("source occurrence order remains meaningful when values compare equal");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: ['same', same] # tags comment")
        );
    }

    #[test]
    fn clearing_duplicate_array_items_uses_tracked_empty_sequence_identity() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags: [same, 'same'] # tags comment",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence_with_sources(&[]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("an explicit empty sequence identifies removal of every source item");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: [] # tags comment")
        );
    }

    #[test]
    fn adding_a_duplicate_value_does_not_reuse_a_removed_items_source_text() {
        let data = COMPLEX_DATA.replace(
            "tags: [one, 'two', three] # tags comment",
            "tags: [same, 'same'] # tags comment",
        );
        let snapshot = complex_documents(&data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "tags".to_owned(),
                    value: sequence_with_sources(&[(None, text("same"))]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("a newly added duplicate has no source occurrence to inherit");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("tags: [same] # tags comment")
        );
    }

    #[test]
    fn nested_custom_array_reordering_uses_requested_occurrence_identity() {
        let schema = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: profile
    type: Profile
primaryKey:
  fields: [id]
secondaryKeys: []
"#;
        let profile = r#"kind: type
name: Profile
custom:
  fields:
    - key: 0
      name: tags
      type: string
      array: true
"#;
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    profile:\n      tags: [one, 'two']\n";
        let snapshot = ProjectDocuments {
            files: vec![
                parse_yaml_document(PathBuf::from("schema.yaml"), schema).expect("schema parses"),
                parse_yaml_document(PathBuf::from("profile.yaml"), profile)
                    .expect("Custom Type parses"),
                parse_yaml_document(PathBuf::from("data.yaml"), data).expect("data parses"),
            ],
        };
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "profile".to_owned(),
                    value: mapping(&[(
                        "tags",
                        sequence_with_sources(&[(Some(1), text("two")), (Some(0), text("one"))]),
                    )]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("nested Custom Type Array reordering is source-safe");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("      tags: ['two', one]\n")
        );
    }

    #[test]
    fn array_of_flags_uses_nested_block_sequence_source_paths() {
        let schema = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: features
    type: Permissions
    array: true
primaryKey:
  fields: [id]
secondaryKeys: []
"#;
        let permissions = r#"kind: type
name: Permissions
flags:
  underlying: int
  members:
    - name: None
      value: 0
    - name: Read
      value: 1
    - name: Write
      value: 2
"#;
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    features:\n      - - Read\n        - Write\n      - - None\n";
        let snapshot = ProjectDocuments {
            files: vec![
                parse_yaml_document(PathBuf::from("schema.yaml"), schema).expect("schema parses"),
                parse_yaml_document(PathBuf::from("permissions.yaml"), permissions)
                    .expect("Flags declaration parses"),
                parse_yaml_document(PathBuf::from("data.yaml"), data).expect("data parses"),
            ],
        };
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "features".to_owned(),
                    value: sequence_with_sources(&[
                        (Some(0), sequence_with_sources(&[(Some(0), text("Read"))])),
                        (Some(1), sequence_with_sources(&[(Some(0), text("None"))])),
                    ]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("nested block Flags values can be edited");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("    features:\n      - - Read\n      - - None\n")
        );

        let emptied_inner_flags = ProjectDocuments {
            files: vec![
                parse_yaml_document(PathBuf::from("schema.yaml"), schema).expect("schema parses"),
                parse_yaml_document(PathBuf::from("permissions.yaml"), permissions)
                    .expect("Flags declaration parses"),
                parse_yaml_document(PathBuf::from("data.yaml"), &dry_run.plan.candidate_source)
                    .expect("edited data parses"),
            ],
        };
        let empty_flags = dry_run_source_record_mutation(
            &emptied_inner_flags,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "features".to_owned(),
                    value: sequence_with_sources(&[
                        (Some(0), sequence_with_sources(&[])),
                        (Some(1), sequence_with_sources(&[(Some(0), text("None"))])),
                    ]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("the last block member in a Flags Array can be removed");
        assert!(
            empty_flags
                .plan
                .candidate_source
                .contains("    features:\n      - []\n      - - None\n")
        );
    }

    #[test]
    fn nullable_custom_value_can_return_to_null_without_touching_its_comment() {
        let snapshot = complex_documents(COMPLEX_DATA);
        let materialized = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "bonus".to_owned(),
                    value: mapping(&[
                        ("credits", AuthoringValue::Null),
                        ("label", AuthoringValue::Null),
                        ("alias", AuthoringValue::Null),
                    ]),
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("nullable Custom Type can be materialized");
        let remapped = complex_documents(&materialized.plan.candidate_source);
        let nulled = dry_run_source_record_mutation(
            &remapped,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "bonus".to_owned(),
                    value: AuthoringValue::Null,
                }],
                ..SourceRecordMutation::default()
            },
        )
        .expect("nullable Custom Type can return to null");

        assert!(
            nulled
                .plan
                .candidate_source
                .contains("bonus: null # bonus comment")
        );
    }

    #[test]
    fn nested_validation_diagnostic_identifies_the_invalid_value_path() {
        let invalid =
            COMPLEX_DATA.replace("credits: 18446744073709551614", "credits: outside-range");
        let snapshot = complex_documents(&invalid);
        let report = validate_documents(&snapshot);
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "E-TABLE-INVALID-RECORD-VALUE")
            .expect("nested type validation diagnostic");

        assert_eq!(diagnostic.value_path.as_deref(), Some("/credits"));
    }

    #[test]
    fn record_addition_preserves_signed_and_unsigned_64_bit_boundaries() {
        let schema = r#"kind: schema
table: item
fields:
  - key: 0
    name: signed
    type: long
  - key: 1
    name: unsigned
    type: ulong
  - key: 2
    name: note
    type: string
primaryKey:
  fields: [signed]
secondaryKeys: []
"#;
        let snapshot = documents(
            schema,
            "data.yaml",
            "kind: data\ntable: item\nrecords: []\n",
        );
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![added(&[
                    ("note", text("boundaries")),
                    ("unsigned", number("18446744073709551615")),
                    ("signed", number("-9223372036854775808")),
                ])],
                ..SourceRecordMutation::default()
            },
        )
        .expect("64-bit boundary text stays representable");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("signed: -9223372036854775808")
        );
        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("unsigned: 18446744073709551615")
        );
    }

    #[test]
    fn record_addition_preserves_existing_comments_blank_lines_and_line_endings() {
        let data = "kind: data\r\ntable: item\r\nrecords:\r\n  # keep before\r\n  - id: 1\r\n    weight: 10 # keep inline\r\n    note: 'one'\r\n\r\n  # keep after\r\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![added(&[
                    ("id", number("2")),
                    ("weight", number("20")),
                    ("note", text("two")),
                ])],
                ..SourceRecordMutation::default()
            },
        )
        .expect("row append is source-preserving");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("  # keep before\r\n")
        );
        assert!(dry_run.plan.candidate_source.contains(
            "weight: 10 # keep inline\r\n    note: 'one'\r\n\r\n  # keep after\r\n  - id: 2\r\n"
        ));
        assert!(
            !dry_run
                .plan
                .candidate_source
                .replace("\r\n", "")
                .contains('\n')
        );
    }

    #[test]
    fn record_mutation_preserves_inline_comment_on_records_sequence() {
        let empty = "kind: data\ntable: item\nrecords: [] # keep empty\n";
        let snapshot = documents(SCHEMA, "data.yaml", empty);
        let added_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![added(&[
                    ("id", number("1")),
                    ("weight", number("10")),
                    ("note", text("one")),
                ])],
                ..SourceRecordMutation::default()
            },
        )
        .expect("inline comment on an empty flow sequence remains safe");
        assert!(
            added_run
                .plan
                .candidate_source
                .contains("records: # keep empty\n  - id: 1")
        );

        let populated = "kind: data\ntable: item\nrecords: # keep records\n  - id: 1\n    weight: 10\n    note: one\n";
        let snapshot = documents(SCHEMA, "data.yaml", populated);
        let deleted_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                deletions: vec![0],
                ..SourceRecordMutation::default()
            },
        )
        .expect("last record can be removed beside the inline comment");
        assert!(
            deleted_run
                .plan
                .candidate_source
                .contains("records: [] # keep records")
        );
    }

    #[test]
    fn record_delete_targets_source_occurrence_not_primary_key_value() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n  - id: 1\n    weight: 20\n    note: second\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                deletions: vec![1],
                ..SourceRecordMutation::default()
            },
        )
        .expect("selected occurrence can be deleted");

        assert!(dry_run.plan.candidate_source.contains("note: first"));
        assert!(!dry_run.plan.candidate_source.contains("note: second"));
        assert_eq!(
            dry_run
                .transformed_documents
                .data()
                .next()
                .unwrap()
                .1
                .records
                .len(),
            1
        );
    }

    #[test]
    fn record_delete_preserves_separator_comment_and_blank_line() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n\n  # keep separator\n  - id: 2\n    weight: 20\n    note: second\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                deletions: vec![0],
                ..SourceRecordMutation::default()
            },
        )
        .expect("selected occurrence can be removed without consuming separator text");

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("records:\n\n  # keep separator\n  - id: 2")
        );
        assert!(!dry_run.plan.candidate_source.contains("note: first"));
    }

    #[test]
    fn record_mutations_compose_edit_delete_and_add_in_one_candidate() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: first\n  - id: 2\n    weight: 20\n    note: second\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                edits: vec![RecordValueEdit {
                    record_index: 0,
                    field: "weight".to_owned(),
                    value: number("11"),
                }],
                additions: vec![added(&[
                    ("id", number("3")),
                    ("weight", number("30")),
                    ("note", text("third")),
                ])],
                deletions: vec![1],
                tag_edits: Vec::new(),
            },
        )
        .expect("all mutation kinds compose");

        assert!(dry_run.plan.candidate_source.contains("weight: 11"));
        assert!(dry_run.plan.candidate_source.contains("note: third"));
        assert!(!dry_run.plan.candidate_source.contains("note: second"));
        assert_eq!(
            dry_run
                .transformed_documents
                .data()
                .next()
                .unwrap()
                .1
                .records
                .len(),
            2
        );
    }

    #[test]
    fn record_addition_preserves_invalid_primitive_input_for_validation() {
        let data = "kind: data\ntable: item\nrecords: []\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![added(&[
                    ("id", number("1")),
                    ("weight", number("not-a-number")),
                    ("note", text("draft")),
                ])],
                ..SourceRecordMutation::default()
            },
        )
        .expect("domain-invalid input remains a YAML-safe candidate");
        let report = validate_documents(&dry_run.transformed_documents);

        assert!(
            dry_run
                .plan
                .candidate_source
                .contains("weight: not-a-number")
        );
        assert!(!report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-TABLE-INVALID-RECORD-VALUE")
        );
    }

    #[test]
    fn record_addition_supports_nullable_fields_and_null_placeholders() {
        let schema = r#"kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ulong
  - key: 1
    name: weight
    type: ulong
    nullable: true
primaryKey:
  fields: [id]
secondaryKeys: []
"#;
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: null\n";
        let snapshot = documents(schema, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                additions: vec![added(&[
                    ("id", number("2")),
                    ("weight", AuthoringValue::Null),
                ])],
                ..SourceRecordMutation::default()
            },
        )
        .expect("nullable shape is supported by the resolved value model");
        assert!(dry_run.plan.candidate_source.contains("weight: null"));
    }

    #[test]
    fn deleting_last_record_restores_an_empty_sequence_without_losing_source_safety() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: only\n\n  # keep trailing\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let dry_run = dry_run_source_record_mutation(
            &snapshot,
            Path::new("data.yaml"),
            &SourceRecordMutation {
                deletions: vec![0],
                ..SourceRecordMutation::default()
            },
        )
        .expect("last record deletion leaves a valid empty data document");

        assert!(dry_run.plan.candidate_source.contains("records: []"));
        assert!(dry_run.plan.candidate_source.contains("# keep trailing"));
        assert!(
            dry_run
                .transformed_documents
                .data()
                .next()
                .unwrap()
                .1
                .records
                .is_empty()
        );
    }
}
