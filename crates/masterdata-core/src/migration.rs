use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::document::{
    DataDocument, FieldDefinition, LoadedDocument, ProjectDocuments, SchemaDocument, SourceDocument,
};
use crate::error::{Diagnostic, ErrorKind, MasterdataError, Result};
use crate::table::{BuildSelection, resolve_tables};
use crate::type_system::{
    FieldModifier, PrimitiveType, ResolvedField, ResolvedType, TypeReference, TypeSystem,
    build_type_system,
};

/// Frontend-independent semantic input for the first Migration slice.
///
/// This is intentionally a Rust-internal command model rather than a wire or
/// CLI schema. The command identifies a logical table and carries the complete
/// new field declaration plus an optional constant initializer.
#[derive(Debug, Clone, PartialEq)]
pub struct AddFieldCommand {
    pub table: String,
    pub field: FieldDefinition,
    pub initializer: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationCommand {
    AddField(AddFieldCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOperation {
    AddField,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationValidation {
    pub valid: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPatch {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationFilePlan {
    pub path: PathBuf,
    pub patches: Vec<MigrationPatch>,
}

/// Deterministic, pre-mutation description of an AddField transformation.
///
/// The plan contains text patches only; it never writes to the filesystem and
/// does not represent a public JSON/CLI result contract.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationPlan {
    pub operation: MigrationOperation,
    pub target_table: String,
    pub field: FieldDefinition,
    pub initializer: Option<Value>,
    pub destructive: bool,
    pub affected_files: Vec<MigrationFilePlan>,
    pub affected_record_count: usize,
    pub validation: MigrationValidation,
}

/// Result of applying a Migration plan to an in-memory source snapshot.
/// `transformed_documents` is never persisted by this module.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationDryRun {
    pub plan: MigrationPlan,
    pub transformed_documents: ProjectDocuments,
}

/// Build and verify a deterministic Migration Plan without mutating source.
pub fn plan_migration(
    documents: &ProjectDocuments,
    command: &MigrationCommand,
) -> Result<MigrationPlan> {
    Ok(prepare_migration(documents, command)?.plan)
}

/// Execute a Migration against an in-memory snapshot only.
///
/// The operation includes canonical reparse and postcondition verification, so
/// a successful result is stronger than merely producing text that looks like
/// a valid patch.
pub fn dry_run_migration(
    documents: &ProjectDocuments,
    command: &MigrationCommand,
) -> Result<MigrationDryRun> {
    prepare_migration(documents, command)
}

fn prepare_migration(
    documents: &ProjectDocuments,
    command: &MigrationCommand,
) -> Result<MigrationDryRun> {
    match command {
        MigrationCommand::AddField(command) => prepare_add_field(documents, command),
    }
}

fn prepare_add_field(
    documents: &ProjectDocuments,
    command: &AddFieldCommand,
) -> Result<MigrationDryRun> {
    let (closure_documents, type_system) =
        resolve_target_snapshot(documents, &command.table, &command.field)?;

    validate_candidate_table_schema(&closure_documents, &type_system, command)?;

    let new_field = ResolvedField {
        key: command.field.key,
        name: command.field.name.clone(),
        base_type: type_system
            .resolve_reference(&command.field.type_name)
            .ok_or_else(|| {
                migration_error(
                    "E-MIGRATION-UNKNOWN-FIELD-TYPE",
                    format!(
                        "AddField declaration refers to unknown type `{}`",
                        command.field.type_name
                    ),
                    schema_path(&closure_documents, &command.table),
                    "MIGRATION-006",
                )
            })?,
        modifier: field_modifier(&command.field),
    };

    let affected_record_count = target_record_count(&closure_documents, &command.table);
    if affected_record_count > 0 && command.initializer.is_none() {
        return Err(migration_error(
            "E-MIGRATION-ADD-FIELD-INITIALIZER-REQUIRED",
            format!(
                "AddField for table `{}` requires an explicit initializer because it has {affected_record_count} existing record(s)",
                command.table
            ),
            schema_path(&closure_documents, &command.table),
            "MIGRATION-006",
        ));
    }

    let initializer = command
        .initializer
        .as_ref()
        .map(|value| {
            type_system
                .validate_field_value(&new_field, value)
                .map_err(|error| {
                    migration_error_from_diagnostic(
                        error.diagnostic().clone(),
                        schema_path(&closure_documents, &command.table),
                        "MIGRATION-006",
                    )
                })?;
            canonicalize_field_value(&type_system, &new_field, value)
        })
        .transpose()?;

    let expected_semantics =
        expected_add_field_semantics(&closure_documents, command, initializer.as_ref())?;

    let mut file_plans = Vec::new();
    let schema_loaded = find_loaded_schema(&closure_documents, &command.table)
        .expect("target schema was resolved before building the snapshot");
    let schema_document = match &schema_loaded.document {
        SourceDocument::Schema(schema) => schema,
        _ => unreachable!("target schema lookup returned a non-schema document"),
    };
    let schema_patches = plan_schema_patch(&schema_loaded.source, schema_document, &command.field)?;
    file_plans.push(MigrationFilePlan {
        path: schema_loaded.path.clone(),
        patches: schema_patches,
    });

    if let Some(initializer) = &initializer {
        for loaded in target_data_documents(&closure_documents, &command.table) {
            let SourceDocument::Data(data) = &loaded.document else {
                unreachable!("target data lookup returned a non-data document")
            };
            if data.records.is_empty() {
                continue;
            }
            let patches = plan_data_patches(&loaded.source, data, &command.field, initializer)?;
            file_plans.push(MigrationFilePlan {
                path: loaded.path.clone(),
                patches,
            });
        }
    }
    file_plans.sort_by(|left, right| left.path.cmp(&right.path));

    let transformed_documents = apply_file_plans(documents, &file_plans)?;
    let (transformed_closure, _post_type_system) =
        resolve_target_snapshot(&transformed_documents, &command.table, &command.field)?;
    if semantic_documents(&transformed_closure) != expected_semantics {
        return Err(migration_error(
            "E-MIGRATION-ADD-FIELD-POSTCONDITION",
            format!(
                "patched source for table `{}` does not match the expected AddField semantic result",
                command.table
            ),
            schema_path(&transformed_documents, &command.table),
            "MIGRATION-015",
        ));
    }

    let plan = MigrationPlan {
        operation: MigrationOperation::AddField,
        target_table: command.table.clone(),
        field: command.field.clone(),
        initializer,
        destructive: false,
        affected_files: file_plans,
        affected_record_count,
        validation: MigrationValidation {
            valid: true,
            diagnostics: Vec::new(),
        },
    };
    Ok(MigrationDryRun {
        plan,
        transformed_documents,
    })
}

fn resolve_target_snapshot(
    documents: &ProjectDocuments,
    table_name: &str,
    new_field: &FieldDefinition,
) -> Result<(ProjectDocuments, TypeSystem)> {
    let schema = find_target_schema(documents, table_name).ok_or_else(|| {
        migration_error(
            "E-MIGRATION-TABLE-NOT-FOUND",
            format!("AddField target table `{table_name}` does not exist"),
            None,
            "MIGRATION-006",
        )
    })?;

    let schemas = documents
        .files
        .iter()
        .filter(|loaded| {
            matches!(&loaded.document, SourceDocument::Schema(candidate) if candidate.table == table_name)
        })
        .count();
    if schemas != 1 {
        return Err(migration_error(
            "E-TABLE-DUPLICATE-SCHEMA",
            format!("table `{table_name}` must have exactly one schema document"),
            Some(schema.path.to_path_buf()),
            "SCHEMA-TABLE-001",
        ));
    }

    let closure_documents = build_resolution_closure(documents, table_name, new_field);
    let type_build = build_type_system(&closure_documents);
    let Some(type_system) = type_build.model else {
        return Err(first_diagnostic_error(
            type_build.diagnostics,
            "E-MIGRATION-TYPE-CLOSURE",
            "the AddField type closure could not be resolved",
            "MIGRATION-005",
        ));
    };

    validate_table_schema_resolution(&closure_documents, &type_system, table_name)?;
    Ok((closure_documents, type_system))
}

fn validate_table_schema_resolution(
    documents: &ProjectDocuments,
    type_system: &TypeSystem,
    table_name: &str,
) -> Result<()> {
    // WHY: Migration must reuse Table/Key owner semantics without turning
    // build-time selected-dataset diagnostics into a Migration success gate.
    // Resolving only schema/type documents validates field/key structure while
    // intentionally excluding record-value and PK/Unique dataset constraints.
    // IF REMOVED: using the full target data set would reintroduce MIGRATION-017
    // violations; replacing this with migration-local checks would duplicate the
    // Table/Key semantic owner contrary to MIGRATION-005.
    // EVIDENCE: docs/specs/schema-migration.md (MIGRATION-005, MIGRATION-017)
    // Regression: target_table_duplicate_primary_key_does_not_block_add_field_resolution;
    // target_table_unrelated_record_diagnostic_does_not_block_add_field_resolution;
    // target_table_schema_resolution_error_blocks_add_field.
    let schema_documents = ProjectDocuments {
        files: documents
            .files
            .iter()
            .filter(|loaded| !matches!(&loaded.document, SourceDocument::Data(_)))
            .cloned()
            .collect(),
    };
    let table_build = resolve_tables(
        &schema_documents,
        type_system,
        &BuildSelection::unfiltered(),
    );
    let Some(tables) = table_build.model else {
        return Err(first_diagnostic_error(
            table_build.diagnostics,
            "E-MIGRATION-RESOLUTION-CLOSURE",
            "the AddField table/schema closure could not be resolved",
            "MIGRATION-005",
        ));
    };
    if !tables.iter().any(|table| table.identity == table_name) {
        return Err(migration_error(
            "E-MIGRATION-TABLE-NOT-FOUND",
            format!("AddField target table `{table_name}` does not exist"),
            schema_path(documents, table_name),
            "MIGRATION-006",
        ));
    }
    Ok(())
}

fn validate_candidate_table_schema(
    documents: &ProjectDocuments,
    type_system: &TypeSystem,
    command: &AddFieldCommand,
) -> Result<()> {
    let mut candidate = documents.clone();
    let loaded = candidate
        .files
        .iter_mut()
        .find(|loaded| {
            matches!(&loaded.document, SourceDocument::Schema(schema) if schema.table == command.table)
        })
        .expect("target schema was resolved before validating AddField declaration");
    let SourceDocument::Schema(schema) = &mut loaded.document else {
        unreachable!("target schema lookup returned a non-schema document")
    };
    schema.fields.push(command.field.clone());
    validate_table_schema_resolution(&candidate, type_system, &command.table)
}

fn expected_add_field_semantics(
    documents: &ProjectDocuments,
    command: &AddFieldCommand,
    initializer: Option<&Value>,
) -> Result<Vec<(PathBuf, SourceDocument)>> {
    let mut expected = Vec::with_capacity(documents.files.len());
    for loaded in &documents.files {
        let mut document = loaded.document.clone();
        match &mut document {
            SourceDocument::Schema(schema) if schema.table == command.table => {
                schema.fields.push(command.field.clone());
            }
            SourceDocument::Data(data) if data.table == command.table => {
                if let Some(initializer) = initializer {
                    for (record_index, record) in data.records.iter_mut().enumerate() {
                        if record.contains_key(&command.field.name) {
                            return Err(migration_error(
                                "E-TABLE-UNKNOWN-RECORD-FIELD",
                                format!(
                                    "record {record_index} already contains member `{}` while the schema does not declare it",
                                    command.field.name
                                ),
                                Some(loaded.path.clone()),
                                "MIGRATION-006",
                            ));
                        }
                        record.insert(command.field.name.clone(), initializer.clone());
                    }
                }
            }
            SourceDocument::Schema(_) | SourceDocument::Data(_) | SourceDocument::Type(_) => {}
        }
        expected.push((loaded.path.clone(), document));
    }
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(expected)
}

fn semantic_documents(documents: &ProjectDocuments) -> Vec<(PathBuf, SourceDocument)> {
    let mut semantics = documents
        .files
        .iter()
        .map(|loaded| (loaded.path.clone(), loaded.document.clone()))
        .collect::<Vec<_>>();
    semantics.sort_by(|left, right| left.0.cmp(&right.0));
    semantics
}

fn build_resolution_closure(
    documents: &ProjectDocuments,
    table_name: &str,
    new_field: &FieldDefinition,
) -> ProjectDocuments {
    // WHY: Migration Resolvable is scoped to the target Table and its type
    // dependencies; project-wide validation would incorrectly turn unrelated
    // diagnostics into AddField blockers.
    // IF REMOVED: an invalid unrelated Table or type would prevent a safe
    // target transformation even though it cannot affect the patch.
    // EVIDENCE: docs/specs/schema-migration.md; docs/spec-changes/0011-cli-surface-and-schema-migration.md
    // Regression: unrelated_invalid_type_does_not_block_add_field_resolution;
    // target_table_unrelated_record_diagnostic_does_not_block_add_field_resolution.
    let mut included = Vec::<LoadedDocument>::new();
    for loaded in &documents.files {
        let include = match &loaded.document {
            SourceDocument::Schema(schema) => schema.table == table_name,
            SourceDocument::Data(data) => data.table == table_name,
            SourceDocument::Type(_) => false,
        };
        if include {
            included.push(loaded.clone());
        }
    }

    let mut type_by_name = BTreeMap::<String, Vec<LoadedDocument>>::new();
    for loaded in &documents.files {
        if let SourceDocument::Type(document) = &loaded.document {
            type_by_name
                .entry(document.name.clone())
                .or_default()
                .push(loaded.clone());
        }
    }
    for documents in type_by_name.values_mut() {
        documents.sort_by(|left, right| left.path.cmp(&right.path));
    }

    let mut pending = BTreeSet::new();
    if let Some(schema) = find_target_schema(documents, table_name) {
        for field in &schema.document.fields {
            add_named_type(&mut pending, &field.type_name);
        }
    }
    add_named_type(&mut pending, &new_field.type_name);

    let mut visited = BTreeSet::new();
    while let Some(name) = pending.pop_first() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let Some(declarations) = type_by_name.get(&name) else {
            continue;
        };
        for loaded in declarations {
            included.push(loaded.clone());
            if let SourceDocument::Type(document) = &loaded.document {
                if let Some(custom) = &document.custom {
                    for field in &custom.fields {
                        add_named_type(&mut pending, &field.type_name);
                    }
                }
            }
        }
    }

    included.sort_by(|left, right| left.path.cmp(&right.path));
    included.dedup_by(|left, right| left.path == right.path);
    ProjectDocuments { files: included }
}

fn add_named_type(pending: &mut BTreeSet<String>, type_name: &str) {
    if PrimitiveType::parse(type_name).is_none() {
        pending.insert(type_name.to_owned());
    }
}

fn canonicalize_field_value(
    type_system: &TypeSystem,
    field: &ResolvedField,
    value: &Value,
) -> Result<Value> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    match field.modifier {
        FieldModifier::Required | FieldModifier::Nullable => {
            canonicalize_reference_value(type_system, &field.base_type, value)
        }
        FieldModifier::Array => {
            let values = value.as_sequence().ok_or_else(|| {
                migration_error(
                    "E-TYPE-ARRAY-DATA-SHAPE",
                    format!("array field `{}` must be a sequence", field.name),
                    None,
                    "TYPE-FIELD-003",
                )
            })?;
            let elements = values
                .iter()
                .map(|value| canonicalize_reference_value(type_system, &field.base_type, value))
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::Sequence(elements))
        }
    }
}

fn canonicalize_reference_value(
    type_system: &TypeSystem,
    reference: &TypeReference,
    value: &Value,
) -> Result<Value> {
    let TypeReference::Named(name) = reference else {
        return Ok(value.clone());
    };
    match type_system.get(name) {
        Some(ResolvedType::Custom { fields, .. }) => {
            let mapping = value.as_mapping().ok_or_else(|| {
                migration_error(
                    "E-TYPE-CUSTOM-DATA-SHAPE",
                    format!("Custom Type `{name}` data must be a mapping"),
                    None,
                    "SCHEMA-CUSTOM-006",
                )
            })?;
            let mut canonical = Mapping::new();
            for field in fields {
                let field_value =
                    mapping
                        .get(Value::String(field.name.clone()))
                        .ok_or_else(|| {
                            migration_error(
                                "E-TYPE-CUSTOM-MISSING-MEMBER",
                                format!("missing Custom Type member `{}`", field.name),
                                None,
                                "SCHEMA-CUSTOM-006",
                            )
                        })?;
                canonical.insert(
                    Value::String(field.name.clone()),
                    canonicalize_field_value(type_system, field, field_value)?,
                );
            }
            Ok(Value::Mapping(canonical))
        }
        Some(ResolvedType::Flags { .. }) => {
            let values = value.as_sequence().ok_or_else(|| {
                migration_error(
                    "E-FLAGS-DATA-NOT-SEQUENCE",
                    format!("Flags Enum `{name}` data must be a sequence"),
                    None,
                    "SCHEMA-FLAGS-003",
                )
            })?;
            let mut values = values.clone();
            values.sort_by_key(|value| value.as_str().unwrap_or_default().to_owned());
            Ok(Value::Sequence(values))
        }
        Some(ResolvedType::ValueObject { .. } | ResolvedType::Enum { .. }) | None => {
            Ok(value.clone())
        }
    }
}

fn plan_schema_patch(
    source: &str,
    schema: &SchemaDocument,
    field: &FieldDefinition,
) -> Result<Vec<MigrationPatch>> {
    // WHY: Patch only the source spans required by AddField instead of
    // serializing the semantic AST, preserving comments and presentation.
    // IF REMOVED: an otherwise valid migration would rewrite unrelated YAML
    // text and violate the source-preserving contract.
    // EVIDENCE: docs/specs/schema-migration.md; docs/adr/0001-yaml-is-source-of-truth.md
    // Regression: migration_add_field_plan_is_deterministic_and_source_preserving.
    let lines = source_lines(source);
    let literal_scalar_content = literal_block_scalar_content_lines(&lines);
    let Some(fields_line) = find_top_level_key(&lines, "fields") else {
        let insertion = append_at_end(
            source,
            &render_fields_block(field, 0, newline_for(source), source.ends_with('\n')),
        );
        return Ok(vec![MigrationPatch {
            start: source.len(),
            end: source.len(),
            replacement: insertion,
        }]);
    };

    let region_end = block_region_end(&lines, fields_line);
    let entry = mapping_entry(lines[fields_line].text).expect("top-level fields key");
    if is_empty_flow_sequence(entry.raw_value) {
        let replacement =
            replace_empty_sequence_line(source, &lines[fields_line], "fields", field)?;
        return Ok(vec![MigrationPatch {
            start: lines[fields_line].start,
            end: lines[fields_line].next_start,
            replacement,
        }]);
    }

    let sequence = find_block_sequence(&lines, fields_line, region_end).ok_or_else(|| {
        migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            "schema `fields` source shape could not be located safely",
            None,
            "MIGRATION-014",
        )
    })?;
    if sequence.items.len() != schema.fields.len() {
        return Err(migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            format!(
                "schema `fields` source has {} sequence item(s), but semantic parsing found {}",
                sequence.items.len(),
                schema.fields.len()
            ),
            None,
            "MIGRATION-015",
        ));
    }
    let position = line_start(
        &lines,
        sequence_append_boundary(
            &lines,
            sequence.items[sequence.items.len() - 1],
            region_end,
            sequence.indent,
            &literal_scalar_content,
        ),
        source.len(),
    );
    let replacement = insertion_at(
        source,
        position,
        &render_field_block(
            field,
            sequence.indent,
            newline_for(source),
            position < source.len() || source.ends_with('\n'),
        ),
    );
    Ok(vec![MigrationPatch {
        start: position,
        end: position,
        replacement,
    }])
}

