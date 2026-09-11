use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_yaml::Value;
use sha2::{Digest, Sha256};

use crate::document::{
    DataDocument, ProjectDocuments, SchemaDocument, SourceDocument, parse_yaml_document,
};
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::type_system::PrimitiveType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordValueEdit {
    pub record_index: usize,
    pub field: String,
    pub value: String,
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
    let editable = editable_fields(schema);
    let mut seen = BTreeSet::new();
    let mut expected = data.clone();
    let mut patches = Vec::new();

    for edit in edits {
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
        let primitive = editable.get(edit.field.as_str()).copied().ok_or_else(|| {
            source_edit_error(
                "E-SOURCE-EDIT-FIELD-READ-ONLY",
                format!(
                    "field `{}` is not an editable Required Primitive non-key field",
                    edit.field
                ),
                Some(path.to_path_buf()),
                "SOURCE-EDIT-002",
            )
        })?;
        let desired = desired_scalar(primitive, &edit.value)?;
        if *current == desired.value {
            continue;
        }
        let span =
            locate_record_member_value(&loaded.source, data, edit.record_index, &edit.field, path)?;
        let replacement =
            render_replacement(&loaded.source, &span, primitive, &edit.value, &desired)?;
        patches.push(SourcePatch {
            start: span.start,
            end: span.end,
            replacement,
        });
        expected.records[edit.record_index].insert(edit.field.clone(), desired.value);
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

fn editable_fields(schema: &SchemaDocument) -> BTreeMap<&str, PrimitiveType> {
    let mut keys = BTreeSet::new();
    if let Some(primary) = &schema.primary_key {
        keys.extend(primary.fields.iter().map(String::as_str));
    }
    for secondary in &schema.secondary_keys {
        keys.extend(secondary.fields.iter().map(String::as_str));
    }
    schema
        .fields
        .iter()
        .filter(|field| !field.nullable && !field.array && !keys.contains(field.name.as_str()))
        .filter_map(|field| {
            PrimitiveType::parse(&field.type_name).map(|primitive| (field.name.as_str(), primitive))
        })
        .collect()
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
}

fn locate_record_member_value(
    source: &str,
    data: &DataDocument,
    record_index: usize,
    field: &str,
    path: &Path,
) -> Result<ValueSpan> {
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
    let item_line = *sequence.items.get(record_index).ok_or_else(|| {
        source_edit_error(
            "E-SOURCE-EDIT-RECORD-NOT-FOUND",
            format!("record[{record_index}] does not exist"),
            Some(path.to_path_buf()),
            "SOURCE-EDIT-001",
        )
    })?;
    let record_end = sequence
        .items
        .get(record_index + 1)
        .copied()
        .unwrap_or(region_end);
    let mut found = Vec::new();
    for (line_index, line) in lines.iter().enumerate().take(record_end).skip(item_line) {
        if literal_content.get(line_index).copied().unwrap_or(false) {
            continue;
        }
        if let Some(mapping) = mapping_span(line.text)
            && mapping.key == field
        {
            found.push((line_index, mapping));
        }
    }
    let [(line_index, mapping)] = found.as_slice() else {
        return Err(source_edit_error(
            "E-SOURCE-EDIT-SOURCE-UNCLASSIFIABLE",
            format!(
                "record[{record_index}] member `{field}` could not be located exactly once in source"
            ),
            Some(path.to_path_buf()),
            "SOURCE-EDIT-006",
        ));
    };
    let line = lines[*line_index];
    let start = line.start + mapping.value_start;
    if mapping.raw_value == "|" {
        let literal = literal_span(source, &lines, *line_index, record_end, &literal_content)?;
        return Ok(ValueSpan {
            start: literal.header_start,
            end: literal.body_end,
            raw: mapping.raw_value.clone(),
            literal: Some(literal),
        });
    }
    Ok(ValueSpan {
        start,
        end: line.start + mapping.value_end,
        raw: mapping.raw_value.clone(),
        literal: None,
    })
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
    let value_end = code.trim_end().len();
    if value_start > value_end {
        return None;
    }
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
                return Some(SequenceRegion { items });
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
    indent.map(|_| SequenceRegion { items })
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
    use super::{RecordValueEdit, dry_run_source_edit};
    use crate::{ProjectDocuments, parse_yaml_document, validate_documents};
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
                    value: "18446744073709551615".to_owned(),
                },
                RecordValueEdit {
                    record_index: 0,
                    field: "note".to_owned(),
                    value: "it's ready".to_owned(),
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
                value: "not-a-number".to_owned(),
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
    fn source_edit_rejects_key_field() {
        let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    weight: 10\n    note: ok\n";
        let snapshot = documents(SCHEMA, "data.yaml", data);
        let error = dry_run_source_edit(
            &snapshot,
            Path::new("data.yaml"),
            &[RecordValueEdit {
                record_index: 0,
                field: "id".to_owned(),
                value: "2".to_owned(),
            }],
        )
        .expect_err("key field remains read-only");
        assert_eq!(error.diagnostic().code, "E-SOURCE-EDIT-FIELD-READ-ONLY");
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
                value: "same".to_owned(),
            }],
        )
        .expect("same semantic value needs no patch");
        assert!(!dry_run.plan.changed);
        assert_eq!(dry_run.plan.candidate_source, data);
    }
}
