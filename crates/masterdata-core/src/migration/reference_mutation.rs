use super::*;

#[derive(Debug, Clone)]
enum ReferenceAction {
    Add(crate::ReferenceDefinition),
    Edit {
        name: String,
        replacement: crate::ReferenceDefinition,
    },
    Remove {
        name: String,
    },
}

pub(super) fn prepare_add(
    documents: &ProjectDocuments,
    command: &AddReferenceCommand,
) -> Result<MigrationDryRun> {
    prepare(
        documents,
        &command.table,
        ReferenceAction::Add(command.reference.clone()),
    )
}

pub(super) fn prepare_edit(
    documents: &ProjectDocuments,
    command: &EditReferenceCommand,
) -> Result<MigrationDryRun> {
    prepare(
        documents,
        &command.table,
        ReferenceAction::Edit {
            name: command.name.clone(),
            replacement: command.reference.clone(),
        },
    )
}

pub(super) fn prepare_remove(
    documents: &ProjectDocuments,
    command: &RemoveReferenceCommand,
) -> Result<MigrationDryRun> {
    prepare(
        documents,
        &command.table,
        ReferenceAction::Remove {
            name: command.name.clone(),
        },
    )
}

fn prepare(
    documents: &ProjectDocuments,
    table: &str,
    action: ReferenceAction,
) -> Result<MigrationDryRun> {
    let target = find_target_schema(documents, table)
        .ok_or_else(|| reference_failure("target Table schema does not exist"))?;
    let source_schema = target.document;
    validate_action_preconditions(source_schema, &action)?;

    let mut expected = documents.clone();
    let mut plans = Vec::new();
    for loaded in &mut expected.files {
        if let SourceDocument::Schema(schema) = &mut loaded.document
            && schema.table == table
        {
            let patches = reference_patches(&loaded.source, &schema.references, &action)?;
            apply_action(schema, &action)?;
            if !patches.is_empty() {
                plans.push(MigrationFilePlan {
                    path: loaded.path.clone(),
                    patches,
                });
            }
        }
    }
    if plans.is_empty() {
        return Err(reference_failure("Reference source patch was empty"));
    }

    let transformed_documents = apply_file_plans(documents, &plans)?;
    validate_reference_project(&transformed_documents)?;
    if semantic_documents(&transformed_documents) != semantic_documents(&expected) {
        return Err(reference_failure(
            "patched source did not match the expected Reference semantic result",
        ));
    }
    plans.sort_by(|left, right| left.path.cmp(&right.path));

    let (operation, field_name, reference) = match &action {
        ReferenceAction::Add(reference) => (
            MigrationOperation::AddReference,
            reference.name.clone(),
            Some(reference.clone()),
        ),
        ReferenceAction::Edit { replacement, .. } => (
            MigrationOperation::EditReference,
            replacement.name.clone(),
            Some(replacement.clone()),
        ),
        ReferenceAction::Remove { name } => {
            (MigrationOperation::RemoveReference, name.clone(), None)
        }
    };
    Ok(MigrationDryRun {
        plan: MigrationPlan {
            operation,
            target_table: table.to_owned(),
            field: FieldDefinition {
                key: 0,
                name: field_name,
                type_name: "reference".to_owned(),
                nullable: false,
                array: false,
            },
            reference,
            initializer: None,
            source_inputs: documents
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            destructive: matches!(action, ReferenceAction::Remove { .. }),
            affected_files: plans,
            affected_record_count: 0,
            validation: MigrationValidation {
                valid: true,
                diagnostics: Vec::new(),
            },
        },
        transformed_documents,
    })
}

fn validate_action_preconditions(schema: &SchemaDocument, action: &ReferenceAction) -> Result<()> {
    match action {
        ReferenceAction::Add(reference) => {
            if schema
                .references
                .iter()
                .any(|item| item.name == reference.name)
            {
                return Err(reference_failure("Reference name is already declared"));
            }
            validate_reference_name_present(reference)
        }
        ReferenceAction::Edit { name, replacement } => {
            if !schema.references.iter().any(|item| item.name == *name) {
                return Err(reference_failure("Reference name does not exist"));
            }
            if replacement.name != *name
                && schema
                    .references
                    .iter()
                    .any(|item| item.name == replacement.name)
            {
                return Err(reference_failure(
                    "edited Reference name is already declared",
                ));
            }
            validate_reference_name_present(replacement)
        }
        ReferenceAction::Remove { name } => {
            if !schema.references.iter().any(|item| item.name == *name) {
                return Err(reference_failure("Reference name does not exist"));
            }
            Ok(())
        }
    }
}

