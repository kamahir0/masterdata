//! Source-preserving authoring operations for persisted Computed Views.
//!
//! The semantic model is owned by `computed_view`; this module only locates
//! the small scalar spans that an editor is allowed to change.  It refuses to
//! guess when a YAML layout cannot be localized safely.

use std::path::{Path, PathBuf};

use crate::computed_view::validate_computed_views;
use crate::document::{ProjectDocuments, SourceDocument, ViewDocument, parse_yaml_document};
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::migration::{MigrationFilePlan, MigrationPatch};
use crate::migration_commit::SourceCommitCandidate;
use crate::source_edit::source_content_identity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewEditPlan {
    pub path: PathBuf,
    pub base_content_identity: String,
    pub candidate_source: String,
    pub candidate_content_identity: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewEditDryRun {
    pub plan: ViewEditPlan,
    pub transformed_documents: ProjectDocuments,
    pub source_candidate: SourceCommitCandidate,
}

/// Plan a View edit against an exact loaded source snapshot.
pub fn dry_run_view_edit(
    documents: &ProjectDocuments,
    path: &Path,
    desired: &ViewDocument,
) -> Result<ViewEditDryRun> {
    let loaded = documents
        .files
        .iter()
        .find(|loaded| loaded.path == path)
        .ok_or_else(|| {
            view_edit_error(
                "E-VIEW-EDIT-FILE-NOT-FOUND",
                "View source is not in the loaded snapshot",
                path,
            )
        })?;
    let SourceDocument::View(current) = &loaded.document else {
        return Err(view_edit_error(
            "E-VIEW-EDIT-NOT-VIEW",
            "source edit target is not a Computed View document",
            path,
        ));
    };
    let patches = view_patches(&loaded.source, current, desired, path)?;
    let candidate_source = apply_patches(&loaded.source, &patches, path)?;
    let reparsed = parse_yaml_document(path.to_path_buf(), &candidate_source).map_err(|error| {
        view_edit_error(
            "E-VIEW-EDIT-POSTCONDITION",
            format!(
                "View candidate no longer parses: {}",
                error.diagnostic().message
            ),
            path,
        )
    })?;
    if reparsed.document != SourceDocument::View(desired.clone()) {
        return Err(view_edit_error(
            "E-VIEW-EDIT-POSTCONDITION",
            "source-preserving View candidate does not match the requested definition",
            path,
        ));
    }
    let mut transformed_documents = documents.clone();
    let index = transformed_documents
        .files
        .iter()
        .position(|item| item.path == path)
        .expect("loaded View path is present");
    transformed_documents.files[index] = reparsed;
    if let Some(diagnostic) = validate_computed_views(&transformed_documents)
        .into_iter()
        .find(|diagnostic| diagnostic.source.as_deref() == Some(path))
    {
        return Err(MasterdataError {
            diagnostic: Box::new(diagnostic),
        });
    }
    let mut source_inputs = documents
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    source_inputs.sort();
    let source_candidate = SourceCommitCandidate {
        source_inputs,
        destructive: false,
        affected_files: vec![MigrationFilePlan {
            path: path.to_path_buf(),
            patches,
        }],
        transformed_documents: transformed_documents.clone(),
    };
    Ok(ViewEditDryRun {
        plan: ViewEditPlan {
            path: path.to_path_buf(),
            base_content_identity: source_content_identity(&loaded.source),
            candidate_content_identity: source_content_identity(&candidate_source),
            changed: candidate_source != loaded.source,
            candidate_source,
        },
        transformed_documents,
        source_candidate,
    })
}

#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

fn view_patches(
    source: &str,
    current: &ViewDocument,
    desired: &ViewDocument,
    path: &Path,
) -> Result<Vec<MigrationPatch>> {
    if desired.kind != "view" {
        return Err(view_edit_error(
            "E-VIEW-EDIT-KIND",
            "View kind cannot be changed",
            path,
        ));
    }
    let lines = lines(source);
    let mut patches = Vec::new();
    if current.name != desired.name {
        patches.push(top_level_scalar_patch(
            source,
            &lines,
            "name",
            &desired.name,
            path,
        )?);
    }
    if current.table != desired.table {
        patches.push(top_level_scalar_patch(
            source,
            &lines,
            "table",
            &desired.table,
            path,
        )?);
    }
    let (header, region_end, item_starts) = columns_region(&lines, path)?;
    if item_starts.len() > current.columns.len() || item_starts.len() < current.columns.len() {
        return Err(view_edit_error(
            "E-VIEW-EDIT-POSTCONDITION",
            "persisted View column count differs from its typed document",
            path,
        ));
    }
    let item_indent = item_starts
        .first()
        .map(|&index| indentation(lines[index].text))
        .unwrap_or(indentation(lines[header].text) + 2);
    let existing_ranges = item_starts
        .iter()
        .enumerate()
        .map(|(index, &start)| {
            let end = item_starts.get(index + 1).copied().unwrap_or(region_end);
            (start, end)
        })
        .collect::<Vec<_>>();
    let common_columns = desired.columns.len().min(current.columns.len());
    for (index, &start) in item_starts.iter().enumerate().take(common_columns) {
        let end = item_starts.get(index + 1).copied().unwrap_or(region_end);
        let name_line = mapping_line(&lines, start, end, "name");
        let expression_line = mapping_line(&lines, start, end, "expression");
        let Some(name_line) = name_line else {
            return Err(view_edit_error(
                "E-VIEW-EDIT-UNLOCATABLE",
                "View column name cannot be localized safely",
                path,
            ));
        };
        let Some(expression_line) = expression_line else {
            return Err(view_edit_error(
                "E-VIEW-EDIT-UNLOCATABLE",
                "View expression cannot be localized safely",
                path,
            ));
        };
        if current.columns[index].name != desired.columns[index].name {
            patches.push(scalar_patch(
                source,
                &lines[name_line],
                "name",
                &desired.columns[index].name,
                path,
            )?);
        }
        if current.columns[index].expression != desired.columns[index].expression {
            patches.push(scalar_patch(
                source,
                &lines[expression_line],
                "expression",
                &desired.columns[index].expression,
                path,
            )?);
        }
    }
    if desired.columns.len() < current.columns.len() {
        for index in (desired.columns.len()..current.columns.len()).rev() {
            let (start, end) = existing_ranges[index];
            if lines[start..end]
                .iter()
                .any(|line| line.text.trim().is_empty() || line.text.trim_start().starts_with('#'))
            {
                return Err(view_edit_error(
                    "E-VIEW-EDIT-UNLOCATABLE",
                    "removing a View column with attached comments or blank lines is refused",
                    path,
                ));
            }
            patches.push(MigrationPatch {
                start: lines[start].start,
                end: lines[end - 1].end,
                replacement: String::new(),
            });
        }
    } else if desired.columns.len() > current.columns.len() {
        let newline = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut replacement = String::new();
        for column in &desired.columns[current.columns.len()..] {
            replacement.push_str(&format!(
                "{}- name: {}{}{}expression: {}{}",
                spaces(item_indent),
                yaml_scalar(&column.name),
                newline,
                spaces(item_indent + 2),
                yaml_scalar(&column.expression),
                newline,
            ));
        }
        if insertion_at_end_without_newline(source, region_end, lines.len()) {
            replacement.insert_str(0, newline);
        }
        let insertion = if region_end == lines.len() {
            source.len()
        } else {
            lines[region_end].start
        };
        patches.push(MigrationPatch {
            start: insertion,
            end: insertion,
            replacement,
        });
    }
    Ok(patches)
}

fn columns_region<'a>(lines: &[Line<'a>], path: &Path) -> Result<(usize, usize, Vec<usize>)> {
    let header = lines
        .iter()
        .position(|line| top_level_key(line.text) == Some("columns"))
        .ok_or_else(|| {
            view_edit_error(
                "E-VIEW-EDIT-UNLOCATABLE",
                "View columns sequence is missing",
                path,
            )
        })?;
    let end = lines
        .iter()
        .enumerate()
        .skip(header + 1)
        .find(|(_, line)| top_level_key(line.text).is_some())
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    let starts = (header + 1..end)
        .filter(|&index| {
            let text = lines[index].text.trim_start();
            indentation(lines[index].text) > indentation(lines[header].text)
                && text.starts_with("-")
                && text
                    .as_bytes()
                    .get(1)
                    .is_some_and(|byte| byte.is_ascii_whitespace())
        })
        .collect::<Vec<_>>();
    Ok((header, end, starts))
}

