use super::*;
use crate::dry_run_view_edit;

pub(super) fn prepare(
    documents: &ProjectDocuments,
    table: &str,
    field: &str,
    rename: Option<&str>,
) -> Result<MigrationDryRun> {
    let original = find_target_schema(documents, table)
        .and_then(|schema| {
            schema
                .document
                .fields
                .iter()
                .find(|candidate| candidate.name == field)
        })
        .cloned()
        .ok_or_else(|| failure("target Table/field does not exist"))?;
    if validate_computed_view_dependencies(documents, table).is_err()
        || reference_depends_on_field(documents, table, field)
        || (rename.is_none() && view_depends_on_field(documents, table, field))
    {
        return Err(failure(
            "field is used by a Reference or Computed View source/target; safe automatic rewrite is not available",
        ));
    }
    let (closure, _) = resolve_target_snapshot(documents, table, &original)?;
    let schema = find_target_schema(&closure, table)
        .expect("resolved schema")
        .document;
    if rename.is_none()
        && (schema
            .primary_key
            .as_ref()
            .is_some_and(|key| key.fields.iter().any(|name| name == field))
            || schema
                .secondary_keys
                .iter()
                .any(|key| key.fields.iter().any(|name| name == field)))
    {
        return Err(failure(
            "DropField target is referenced by a Primary or Secondary Key",
        ));
    }
    let mut expected = documents.clone();
    let mut plans = Vec::new();
    let mut file_indices = (0..expected.files.len()).collect::<Vec<_>>();
    // A dependent View is validated against the transformed Table semantics.
    // Process schema/data documents first so a rename never depends on source
    // discovery order when the View file sorts before its target schema.
    file_indices
        .sort_by_key(|&index| matches!(expected.files[index].document, SourceDocument::View(_)));
    for file_index in file_indices {
        let path = expected.files[file_index].path.clone();
        let view_patch = if rename.is_some()
            && let SourceDocument::View(view) = &expected.files[file_index].document
            && view.table == table
        {
            let mut desired = view.clone();
            let mut changed = false;
            for column in &mut desired.columns {
                if let Some(expression) = crate::rename_field_references(
                    &column.expression,
                    field,
                    rename.expect("rename checked"),
                ) {
                    column.expression = expression;
                    changed = true;
                }
            }
            if changed {
                let view_dry_run = dry_run_view_edit(&expected, &path, &desired)?;
                expected.files[file_index].document = SourceDocument::View(desired);
                Some(
                    view_dry_run.source_candidate.affected_files[0]
                        .patches
                        .clone(),
                )
            } else {
                Some(Vec::new())
            }
        } else {
            None
        };
        let patches = if let Some(patches) = view_patch {
            patches
        } else {
            let loaded = &mut expected.files[file_index];
            match &mut loaded.document {
                SourceDocument::Schema(schema) if schema.table == table => {
                    let index = schema
                        .fields
                        .iter()
                        .position(|candidate| candidate.name == field)
                        .expect("resolved field");
                    let patches =
                        schema_patches(&loaded.source, index, schema.fields.len(), field, rename)?;
                    if let Some(name) = rename {
                        schema.fields[index].name = name.into();
                        if let Some(key) = &mut schema.primary_key {
                            replace_names(&mut key.fields, field, name);
                        }
                        for key in &mut schema.secondary_keys {
                            replace_names(&mut key.fields, field, name);
                        }
                    } else {
                        schema.fields.remove(index);
                    }
                    patches
                }
                SourceDocument::Data(data) if data.table == table && !data.records.is_empty() => {
                    for record in &mut data.records {
                        let value = record
                            .remove(field)
                            .ok_or_else(|| failure("target record member is missing"))?;
                        if let Some(name) = rename {
                            if record.contains_key(name) {
                                return Err(failure("renamed record member already exists"));
                            }
                            record.insert(name.into(), value);
                        }
                    }
                    data_patches(&loaded.source, data.records.len(), field, rename)?
                }
                _ => Vec::new(),
            }
        };
        if !patches.is_empty() {
            plans.push(MigrationFilePlan { path, patches });
        }
    }
    // Validate the transformed closure rather than the removed field's old type.
    // Compare all semantic documents independently of that closure so an omitted
    // dependency cannot hide an unintended source change (MIGRATION-015/017).
    let probe = FieldDefinition {
        key: 0,
        name: "probe".into(),
        type_name: "int".into(),
        nullable: false,
        array: false,
    };
    resolve_target_snapshot(&expected, table, &probe)?;
    plans.sort_by(|a, b| a.path.cmp(&b.path));
    let transformed = apply_file_plans(documents, &plans)?;
    resolve_target_snapshot(&transformed, table, &probe)?;
    if semantic_documents(&transformed) != semantic_documents(&expected) {
        return Err(failure("field transformation postcondition failed"));
    }
    Ok(MigrationDryRun {
        plan: MigrationPlan {
            operation: if rename.is_some() {
                MigrationOperation::RenameField
            } else {
                MigrationOperation::DropField
            },
            target_table: table.into(),
            field: original,
            reference: None,
            initializer: None,
            source_inputs: documents
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            destructive: rename.is_none(),
            affected_files: plans,
            affected_record_count: target_record_count(documents, table),
            validation: MigrationValidation {
                valid: true,
                diagnostics: Vec::new(),
            },
        },
        transformed_documents: transformed,
    })
}
fn replace_names(names: &mut [String], old: &str, new: &str) {
    for name in names {
        if name == old {
            *name = new.into();
        }
    }
}