fn validate_reference_name_present(reference: &crate::ReferenceDefinition) -> Result<()> {
    if reference.name.is_empty() {
        Err(reference_failure("Reference name must not be empty"))
    } else {
        Ok(())
    }
}

fn apply_action(schema: &mut SchemaDocument, action: &ReferenceAction) -> Result<()> {
    match action {
        ReferenceAction::Add(reference) => schema.references.push(reference.clone()),
        ReferenceAction::Edit { name, replacement } => {
            let index = schema
                .references
                .iter()
                .position(|item| item.name == *name)
                .ok_or_else(|| reference_failure("Reference name does not exist"))?;
            schema.references[index] = replacement.clone();
        }
        ReferenceAction::Remove { name } => {
            let index = schema
                .references
                .iter()
                .position(|item| item.name == *name)
                .ok_or_else(|| reference_failure("Reference name does not exist"))?;
            schema.references.remove(index);
        }
    }
    Ok(())
}

fn reference_patches(
    source: &str,
    references: &[crate::ReferenceDefinition],
    action: &ReferenceAction,
) -> Result<Vec<MigrationPatch>> {
    let lines = source_lines(source);
    let newline = newline_for(source);
    let rendered = |reference: &crate::ReferenceDefinition, indent: usize| {
        render_reference(reference, indent, newline)
    };
    let Some(key_line) = find_top_level_key(&lines, "references") else {
        let ReferenceAction::Add(reference) = action else {
            return Err(reference_failure(
                "Reference sequence source cannot be located safely",
            ));
        };
        let text = format!("references:{}{}", newline, rendered(reference, 2));
        return Ok(vec![MigrationPatch {
            start: source.len(),
            end: source.len(),
            replacement: insertion_at(source, source.len(), &text),
        }]);
    };
    let region_end = block_region_end(&lines, key_line);
    let region = find_block_sequence(&lines, key_line, region_end);
    let Some(region) = region else {
        let entry = mapping_entry(lines[key_line].text).ok_or_else(|| {
            reference_failure("Reference declaration source cannot be located safely")
        })?;
        if !is_empty_flow_sequence(entry.raw_value) {
            return Err(reference_failure(
                "Reference declaration source is not a block sequence",
            ));
        }
        let ReferenceAction::Add(reference) = action else {
            return Err(reference_failure(
                "Reference sequence source cannot be located safely",
            ));
        };
        let colon = mapping_colon(lines[key_line].text)
            .ok_or_else(|| reference_failure("Reference mapping colon is missing"))?;
        let raw = strip_yaml_comment(lines[key_line].text);
        let after_colon = &raw[colon + 1..];
        let value_start = colon + 1 + after_colon.len() - after_colon.trim_start().len();
        let replacement = format!("{}{}", newline, rendered(reference, 2));
        return Ok(vec![MigrationPatch {
            start: lines[key_line].start + value_start,
            end: lines[key_line].start + value_start + entry.raw_value.trim().len(),
            replacement,
        }]);
    };
    if region.items.len() != references.len() {
        return Err(reference_failure(
            "Reference source/semantic count mismatch",
        ));
    }

    let item_range = |index: usize| {
        let start = region.items[index];
        let end = region.items.get(index + 1).copied().unwrap_or(region_end);
        (start, end)
    };
    let item_name = |start: usize, end: usize| {
        (start..end).find_map(|line| {
            mapping_entry(lines[line].text)
                .and_then(|entry| (entry.key == "name").then(|| entry.raw_value.trim().to_owned()))
        })
    };
    let find_index = |name: &str| {
        region.items.iter().enumerate().find_map(|(index, start)| {
            let end = region.items.get(index + 1).copied().unwrap_or(region_end);
            let candidate = item_name(*start, end)?;
            (decode_mapping_key(&candidate).as_deref() == Some(name)).then_some(index)
        })
    };
    let plain_item = |start: usize, end: usize| {
        (start..end).all(|line| {
            !lines[line].text.trim().is_empty() && comment_start(lines[line].text).is_none()
        })
    };

    match action {
        ReferenceAction::Add(reference) => {
            let boundary = sequence_append_boundary(
                &lines,
                *region.items.last().expect("non-empty reference sequence"),
                region_end,
                region.indent,
                &literal_block_scalar_content_lines(&lines),
            );
            let position = line_start(&lines, boundary, source.len());
            Ok(vec![MigrationPatch {
                start: position,
                end: position,
                replacement: insertion_at(source, position, &rendered(reference, region.indent)),
            }])
        }
        ReferenceAction::Edit { name, replacement } => {
            let index = find_index(name).ok_or_else(|| {
                reference_failure("Reference name source cannot be located safely")
            })?;
            let (start, end) = item_range(index);
            if !plain_item(start, end) {
                return Err(reference_failure(
                    "Reference item contains comments or blank lines; edit it manually",
                ));
            }
            Ok(vec![MigrationPatch {
                start: lines[start].start,
                end: line_start(&lines, end, source.len()),
                replacement: rendered(replacement, region.indent),
            }])
        }
        ReferenceAction::Remove { name } => {
            let index = find_index(name).ok_or_else(|| {
                reference_failure("Reference name source cannot be located safely")
            })?;
            let (start, end) = item_range(index);
            if !plain_item(start, end) {
                return Err(reference_failure(
                    "Reference item contains comments or blank lines; remove it manually",
                ));
            }
            if region.items.len() == 1 {
                let key_text = lines[key_line].text;
                if comment_start(key_text).is_some() {
                    return Err(reference_failure(
                        "Reference key line contains a comment; remove it manually",
                    ));
                }
                return Ok(vec![MigrationPatch {
                    start: lines[key_line].start,
                    end: line_start(&lines, region_end, source.len()),
                    replacement: format!("references: []{}", newline),
                }]);
            }
            Ok(vec![MigrationPatch {
                start: lines[start].start,
                end: line_start(&lines, end, source.len()),
                replacement: String::new(),
            }])
        }
    }
}

