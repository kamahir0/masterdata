//! Lossless, typed edits for the v1 Project Settings surface.
//!
//! `toml::Value` remains the validation/parser authority, but it is never used
//! to serialize the candidate.  The small locator below keeps the exact source
//! bytes for unrelated sections, comments, quote style, and line endings.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::config::PublishTargetKind;
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::source_edit::source_content_identity;
use crate::table::is_tag_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectConfigEditOperation {
    AddProfile {
        name: String,
        include_tags: Vec<String>,
        exclude_tags: Vec<String>,
    },
    UpdateProfile {
        name: String,
        include_tags: Vec<String>,
        exclude_tags: Vec<String>,
    },
    AddPublishTarget {
        kind: PublishTargetKind,
        path: String,
    },
    UpdatePublishTargetPath {
        index: usize,
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigEditPreview {
    pub base_content_identity: String,
    pub candidate_content_identity: String,
    pub candidate_source: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    start: usize,
    text: &'a str,
}

#[derive(Debug, Clone)]
struct Section {
    start_line: usize,
    end_line: usize,
    path: Vec<String>,
    array: bool,
}

#[derive(Debug, Clone)]
struct Property {
    value_start: usize,
    value_end: usize,
}

#[derive(Debug, Clone)]
struct ArrayEntry {
    start: usize,
    end: usize,
    token_start: usize,
    token_end: usize,
    value: String,
}

#[derive(Debug, Clone)]
struct Patch {
    start: usize,
    end: usize,
    replacement: String,
}

pub fn preview_project_config_edit(
    source: &str,
    operation: &ProjectConfigEditOperation,
) -> Result<ProjectConfigEditPreview> {
    parse_toml(source)?;
    let lines = source_lines(source);
    let sections = sections(source, &lines)?;
    let mut patches = Vec::new();

    match operation {
        ProjectConfigEditOperation::AddProfile {
            name,
            include_tags,
            exclude_tags,
        } => {
            validate_new_profile_name(name, &sections)?;
            patches.push(append_profile(source, name, include_tags, exclude_tags));
        }
        ProjectConfigEditOperation::UpdateProfile {
            name,
            include_tags,
            exclude_tags,
        } => {
            let section = profile_section(&sections, name).ok_or_else(|| {
                config_edit_error(
                    "E-CONFIG-EDIT-PROFILE-NOT-FOUND",
                    format!("Build Profile `{name}` was not found"),
                )
            })?;
            patches.extend(update_profile_property(
                source,
                &lines,
                section,
                "include_tags",
                include_tags,
            )?);
            patches.extend(update_profile_property(
                source,
                &lines,
                section,
                "exclude_tags",
                exclude_tags,
            )?);
        }
        ProjectConfigEditOperation::AddPublishTarget { kind, path } => {
            patches.push(append_publish_target(source, *kind, path));
        }
        ProjectConfigEditOperation::UpdatePublishTargetPath { index, path } => {
            let target_sections = sections
                .iter()
                .filter(|section| section.array && section.path == ["publish", "targets"])
                .collect::<Vec<_>>();
            let section = target_sections.get(*index).ok_or_else(|| {
                config_edit_error(
                    "E-CONFIG-EDIT-TARGET-NOT-FOUND",
                    format!("publish target occurrence[{index}] was not found"),
                )
            })?;
            let property = property_in_section(&lines, section, "path").ok_or_else(|| {
                config_edit_error(
                    "E-CONFIG-EDIT-TARGET-UNSUPPORTED",
                    "publish target path is not a directly editable string property",
                )
            })?;
            patches.push(update_string_property(source, &lines, property, path)?);
        }
    }

    let candidate_source = apply_patches(source, &patches)?;
    parse_toml(&candidate_source)?;
    Ok(ProjectConfigEditPreview {
        base_content_identity: source_content_identity(source),
        candidate_content_identity: source_content_identity(&candidate_source),
        changed: candidate_source != source,
        candidate_source,
    })
}

fn parse_toml(source: &str) -> Result<toml::Value> {
    toml::from_str(source).map_err(|error| {
        config_edit_error(
            "E-CONFIG-EDIT-TOML-PARSE",
            format!("configuration is not valid TOML: {error}"),
        )
    })
}

fn source_lines(source: &str) -> Vec<SourceLine<'_>> {
    let mut result = Vec::new();
    let mut start = 0;
    for segment in source.split_inclusive('\n') {
        let next_start = start + segment.len();
        let text = segment.strip_suffix('\n').unwrap_or(segment);
        let text = text.strip_suffix('\r').unwrap_or(text);
        result.push(SourceLine { start, text });
        start = next_start;
    }
    if source.is_empty() {
        result.push(SourceLine { start: 0, text: "" });
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MultilineString {
    Basic,
    Literal,
}

fn sections(source: &str, lines: &[SourceLine<'_>]) -> Result<Vec<Section>> {
    let mut result = Vec::new();
    let mut multiline = None;
    for (line, source_line) in lines.iter().enumerate() {
        let uncommented = section_line_code(source_line.text, &mut multiline);
        let code = uncommented.trim();
        let (array, inner) =
            if let Some(inner) = code.strip_prefix("[[").and_then(|v| v.strip_suffix("]]")) {
                (true, inner.trim())
            } else if let Some(inner) = code.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                (false, inner.trim())
            } else {
                continue;
            };
        let path = parse_key_path(inner).ok_or_else(|| {
            config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                format!("table header on line {} cannot be located safely", line + 1),
            )
        })?;
        result.push(Section {
            start_line: line,
            end_line: lines.len(),
            path,
            array,
        });
    }
    for index in 0..result.len() {
        result[index].end_line = result
            .get(index + 1)
            .map_or(lines.len(), |next| next.start_line);
    }
    let _ = source;
    Ok(result)
}

fn section_line_code(line: &str, multiline: &mut Option<MultilineString>) -> String {
    if let Some(kind) = *multiline {
        let delimiter = match kind {
            MultilineString::Basic => "\"\"\"",
            MultilineString::Literal => "'''",
        };
        if multiline_delimiter_position(line, delimiter, 0).is_some() {
            *multiline = None;
        }
        return String::new();
    }

    let bytes = line.as_bytes();
    let mut index = 0usize;
    let mut quote = None;
    while index < bytes.len() {
        if quote.is_none() && bytes[index] == b'#' {
            return line[..index].to_owned();
        }
        if quote.is_none() && bytes[index..].starts_with(b"\"\"\"") {
            if let Some(close) = multiline_delimiter_position(line, "\"\"\"", index + 3) {
                index = close + 3;
                continue;
            }
            *multiline = Some(MultilineString::Basic);
            return line[..index].to_owned();
        }
        if quote.is_none() && bytes[index..].starts_with(b"'''") {
            if let Some(close) = multiline_delimiter_position(line, "'''", index + 3) {
                index = close + 3;
                continue;
            }
            *multiline = Some(MultilineString::Literal);
            return line[..index].to_owned();
        }
        match quote {
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                if bytes[index] == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') if bytes[index] == b'\'' => quote = None,
            Some(_) => {}
            None if bytes[index] == b'"' || bytes[index] == b'\'' => quote = Some(bytes[index]),
            None => {}
        }
        index += 1;
    }
    line.to_owned()
}

fn multiline_delimiter_position(line: &str, delimiter: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let delimiter_bytes = delimiter.as_bytes();
    let mut index = start;
    while index + delimiter_bytes.len() <= bytes.len() {
        if &bytes[index..index + delimiter_bytes.len()] == delimiter_bytes {
            if delimiter == "\"\"\"" {
                let backslashes = bytes[..index]
                    .iter()
                    .rev()
                    .take_while(|byte| **byte == b'\\')
                    .count();
                if backslashes % 2 == 1 {
                    index += 1;
                    continue;
                }
            }
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_key_path(input: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let bytes = input.as_bytes();
    let mut start = 0;
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        match quote {
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
            Some(b'\'') => {
                if bytes[index] == b'\'' {
                    quote = None;
                }
                index += 1;
            }
            Some(_) => return None,
            None => match bytes[index] {
                b'"' | b'\'' => {
                    quote = Some(bytes[index]);
                    index += 1;
                }
                b'.' => {
                    parts.push(decode_key(input[start..index].trim())?);
                    start = index + 1;
                    index += 1;
                }
                _ => index += 1,
            },
        }
    }
    if quote.is_some() {
        return None;
    }
    let last = input[start..].trim();
    (!last.is_empty()).then(|| {
        parts.push(decode_key(last)?);
        Some(parts)
    })?
}

fn decode_key(value: &str) -> Option<String> {
    if value.starts_with('"') {
        toml::from_str::<toml::Value>(&format!("key = {value}"))
            .ok()?
            .get("key")?
            .as_str()
            .map(str::to_owned)
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        Some(value[1..value.len() - 1].to_owned())
    } else {
        (!value.is_empty()).then(|| value.to_owned())
    }
}

fn profile_section<'a>(sections: &'a [Section], name: &str) -> Option<&'a Section> {
    sections
        .iter()
        .find(|section| !section.array && section.path == ["build", "profiles", name])
}

fn validate_new_profile_name(name: &str, sections: &[Section]) -> Result<()> {
    if !is_tag_name(name) {
        return Err(config_edit_error(
            "E-CONFIG-EDIT-PROFILE-NAME",
            format!("Build Profile name `{name}` is not lowercase kebab-case"),
        ));
    }
    if profile_section(sections, name).is_some() {
        return Err(config_edit_error(
            "E-CONFIG-EDIT-PROFILE-COLLISION",
            format!("Build Profile `{name}` already exists"),
        ));
    }
    Ok(())
}

fn property_in_section(lines: &[SourceLine<'_>], section: &Section, key: &str) -> Option<Property> {
    (section.start_line + 1..section.end_line).find_map(|line| assignment(lines[line], key))
}

fn assignment(line: SourceLine<'_>, key: &str) -> Option<Property> {
    let code = strip_comment(line.text);
    let equals = top_level_byte(&code, b'=')?;
    let left = code[..equals].trim();
    if decode_key(left)? != key {
        return None;
    }
    let after = &code[equals + 1..];
    let leading = after.len() - after.trim_start().len();
    let raw_value = &after[leading..];
    let value_start = line.start + equals + 1 + leading;
    let value_end = value_start + raw_value.trim_end().len();
    Some(Property {
        value_start,
        value_end,
    })
}

fn update_profile_property(
    source: &str,
    lines: &[SourceLine<'_>],
    section: &Section,
    key: &str,
    desired: &[String],
) -> Result<Vec<Patch>> {
    if let Some(property) = property_in_section(lines, section, key) {
        let value = source[property.value_start..property.value_end].trim();
        if !value.starts_with('[') {
            return Err(config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                format!("`{key}` is not a directly editable string array"),
            ));
        }
        let array_offset = source[property.value_start..property.value_end]
            .find('[')
            .ok_or_else(|| {
                config_edit_error(
                    "E-CONFIG-EDIT-UNSUPPORTED",
                    format!("`{key}` array start could not be located safely"),
                )
            })?;
        let array_start = property.value_start + array_offset;
        let end = matching_array_end(source, array_start, source.len()).ok_or_else(|| {
            config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                "multiline array end could not be located safely",
            )
        })?;
        let span = ArraySpan {
            start: array_start,
            end,
        };
        let replacement = update_string_array(source, span, desired)?;
        return Ok(vec![Patch {
            start: span.start,
            end: span.end,
            replacement,
        }]);
    }

    if desired.is_empty() {
        return Ok(Vec::new());
    }
    let insertion = lines
        .get(section.end_line)
        .map_or(source.len(), |line| line.start);
    let newline = newline_for(source);
    let prefix = if insertion > 0 && !source[..insertion].ends_with('\n') {
        newline
    } else {
        ""
    };
    Ok(vec![Patch {
        start: insertion,
        end: insertion,
        replacement: format!("{prefix}{key} = {}{newline}", render_string_array(desired)),
    }])
}

#[derive(Clone, Copy)]
struct ArraySpan {
    start: usize,
    end: usize,
}

fn matching_array_end(source: &str, start: usize, limit: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut depth = 0usize;
    for (index, byte) in bytes
        .iter()
        .enumerate()
        .take(limit.min(bytes.len()))
        .skip(start)
    {
        match quote {
            Some(b'"') => {
                if *byte == b'\\' {
                    continue;
                }
                if *byte == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') if *byte == b'\'' => quote = None,
            Some(_) => {}
            None => match *byte {
                b'"' | b'\'' => quote = Some(*byte),
                b'[' => depth += 1,
                b']' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(index + 1);
                    }
                }
                _ => {}
            },
        }
    }
    None
}

fn update_string_array(source: &str, span: ArraySpan, desired: &[String]) -> Result<String> {
    let raw = &source[span.start..span.end];
    let existing = parse_string_array_values(raw)?;
    if existing == desired {
        return Ok(raw.to_owned());
    }
    if raw.contains('#') {
        return update_commented_string_array(source, span, &existing, desired);
    }

    let entries = array_entries(source, span)?;
    if desired.is_empty() {
        return Ok("[]".to_owned());
    }
    let mut used = BTreeSet::new();
    let mut rendered = Vec::with_capacity(desired.len());
    for (index, value) in desired.iter().enumerate() {
        let matched = entries
            .iter()
            .enumerate()
            .find(|(entry_index, entry)| !used.contains(entry_index) && entry.value == *value)
            .map(|(entry_index, entry)| {
                used.insert(entry_index);
                entry
            });
        if let Some(entry) = matched {
            rendered.push(source[entry.start..entry.end].to_owned());
        } else if let Some(entry) = entries.get(index).filter(|_| !used.contains(&index)) {
            used.insert(index);
            let mut replacement = source[entry.start..entry.end].to_owned();
            let relative_start = entry.token_start - entry.start;
            let relative_end = entry.token_end - entry.start;
            replacement.replace_range(relative_start..relative_end, &render_toml_string(value));
            rendered.push(replacement);
        } else {
            rendered.push(format!(" {}", render_toml_string(value)));
        }
    }
    let multiline = raw.contains('\n');
    if multiline {
        let newline = if raw.contains("\r\n") { "\r\n" } else { "\n" };
        let indent = entries
            .first()
            .map(|entry| {
                let line_start = source[..entry.token_start]
                    .rfind('\n')
                    .map_or(span.start, |index| index + 1);
                source[line_start..entry.token_start].to_owned()
            })
            .unwrap_or_else(|| "    ".to_owned());
        Ok(format!(
            "[{newline}{}{newline}]",
            rendered
                .into_iter()
                .map(|entry| format!("{indent}{}", entry.trim()))
                .collect::<Vec<_>>()
                .join(&format!(",{newline}"))
        ))
    } else {
        Ok(format!(
            "[{}]",
            rendered
                .into_iter()
                .map(|entry| entry.trim().to_owned())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn parse_string_array_values(raw: &str) -> Result<Vec<String>> {
    let value = toml::from_str::<toml::Value>(&format!("value = {raw}")).map_err(|_| {
        config_edit_error(
            "E-CONFIG-EDIT-UNSUPPORTED",
            "profile array could not be parsed as a string array",
        )
    })?;
    value
        .get("value")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                "profile property is not a TOML array",
            )
        })?
        .iter()
        .map(|entry| {
            entry.as_str().map(str::to_owned).ok_or_else(|| {
                config_edit_error(
                    "E-CONFIG-EDIT-UNSUPPORTED",
                    "profile array contains a non-string entry",
                )
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct StringToken {
    start: usize,
    end: usize,
    quote: u8,
}

fn string_tokens_in_array(source: &str, span: ArraySpan) -> Result<Vec<StringToken>> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut index = span.start + 1;
    let limit = span.end.saturating_sub(1);
    let mut in_comment = false;
    while index < limit {
        let byte = bytes[index];
        if in_comment {
            if byte == b'\n' {
                in_comment = false;
            }
            index += 1;
            continue;
        }
        if byte == b'#' {
            in_comment = true;
            index += 1;
            continue;
        }
        if byte != b'"' && byte != b'\'' {
            index += 1;
            continue;
        }
        if index + 2 < limit && bytes[index + 1] == byte && bytes[index + 2] == byte {
            return Err(config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                "multiline strings are not supported as profile tag entries",
            ));
        }
        let quote = byte;
        let token_start = index;
        index += 1;
        while index < limit {
            if quote == b'"' && bytes[index] == b'\\' {
                index = (index + 2).min(limit);
                continue;
            }
            if bytes[index] == quote {
                index += 1;
                tokens.push(StringToken {
                    start: token_start,
                    end: index,
                    quote,
                });
                break;
            }
            index += 1;
        }
        if tokens.last().is_none_or(|token| token.start != token_start) {
            return Err(config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                "profile string entry could not be located safely",
            ));
        }
    }
    Ok(tokens)
}

fn update_commented_string_array(
    source: &str,
    span: ArraySpan,
    existing: &[String],
    desired: &[String],
) -> Result<String> {
    let tokens = string_tokens_in_array(source, span)?;
    if tokens.len() != existing.len() {
        return Err(config_edit_error(
            "E-CONFIG-EDIT-UNSUPPORTED",
            "profile array token identity is ambiguous",
        ));
    }
    let mut result = source[span.start..span.end].to_owned();
    if desired.len() < existing.len() {
        let mut removals = tokens[desired.len()..]
            .iter()
            .map(|token| (token.start, token.end))
            .collect::<Vec<_>>();
        let scan_start = desired
            .last()
            .and_then(|_| tokens.get(desired.len().saturating_sub(1)))
            .map_or(span.start + 1, |token| token.end);
        removals.extend(commented_array_commas(source, scan_start, span.end - 1));
        removals.sort_unstable_by_key(|(start, _)| *start);
        for (start, end) in removals.into_iter().rev() {
            result.replace_range(start - span.start..end - span.start, "");
        }
        for index in (0..desired.len()).rev() {
            let token = &tokens[index];
            let replacement = render_toml_string_with_quote(&desired[index], token.quote);
            result.replace_range(
                token.start - span.start..token.end - span.start,
                &replacement,
            );
        }
        return Ok(result);
    }

    for index in (0..existing.len()).rev() {
        let token = &tokens[index];
        let replacement = render_toml_string_with_quote(&desired[index], token.quote);
        result.replace_range(
            token.start - span.start..token.end - span.start,
            &replacement,
        );
    }
    if desired.len() == existing.len() {
        return Ok(result);
    }

    let last = tokens.last().ok_or_else(|| {
        config_edit_error(
            "E-CONFIG-EDIT-UNSUPPORTED",
            "cannot extend a commented empty profile array safely",
        )
    })?;
    let raw = &source[span.start..span.end];
    let relative_last_end = last.end - span.start;
    let tail = &raw[relative_last_end..raw.len() - 1];
    let before_comment_or_newline = tail.split(['#', '\n', '\r']).next().unwrap_or_default();
    if !before_comment_or_newline.contains(',') {
        result.insert(relative_last_end, ',');
    }

    let close = result.rfind(']').ok_or_else(|| {
        config_edit_error(
            "E-CONFIG-EDIT-UNSUPPORTED",
            "profile array closing delimiter could not be located safely",
        )
    })?;
    let newline = if raw.contains("\r\n") { "\r\n" } else { "\n" };
    let close_line_start = result[..close].rfind('\n').map_or(0, |index| index + 1);
    let closing_indent = result[close_line_start..close].to_owned();
    let indent = format!("{closing_indent}  ");
    let addition = desired[existing.len()..]
        .iter()
        .map(|value| format!("{indent}{}", render_toml_string(value)))
        .collect::<Vec<_>>()
        .join(&format!(",{newline}"));
    result.insert_str(close_line_start, &format!("{addition}{newline}"));
    Ok(result)
}

fn commented_array_commas(source: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut index = start;
    let mut quote = None;
    let mut in_comment = false;
    let mut commas = Vec::new();
    while index < end {
        let byte = bytes[index];
        if in_comment {
            if byte == b'\n' {
                in_comment = false;
            }
            index += 1;
            continue;
        }
        match quote {
            Some(b'"') => {
                if byte == b'\\' {
                    index = (index + 2).min(end);
                    continue;
                }
                if byte == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') if byte == b'\'' => quote = None,
            Some(_) => {}
            None => match byte {
                b'#' => in_comment = true,
                b'"' | b'\'' => quote = Some(byte),
                b',' => commas.push((index, index + 1)),
                _ => {}
            },
        }
        index += 1;
    }
    commas
}

fn array_entries(source: &str, span: ArraySpan) -> Result<Vec<ArrayEntry>> {
    if source.as_bytes().get(span.start) != Some(&b'[')
        || source.as_bytes().get(span.end.saturating_sub(1)) != Some(&b']')
    {
        return Err(config_edit_error(
            "E-CONFIG-EDIT-UNSUPPORTED",
            "string array delimiters could not be located safely",
        ));
    }
    let mut result = Vec::new();
    let mut segment_start = span.start + 1;
    let mut quote = None;
    let mut index = segment_start;
    while index < span.end - 1 {
        let byte = source.as_bytes()[index];
        match quote {
            Some(b'"') => {
                if byte == b'\\' {
                    index += 2;
                    continue;
                }
                if byte == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') if byte == b'\'' => quote = None,
            Some(_) => {}
            None => match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'#' => {
                    while index < span.end - 1 && source.as_bytes()[index] != b'\n' {
                        index += 1;
                    }
                    continue;
                }
                b',' => {
                    push_array_entry(source, segment_start, index, &mut result)?;
                    segment_start = index + 1;
                }
                _ => {}
            },
        }
        index += 1;
    }
    push_array_entry(source, segment_start, span.end - 1, &mut result)?;
    Ok(result)
}

fn push_array_entry(
    source: &str,
    start: usize,
    end: usize,
    entries: &mut Vec<ArrayEntry>,
) -> Result<()> {
    let segment = &source[start..end];
    let code = strip_comment(segment);
    let leading = code.len() - code.trim_start().len();
    let token = code[leading..].trim();
    if token.is_empty() {
        return Ok(());
    }
    let token_start = start + leading;
    let token_end = token_start + token.len();
    let value = toml::from_str::<toml::Value>(&format!("value = {token}"))
        .ok()
        .and_then(|value| {
            value
                .get("value")
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            config_edit_error(
                "E-CONFIG-EDIT-UNSUPPORTED",
                "profile array contains a non-string entry",
            )
        })?;
    entries.push(ArrayEntry {
        start,
        end,
        token_start,
        token_end,
        value,
    });
    Ok(())
}

fn append_profile(
    source: &str,
    name: &str,
    include_tags: &[String],
    exclude_tags: &[String],
) -> Patch {
    let newline = newline_for(source);
    let prefix = if source.is_empty() || source.ends_with('\n') {
        "".to_owned()
    } else {
        newline.to_owned()
    };
    let mut replacement = format!("{prefix}[build.profiles.{name}]{newline}");
    if !include_tags.is_empty() {
        replacement.push_str(&format!(
            "include_tags = {}{newline}",
            render_string_array(include_tags)
        ));
    }
    if !exclude_tags.is_empty() {
        replacement.push_str(&format!(
            "exclude_tags = {}{newline}",
            render_string_array(exclude_tags)
        ));
    }
    Patch {
        start: source.len(),
        end: source.len(),
        replacement,
    }
}

fn append_publish_target(source: &str, kind: PublishTargetKind, path: &str) -> Patch {
    let newline = newline_for(source);
    let prefix = if source.is_empty() || source.ends_with('\n') {
        "".to_owned()
    } else {
        newline.to_owned()
    };
    Patch {
        start: source.len(),
        end: source.len(),
        replacement: format!(
            "{prefix}[[publish.targets]]{newline}kind = \"{}\"{newline}path = {}{newline}",
            match kind {
                PublishTargetKind::CSharp => "csharp",
                PublishTargetKind::Binary => "binary",
            },
            render_toml_string(path),
        ),
    }
}

fn update_string_property(
    source: &str,
    _lines: &[SourceLine<'_>],
    property: Property,
    desired: &str,
) -> Result<Patch> {
    let raw = source[property.value_start..property.value_end].trim();
    if !(raw.starts_with('"') || raw.starts_with('\'')) {
        return Err(config_edit_error(
            "E-CONFIG-EDIT-UNSUPPORTED",
            "target path is not a directly editable TOML string",
        ));
    }
    let leading = source[property.value_start..property.value_end].len()
        - source[property.value_start..property.value_end]
            .trim_start()
            .len();
    let token_start = property.value_start + leading;
    let token_end = token_start + raw.len();
    Ok(Patch {
        start: token_start,
        end: token_end,
        replacement: render_toml_string_with_quote(desired, raw.as_bytes()[0]),
    })
}

fn render_string_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| render_toml_string(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render_toml_string(value: &str) -> String {
    render_toml_string_with_quote(value, b'"')
}

fn render_toml_string_with_quote(value: &str, quote: u8) -> String {
    if quote == b'\'' && !value.contains('\'') {
        return format!("'{value}'");
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn strip_comment(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        match quote {
            Some(b'"') => {
                if bytes[index] == b'\\' {
                    index += 2;
                    continue;
                }
                if bytes[index] == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') if bytes[index] == b'\'' => quote = None,
            Some(_) => {}
            None => match bytes[index] {
                b'"' | b'\'' => quote = Some(bytes[index]),
                b'#' => return value[..index].to_owned(),
                _ => {}
            },
        }
        index += 1;
    }
    value.to_owned()
}

fn top_level_byte(value: &str, needle: u8) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut quote = None;
    for (index, byte) in bytes.iter().enumerate() {
        match quote {
            Some(b'"') => {
                if *byte == b'\\' {
                    continue;
                }
                if *byte == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') if *byte == b'\'' => quote = None,
            Some(_) => {}
            None if *byte == b'"' || *byte == b'\'' => quote = Some(*byte),
            None if *byte == needle => return Some(index),
            None => {}
        }
    }
    None
}

fn newline_for(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn apply_patches(source: &str, patches: &[Patch]) -> Result<String> {
    let mut sorted = patches.to_vec();
    sorted.sort_by_key(|patch| (patch.start, patch.end));
    let mut result = source.to_owned();
    let mut previous_end = source.len();
    for patch in sorted.into_iter().rev() {
        if patch.start > patch.end
            || patch.end > source.len()
            || patch.end > previous_end
            || !source.is_char_boundary(patch.start)
            || !source.is_char_boundary(patch.end)
        {
            return Err(config_edit_error(
                "E-CONFIG-EDIT-PATCH",
                "configuration edit ranges overlap or are not valid UTF-8 boundaries",
            ));
        }
        result.replace_range(patch.start..patch.end, &patch.replacement);
        previous_end = patch.start;
    }
    Ok(result)
}

fn config_edit_error(code: &str, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Config, message)
        .with_related_requirement("CONFIG-EDIT-002")
}

#[cfg(test)]
mod tests {
    use super::{ProjectConfigEditOperation, preview_project_config_edit};
    use crate::PublishTargetKind;

    const SOURCE: &str = "[project]\r\nid = \"test\"\r\nname = \"Test\" # keep\r\nversion = \"0.1.0\"\r\n\r\n[sources]\r\nroots = [\"sources\"]\r\n\r\n[build]\r\nartifact_dir = \".masterdata/output\"\r\ncache = \".masterdata/cache\"\r\n\r\n[build.profiles.prod]\r\ninclude_tags = [\"release\", 'keep'] # profile comment\r\nexclude_tags = [\r\n  \"debug\",\r\n]\r\n\r\n[[publish.targets]]\r\nkind = \"csharp\"\r\npath = \"Generated\" # target comment\r\n";

    #[test]
    fn profile_update_preserves_unrelated_bytes_and_quote_style() {
        let preview = preview_project_config_edit(
            SOURCE,
            &ProjectConfigEditOperation::UpdateProfile {
                name: "prod".into(),
                include_tags: vec!["keep".into(), "new-tag".into()],
                exclude_tags: vec!["debug".into()],
            },
        )
        .expect("profile edit");
        assert!(
            preview
                .candidate_source
                .contains("include_tags = ['keep', \"new-tag\"] # profile comment")
        );
        assert!(
            preview
                .candidate_source
                .contains("name = \"Test\" # keep\r\n")
        );
        assert!(
            preview
                .candidate_source
                .contains("exclude_tags = [\r\n  \"debug\",\r\n]")
        );
        assert!(!preview.candidate_source.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn new_profile_and_target_are_appended_without_reserializing() {
        let profile = preview_project_config_edit(
            SOURCE,
            &ProjectConfigEditOperation::AddProfile {
                name: "debug".into(),
                include_tags: vec!["debug".into()],
                exclude_tags: Vec::new(),
            },
        )
        .expect("new profile");
        assert!(
            profile
                .candidate_source
                .ends_with("include_tags = [\"debug\"]\r\n")
        );
        let target = preview_project_config_edit(
            SOURCE,
            &ProjectConfigEditOperation::AddPublishTarget {
                kind: PublishTargetKind::Binary,
                path: "data/masterdata.bytes".into(),
            },
        )
        .expect("new target");
        assert!(
            target
                .candidate_source
                .contains("kind = \"binary\"\r\npath = \"data/masterdata.bytes\"")
        );
    }

    #[test]
    fn multiline_string_header_is_not_treated_as_profile_section() {
        let source = "[unknown]\ntext = \"\"\"\n[build.profiles.prod]\ninclude_tags = [\"decoy\"]\n\"\"\"\n[build.profiles.prod]\ninclude_tags = [\"real\"]\n";
        let preview = preview_project_config_edit(
            source,
            &ProjectConfigEditOperation::UpdateProfile {
                name: "prod".into(),
                include_tags: vec!["updated".into()],
                exclude_tags: Vec::new(),
            },
        )
        .expect("profile edit");
        assert!(
            preview
                .candidate_source
                .contains("include_tags = [\"decoy\"]")
        );
        assert!(
            preview
                .candidate_source
                .contains("include_tags = [\"updated\"]")
        );
    }

    #[test]
    fn commented_multiline_array_noop_preserves_exact_bytes_and_extension_keeps_comment() {
        let source =
            "[build.profiles.prod]\ninclude_tags = [\n  \"a\", # keep a\n  \"b\", # keep b\n]\n";
        let noop = preview_project_config_edit(
            source,
            &ProjectConfigEditOperation::UpdateProfile {
                name: "prod".into(),
                include_tags: vec!["a".into(), "b".into()],
                exclude_tags: Vec::new(),
            },
        )
        .expect("noop");
        assert_eq!(noop.candidate_source, source);
        assert!(!noop.changed);

        let single = "[build.profiles.prod]\ninclude_tags = [\"a\" # keep a\n]\n";
        let extended = preview_project_config_edit(
            single,
            &ProjectConfigEditOperation::UpdateProfile {
                name: "prod".into(),
                include_tags: vec!["a".into(), "b".into()],
                exclude_tags: Vec::new(),
            },
        )
        .expect("extend");
        assert!(extended.candidate_source.contains("\"a\", # keep a"));
        assert!(extended.candidate_source.contains("\"b\""));
        assert!(toml::from_str::<toml::Value>(&extended.candidate_source).is_ok());
    }

    #[test]
    fn commented_array_removal_preserves_comments_and_valid_toml() {
        let source = "[build.profiles.prod]\ninclude_tags = [\n  \"a\", # keep a\n  \"b\", # keep b\n  \"c\" # keep c\n]\n";
        let preview = preview_project_config_edit(
            source,
            &ProjectConfigEditOperation::UpdateProfile {
                name: "prod".into(),
                include_tags: vec!["a".into()],
                exclude_tags: Vec::new(),
            },
        )
        .expect("remove entries");
        assert!(preview.candidate_source.contains("# keep a"));
        assert!(preview.candidate_source.contains("# keep b"));
        assert!(preview.candidate_source.contains("# keep c"));
        assert!(!preview.candidate_source.contains("\"b\""));
        assert!(!preview.candidate_source.contains("\"c\""));
        let parsed = toml::from_str::<toml::Value>(&preview.candidate_source).expect("valid TOML");
        let tags = parsed["build"]["profiles"]["prod"]["include_tags"]
            .as_array()
            .expect("tags");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), Some("a"));
    }

    #[test]
    fn target_path_update_preserves_inline_comment_and_uses_occurrence() {
        let preview = preview_project_config_edit(
            SOURCE,
            &ProjectConfigEditOperation::UpdatePublishTargetPath {
                index: 0,
                path: "Updated/Generated".into(),
            },
        )
        .expect("target update");
        assert!(
            preview
                .candidate_source
                .contains("path = \"Updated/Generated\" # target comment")
        );
    }
}