fn plan_data_patches(
    source: &str,
    data: &DataDocument,
    field: &FieldDefinition,
    initializer: &Value,
) -> Result<Vec<MigrationPatch>> {
    let lines = source_lines(source);
    let literal_scalar_content = literal_block_scalar_content_lines(&lines);
    let Some(records_line) = find_top_level_key(&lines, "records") else {
        return Err(migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            "data `records` source shape could not be located safely",
            None,
            "MIGRATION-014",
        ));
    };
    let region_end = block_region_end(&lines, records_line);
    let entry = mapping_entry(lines[records_line].text).expect("top-level records key");
    if is_empty_flow_sequence(entry.raw_value) {
        return Err(migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            "data source has semantic records but an empty flow `records` sequence",
            None,
            "MIGRATION-015",
        ));
    }
    let sequence = find_block_sequence(&lines, records_line, region_end).ok_or_else(|| {
        migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            "data `records` source shape could not be located safely",
            None,
            "MIGRATION-014",
        )
    })?;
    if sequence.items.len() != data.records.len() {
        return Err(migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            format!(
                "data `records` source has {} sequence item(s), but semantic parsing found {}",
                sequence.items.len(),
                data.records.len()
            ),
            None,
            "MIGRATION-015",
        ));
    }

    let member_indent = record_member_indent(&lines, &sequence)?;
    let mut patches = Vec::with_capacity(sequence.items.len());
    for (index, _) in sequence.items.iter().enumerate() {
        let next_boundary = sequence.items.get(index + 1).copied().unwrap_or(region_end);
        let position = line_start(
            &lines,
            sequence_append_boundary(
                &lines,
                sequence.items[index],
                next_boundary,
                sequence.indent,
                &literal_scalar_content,
            ),
            source.len(),
        );
        let replacement = insertion_at(
            source,
            position,
            &render_record_member(
                field,
                initializer,
                member_indent,
                newline_for(source),
                position < source.len() || source.ends_with('\n'),
            )?,
        );
        patches.push(MigrationPatch {
            start: position,
            end: position,
            replacement,
        });
    }
    Ok(patches)
}