fn mapping_line(lines: &[Line<'_>], start: usize, end: usize, key: &str) -> Option<usize> {
    (start..end).find(|&index| {
        let text = lines[index].text.trim_start();
        let text = text.strip_prefix("- ").unwrap_or(text);
        text.strip_prefix(key)
            .is_some_and(|rest| rest.starts_with(':') || rest.starts_with(char::is_whitespace))
    })
}

fn top_level_scalar_patch(
    source: &str,
    lines: &[Line<'_>],
    key: &str,
    value: &str,
    path: &Path,
) -> Result<MigrationPatch> {
    let line = lines
        .iter()
        .find(|line| top_level_key(line.text) == Some(key))
        .ok_or_else(|| {
            view_edit_error(
                "E-VIEW-EDIT-UNLOCATABLE",
                format!("View `{key}` is missing"),
                path,
            )
        })?;
    scalar_patch(source, line, key, value, path)
}

fn scalar_patch(
    source: &str,
    line: &Line<'_>,
    key: &str,
    value: &str,
    path: &Path,
) -> Result<MigrationPatch> {
    let text = line.text;
    let trimmed = text.trim_start();
    let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let colon = trimmed.find(':').ok_or_else(|| {
        view_edit_error(
            "E-VIEW-EDIT-UNLOCATABLE",
            format!("View `{key}` has no scalar separator"),
            path,
        )
    })?;
    if trimmed[..colon].trim() != key {
        return Err(view_edit_error(
            "E-VIEW-EDIT-UNLOCATABLE",
            format!("View `{key}` is not a scalar mapping"),
            path,
        ));
    }
    let value_start_in_trimmed =
        colon + 1 + trimmed[colon + 1..].len() - trimmed[colon + 1..].trim_start().len();
    let raw = trimmed[value_start_in_trimmed..].trim_end();
    if raw.is_empty() || raw == "|" || raw == ">" || raw.starts_with('|') || raw.starts_with('>') {
        return Err(view_edit_error(
            "E-VIEW-EDIT-UNLOCATABLE",
            format!("View `{key}` is not a single-line scalar"),
            path,
        ));
    }
    let prefix_offset = line.text.len() - trimmed.len();
    let raw_value = scalar_value_without_comment(raw);
    let start = line.start + prefix_offset + value_start_in_trimmed;
    let end = start + raw_value.len();
    if end > source.len() || !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err(view_edit_error(
            "E-VIEW-EDIT-UNLOCATABLE",
            format!("View `{key}` has invalid UTF-8 span"),
            path,
        ));
    }
    Ok(MigrationPatch {
        start,
        end,
        replacement: yaml_scalar_like(raw_value, value),
    })
}

fn scalar_value_without_comment(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
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
            Some(b'\'') => {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                    continue;
                }
                if bytes[index] == b'\'' {
                    quote = None;
                }
            }
            None => {
                if bytes[index] == b'"' || bytes[index] == b'\'' {
                    quote = Some(bytes[index]);
                } else if bytes[index] == b'#'
                    && (index == 0 || bytes[index - 1].is_ascii_whitespace())
                {
                    return raw[..index].trim_end();
                }
            }
            _ => {}
        }
        index += 1;
    }
    raw
}

