use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_yaml::Value;

use crate::document::{DataDocument, FieldDefinition, ProjectDocuments, SchemaDocument};
use crate::error::{Diagnostic, ErrorKind, MasterdataError, Result};
use crate::type_system::{FieldModifier, ResolvedField, TypeSystem, is_csharp_reserved_keyword};

/// The resolved selector passed to the Table construction boundary.
///
/// The current application uses [`BuildSelection::unfiltered`] by default. A
/// caller that already resolved a project profile can provide the same small
/// value object to exercise the approved selection-before-constraints order
/// without making the Table resolver aware of TOML or CLI concerns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildSelection {
    include_tags: BTreeSet<String>,
    exclude_tags: BTreeSet<String>,
}

impl BuildSelection {
    pub fn unfiltered() -> Self {
        Self::default()
    }

    pub fn new<I, J, SI, SJ>(include_tags: I, exclude_tags: J) -> Result<Self>
    where
        I: IntoIterator<Item = SI>,
        J: IntoIterator<Item = SJ>,
        SI: Into<String>,
        SJ: Into<String>,
    {
        let include_tags = collect_selection_tags(include_tags, "include_tags")?;
        let exclude_tags = collect_selection_tags(exclude_tags, "exclude_tags")?;
        if include_tags.intersection(&exclude_tags).next().is_some() {
            return Err(MasterdataError::new(
                "E-BUILD-PROFILE-TAG-OVERLAP",
                ErrorKind::Config,
                "Build Selection include and exclude sets must not overlap",
            ));
        }
        Ok(Self {
            include_tags,
            exclude_tags,
        })
    }

    pub fn include_tags(&self) -> &BTreeSet<String> {
        &self.include_tags
    }

    pub fn exclude_tags(&self) -> &BTreeSet<String> {
        &self.exclude_tags
    }

    pub fn is_selected(&self, record_tags: &BTreeSet<String>) -> bool {
        (self.include_tags.is_empty()
            || record_tags
                .iter()
                .any(|tag| self.include_tags.contains(tag)))
            && !record_tags
                .iter()
                .any(|tag| self.exclude_tags.contains(tag))
    }
}