fn apply_file_plans(
    documents: &ProjectDocuments,
    file_plans: &[MigrationFilePlan],
) -> Result<ProjectDocuments> {
    let mut transformed = documents.clone();
    for plan in file_plans {
        let index = transformed
            .files
            .iter()
            .position(|loaded| loaded.path == plan.path)
            .ok_or_else(|| {
                migration_error(
                    "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
                    format!(
                        "planned source file `{}` is missing from the snapshot",
                        plan.path.display()
                    ),
                    Some(plan.path.clone()),
                    "MIGRATION-015",
                )
            })?;
        let source = &transformed.files[index].source;
        let patched = apply_patches(source, &plan.patches)?;
        let reparsed =
            crate::document::parse_yaml_document(plan.path.clone(), &patched).map_err(|error| {
                migration_error_from_diagnostic(
                    error.diagnostic().clone(),
                    Some(plan.path.clone()),
                    "MIGRATION-015",
                )
            })?;
        transformed.files[index] = reparsed;
    }
    Ok(transformed)
}

fn apply_patches(source: &str, patches: &[MigrationPatch]) -> Result<String> {
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
            return Err(migration_error(
                "E-MIGRATION-PATCH-INVALID",
                "migration patch ranges overlap or are not valid UTF-8 boundaries",
                None,
                "MIGRATION-015",
            ));
        }
        result.replace_range(patch.start..patch.end, &patch.replacement);
        previous_start = patch.start;
    }
    Ok(result)
}

