use super::*;

fn failure(code: &str, message: impl Into<String>, path: Option<PathBuf>) -> MasterdataError {
    migration_error(code, message, path, "FIELD-DECL-001")
}

pub(super) fn prepare(
    documents: &ProjectDocuments,
    command: &ChangeFieldDeclarationCommand,
) -> Result<MigrationDryRun> {
    let schema_loaded = find_target_schema(documents, &command.table).ok_or_else(|| {
        failure(
            "E-FIELD-DECL-TABLE",
            format!("Table `{}` has no schema", command.table),
            None,
        )
    })?;
    let old = schema_loaded
        .document
        .fields
        .iter()
        .find(|field| field.name == command.field)
        .ok_or_else(|| {
            failure(
                "E-FIELD-DECL-FIELD",
                "field does not exist",
                Some(schema_loaded.path.to_path_buf()),
            )
        })?
        .clone();
    let next = &command.declaration;
    if old.key != next.key || old.name != next.name {
        return Err(failure(
            "E-FIELD-DECL-IDENTITY",
            "type/modifier change cannot alter MessagePack key or field name",
            Some(schema_loaded.path.to_path_buf()),
        ));
    }
    if &old == next {
        return Err(failure(
            "E-FIELD-DECL-NO-CHANGE",
            "field declaration is unchanged",
            Some(schema_loaded.path.to_path_buf()),
        ));
    }
    if schema_loaded
        .document
        .primary_key
        .as_ref()
        .is_some_and(|key| key.fields.contains(&old.name))
        || schema_loaded
            .document
            .secondary_keys
            .iter()
            .any(|key| key.fields.contains(&old.name))
    {
        return Err(failure(
            "E-FIELD-DECL-KEY-DEPENDENCY",
            format!("field `{}` is used by a key", old.name),
            Some(schema_loaded.path.to_path_buf()),
        ));
    }
    let references = field_mutation::reference_dependencies(documents, &command.table, &old.name);
    if !references.is_empty() {
        let names = references
            .iter()
            .map(|(table, name)| format!("{table}.{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(failure(
            "E-FIELD-DECL-REFERENCE-DEPENDENCY",
            format!("field `{}` is used by Reference {names}", old.name),
            Some(schema_loaded.path.to_path_buf()),
        ));
    }

    let mut expected = documents.clone();
    let expected_schema = expected
        .files
        .iter_mut()
        .find(|loaded| {
            matches!(&loaded.document, SourceDocument::Schema(schema) if schema.table == command.table)
        })
        .expect("resolved target schema");
    let SourceDocument::Schema(schema) = &mut expected_schema.document else {
        unreachable!("target schema changed kind")
    };
    let field_index = schema
        .fields
        .iter()
        .position(|field| field.name == command.field)
        .expect("resolved target field");
    schema.fields[field_index] = next.clone();
    let (expected_closure, type_system) = resolve_target_snapshot(&expected, &command.table, next)?;
    let resolved = ResolvedField {
        key: next.key,
        name: next.name.clone(),
        base_type: type_system
            .resolve_reference(&next.type_name)
            .ok_or_else(|| {
                failure(
                    "E-FIELD-DECL-TYPE",
                    format!("unknown type `{}`", next.type_name),
                    Some(schema_loaded.path.to_path_buf()),
                )
            })?,
        modifier: field_modifier(next),
    };
    let mut count = 0;
    for loaded in &expected_closure.files {
        let records = match &loaded.document {
            SourceDocument::Schema(schema) if schema.table == command.table => {
                schema.records.as_ref()
            }
            SourceDocument::Data(data) if data.table == command.table => Some(&data.records),
            _ => None,
        };
        if let Some(records) = records {
            for (index, record) in records.iter().enumerate() {
                let value = record.get(&next.name).ok_or_else(|| {
                    failure(
                        "E-FIELD-DECL-MISSING-VALUE",
                        format!(
                            "record {index} in `{}` has no `{}` value",
                            loaded.path.display(),
                            next.name
                        ),
                        Some(loaded.path.clone()),
                    )
                })?;
                type_system
                    .validate_field_value(&resolved, value)
                    .map_err(|error| {
                        failure(
                            "E-FIELD-DECL-INVALID-VALUE",
                            format!(
                                "record {index} in `{}` cannot use {}: {}",
                                loaded.path.display(),
                                next.type_name,
                                error.diagnostic().message
                            ),
                            Some(loaded.path.clone()),
                        )
                    })?;
                count += 1;
            }
        }
    }
    let patches = declaration_patches(
        &documents
            .files
            .iter()
            .find(|file| file.path == schema_loaded.path)
            .expect("target schema source")
            .source,
        field_index,
        schema_loaded.document.fields.len(),
        &old,
        next,
    )?;
    let file_plans = vec![MigrationFilePlan {
        path: schema_loaded.path.to_path_buf(),
        patches,
    }];
    let transformed = apply_file_plans(documents, &file_plans)?;
    resolve_target_snapshot(&transformed, &command.table, next)?;
    if semantic_documents(&transformed) != semantic_documents(&expected) {
        return Err(failure(
            "E-FIELD-DECL-POSTCONDITION",
            "patched field declaration does not match intended semantics",
            Some(schema_loaded.path.to_path_buf()),
        ));
    }
    Ok(MigrationDryRun {
        plan: MigrationPlan {
            operation: MigrationOperation::ChangeFieldDeclaration,
            target_table: command.table.clone(),
            field: next.clone(),
            reference: None,
            initializer: None,
            source_inputs: documents
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            destructive: false,
            affected_files: file_plans,
            affected_record_count: count,
            validation: MigrationValidation {
                valid: true,
                diagnostics: Vec::new(),
            },
        },
        transformed_documents: transformed,
    })
}