fn lines(source: &str) -> Vec<Line<'_>> {
    let mut result = Vec::new();
    let mut start = 0;
    while start < source.len() {
        let next = source[start..]
            .find('\n')
            .map(|offset| start + offset + 1)
            .unwrap_or(source.len());
        let end = if source[start..next].ends_with('\n') {
            next - 1
        } else {
            next
        };
        let end = if end > start && source.as_bytes()[end - 1] == b'\r' {
            end - 1
        } else {
            end
        };
        result.push(Line {
            start,
            end,
            text: &source[start..end],
        });
        start = next;
    }
    if source.is_empty() {
        result.push(Line {
            start: 0,
            end: 0,
            text: "",
        });
    }
    result
}

fn indentation(text: &str) -> usize {
    text.bytes().take_while(|byte| *byte == b' ').count()
}

fn top_level_key(text: &str) -> Option<&str> {
    if indentation(text) != 0 || text.trim().is_empty() || text.trim_start().starts_with('#') {
        return None;
    }
    text.split_once(':')
        .map(|(key, _)| key.trim())
        .filter(|key| !key.is_empty())
}

fn yaml_scalar(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        && !value.is_empty()
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn yaml_scalar_like(raw: &str, value: &str) -> String {
    if raw.starts_with('"') {
        serde_json::to_string(value).expect("JSON string serialization cannot fail")
    } else if raw.starts_with('\'') {
        format!("'{}'", value.replace('\'', "''"))
    } else {
        yaml_scalar(value)
    }
}

fn spaces(count: usize) -> String {
    " ".repeat(count)
}

fn insertion_at_end_without_newline(source: &str, region_end: usize, line_count: usize) -> bool {
    region_end == line_count
        && !source.is_empty()
        && !source.ends_with('\n')
        && !source.ends_with('\r')
}

fn apply_patches(source: &str, patches: &[MigrationPatch], path: &Path) -> Result<String> {
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
            || patch.end > result.len()
            || patch.end > previous_start
            || !source.is_char_boundary(patch.start)
            || !source.is_char_boundary(patch.end)
        {
            return Err(view_edit_error(
                "E-VIEW-EDIT-PATCH",
                "View edit spans overlap or are invalid",
                path,
            ));
        }
        result.replace_range(patch.start..patch.end, &patch.replacement);
        previous_start = patch.start;
    }
    Ok(result)
}