fn render_fields_block(
    field: &FieldDefinition,
    field_indent: usize,
    newline: &str,
    trailing_newline: bool,
) -> String {
    let mut lines = vec![format!("{}fields:", spaces(field_indent))];
    lines.extend(render_field_lines(field, field_indent + 2));
    join_rendered_lines(lines, newline, trailing_newline)
}

fn render_field_block(
    field: &FieldDefinition,
    item_indent: usize,
    newline: &str,
    trailing_newline: bool,
) -> String {
    join_rendered_lines(
        render_field_lines(field, item_indent),
        newline,
        trailing_newline,
    )
}

fn render_field_lines(field: &FieldDefinition, item_indent: usize) -> Vec<String> {
    let indent = spaces(item_indent);
    let nested = spaces(item_indent + 2);
    let mut lines = vec![
        format!("{indent}- key: {}", field.key),
        format!("{nested}name: {}", field.name),
        format!("{nested}type: {}", field.type_name),
    ];
    if field.nullable {
        lines.push(format!("{nested}nullable: true"));
    }
    if field.array {
        lines.push(format!("{nested}array: true"));
    }
    lines
}

fn render_record_member(
    field: &FieldDefinition,
    initializer: &Value,
    member_indent: usize,
    newline: &str,
    trailing_newline: bool,
) -> Result<String> {
    let lines = render_value_lines(initializer, member_indent, Some(&field.name))?;
    Ok(join_rendered_lines(lines, newline, trailing_newline))
}