pub(super) fn declaration_patches(
    source: &str,
    index: usize,
    count: usize,
    old: &FieldDefinition,
    next: &FieldDefinition,
) -> Result<Vec<MigrationPatch>> {
    let lines = source_lines(source);
    let header = find_top_level_key(&lines, "fields")
        .ok_or_else(|| failure("E-FIELD-DECL-SOURCE", "fields source missing", None))?;
    let end = block_region_end(&lines, header);
    let region = find_block_sequence(&lines, header, end)
        .ok_or_else(|| failure("E-FIELD-DECL-SOURCE", "fields sequence unavailable", None))?;
    if region.items.len() != count {
        return Err(failure(
            "E-FIELD-DECL-SOURCE",
            "field source count mismatch",
            None,
        ));
    }
    let start = region.items[index];
    let end = region.items.get(index + 1).copied().unwrap_or(end);
    let mut type_line = None;
    let mut nullable_line = None;
    let mut array_line = None;
    for (line_index, line) in lines.iter().enumerate().take(end).skip(start) {
        if let Some(entry) = mapping_entry(line.text) {
            match entry.key.as_str() {
                "type" => type_line = Some(line_index),
                "nullable" => nullable_line = Some(line_index),
                "array" => array_line = Some(line_index),
                _ => {}
            }
        }
    }
    let type_line = type_line
        .ok_or_else(|| failure("E-FIELD-DECL-SOURCE", "field type source missing", None))?;
    let mut patches = Vec::new();
    if old.type_name != next.type_name {
        patches.push(value_patch(&lines[type_line], &next.type_name)?);
    }
    let mut missing = String::new();
    for (key, line, value, old_value) in [
        ("nullable", nullable_line, next.nullable, old.nullable),
        ("array", array_line, next.array, old.array),
    ] {
        if value == old_value {
            continue;
        }
        if let Some(line) = line {
            patches.push(value_patch(
                &lines[line],
                if value { "true" } else { "false" },
            )?);
        } else if value {
            missing.push_str(&format!(
                "{}{}: true{}",
                spaces(yaml_indent(lines[type_line].text)),
                key,
                newline_for(source)
            ));
        }
    }
    if !missing.is_empty() {
        patches.push(MigrationPatch {
            start: lines[type_line].next_start,
            end: lines[type_line].next_start,
            replacement: missing,
        });
    }
    Ok(patches)
}

fn value_patch(line: &SourceLine<'_>, next: &str) -> Result<MigrationPatch> {
    let (_, rest) = sequence_item_parts(line.text).unwrap_or((false, line.text.trim_start()));
    let colon = mapping_colon(rest)
        .ok_or_else(|| failure("E-FIELD-DECL-SOURCE", "mapping colon missing", None))?;
    let raw = strip_yaml_comment(&rest[colon + 1..]).trim();
    let start = line.start
        + (rest.as_ptr() as usize - line.text.as_ptr() as usize)
        + colon
        + 1
        + rest[colon + 1..].len()
        - rest[colon + 1..].trim_start().len();
    let replacement = if raw.starts_with('\'') {
        format!("'{}'", next.replace('\'', "''"))
    } else if raw.starts_with('"') {
        serde_json::to_string(next).expect("string")
    } else {
        next.to_owned()
    };
    Ok(MigrationPatch {
        start,
        end: start + raw.len(),
        replacement,
    })
}
