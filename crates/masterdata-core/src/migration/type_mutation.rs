//! 型の操作はsemantic ASTで決定し、source locatorは位置の対応だけを担当する。
use super::*;
use crate::{
    ConversionDefinition, EnumMember, SourceCommitCandidate, TypeDocument, TypeFieldDefinition,
    resolve_type_system,
};
mod source;

#[derive(Debug, Clone, PartialEq)]
pub struct TypeMigrationCommand {
    pub target: String,
    pub operation: TypeMigrationOperation,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TypeMigrationOperation {
    SetValueObjectConversions(ConversionDefinition),
    AddEnumMember(EnumMember),
    RenameEnumMember {
        member: String,
        new_name: String,
    },
    DropEnumMember {
        member: String,
    },
    AddCustomField {
        field: TypeFieldDefinition,
        initializer: Option<Value>,
    },
    RenameCustomField {
        field: String,
        new_name: String,
    },
    DropCustomField {
        field: String,
    },
}
impl TypeMigrationOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SetValueObjectConversions(_) => "SetValueObjectConversions",
            Self::AddEnumMember(_) => "AddEnumMember",
            Self::RenameEnumMember { .. } => "RenameEnumMember",
            Self::DropEnumMember { .. } => "DropEnumMember",
            Self::AddCustomField { .. } => "AddCustomField",
            Self::RenameCustomField { .. } => "RenameCustomField",
            Self::DropCustomField { .. } => "DropCustomField",
        }
    }
    pub fn selector(&self) -> &str {
        match self {
            Self::SetValueObjectConversions(_) => "conversions",
            Self::AddEnumMember(member) => &member.name,
            Self::RenameEnumMember { member, .. } | Self::DropEnumMember { member } => member,
            Self::AddCustomField { field, .. } => &field.name,
            Self::RenameCustomField { field, .. } | Self::DropCustomField { field } => field,
        }
    }
    pub fn destructive(&self) -> bool {
        matches!(
            self,
            Self::DropEnumMember { .. } | Self::DropCustomField { .. }
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMigrationDryRun {
    pub command: TypeMigrationCommand,
    pub affected_occurrence_count: usize,
    pub candidate: SourceCommitCandidate,
}
fn failure(message: impl Into<String>) -> MasterdataError {
    migration_error("E-TYPE-MIGRATION", message, None, "TYPE-MIGRATION-014")
}

// WHY: reverse dependencies identify occurrences; forward closure validates their
// declarations without allowing unrelated invalid types to become a global gate.
// All classified bytes are still retained for stale preflight (TYPE-MIGRATION-015).
// EVIDENCE: docs/specs/type-migration.md (TYPE-MIGRATION-014).
fn closure(
    documents: &ProjectDocuments,
    target: &str,
    extra: Option<&str>,
) -> Result<(ProjectDocuments, BTreeSet<String>)> {
    let mut dependent = BTreeSet::from([target.to_owned()]);
    loop {
        let before = dependent.len();
        for (_, ty) in documents.types() {
            if ty
                .custom
                .as_ref()
                .is_some_and(|c| c.fields.iter().any(|f| dependent.contains(&f.type_name)))
            {
                dependent.insert(ty.name.clone());
            }
        }
        if dependent.len() == before {
            break;
        }
    }
    let mut pending = dependent.clone();
    if let Some(extra) = extra {
        add_named_type(&mut pending, extra);
    }
    let mut names = BTreeSet::new();
    while let Some(name) = pending.pop_first() {
        if !names.insert(name.clone()) {
            continue;
        }
        let matches: Vec<_> = documents
            .types()
            .filter(|(_, ty)| ty.name == name)
            .collect();
        if matches.len() != 1 {
            return Err(failure(format!(
                "type `{name}` cannot be resolved uniquely"
            )));
        }
        if let Some(custom) = &matches[0].1.custom {
            for f in &custom.fields {
                add_named_type(&mut pending, &f.type_name);
            }
        }
    }
    // Unknown references cannot safely be classified as unrelated to the target.
    fn known(documents: &ProjectDocuments, name: &str, seen: &mut BTreeSet<String>) -> bool {
        if PrimitiveType::parse(name).is_some() || !seen.insert(name.into()) {
            return true;
        }
        let declarations: Vec<_> = documents
            .types()
            .filter(|(_, ty)| ty.name == name)
            .collect();
        declarations.len() == 1
            && declarations[0].1.custom.as_ref().is_none_or(|c| {
                c.fields
                    .iter()
                    .all(|f| known(documents, &f.type_name, seen))
            })
    }
    for (_, data) in documents.data() {
        let schemas: Vec<_> = documents
            .schemas()
            .filter(|(_, s)| s.table == data.table)
            .collect();
        if schemas.len() != 1
            || schemas[0]
                .1
                .fields
                .iter()
                .any(|f| !known(documents, &f.type_name, &mut BTreeSet::new()))
        {
            return Err(failure(format!(
                "data table `{}` dependencies cannot be classified",
                data.table
            )));
        }
    }
    let files = documents
        .files
        .iter()
        .filter(
            |file| matches!(&file.document, SourceDocument::Type(ty) if names.contains(&ty.name)),
        )
        .cloned()
        .collect();
    Ok((ProjectDocuments { files }, dependent))
}
pub fn migration_type_declaration(
    documents: &ProjectDocuments,
    target: &str,
) -> Result<TypeDocument> {
    let (scope, _) = closure(documents, target, None)?;
    resolve_type_system(&scope)?;
    Ok(scope
        .types()
        .find(|(_, ty)| ty.name == target)
        .ok_or_else(|| failure("target type not found"))?
        .1
        .clone())
}

pub fn dry_run_type_migration(
    documents: &ProjectDocuments,
    command: &TypeMigrationCommand,
) -> Result<TypeMigrationDryRun> {
    use TypeMigrationOperation as Op;
    let extra = match &command.operation {
        Op::AddCustomField { field, .. } => Some(field.type_name.as_str()),
        _ => None,
    };
    let (scope, dependent) = closure(documents, &command.target, extra)?;
    let types = resolve_type_system(&scope)?;
    let mut expected = documents.clone();
    let target = expected
        .files
        .iter_mut()
        .find(|f| matches!(&f.document, SourceDocument::Type(t) if t.name == command.target))
        .ok_or_else(|| failure("type not found"))?;
    let SourceDocument::Type(ty) = &mut target.document else {
        unreachable!()
    };
    match &command.operation {
        Op::SetValueObjectConversions(value) => {
            ty.value_object
                .as_mut()
                .ok_or_else(|| failure("target is not a Value Object"))?
                .conversions = value.clone()
        }
        Op::AddEnumMember(member) => members_mut(ty)?.push(member.clone()),
        Op::RenameEnumMember { member, new_name } => {
            protected_none(ty, member)?;
            members_mut(ty)?
                .iter_mut()
                .find(|m| &m.name == member)
                .ok_or_else(|| failure("member not found"))?
                .name = new_name.clone();
        }
        Op::DropEnumMember { member } => {
            protected_none(ty, member)?;
            let members = members_mut(ty)?;
            let index = members
                .iter()
                .position(|m| &m.name == member)
                .ok_or_else(|| failure("member not found"))?;
            members.remove(index);
        }
        Op::AddCustomField { field, .. } => ty
            .custom
            .as_mut()
            .ok_or_else(|| failure("target is not Custom Type"))?
            .fields
            .push(field.clone()),
        Op::RenameCustomField { field, new_name } => {
            ty.custom
                .as_mut()
                .ok_or_else(|| failure("target is not Custom Type"))?
                .fields
                .iter_mut()
                .find(|f| &f.name == field)
                .ok_or_else(|| failure("field not found"))?
                .name = new_name.clone();
        }
        Op::DropCustomField { field } => {
            let fields = &mut ty
                .custom
                .as_mut()
                .ok_or_else(|| failure("target is not Custom Type"))?
                .fields;
            let index = fields
                .iter()
                .position(|f| &f.name == field)
                .ok_or_else(|| failure("field not found"))?;
            fields.remove(index);
        }
    }
    let (after_scope, _) = closure(&expected, &command.target, extra)?;
    let after_types = resolve_type_system(&after_scope)?;
    if let Op::AddCustomField {
        field,
        initializer: Some(value),
    } = &command.operation
    {
        let resolved = resolve_field(
            &after_types,
            &field.name,
            &field.type_name,
            field.key,
            field.nullable,
            field.array,
        )?;
        after_types.validate_field_value(&resolved, value)?;
    }
    let mut count = 0;
    for loaded in &mut expected.files {
        // Settings and member append do not change data representation or existing
        // symbolic occurrences. Unrelated value errors are not their precondition.
        // EVIDENCE: TYPE-MIGRATION-005; TYPE-MIGRATION-006; TYPE-MIGRATION-014.
        if matches!(
            command.operation,
            Op::SetValueObjectConversions(_) | Op::AddEnumMember(_)
        ) {
            continue;
        }
        let SourceDocument::Data(data) = &mut loaded.document else {
            continue;
        };
        let schema = documents
            .schemas()
            .find(|(_, s)| s.table == data.table)
            .expect("classified table")
            .1;
        for (index, record) in data.records.iter_mut().enumerate() {
            for f in &schema.fields {
                if !dependent.contains(&f.type_name) {
                    continue;
                }
                let value = record.get_mut(&f.name).ok_or_else(|| {
                    failure(format!("{} record {index} missing {}", data.table, f.name))
                })?;
                let resolved =
                    resolve_field(&types, &f.name, &f.type_name, f.key, f.nullable, f.array)?;
                transform_field(&types, &resolved, value, command, &dependent, &mut count)?;
            }
        }
    }
    let mut file_plans = Vec::new();
    let mut transformed = documents.clone();
    for ((before, desired), actual) in documents
        .files
        .iter()
        .zip(&expected.files)
        .zip(&mut transformed.files)
    {
        if before.document == desired.document {
            continue;
        }
        let patches = source::patch_document(before, &desired.document, command)?;
        let file_plan = MigrationFilePlan {
            path: before.path.clone(),
            patches,
        };
        let one = apply_file_plans(documents, std::slice::from_ref(&file_plan))?;
        *actual = one
            .files
            .into_iter()
            .find(|f| f.path == before.path)
            .expect("patched file");
        if actual.document != desired.document {
            return Err(failure("source patch differs from expected semantic state")
                .with_source(before.path.clone()));
        }
        file_plans.push(file_plan);
    }
    // Re-resolve the reparsed candidate, independently of the expected AST.
    let (post, _) = closure(&transformed, &command.target, extra)?;
    resolve_type_system(&post)?;
    if let Some(diagnostic) = crate::validate_computed_views(&transformed)
        .into_iter()
        .next()
    {
        return Err(failure(format!(
            "Computed View validation failed after Type Migration: {}",
            diagnostic.message
        )));
    }
    validate_affected_references(&transformed, &dependent)?;
    file_plans.sort_by(|a, b| a.path.cmp(&b.path));
    let mut source_inputs: Vec<_> = documents.files.iter().map(|f| f.path.clone()).collect();
    source_inputs.sort();
    Ok(TypeMigrationDryRun {
        command: command.clone(),
        affected_occurrence_count: count,
        candidate: SourceCommitCandidate {
            source_inputs,
            destructive: command.operation.destructive(),
            affected_files: file_plans,
            transformed_documents: transformed,
        },
    })
}

// Reference components are limited to primitive, Value Object, and Enum
// semantics, but a Type Migration can still transform an Enum/Value Object
// used by a source or target key. Re-resolve only the affected relationship
// closure after the canonical source patches are reparsed; unrelated invalid
// tables remain outside Type Migration's existing closure contract.
fn validate_affected_references(
    documents: &ProjectDocuments,
    dependent: &BTreeSet<String>,
) -> Result<()> {
    let schemas = documents.schemas().collect::<Vec<_>>();
    let mut tables = BTreeSet::new();
    for (_, schema) in &schemas {
        let affected = schema.references.iter().any(|reference| {
            let source_affected = reference.fields.iter().any(|field_name| {
                schema
                    .fields
                    .iter()
                    .find(|field| field.name == *field_name)
                    .is_some_and(|field| dependent.contains(&field.type_name))
            });
            let target_affected = schemas
                .iter()
                .find(|(_, target)| target.table == reference.target.table)
                .is_some_and(|(_, target)| {
                    reference.target.fields.iter().any(|field_name| {
                        target
                            .fields
                            .iter()
                            .find(|field| field.name == *field_name)
                            .is_some_and(|field| dependent.contains(&field.type_name))
                    })
                });
            source_affected || target_affected
        });
        if affected {
            tables.insert(schema.table.clone());
        }
    }
    if tables.is_empty() {
        return Ok(());
    }

    // Include the transitive target closure because resolving an affected
    // table must not silently omit another Reference declaration on a target
    // table. An unknown target remains absent and therefore fails closed in
    // the shared resolver.
    loop {
        let before = tables.len();
        for (_, schema) in &schemas {
            if tables.contains(&schema.table) {
                tables.extend(
                    schema
                        .references
                        .iter()
                        .map(|reference| reference.target.table.clone()),
                );
            }
        }
        if tables.len() == before {
            break;
        }
    }

    let mut type_names = BTreeSet::new();
    for (_, schema) in &schemas {
        if tables.contains(&schema.table) {
            type_names.extend(schema.fields.iter().map(|field| field.type_name.clone()));
        }
    }
    loop {
        let before = type_names.len();
        for (_, type_document) in documents.types() {
            if type_names.contains(&type_document.name)
                && let Some(custom) = &type_document.custom
            {
                type_names.extend(custom.fields.iter().map(|field| field.type_name.clone()));
            }
        }
        if type_names.len() == before {
            break;
        }
    }

    let scoped = ProjectDocuments {
        files: documents
            .files
            .iter()
            .filter(|loaded| match &loaded.document {
                SourceDocument::Schema(schema) => tables.contains(&schema.table),
                SourceDocument::Data(data) => tables.contains(&data.table),
                SourceDocument::Type(type_document) => type_names.contains(&type_document.name),
                SourceDocument::View(_) => false,
            })
            .cloned()
            .collect(),
    };
    let type_build = build_type_system(&scoped);
    let type_system = type_build.model.ok_or_else(|| {
        first_diagnostic_error(
            type_build.diagnostics,
            "E-TYPE-MIGRATION-REFERENCE",
            "affected Reference type closure could not be resolved after Type Migration",
            "TYPE-MIGRATION-014",
        )
    })?;
    let table_build = resolve_tables(&scoped, &type_system, &BuildSelection::unfiltered());
    if table_build.diagnostics.is_empty() {
        Ok(())
    } else {
        Err(first_diagnostic_error(
            table_build.diagnostics,
            "E-TYPE-MIGRATION-REFERENCE",
            "affected Reference semantics are invalid after Type Migration",
            "TYPE-MIGRATION-014",
        ))
    }
}

fn members_mut(ty: &mut TypeDocument) -> Result<&mut Vec<EnumMember>> {
    if let Some(e) = &mut ty.enum_definition {
        Ok(&mut e.members)
    } else if let Some(f) = &mut ty.flags {
        Ok(&mut f.members)
    } else {
        Err(failure("target is not Enum or Flags"))
    }
}
fn protected_none(ty: &TypeDocument, member: &str) -> Result<()> {
    if ty.flags.is_some() && member == "None" {
        Err(failure("Flags None = 0 cannot be renamed or dropped"))
    } else {
        Ok(())
    }
}
fn resolve_field(
    types: &TypeSystem,
    name: &str,
    ty: &str,
    key: u32,
    nullable: bool,
    array: bool,
) -> Result<ResolvedField> {
    if nullable && array {
        return Err(failure("Nullable Array cannot be classified"));
    }
    Ok(ResolvedField {
        key,
        name: name.into(),
        base_type: types
            .resolve_reference(ty)
            .ok_or_else(|| failure("unknown field type"))?,
        modifier: if array {
            FieldModifier::Array
        } else if nullable {
            FieldModifier::Nullable
        } else {
            FieldModifier::Required
        },
    })
}
fn transform_field(
    types: &TypeSystem,
    field: &ResolvedField,
    value: &mut Value,
    command: &TypeMigrationCommand,
    dependent: &BTreeSet<String>,
    count: &mut usize,
) -> Result<()> {
    if !dependent.contains(field.base_type.source_name()) {
        return Ok(());
    }
    if value.is_null() {
        return if field.modifier == FieldModifier::Nullable {
            Ok(())
        } else {
            Err(failure("non-null occurrence required"))
        };
    }
    if field.modifier == FieldModifier::Array {
        for value in value
            .as_sequence_mut()
            .ok_or_else(|| failure("expected array"))?
        {
            transform_value(types, &field.base_type, value, command, dependent, count)?;
        }
    } else {
        transform_value(types, &field.base_type, value, command, dependent, count)?;
    }
    Ok(())
}
fn transform_value(
    types: &TypeSystem,
    reference: &TypeReference,
    value: &mut Value,
    command: &TypeMigrationCommand,
    dependent: &BTreeSet<String>,
    count: &mut usize,
) -> Result<()> {
    use TypeMigrationOperation as Op;
    let TypeReference::Named(name) = reference else {
        return Ok(());
    };
    if let Some(ResolvedType::Custom { fields, .. }) = types.get(name) {
        let map = value
            .as_mapping_mut()
            .ok_or_else(|| failure("expected Custom mapping"))?;
        if map
            .keys()
            .any(|key| !fields.iter().any(|f| key.as_str() == Some(&f.name)))
        {
            return Err(failure("unknown Custom member cannot be classified safely"));
        }
        for f in fields {
            if !dependent.contains(f.base_type.source_name()) {
                continue;
            }
            transform_field(
                types,
                f,
                map.get_mut(Value::String(f.name.clone()))
                    .ok_or_else(|| failure("missing custom field"))?,
                command,
                dependent,
                count,
            )?;
        }
    }
    if name != &command.target {
        return Ok(());
    }
    if let Some(ResolvedType::Custom { fields, .. }) = types.get(name) {
        let map = value
            .as_mapping()
            .ok_or_else(|| failure("expected Custom mapping"))?;
        if fields
            .iter()
            .any(|f| !map.contains_key(Value::String(f.name.clone())))
        {
            return Err(failure("missing Custom member"));
        }
    }
    match &command.operation {
        Op::RenameEnumMember { member, .. } | Op::DropEnumMember { member } => {
            types.validate_reference_value(reference, value)?;
            let mut change = |v: &mut Value| -> Result<()> {
                if v.as_str() == Some(member) {
                    *count += 1;
                    if let Op::RenameEnumMember { new_name, .. } = &command.operation {
                        *v = Value::String(new_name.clone());
                    } else {
                        return Err(failure(
                            "member has existing data occurrences; drop refused",
                        ));
                    }
                }
                Ok(())
            };
            if let Some(values) = value.as_sequence_mut() {
                for v in values {
                    change(v)?;
                }
            } else {
                change(value)?;
            }
        }
        Op::AddCustomField { field, initializer } => {
            *count += 1;
            value
                .as_mapping_mut()
                .ok_or_else(|| failure("expected Custom mapping"))?
                .insert(
                    Value::String(field.name.clone()),
                    initializer.clone().ok_or_else(|| {
                        failure("existing Custom values require an explicit initializer")
                    })?,
                );
        }
        Op::RenameCustomField { field, new_name } => {
            *count += 1;
            let map = value
                .as_mapping_mut()
                .ok_or_else(|| failure("expected Custom mapping"))?;
            *map = std::mem::take(map)
                .into_iter()
                .map(|(key, value)| {
                    (
                        if key.as_str() == Some(field) {
                            Value::String(new_name.clone())
                        } else {
                            key
                        },
                        value,
                    )
                })
                .collect();
        }
        Op::DropCustomField { field } => {
            *count += 1;
            value
                .as_mapping_mut()
                .ok_or_else(|| failure("expected Custom mapping"))?
                .remove(Value::String(field.clone()));
        }
        _ => {}
    }
    Ok(())
}