fn render_value_lines(value: &Value, indent: usize, key: Option<&str>) -> Result<Vec<String>> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            let prefix = key.map_or_else(String::new, |key| format!("{key}: "));
            Ok(vec![format!(
                "{}{}{}",
                spaces(indent),
                prefix,
                render_scalar(value)?
            )])
        }
        Value::Sequence(values) => {
            if values.is_empty() {
                let prefix = key.map_or_else(String::new, |key| format!("{key}: "));
                return Ok(vec![format!("{}{}[]", spaces(indent), prefix)]);
            }
            let mut lines = Vec::new();
            if let Some(key) = key {
                lines.push(format!("{}{}:", spaces(indent), key));
                lines.extend(render_sequence_items(values, indent + 2)?);
            } else {
                lines.extend(render_sequence_items(values, indent)?);
            }
            Ok(lines)
        }
        Value::Mapping(mapping) => {
            if mapping.is_empty() {
                let prefix = key.map_or_else(String::new, |key| format!("{key}: "));
                return Ok(vec![format!("{}{}{{}}", spaces(indent), prefix)]);
            }
            let mut lines = Vec::new();
            if let Some(key) = key {
                lines.push(format!("{}{}:", spaces(indent), key));
                lines.extend(render_mapping_members(mapping, indent + 2)?);
            } else {
                lines.extend(render_mapping_members(mapping, indent)?);
            }
            Ok(lines)
        }
        Value::Tagged(_) => Err(migration_error(
            "E-MIGRATION-INITIALIZER-UNSUPPORTED",
            "AddField initializer cannot contain an explicit YAML tag",
            None,
            "YAML-SUBSET-006",
        )),
    }
}