fn view_edit_error(code: &str, message: impl Into<String>, path: &Path) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement("ADV-VIEW-005")
}

#[cfg(test)]
mod tests {
    use super::dry_run_view_edit;
    use crate::{
        ProjectDocuments, SourceDocument, ViewColumnDefinition, ViewDocument, parse_yaml_document,
    };
    use std::path::PathBuf;

    fn documents(source: &str) -> ProjectDocuments {
        ProjectDocuments {
            files: vec![
                parse_yaml_document(
                    PathBuf::from("schemas/item.yaml"),
                    "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: name\n    type: string\n",
                )
                .unwrap(),
                parse_yaml_document(PathBuf::from("views/item.yaml"), source).unwrap(),
            ],
        }
    }

    #[test]
    fn view_edit_patches_expression_without_reformatting_unrelated_source() {
        let source = "kind: view\nname: itemDisplay\ntable: item\ncolumns:\n  - name: label\n    expression: name + \"!\"\n# keep\n";
        let desired = ViewDocument {
            kind: "view".into(),
            name: "itemDisplay".into(),
            table: "item".into(),
            columns: vec![ViewColumnDefinition {
                name: "label".into(),
                expression: "name + \"?\"".into(),
            }],
        };
        let result = dry_run_view_edit(
            &documents(source),
            PathBuf::from("views/item.yaml").as_path(),
            &desired,
        )
        .unwrap();
        assert!(
            result
                .plan
                .candidate_source
                .contains("expression: 'name + \"?\"'")
        );
        assert!(result.plan.candidate_source.contains("# keep"));
        assert!(matches!(
            result.transformed_documents.files[1].document,
            SourceDocument::View(_)
        ));
    }

    #[test]
    fn view_edit_adds_and_removes_columns_without_reformatting_existing_source() {
        let source = "kind: view\nname: itemDisplay\ntable: item\ncolumns:\n  - name: label\n    expression: name\n";
        let desired = ViewDocument {
            kind: "view".into(),
            name: "itemDisplay".into(),
            table: "item".into(),
            columns: vec![
                ViewColumnDefinition {
                    name: "label".into(),
                    expression: "name".into(),
                },
                ViewColumnDefinition {
                    name: "suffix".into(),
                    expression: "\"!\"".into(),
                },
            ],
        };
        let added = dry_run_view_edit(
            &documents(source),
            PathBuf::from("views/item.yaml").as_path(),
            &desired,
        )
        .expect("column add");
        assert!(added.plan.candidate_source.contains("name: suffix"));
        assert_eq!(
            added.transformed_documents.files[1].document,
            SourceDocument::View(desired.clone())
        );

        let removed = dry_run_view_edit(
            &documents(&added.plan.candidate_source),
            PathBuf::from("views/item.yaml").as_path(),
            &ViewDocument {
                columns: vec![desired.columns[0].clone()],
                ..desired
            },
        )
        .expect("column remove");
        assert!(!removed.plan.candidate_source.contains("name: suffix"));
        assert!(removed.plan.candidate_source.contains("expression: name"));
    }

    #[test]
    fn view_edit_preserves_inline_comments_and_adds_a_separator_without_final_newline() {
        let source = "kind: view\nname: itemDisplay # view name\ntable: item\ncolumns:\n  - name: label\n    expression: name # expression note";
        let desired = ViewDocument {
            kind: "view".into(),
            name: "itemLabels".into(),
            table: "item".into(),
            columns: vec![
                ViewColumnDefinition {
                    name: "label".into(),
                    expression: "name + \"!\"".into(),
                },
                ViewColumnDefinition {
                    name: "suffix".into(),
                    expression: "\"?\"".into(),
                },
            ],
        };
        let result = dry_run_view_edit(
            &documents(source),
            PathBuf::from("views/item.yaml").as_path(),
            &desired,
        )
        .expect("comments and final newline are safe to preserve");
        assert!(
            result
                .plan
                .candidate_source
                .contains("name: itemLabels # view name")
        );
        assert!(
            result
                .plan
                .candidate_source
                .contains("expression: 'name + \"!\"' # expression note")
        );
        assert!(result.plan.candidate_source.contains("\n  - name: suffix"));
        assert_eq!(
            result.transformed_documents.files[1].document,
            SourceDocument::View(desired)
        );
    }
}