fn render_reference(
    reference: &crate::ReferenceDefinition,
    indent: usize,
    newline: &str,
) -> String {
    let spaces = " ".repeat(indent);
    let fields = reference
        .fields
        .iter()
        .map(|field| yaml_scalar(field))
        .collect::<Vec<_>>()
        .join(", ");
    let target_fields = reference
        .target
        .fields
        .iter()
        .map(|field| yaml_scalar(field))
        .collect::<Vec<_>>()
        .join(", ");
    let csharp_name = reference
        .csharp_name
        .as_ref()
        .map(|value| format!("{spaces}  csharpName: {}{newline}", yaml_scalar(value)))
        .unwrap_or_default();
    format!(
        "{spaces}- name: {}{newline}{csharp_name}{spaces}  fields: [{fields}]{newline}{spaces}  target:{newline}{spaces}    table: {}{newline}{spaces}    fields: [{target_fields}]{newline}",
        yaml_scalar(&reference.name),
        yaml_scalar(&reference.target.table),
    )
}

fn yaml_scalar(value: &str) -> String {
    serde_yaml::to_string(value)
        .expect("string scalar serialization")
        .trim_end()
        .to_owned()
}

fn validate_reference_project(documents: &ProjectDocuments) -> Result<()> {
    let type_build = build_type_system(documents);
    if !type_build.diagnostics.is_empty() {
        return Err(first_diagnostic_error(
            type_build.diagnostics,
            "E-REFERENCE-MUTATION-INVALID",
            "Reference mutation produced an invalid Type System",
            "REF-006",
        ));
    }
    let type_system = type_build
        .model
        .ok_or_else(|| reference_failure("Reference mutation could not resolve the Type System"))?;
    let table_build = resolve_tables(documents, &type_system, &BuildSelection::unfiltered());
    if !table_build.diagnostics.is_empty() {
        return Err(first_diagnostic_error(
            table_build.diagnostics,
            "E-REFERENCE-MUTATION-INVALID",
            "Reference mutation produced an invalid Table relationship",
            "REF-006",
        ));
    }
    Ok(())
}

fn reference_failure(message: &str) -> MasterdataError {
    migration_error(
        "E-MIGRATION-REFERENCE-PRECONDITION",
        message,
        None,
        "GUI-TABLE-INT-009",
    )
}