fn render_sequence_items(values: &[Value], indent: usize) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    for value in values {
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                lines.push(format!("{}- {}", spaces(indent), render_scalar(value)?));
            }
            Value::Sequence(_) | Value::Mapping(_) => {
                lines.push(format!("{}-", spaces(indent)));
                lines.extend(render_value_lines(value, indent + 2, None)?);
            }
            Value::Tagged(_) => {
                return Err(migration_error(
                    "E-MIGRATION-INITIALIZER-UNSUPPORTED",
                    "AddField initializer cannot contain an explicit YAML tag",
                    None,
                    "YAML-SUBSET-006",
                ));
            }
        }
    }
    Ok(lines)
}

fn render_mapping_members(mapping: &Mapping, indent: usize) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    for (key, value) in mapping {
        let key = key.as_str().ok_or_else(|| {
            migration_error(
                "E-MIGRATION-INITIALIZER-UNSUPPORTED",
                "Custom Type initializer mapping keys must be strings",
                None,
                "YAML-SUBSET-016",
            )
        })?;
        let key = render_mapping_key(key);
        let mut value_lines = render_value_lines(value, indent, Some(&key))?;
        lines.append(&mut value_lines);
    }
    Ok(lines)
}

fn render_mapping_key(value: &str) -> String {
    if is_table_field_name(value) || value == "$tags" {
        value.to_owned()
    } else {
        serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
    }
}

fn render_scalar(value: &Value) -> Result<String> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => serde_json::to_string(value).map_err(|error| {
            migration_error(
                "E-MIGRATION-INITIALIZER-UNSUPPORTED",
                format!("could not render string initializer: {error}"),
                None,
                "MIGRATION-006",
            )
        }),
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => Err(migration_error(
            "E-MIGRATION-INITIALIZER-UNSUPPORTED",
            "initializer value is not a scalar",
            None,
            "MIGRATION-006",
        )),
    }
}

fn replace_empty_sequence_line(
    source: &str,
    line: &SourceLine<'_>,
    key: &str,
    field: &FieldDefinition,
) -> Result<String> {
    let code_end = comment_start(line.text).unwrap_or(line.text.len());
    let code = &line.text[..code_end];
    let colon = mapping_colon(code).ok_or_else(|| {
        migration_error(
            "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
            format!("mapping key `{key}` could not be located"),
            None,
            "MIGRATION-014",
        )
    })?;
    let prefix = &line.text[..=colon];
    let comment = comment_start(line.text)
        .map(|start| format!(" {}", line.text[start..].trim_start()))
        .unwrap_or_default();
    let trailing_newline = line.next_start > line.content_end;
    let mut lines = vec![format!("{prefix}{comment}")];
    lines.extend(render_field_lines(field, yaml_indent(line.text) + 2));
    Ok(join_rendered_lines(
        lines,
        newline_for(source),
        trailing_newline,
    ))
}