fn reference_depends_on_field(documents: &ProjectDocuments, table: &str, field: &str) -> bool {
    documents.schemas().any(|(_, schema)| {
        schema.table == table
            && schema
                .references
                .iter()
                .any(|reference| reference.fields.iter().any(|name| name == field))
            || schema.references.iter().any(|reference| {
                reference.target.table == table
                    && reference.target.fields.iter().any(|name| name == field)
            })
    })
}

fn view_depends_on_field(documents: &ProjectDocuments, table: &str, field: &str) -> bool {
    documents.views().any(|(_, view)| {
        view.table == table
            && view
                .columns
                .iter()
                .any(|column| !crate::field_reference_spans(&column.expression, field).is_empty())
    })
}

fn failure(message: &str) -> MasterdataError {
    migration_error(
        "E-MIGRATION-FIELD-PRECONDITION",
        message,
        None,
        "MIGRATION-015",
    )
}
fn sequence<'a>(
    source: &'a str,
    key: &str,
    count: usize,
) -> Result<(Vec<SourceLine<'a>>, SequenceRegion, usize)> {
    let lines = source_lines(source);
    let header = find_top_level_key(&lines, key).ok_or_else(|| failure("sequence not found"))?;
    let end = block_region_end(&lines, header);
    let region = find_block_sequence(&lines, header, end)
        .ok_or_else(|| failure("sequence source cannot be located"))?;
    if region.items.len() != count {
        return Err(failure("sequence source/semantic count mismatch"));
    }
    Ok((lines, region, end))
}
fn schema_patches(
    source: &str,
    index: usize,
    count: usize,
    field: &str,
    rename: Option<&str>,
) -> Result<Vec<MigrationPatch>> {
    let (lines, region, end) = sequence(source, "fields", count)?;
    let start = region.items[index];
    let end = region.items.get(index + 1).copied().unwrap_or(end);
    let mut patches = if let Some(name) = rename {
        let line = (start..end)
            .find(|&i| mapping_entry(lines[i].text).is_some_and(|entry| entry.key == "name"))
            .ok_or_else(|| failure("field name source not found"))?;
        vec![scalar_value_patch(&lines[line], name)?]
    } else {
        remove_lines(source, &lines, start, end, false)?
    };
    if let Some(name) = rename {
        for key in ["primaryKey", "secondaryKeys"] {
            if let Some(start) = find_top_level_key(&lines, key) {
                let end = block_region_end(&lines, start);
                patches.extend(key_reference_patches(
                    source, &lines, start, end, field, name,
                )?);
            }
        }
    }
    Ok(patches)
}
fn scalar_value_patch(line: &SourceLine<'_>, new: &str) -> Result<MigrationPatch> {
    let (_, rest) = sequence_item_parts(line.text).unwrap_or((false, line.text.trim_start()));
    let colon = mapping_colon(rest).ok_or_else(|| failure("mapping colon missing"))?;
    let raw = strip_yaml_comment(&rest[colon + 1..]).trim();
    let start = line.start
        + (rest.as_ptr() as usize - line.text.as_ptr() as usize)
        + colon
        + 1
        + rest[colon + 1..].len()
        - rest[colon + 1..].trim_start().len();
    Ok(MigrationPatch {
        start,
        end: start + raw.len(),
        replacement: quoted_like(raw, new),
    })
}
fn quoted_like(raw: &str, new: &str) -> String {
    if raw.starts_with('\'') {
        format!("'{}'", new.replace('\'', "''"))
    } else if raw.starts_with('"') {
        serde_json::to_string(new).expect("string")
    } else {
        new.into()
    }
}
fn data_patches(
    source: &str,
    count: usize,
    field: &str,
    rename: Option<&str>,
) -> Result<Vec<MigrationPatch>> {
    let (lines, region, end) = sequence(source, "records", count)?;
    let literal = literal_block_scalar_content_lines(&lines);
    let indent = record_member_indent(&lines, &region)?;
    let mut patches = Vec::new();
    for (index, &start) in region.items.iter().enumerate() {
        let end = region.items.get(index + 1).copied().unwrap_or(end);
        let members = (start..end)
            .filter(|&i| {
                !literal[i]
                    && (i == start || yaml_indent(lines[i].text) == indent)
                    && mapping_entry(lines[i].text).is_some()
            })
            .collect::<Vec<_>>();
        let pos = members
            .iter()
            .position(|&i| mapping_entry(lines[i].text).is_some_and(|entry| entry.key == field))
            .ok_or_else(|| failure("record field source missing"))?;
        let line = members[pos];
        if let Some(name) = rename {
            let (_, rest) = sequence_item_parts(lines[line].text)
                .unwrap_or((false, lines[line].text.trim_start()));
            let colon = mapping_colon(rest).ok_or_else(|| failure("member colon missing"))?;
            let raw = rest[..colon].trim_end();
            let offset =
                lines[line].start + (rest.as_ptr() as usize - lines[line].text.as_ptr() as usize);
            patches.push(MigrationPatch {
                start: offset,
                end: offset + raw.len(),
                replacement: quoted_like(raw, name),
            });
        } else {
            if members.len() == 1 {
                return Err(failure(
                    "DropField would leave a record without an explicit mapping",
                ));
            }
            let finish = members.get(pos + 1).copied().unwrap_or(end);
            patches.extend(remove_lines(source, &lines, line, finish, line == start)?);
            if line == start {
                let next = members[1];
                patches.push(MigrationPatch {
                    start: lines[next].start,
                    end: lines[next].start + indent,
                    replacement: format!("{}- ", spaces(region.indent)),
                });
            }
        }
    }
    Ok(patches)
}
// Remove only semantic lines. Standalone comments/blank lines remain exact;
// inline comments become standalone comments instead of being discarded.
// Literal scalar content is data, including lines that look like comments.
// EVIDENCE: MIGRATION-014; docs/specs/schema-migration.md.
fn remove_lines(
    source: &str,
    lines: &[SourceLine<'_>],
    start: usize,
    end: usize,
    _first: bool,
) -> Result<Vec<MigrationPatch>> {
    let literal = literal_block_scalar_content_lines(lines);
    let mut patches = Vec::new();
    for i in start..end {
        if !literal[i] && is_ignorable_line(lines[i].text) {
            continue;
        }
        let replacement = if !literal[i] {
            comment_start(lines[i].text)
                .map(|at| {
                    format!(
                        "{}{}{}",
                        spaces(yaml_indent(lines[i].text)),
                        &lines[i].text[at..],
                        if lines[i].next_start > lines[i].content_end {
                            newline_for(source)
                        } else {
                            ""
                        }
                    )
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        patches.push(MigrationPatch {
            start: lines[i].start,
            end: lines[i].next_start,
            replacement,
        });
    }
    Ok(patches)
}
fn key_reference_patches(
    source: &str,
    lines: &[SourceLine<'_>],
    start: usize,
    end: usize,
    old: &str,
    new: &str,
) -> Result<Vec<MigrationPatch>> {
    let mut result = Vec::new();
    let mut flow = false;
    for line in &lines[start..end] {
        let code = strip_yaml_comment(line.text);
        if is_ignorable_line(code) {
            continue;
        }
        let bytes = code.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'[' {
                flow = true;
                i += 1;
                continue;
            }
            if bytes[i] == b']' {
                flow = false;
                i += 1;
                continue;
            }
            if flow && !bytes[i].is_ascii_whitespace() && bytes[i] != b',' {
                let begin = i;
                if bytes[i] == b'\'' || bytes[i] == b'"' {
                    let quote = bytes[i];
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == quote {
                            i += 1;
                            if quote == b'\'' && i < bytes.len() && bytes[i] == quote {
                                i += 1;
                                continue;
                            }
                            break;
                        }
                        if quote == b'"' && bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                } else {
                    while i < bytes.len() && !matches!(bytes[i], b',' | b']') {
                        i += 1;
                    }
                }
                let raw = code[begin..i].trim_end();
                if decode_mapping_key(raw).as_deref() == Some(old) {
                    result.push(MigrationPatch {
                        start: line.start + begin,
                        end: line.start + begin + raw.len(),
                        replacement: quoted_like(raw, new),
                    });
                }
                continue;
            }
            i += 1;
        }
        if !flow
            && let Some((true, rest)) = sequence_item_parts(code)
            && mapping_colon(rest).is_none()
            && decode_mapping_key(rest.trim()).as_deref() == Some(old)
        {
            let offset = line.start + (rest.as_ptr() as usize - line.text.as_ptr() as usize);
            result.push(MigrationPatch {
                start: offset,
                end: offset + rest.trim_end().len(),
                replacement: quoted_like(rest.trim(), new),
            });
        }
    }
    let _ = source;
    Ok(result)
}