/// A record whose values have passed the schema/type validator. `$tags` is
/// deliberately absent: it is selection metadata, not domain data.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRecord {
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPrimaryKey {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSecondaryKey {
    pub fields: Vec<String>,
    pub non_unique: bool,
    /// The zero-based backend index ordinal is derived from secondary-key
    /// declaration order. It is not a persisted identity.
    pub index_no: usize,
}

/// Canonical, validated logical Table model consumed by code generation.
/// Field and member declaration order is retained in the vectors; records are
/// canonicalized by the declared Primary Key after validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTable {
    pub identity: String,
    pub csharp_name: String,
    pub fields: Vec<ResolvedField>,
    pub primary_key: ResolvedPrimaryKey,
    pub secondary_keys: Vec<ResolvedSecondaryKey>,
    pub records: Vec<ResolvedRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct TableBuild {
    pub model: Option<Vec<ResolvedTable>>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Resolve all logical tables after the Type System has been resolved.
///
/// Record tags are checked before selection, selected records are then typed,
/// and only the selected dataset participates in key constraints and ordering
/// (BUILD-SELECT-010/011, SCHEMA-TABLE-007/008).
pub fn resolve_tables(
    documents: &ProjectDocuments,
    type_system: &TypeSystem,
    selection: &BuildSelection,
) -> TableBuild {
    let mut diagnostics = Vec::new();
    let mut schemas = BTreeMap::<String, (&PathBuf, &SchemaDocument)>::new();
    let mut tables = BTreeSet::new();

    for loaded in &documents.files {
        if let Some(table) = loaded.document.table_identity()
            && !table.is_empty()
        {
            tables.insert(table.to_owned());
        }
        if let crate::document::SourceDocument::Schema(schema) = &loaded.document {
            if !is_table_name(&schema.table) {
                diagnostics.push(table_diagnostic(
                    "E-TABLE-INVALID-NAME",
                    format!("table name `{}` is not lowercase kebab-case", schema.table),
                    &loaded.path,
                    "SCHEMA-TABLE-002",
                ));
            }
            if let Some(previous) = schemas.insert(schema.table.clone(), (&loaded.path, schema)) {
                diagnostics.push(
                    table_diagnostic(
                        "E-TABLE-DUPLICATE-SCHEMA",
                        format!(
                            "table `{}` has more than one schema document (also in {})",
                            schema.table,
                            previous.0.display()
                        ),
                        &loaded.path,
                        "SCHEMA-TABLE-001",
                    )
                    .with_schema_path(format!("table:{}", schema.table)),
                );
            }
        }
    }

    for (path, data) in documents.data() {
        if !schemas.contains_key(&data.table) {
            diagnostics.push(table_diagnostic(
                "E-TABLE-UNKNOWN-DATA-TABLE",
                format!("data document refers to unknown table `{}`", data.table),
                path,
                "SCHEMA-TABLE-001",
            ));
            validate_record_tags(data, path, &mut diagnostics);
        }
    }

    let mut resolved_tables = Vec::with_capacity(schemas.len());
    for (table_name, (schema_path, schema)) in schemas {
        let resolved_fields =
            resolve_schema_fields(schema, type_system, schema_path, &mut diagnostics);
        let field_by_name = resolved_fields
            .iter()
            .map(|field| (field.name.as_str(), field.clone()))
            .collect::<BTreeMap<_, _>>();

        let primary_key = resolve_primary_key(
            schema,
            &resolved_fields,
            &field_by_name,
            type_system,
            schema_path,
            &mut diagnostics,
        );
        let secondary_keys = resolve_secondary_keys(
            schema,
            &resolved_fields,
            &field_by_name,
            type_system,
            schema_path,
            &mut diagnostics,
        );

        let Some(primary_key_fields) = primary_key else {
            continue;
        };
        let Some(secondary_key_fields) = secondary_keys else {
            continue;
        };

        validate_secondary_shape_constraints(
            &primary_key_fields,
            &secondary_key_fields,
            schema_path,
            &mut diagnostics,
        );
        validate_query_name_collisions(
            &primary_key_fields,
            &secondary_key_fields,
            schema_path,
            &mut diagnostics,
        );

        let mut selected_records = Vec::new();
        for (data_path, data) in documents
            .data()
            .filter(|(_, data)| data.table == table_name)
        {
            for (record_index, record) in data.records.iter().enumerate() {
                let tags = match record_tags(record, data_path, record_index, &mut diagnostics) {
                    Some(tags) => tags,
                    None => continue,
                };
                if !selection.is_selected(&tags) {
                    continue;
                }
                if let Some(validated) = validate_record(
                    record,
                    &resolved_fields,
                    type_system,
                    data_path,
                    record_index,
                    &mut diagnostics,
                ) {
                    selected_records.push(RecordCandidate {
                        record: ResolvedRecord { fields: validated },
                        path: data_path,
                        record_index,
                    });
                }
            }
        }

        validate_key_uniqueness(
            &selected_records,
            &primary_key_fields,
            &secondary_key_fields,
            type_system,
            table_name.as_str(),
            &mut diagnostics,
        );

        if let Err(error) =
            sort_records_by_primary_key(&mut selected_records, &primary_key_fields, type_system)
        {
            diagnostics.push(
                table_diagnostic(
                    "E-TABLE-KEY-COMPARISON",
                    error.diagnostic().message.clone(),
                    schema_path,
                    "SCHEMA-TABLE-008",
                )
                .with_schema_path(format!("table:{}", table_name)),
            );
        }

        let csharp_name = schema
            .csharp_name
            .clone()
            .unwrap_or_else(|| table_to_csharp_name(&schema.table));
        for field in &resolved_fields {
            let property_name = uppercase_first_ascii(&field.name);
            if property_name == csharp_name {
                diagnostics.push(table_diagnostic(
                    "E-TABLE-GENERATED-MEMBER-COLLISION",
                    format!(
                        "Table field `{}` generates property `{property_name}`, which collides with generated type `{csharp_name}`",
                        field.name
                    ),
                    schema_path,
                    "SCHEMA-TABLE-003",
                ));
            }
        }
        resolved_tables.push(ResolvedTable {
            identity: table_name,
            csharp_name,
            fields: resolved_fields,
            primary_key: ResolvedPrimaryKey {
                fields: primary_key_fields
                    .iter()
                    .map(|field| field.name.clone())
                    .collect(),
            },
            secondary_keys: secondary_key_fields
                .iter()
                .map(|(fields, non_unique, index_no)| ResolvedSecondaryKey {
                    fields: fields.iter().map(|field| field.name.clone()).collect(),
                    non_unique: *non_unique,
                    index_no: *index_no,
                })
                .collect(),
            records: selected_records
                .into_iter()
                .map(|candidate| candidate.record)
                .collect(),
        });
    }

    if diagnostics.is_empty() {
        TableBuild {
            model: Some(resolved_tables),
            diagnostics,
        }
    } else {
        TableBuild {
            model: None,
            diagnostics,
        }
    }
}

fn collect_selection_tags<I, S>(tags: I, field: &str) -> Result<BTreeSet<String>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut result = BTreeSet::new();
    for tag in tags {
        let tag = tag.into();
        if !is_tag_name(&tag) {
            return Err(MasterdataError::new(
                "E-BUILD-PROFILE-INVALID-TAG",
                ErrorKind::Config,
                format!("{field} contains invalid tag `{tag}`"),
            ));
        }
        if !result.insert(tag.clone()) {
            return Err(MasterdataError::new(
                "E-BUILD-PROFILE-DUPLICATE-TAG",
                ErrorKind::Config,
                format!("{field} contains duplicate tag `{tag}`"),
            ));
        }
    }
    Ok(result)
}

fn resolve_schema_fields(
    schema: &SchemaDocument,
    type_system: &TypeSystem,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ResolvedField> {
    let mut keys = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut fields = Vec::with_capacity(schema.fields.len());
    for field in &schema.fields {
        if !keys.insert(field.key) {
            diagnostics.push(table_diagnostic(
                "E-TABLE-DUPLICATE-FIELD-KEY",
                format!(
                    "duplicate MessagePack key {} in table `{}`",
                    field.key, schema.table
                ),
                path,
                "SCHEMA-KEY-001",
            ));
        }
        if !names.insert(field.name.clone()) {
            diagnostics.push(table_diagnostic(
                "E-TABLE-DUPLICATE-FIELD-NAME",
                format!(
                    "duplicate field name `{}` in table `{}`",
                    field.name, schema.table
                ),
                path,
                "SCHEMA-TABLE-003",
            ));
        }
        if !is_table_field_name(&field.name) || is_csharp_reserved_keyword(&field.name) {
            diagnostics.push(table_diagnostic(
                "E-TABLE-INVALID-FIELD-NAME",
                format!(
                    "field name `{}` is not a valid Table source name",
                    field.name
                ),
                path,
                "SCHEMA-TABLE-003",
            ));
        }
        if field.nullable && field.array {
            diagnostics.push(table_diagnostic(
                "E-TABLE-INVALID-FIELD-MODIFIERS",
                format!("field `{}` cannot be both nullable and array", field.name),
                path,
                "TYPE-FIELD-002",
            ));
        }
        let Some(base_type) = type_system.resolve_reference(&field.type_name) else {
            diagnostics.push(table_diagnostic(
                "E-TYPE-UNKNOWN-REFERENCE",
                format!("unknown type reference `{}`", field.type_name),
                path,
                "SCHEMA-TABLE-003",
            ));
            continue;
        };
        fields.push(ResolvedField {
            key: field.key,
            name: field.name.clone(),
            base_type,
            modifier: table_modifier(field),
        });
    }
    fields
}

fn resolve_primary_key(
    schema: &SchemaDocument,
    _resolved_fields: &[ResolvedField],
    field_by_name: &BTreeMap<&str, ResolvedField>,
    type_system: &TypeSystem,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<ResolvedField>> {
    let Some(primary_key) = &schema.primary_key else {
        diagnostics.push(table_diagnostic(
            "E-TABLE-MISSING-PRIMARY-KEY",
            format!(
                "table `{}` must declare exactly one primaryKey",
                schema.table
            ),
            path,
            "INDEX-PRIMARY-001",
        ));
        return None;
    };
    if primary_key.fields.is_empty() {
        diagnostics.push(table_diagnostic(
            "E-TABLE-EMPTY-PRIMARY-KEY",
            format!(
                "table `{}` primaryKey.fields must not be empty",
                schema.table
            ),
            path,
            "INDEX-PRIMARY-001",
        ));
        return None;
    }
    resolve_key_fields(
        &primary_key.fields,
        field_by_name,
        type_system,
        path,
        diagnostics,
        "primary",
    )
}

type ResolvedSecondaryFields = Vec<(Vec<ResolvedField>, bool, usize)>;

fn resolve_secondary_keys(
    schema: &SchemaDocument,
    _resolved_fields: &[ResolvedField],
    field_by_name: &BTreeMap<&str, ResolvedField>,
    type_system: &TypeSystem,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedSecondaryFields> {
    let mut result = Vec::with_capacity(schema.secondary_keys.len());
    let mut shapes = BTreeSet::new();
    for (index_no, secondary) in schema.secondary_keys.iter().enumerate() {
        if secondary.fields.is_empty() {
            diagnostics.push(table_diagnostic(
                "E-TABLE-EMPTY-SECONDARY-KEY",
                format!("secondaryKeys[{index_no}].fields must not be empty"),
                path,
                "INDEX-SECONDARY-001",
            ));
            continue;
        }
        let Some(fields) = resolve_key_fields(
            &secondary.fields,
            field_by_name,
            type_system,
            path,
            diagnostics,
            "secondary",
        ) else {
            continue;
        };
        let shape = secondary.fields.clone();
        if !shapes.insert(shape) {
            diagnostics.push(table_diagnostic(
                "E-TABLE-DUPLICATE-SECONDARY-SHAPE",
                format!(
                    "secondary key fields {:?} are declared more than once",
                    secondary.fields
                ),
                path,
                "INDEX-SECONDARY-002",
            ));
        }
        result.push((fields, secondary.non_unique, index_no));
    }
    Some(result)
}

fn resolve_key_fields(
    names: &[String],
    field_by_name: &BTreeMap<&str, ResolvedField>,
    type_system: &TypeSystem,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    key_kind: &str,
) -> Option<Vec<ResolvedField>> {
    let mut seen = BTreeSet::new();
    let mut fields = Vec::with_capacity(names.len());
    let mut valid = true;
    for name in names {
        if !seen.insert(name.as_str()) {
            diagnostics.push(table_diagnostic(
                if key_kind == "primary" {
                    "E-TABLE-DUPLICATE-PRIMARY-COMPONENT"
                } else {
                    "E-TABLE-DUPLICATE-SECONDARY-COMPONENT"
                },
                format!("{key_kind} key contains field `{name}` more than once"),
                path,
                if key_kind == "primary" {
                    "INDEX-PRIMARY-001"
                } else {
                    "INDEX-SECONDARY-001"
                },
            ));
            valid = false;
            continue;
        }
        let Some(field) = field_by_name.get(name.as_str()).cloned() else {
            diagnostics.push(table_diagnostic(
                if key_kind == "primary" {
                    "E-TABLE-UNKNOWN-PRIMARY-FIELD"
                } else {
                    "E-TABLE-UNKNOWN-SECONDARY-FIELD"
                },
                format!("{key_kind} key refers to unknown field `{name}`"),
                path,
                if key_kind == "primary" {
                    "INDEX-PRIMARY-001"
                } else {
                    "INDEX-SECONDARY-001"
                },
            ));
            valid = false;
            continue;
        };
        if !(type_system.is_field_key_compatible(&field.base_type, field.modifier)
            && type_system.is_field_comparison_capable(&field.base_type, field.modifier))
        {
            diagnostics.push(table_diagnostic(
                if key_kind == "primary" {
                    "E-TABLE-INVALID-PRIMARY-CAPABILITY"
                } else {
                    "E-TABLE-INVALID-SECONDARY-CAPABILITY"
                },
                format!(
                    "{key_kind} key field `{name}` must be a required scalar with key and comparison capability"
                ),
                path,
                if key_kind == "primary" {
                    "INDEX-PRIMARY-003"
                } else {
                    "INDEX-SECONDARY-003"
                },
            ));
            valid = false;
        }
        fields.push(field);
    }
    valid.then_some(fields)
}

fn validate_query_name_collisions(
    primary: &[ResolvedField],
    secondary: &[(Vec<ResolvedField>, bool, usize)],
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = BTreeMap::<String, String>::new();
    let primary_name = query_name(primary);
    names.insert(primary_name, "primary key".to_owned());
    for (fields, _, index_no) in secondary {
        let query_name = query_name(fields);
        if let Some(previous) =
            names.insert(query_name.clone(), format!("secondary key {index_no}"))
        {
            diagnostics.push(table_diagnostic(
                "E-TABLE-QUERY-NAME-COLLISION",
                format!("generated query `{query_name}` collides with {previous}"),
                path,
                "INDEX-SECONDARY-005",
            ));
        }
    }
}

fn validate_secondary_shape_constraints(
    primary: &[ResolvedField],
    secondary: &[(Vec<ResolvedField>, bool, usize)],
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let primary_shape = primary
        .iter()
        .map(|field| field.name.as_str())
        .collect::<Vec<_>>();
    for (fields, _, index_no) in secondary {
        let secondary_shape = fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>();
        if primary_shape == secondary_shape {
            diagnostics.push(table_diagnostic(
                "E-TABLE-SECONDARY-PRIMARY-SHAPE",
                format!("secondary key {index_no} has the same field shape as the Primary Key"),
                path,
                "INDEX-SECONDARY-002",
            ));
        }
    }
}

fn query_name(fields: &[ResolvedField]) -> String {
    let mut result = String::from("FindBy");
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            result.push_str("And");
        }
        result.push_str(&uppercase_first_ascii(&field.name));
    }
    result
}

fn validate_key_uniqueness(
    records: &[RecordCandidate<'_>],
    primary: &[ResolvedField],
    secondary: &[(Vec<ResolvedField>, bool, usize)],
    type_system: &TypeSystem,
    table_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for left_index in 0..records.len() {
        for right_index in (left_index + 1)..records.len() {
            if compare_record_key(
                &records[left_index].record,
                &records[right_index].record,
                primary,
                type_system,
            )
            .is_ok_and(|ordering| ordering == Ordering::Equal)
            {
                diagnostics.push(key_duplicate_diagnostic(
                    "E-TABLE-DUPLICATE-PRIMARY-VALUE",
                    format!("table `{table_name}` contains duplicate Primary Key values"),
                    &records[right_index],
                    "INDEX-PRIMARY-001",
                ));
            }
            for (fields, non_unique, _) in secondary {
                if !non_unique
                    && compare_record_key(
                        &records[left_index].record,
                        &records[right_index].record,
                        fields,
                        type_system,
                    )
                    .is_ok_and(|ordering| ordering == Ordering::Equal)
                {
                    diagnostics.push(key_duplicate_diagnostic(
                        "E-TABLE-DUPLICATE-UNIQUE-SECONDARY-VALUE",
                        format!(
                            "table `{table_name}` contains duplicate unique Secondary Key values"
                        ),
                        &records[right_index],
                        "INDEX-UNIQUE-001",
                    ));
                }
            }
        }
    }
}

fn sort_records_by_primary_key(
    records: &mut [RecordCandidate<'_>],
    primary: &[ResolvedField],
    type_system: &TypeSystem,
) -> Result<()> {
    let mut comparison_error = None;
    records.sort_by(|left, right| {
        match compare_record_key(&left.record, &right.record, primary, type_system) {
            Ok(ordering) => ordering,
            Err(error) => {
                comparison_error = Some(error);
                Ordering::Equal
            }
        }
    });
    comparison_error.map_or(Ok(()), Err)
}

fn compare_record_key(
    left: &ResolvedRecord,
    right: &ResolvedRecord,
    fields: &[ResolvedField],
    type_system: &TypeSystem,
) -> Result<Ordering> {
    for field in fields {
        let left_value = left.fields.get(&field.name).ok_or_else(|| {
            MasterdataError::new(
                "E-TABLE-KEY-VALUE-MISSING",
                ErrorKind::Validation,
                format!("validated record is missing key field `{}`", field.name),
            )
        })?;
        let right_value = right.fields.get(&field.name).ok_or_else(|| {
            MasterdataError::new(
                "E-TABLE-KEY-VALUE-MISSING",
                ErrorKind::Validation,
                format!("validated record is missing key field `{}`", field.name),
            )
        })?;
        let ordering = type_system.compare_field_values(field, left_value, right_value)?;
        if ordering != Ordering::Equal {
            return Ok(ordering);
        }
    }
    Ok(Ordering::Equal)
}

fn validate_record(
    record: &BTreeMap<String, Value>,
    fields: &[ResolvedField],
    type_system: &TypeSystem,
    path: &Path,
    record_index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<BTreeMap<String, Value>> {
    let field_names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut valid = true;
    for name in record.keys() {
        if name != "$tags" && !field_names.contains(name.as_str()) {
            diagnostics.push(record_diagnostic(
                "E-TABLE-UNKNOWN-RECORD-FIELD",
                format!("record contains unknown field `{name}`"),
                path,
                record_index,
                "SCHEMA-TABLE-006",
            ));
            valid = false;
        }
    }
    let mut values = BTreeMap::new();
    for field in fields {
        let Some(value) = record.get(&field.name) else {
            diagnostics.push(record_diagnostic(
                "E-TABLE-MISSING-REQUIRED-FIELD",
                format!("record omits field `{}`", field.name),
                path,
                record_index,
                "SCHEMA-TABLE-006",
            ));
            valid = false;
            continue;
        };
        if let Err(error) = type_system.validate_field_value(field, value) {
            diagnostics.push(
                record_diagnostic(
                    "E-TABLE-INVALID-RECORD-VALUE",
                    format!(
                        "field `{}` is invalid: {}",
                        field.name,
                        error.diagnostic().message
                    ),
                    path,
                    record_index,
                    "SCHEMA-TABLE-006",
                )
                .with_related_requirement(requirement_for_field_modifier(field.modifier)),
            );
            valid = false;
        }
        if field.name != "$tags" {
            values.insert(field.name.clone(), value.clone());
        }
    }
    valid.then_some(values)
}

fn requirement_for_field_modifier(modifier: FieldModifier) -> &'static str {
    match modifier {
        FieldModifier::Required => "TYPE-FIELD-003",
        FieldModifier::Nullable => "TYPE-FIELD-004",
        FieldModifier::Array => "TYPE-FIELD-005",
    }
}

fn validate_record_tags(data: &DataDocument, path: &Path, diagnostics: &mut Vec<Diagnostic>) {
    for (index, record) in data.records.iter().enumerate() {
        let _ = record_tags(record, path, index, diagnostics);
    }
}

fn record_tags(
    record: &BTreeMap<String, Value>,
    path: &Path,
    record_index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<BTreeSet<String>> {
    let Some(value) = record.get("$tags") else {
        return Some(BTreeSet::new());
    };
    let Some(sequence) = value.as_sequence() else {
        diagnostics.push(record_diagnostic(
            "E-BUILD-RECORD-TAGS-SHAPE",
            "$tags must be a YAML sequence",
            path,
            record_index,
            "BUILD-SELECT-003",
        ));
        return None;
    };
    let mut tags = BTreeSet::new();
    let mut valid = true;
    for tag in sequence {
        let Some(tag) = tag.as_str() else {
            diagnostics.push(record_diagnostic(
                "E-BUILD-RECORD-TAGS-TYPE",
                "$tags entries must be strings",
                path,
                record_index,
                "BUILD-SELECT-002",
            ));
            valid = false;
            continue;
        };
        if !is_tag_name(tag) {
            diagnostics.push(record_diagnostic(
                "E-BUILD-RECORD-TAGS-NAME",
                format!("record tag `{tag}` is not lowercase kebab-case"),
                path,
                record_index,
                "BUILD-SELECT-002",
            ));
            valid = false;
        }
        if !tags.insert(tag.to_owned()) {
            diagnostics.push(record_diagnostic(
                "E-BUILD-RECORD-TAGS-DUPLICATE",
                format!("record tag `{tag}` is duplicated"),
                path,
                record_index,
                "BUILD-SELECT-004",
            ));
            valid = false;
        }
    }
    valid.then_some(tags)
}

fn key_duplicate_diagnostic(
    code: &str,
    message: String,
    record: &RecordCandidate<'_>,
    requirement: &str,
) -> Diagnostic {
    record_diagnostic(code, message, record.path, record.record_index, requirement)
}

fn record_diagnostic(
    code: &str,
    message: impl Into<String>,
    path: &Path,
    record_index: usize,
    requirement: &str,
) -> Diagnostic {
    Diagnostic::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_record_identity(format!("record[{record_index}]"))
        .with_related_requirement(requirement)
}

fn table_diagnostic(
    code: &str,
    message: impl Into<String>,
    path: &Path,
    requirement: &str,
) -> Diagnostic {
    Diagnostic::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement(requirement)
}

fn table_modifier(field: &FieldDefinition) -> FieldModifier {
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

fn is_table_name(value: &str) -> bool {
    let mut segments = value.split('-');
    let Some(first) = segments.next() else {
        return false;
    };
    is_lower_alphanumeric_segment(first) && segments.all(is_lower_alphanumeric_segment)
}

fn is_lower_alphanumeric_segment(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn is_tag_name(value: &str) -> bool {
    let mut segments = value.split('-');
    let Some(first) = segments.next() else {
        return false;
    };
    is_lower_alphanumeric_segment(first)
        && segments.all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn table_to_csharp_name(value: &str) -> String {
    value.split('-').map(uppercase_first_ascii).collect()
}

fn uppercase_first_ascii(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

struct RecordCandidate<'a> {
    record: ResolvedRecord,
    path: &'a Path,
    record_index: usize,
}