fn find_target_schema<'a>(
    documents: &'a ProjectDocuments,
    table_name: &str,
) -> Option<LoadedSchema<'a>> {
    documents
        .files
        .iter()
        .find_map(|loaded| match &loaded.document {
            SourceDocument::Schema(schema) if schema.table == table_name => Some(LoadedSchema {
                path: &loaded.path,
                document: schema,
            }),
            _ => None,
        })
}

fn find_loaded_schema<'a>(
    documents: &'a ProjectDocuments,
    table_name: &str,
) -> Option<&'a LoadedDocument> {
    documents.files.iter().find(|loaded| {
        matches!(&loaded.document, SourceDocument::Schema(schema) if schema.table == table_name)
    })
}

fn target_data_documents<'a>(
    documents: &'a ProjectDocuments,
    table_name: &str,
) -> impl Iterator<Item = &'a LoadedDocument> {
    documents.files.iter().filter(move |loaded| {
        matches!(&loaded.document, SourceDocument::Data(data) if data.table == table_name)
    })
}

fn target_record_count(documents: &ProjectDocuments, table_name: &str) -> usize {
    target_data_documents(documents, table_name)
        .filter_map(|loaded| match &loaded.document {
            SourceDocument::Data(data) => Some(data.records.len()),
            _ => None,
        })
        .sum()
}

fn schema_path(documents: &ProjectDocuments, table_name: &str) -> Option<PathBuf> {
    find_loaded_schema(documents, table_name).map(|loaded| loaded.path.clone())
}

fn field_modifier(field: &FieldDefinition) -> FieldModifier {
    if field.nullable {
        FieldModifier::Nullable
    } else if field.array {
        FieldModifier::Array
    } else {
        FieldModifier::Required
    }
}

fn is_table_field_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|character| character.is_ascii_alphanumeric())
}

fn migration_error(
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

fn migration_error_from_diagnostic(
    diagnostic: Diagnostic,
    source: Option<PathBuf>,
    requirement: &str,
) -> MasterdataError {
    let mut diagnostic = diagnostic;
    if diagnostic.source.is_none() {
        diagnostic.source = source;
    }
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

fn first_diagnostic_error(
    diagnostics: Vec<Diagnostic>,
    fallback_code: &str,
    fallback_message: &str,
    requirement: &str,
) -> MasterdataError {
    diagnostics.into_iter().next().map_or_else(
        || migration_error(fallback_code, fallback_message, None, requirement),
        |diagnostic| migration_error_from_diagnostic(diagnostic, None, requirement),
    )
}

#[derive(Debug, Clone, Copy)]
struct LoadedSchema<'a> {
    path: &'a Path,
    document: &'a SchemaDocument,
}

#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    start: usize,
    content_end: usize,
    next_start: usize,
    text: &'a str,
}

#[derive(Debug, Clone)]
struct ParsedMappingEntry<'a> {
    key: String,
    raw_value: &'a str,
}

#[derive(Debug, Clone)]
struct SequenceRegion {
    indent: usize,
    items: Vec<usize>,
}

fn source_lines(source: &str) -> Vec<SourceLine<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for segment in source.split_inclusive('\n') {
        let next_start = start + segment.len();
        let content = segment.strip_suffix('\n').unwrap_or(segment);
        let content = content.strip_suffix('\r').unwrap_or(content);
        let content_end = start + content.len();
        lines.push(SourceLine {
            start,
            content_end,
            next_start,
            text: content,
        });
        start = next_start;
    }
    if source.is_empty() {
        lines.push(SourceLine {
            start: 0,
            content_end: 0,
            next_start: 0,
            text: "",
        });
    }
    lines
}

fn find_top_level_key(lines: &[SourceLine<'_>], key: &str) -> Option<usize> {
    lines.iter().position(|line| {
        yaml_indent(line.text) == 0
            && !is_ignorable_line(line.text)
            && mapping_entry(line.text).is_some_and(|entry| entry.key == key)
    })
}

fn block_region_end(lines: &[SourceLine<'_>], key_line: usize) -> usize {
    let key_indent = yaml_indent(lines[key_line].text);
    for (index, line_entry) in lines.iter().enumerate().skip(key_line + 1) {
        let line = line_entry.text;
        if is_ignorable_line(line) {
            continue;
        }
        let indent = yaml_indent(line);
        if indent <= key_indent && !is_sequence_item(line) {
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
                    indent: expected_indent,
                    items,
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
    indent.map(|indent| SequenceRegion { indent, items })
}

fn record_member_indent(lines: &[SourceLine<'_>], sequence: &SequenceRegion) -> Result<usize> {
    for &item in &sequence.items {
        let item_indent = sequence.indent;
        let item_text = lines[item].text;
        if let Some((_, rest)) = sequence_item_parts(item_text)
            && !rest.trim().is_empty()
            && mapping_colon(rest).is_some()
        {
            return Ok(item_indent + 2);
        }
        let next = sequence
            .items
            .iter()
            .copied()
            .find(|candidate| *candidate > item)
            .unwrap_or(lines.len());
        for line in &lines[(item + 1)..next] {
            if is_ignorable_line(line.text) {
                continue;
            }
            let indent = yaml_indent(line.text);
            if indent > item_indent && mapping_entry(line.text).is_some() {
                return Ok(indent);
            }
            if indent <= item_indent {
                break;
            }
        }
    }
    Err(migration_error(
        "E-MIGRATION-SOURCE-UNCLASSIFIABLE",
        "record mapping member indentation could not be located safely",
        None,
        "MIGRATION-014",
    ))
}

fn sequence_append_boundary(
    lines: &[SourceLine<'_>],
    item: usize,
    boundary: usize,
    sequence_indent: usize,
    literal_scalar_content: &[bool],
) -> usize {
    let mut index = item + 1;
    while index < boundary {
        let line = lines[index].text;
        if literal_scalar_content[index] {
            index += 1;
            continue;
        }
        if is_ignorable_line(line) {
            let next_significant = ((index + 1)..boundary)
                .find(|candidate| !is_ignorable_line(lines[*candidate].text));
            if next_significant
                .is_none_or(|candidate| yaml_indent(lines[candidate].text) <= sequence_indent)
            {
                return index;
            }
            index += 1;
            continue;
        }
        if yaml_indent(line) <= sequence_indent {
            return index;
        }
        index += 1;
    }
    boundary
}

fn literal_block_scalar_content_lines(lines: &[SourceLine<'_>]) -> Vec<bool> {
    // WHY: A `#...` line indented inside a bare literal block scalar is scalar
    // data, so it must not be mistaken for a comment that marks an AddField
    // append boundary.
    // IF REMOVED: AddField can insert a record member into the scalar body,
    // corrupting the transformed semantic snapshot or its source placement.
    // EVIDENCE: docs/specs/yaml-subset.md (YAML-SUBSET-015); docs/specs/schema-migration.md (MIGRATION-006, MIGRATION-014, MIGRATION-015)
    // Regression: add_field_appends_after_literal_block_scalar_hash_content.
    let mut content_lines = vec![false; lines.len()];
    let mut parent_indent = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(indent) = parent_indent {
            if line.text.trim().is_empty() || yaml_indent(line.text) > indent {
                content_lines[index] = true;
                continue;
            }
            parent_indent = None;
        }

        let code = strip_yaml_comment(line.text);
        if code.trim().is_empty() {
            continue;
        }
        if mapping_entry(code).is_some_and(|entry| entry.raw_value.trim() == "|") {
            parent_indent = Some(yaml_indent(line.text));
        }
    }
    content_lines
}

fn mapping_entry(line: &str) -> Option<ParsedMappingEntry<'_>> {
    let code = strip_yaml_comment(line);
    let (_, mapping) = sequence_item_parts(code).unwrap_or((false, code.trim_start()));
    let colon = mapping_colon(mapping)?;
    let key = mapping[..colon].trim();
    let key = decode_mapping_key(key)?;
    let raw_value = mapping[colon + 1..].trim_start();
    Some(ParsedMappingEntry { key, raw_value })
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

fn sequence_item_parts(line: &str) -> Option<(bool, &str)> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('-')?;
    if rest.is_empty() || rest.as_bytes().first().is_some_and(u8::is_ascii_whitespace) {
        Some((true, rest.trim_start()))
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

fn comment_start(value: &str) -> Option<usize> {
    let code = strip_yaml_comment(value);
    (code.len() < value.len()).then_some(code.len())
}

fn is_empty_flow_sequence(value: &str) -> bool {
    let value = value.trim();
    value.starts_with('[') && value.ends_with(']') && value[1..value.len() - 1].trim().is_empty()
}

fn line_start(lines: &[SourceLine<'_>], index: usize, source_len: usize) -> usize {
    lines.get(index).map_or(source_len, |line| line.start)
}

fn yaml_indent(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn spaces(count: usize) -> String {
    " ".repeat(count)
}

fn newline_for(source: &str) -> &str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn join_rendered_lines(lines: Vec<String>, newline: &str, trailing_newline: bool) -> String {
    let mut result = lines.join(newline);
    if trailing_newline {
        result.push_str(newline);
    }
    result
}

fn insertion_at(source: &str, position: usize, rendered: &str) -> String {
    if position > 0 && !source[..position].ends_with(['\n', '\r']) {
        format!("{}{}", newline_for(source), rendered)
    } else {
        rendered.to_owned()
    }
}

fn append_at_end(source: &str, rendered: &str) -> String {
    insertion_at(source, source.len(), rendered)
}
