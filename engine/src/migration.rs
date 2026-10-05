//! Pure, source-preserving structural plans. Native authorization is separate.
use crate::{
    Error, Result,
    creation::Declaration,
    project::Project,
    semantic::{self, Field, Table},
    source::{self, Document, Node, Patch, Raw, Value},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    Type {
        command: crate::type_migration::Command,
    },
    AddField {
        table: String,
        declaration: Declaration,
        initializer: Option<Value>,
        position: Option<usize>,
    },
    RenameField {
        table: String,
        field: String,
        new_name: String,
    },
    DropField {
        table: String,
        field: String,
    },
    SetFieldDeclaration {
        table: String,
        field: String,
        type_name: String,
        nullable: bool,
        array: bool,
    },
}
impl Command {
    pub fn table(&self) -> Result<&str> {
        match self {
            Self::AddField { table, .. }
            | Self::RenameField { table, .. }
            | Self::DropField { table, .. }
            | Self::SetFieldDeclaration { table, .. } => Ok(table),
            Self::Type { .. } => Err(resolution("Table operation required")),
        }
    }
    pub fn destructive(&self) -> bool {
        match self {
            Self::Type { command } => command.destructive(),
            Self::DropField { .. } => true,
            _ => false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub before: Arc<Document>,
    pub after: Arc<Document>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub command: Command,
    pub destructive: bool,
    pub affected_records: usize,
    pub candidates: BTreeMap<String, Candidate>,
}
fn resolution(message: impl Into<String>) -> Error {
    Error::new("E-MIGRATION-RESOLUTION", message)
}
fn precondition(message: impl Into<String>) -> Error {
    Error::new("E-MIGRATION-PRECONDITION", message)
}

pub(crate) fn declaration_value(d: &Declaration) -> Value {
    let mut members = vec![
        ("key".into(), Value::Literal(d.key.clone())),
        ("name".into(), Value::Text(d.name.clone())),
        ("type".into(), Value::Text(d.type_name.clone())),
    ];
    if d.nullable {
        members.push(("nullable".into(), Value::Literal("true".into())));
    }
    if d.array {
        members.push(("array".into(), Value::Literal("true".into())));
    }
    Value::Mapping(members)
}
pub(crate) fn field_declaration(d: &Declaration, types: &semantic::Types) -> Result<Field> {
    let doc = Document::parse(source::render_block(
        &Value::Mapping(vec![(
            "fields".into(),
            Value::Sequence(vec![declaration_value(d)]),
        )]),
        0,
        "\n",
    )?)?;
    let field = semantic::fields(doc.root.required("fields")?)?.remove(0);
    semantic::shape(&field, types)?;
    Ok(field)
}
pub(crate) fn field_resolution(project: &Project, field: &Field) -> Result<()> {
    semantic::shape(field, &project.types)?;
    fn names(project: &Project, name: &str) -> Result<()> {
        if semantic::primitive(name) {
            return Ok(());
        }
        // Project opening / changed-source installation maintains this locator
        // index. Ordinary type selection never enumerates the project to prove
        // a required declaration's uniqueness.
        let declarations = project
            .type_declarations
            .get(name)
            .map_or(0, |paths| paths.len());
        if declarations != 1 {
            return Err(resolution(format!(
                "{name}: type declaration is ambiguous or missing"
            )));
        }
        if let Some(semantic::Type::Custom { fields }) = project.types.get(name) {
            for field in fields {
                names(project, &field.type_name)?;
            }
        }
        Ok(())
    }
    names(project, &field.type_name)
}
pub(crate) fn constant(field: &Field, value: &Value, types: &semantic::Types) -> Result<()> {
    let doc = Document::parse(source::render_block(
        &Value::Mapping(vec![("value".into(), value.clone())]),
        0,
        "\n",
    )?)?;
    semantic::interpret(field, doc.root.required("value")?, types)
        .map_err(|problem| precondition(problem.message))?;
    Ok(())
}
fn classification(project: &Project, target: &str) -> Result<()> {
    if project
        .sources
        .values()
        .filter(|source| {
            source.kind.as_deref() == Some("schema") && source.binding.as_deref() == Some(target)
        })
        .count()
        != 1
    {
        return Err(resolution("logical Table declaration is ambiguous"));
    }
    for (path, source) in &project.sources {
        let doc = source
            .document
            .as_ref()
            .ok_or_else(|| resolution(format!("{path}: source cannot be classified")))?;
        match source.kind.as_deref() {
            Some("schema" | "data") => {
                let table = doc.root.required("table")?.text()?;
                if table.is_empty() {
                    return Err(resolution(format!("{path}: Table binding missing")));
                }
                if source.kind.as_deref() == Some("schema")
                    && let Some(references) = doc.root.get("references")
                {
                    for item in references.items()? {
                        let binding = item.value.required("target")?.required("table")?.text()?;
                        if binding == target
                            && !project.tables.values().any(|table| table.source == *path)
                        {
                            return Err(resolution(format!(
                                "{path}: affected Reference schema cannot be resolved"
                            )));
                        }
                    }
                }
            }
            Some("type") => {
                doc.root.required("name")?.text()?;
            }
            _ => return Err(resolution(format!("{path}: unknown source kind"))),
        }
    }
    Ok(())
}
fn rename_components(fields: &mut [String], old: &str, new: &str) {
    for field in fields {
        if field == old {
            *field = new.into();
        }
    }
}
fn has_dependency(project: &Project, target: &Table, field: &str) -> bool {
    std::iter::once(&target.primary)
        .chain(&target.secondary)
        .any(|key| key.fields.iter().any(|f| f == field))
        || project.tables.values().any(|table| {
            table.references.iter().any(|r| {
                (table.name == target.name && r.fields.iter().any(|f| f == field))
                    || (r.target_table == target.name && r.target_fields.iter().any(|f| f == field))
            })
        })
}
fn expected_tables(
    project: &Project,
    target: &Table,
    command: &Command,
) -> Result<BTreeMap<String, Arc<Table>>> {
    let mut expected = project.tables.clone();
    let changed = Arc::make_mut(expected.get_mut(&target.name).unwrap());
    match command {
        Command::Type { .. } => return Err(resolution("Table operation required")),
        Command::AddField {
            declaration,
            position,
            ..
        } => {
            let field = field_declaration(declaration, &project.types)?;
            field_resolution(project, &field)?;
            if changed
                .fields
                .iter()
                .any(|f| f.name == field.name || f.key == field.key)
            {
                return Err(precondition("field name or MessagePack key collides"));
            }
            let at = position.unwrap_or(changed.fields.len());
            if at > changed.fields.len() {
                return Err(precondition("field insertion position is outside schema"));
            }
            changed.fields.insert(at, field);
        }
        Command::RenameField {
            field, new_name, ..
        } => {
            if new_name != field && changed.fields.iter().any(|f| f.name == *new_name) {
                return Err(precondition(format!(
                    "{new_name}: field name already exists"
                )));
            }
            let current = changed
                .fields
                .iter_mut()
                .find(|f| f.name == *field)
                .ok_or_else(|| precondition("field not found"))?;
            current.name = new_name.clone();
            field_resolution(project, current)?;
            for table in expected.values_mut() {
                let table = Arc::make_mut(table);
                if table.name == target.name {
                    rename_components(&mut table.primary.fields, field, new_name);
                    for key in &mut table.secondary {
                        rename_components(&mut key.fields, field, new_name);
                    }
                }
                for reference in &mut table.references {
                    let touched = (table.name == target.name && reference.fields.contains(field))
                        || (reference.target_table == target.name
                            && reference.target_fields.contains(field));
                    if touched
                        && project.declaration_problems.iter().any(|p| {
                            p.source == table.source
                                && p.code.starts_with("E-REFERENCE")
                                && p.message == reference.name
                        })
                    {
                        return Err(resolution(format!(
                            "{}: Reference {} cannot be resolved",
                            table.source, reference.name
                        )));
                    }
                    if table.name == target.name {
                        rename_components(&mut reference.fields, field, new_name);
                    }
                    if reference.target_table == target.name {
                        rename_components(&mut reference.target_fields, field, new_name);
                    }
                }
            }
        }
        Command::DropField { field, .. } => {
            if !changed.fields.iter().any(|f| f.name == *field) {
                return Err(precondition("field not found"));
            }
            if has_dependency(project, target, field) {
                return Err(precondition(format!(
                    "{field}: Key or Reference depends on this field"
                )));
            }
            field_resolution(
                project,
                changed.fields.iter().find(|f| f.name == *field).unwrap(),
            )?;
            changed.fields.retain(|f| f.name != *field);
        }
        Command::SetFieldDeclaration {
            field,
            type_name,
            nullable,
            array,
            ..
        } => {
            let current = changed
                .fields
                .iter_mut()
                .find(|f| f.name == *field)
                .ok_or_else(|| precondition("field not found"))?;
            current.type_name = type_name.clone();
            current.nullable = *nullable;
            current.array = *array;
            semantic::shape(current, &project.types)?;
            field_resolution(project, current)?;
        }
    }
    Ok(expected)
}
pub(crate) fn scalar_patch(
    doc: &Document,
    node: &Node,
    value: &Value,
    patches: &mut Vec<Patch>,
) -> Result<()> {
    source::derive_patch(doc, node, value, patches)
}
fn component_patches(
    doc: &Document,
    node: &Node,
    old: &str,
    new: &str,
    patches: &mut Vec<Patch>,
) -> Result<()> {
    for item in node.items()? {
        if item.value.text()? == old {
            scalar_patch(doc, &item.value, &Value::Text(new.into()), patches)?;
        }
    }
    Ok(())
}
fn reference_patches(
    doc: &Document,
    table: &Table,
    target: &str,
    old: &str,
    new: &str,
    patches: &mut Vec<Patch>,
) -> Result<()> {
    if table.name == target {
        component_patches(
            doc,
            doc.root.required("primaryKey")?.required("fields")?,
            old,
            new,
            patches,
        )?;
        if let Some(keys) = doc.root.get("secondaryKeys") {
            for key in keys.items()? {
                component_patches(doc, key.value.required("fields")?, old, new, patches)?;
            }
        }
    }
    if let Some(references) = doc.root.get("references") {
        for (item, reference) in references.items()?.iter().zip(&table.references) {
            if table.name == target {
                component_patches(doc, item.value.required("fields")?, old, new, patches)?;
            }
            if reference.target_table == target {
                component_patches(
                    doc,
                    item.value.required("target")?.required("fields")?,
                    old,
                    new,
                    patches,
                )?;
            }
        }
    }
    Ok(())
}
pub(crate) fn ordered_value(node: &Node, expected: &Value) -> bool {
    match (&node.raw, expected) {
        (Raw::Mapping(members), Value::Mapping(values)) => {
            members.len() == values.len()
                && members.iter().zip(values).all(|(member, (name, value))| {
                    member.name == *name && ordered_value(&member.value, value)
                })
        }
        (Raw::Sequence(items), Value::Sequence(values)) => {
            items.len() == values.len()
                && items
                    .iter()
                    .zip(values)
                    .all(|(item, value)| ordered_value(&item.value, value))
        }
        _ => source::matches_value(node, expected),
    }
}
fn expected_records(doc: &Document, command: &Command) -> Result<Value> {
    let mut expected = doc.root.value();
    let Value::Mapping(root) = &mut expected else {
        unreachable!()
    };
    let Value::Sequence(records) = &mut root
        .iter_mut()
        .find(|(name, _)| name == "records")
        .ok_or_else(|| resolution("records missing"))?
        .1
    else {
        return Err(resolution("record sequence required"));
    };
    for record in records {
        let Value::Mapping(members) = record else {
            return Err(resolution("record mapping required"));
        };
        match command {
            Command::Type { .. } => return Err(resolution("Table operation required")),
            Command::AddField {
                declaration,
                initializer,
                ..
            } => {
                if members.iter().any(|(name, _)| name == &declaration.name) {
                    return Err(precondition("record already contains new field"));
                }
                members.push((
                    declaration.name.clone(),
                    initializer
                        .clone()
                        .ok_or_else(|| precondition("explicit constant initializer required"))?,
                ));
            }
            Command::RenameField {
                field, new_name, ..
            } => {
                if new_name != field && members.iter().any(|(name, _)| name == new_name) {
                    return Err(precondition("record member rename collides"));
                }
                if let Some((name, _)) = members.iter_mut().find(|(name, _)| name == field) {
                    *name = new_name.clone();
                }
            }
            Command::DropField { field, .. } => members.retain(|(name, _)| name != field),
            Command::SetFieldDeclaration { .. } => {}
        }
    }
    Ok(expected)
}
pub fn derive(project: &Project, command: Command) -> Result<Plan> {
    if let Command::Type { command } = &command {
        return crate::type_migration::derive(project, command.clone());
    }
    let target = project
        .tables
        .get(command.table()?)
        .ok_or_else(|| resolution("logical Table cannot be resolved"))?;
    classification(project, &target.name)?;
    let expected = expected_tables(project, target, &command)?;
    let mut documents = project
        .sources
        .iter()
        .map(|(path, source)| (path.clone(), source.document.clone().unwrap()))
        .collect::<BTreeMap<_, _>>();
    let schema = &documents[&target.source];
    let fields = schema.root.required("fields")?;
    let schema = match &command {
        Command::Type { .. } => return Err(resolution("Table operation required")),
        Command::AddField {
            declaration,
            position,
            initializer,
            ..
        } => {
            if let Some(value) = initializer {
                constant(
                    expected[&target.name]
                        .fields
                        .iter()
                        .find(|field| field.name == declaration.name)
                        .unwrap(),
                    value,
                    &project.types,
                )?;
            }
            schema.insert_sequence(
                fields,
                position.unwrap_or(target.fields.len()),
                &declaration_value(declaration),
            )?
        }
        Command::DropField { field, .. } => schema.remove_sequence(
            fields,
            target.fields.iter().position(|f| f.name == *field).unwrap(),
        )?,
        Command::RenameField {
            field, new_name, ..
        } => {
            let index = target.fields.iter().position(|f| f.name == *field).unwrap();
            let mut patches = vec![];
            scalar_patch(
                schema,
                fields.items()?[index].value.required("name")?,
                &Value::Text(new_name.clone()),
                &mut patches,
            )?;
            schema.patched(patches)?
        }
        Command::SetFieldDeclaration {
            field,
            type_name,
            nullable,
            array,
            ..
        } => {
            let index = target.fields.iter().position(|f| f.name == *field).unwrap();
            let declaration = &fields.items()?[index].value;
            let mut patches = vec![];
            scalar_patch(
                schema,
                declaration.required("type")?,
                &Value::Text(type_name.clone()),
                &mut patches,
            )?;
            for (name, value) in [("nullable", *nullable), ("array", *array)] {
                let value = Value::Literal(value.to_string());
                if let Some(node) = declaration.get(name) {
                    scalar_patch(schema, node, &value, &mut patches)?;
                } else {
                    schema.derive_member_add(declaration, name, &value, &mut patches)?;
                }
            }
            schema.patched(patches)?
        }
    };
    documents.insert(target.source.clone(), Arc::new(schema));
    let mut affected_records = 0;
    for (path, doc) in &mut documents {
        let mut patches = vec![];
        if let Command::RenameField {
            field, new_name, ..
        } = &command
            && let Some(table) = project.tables.values().find(|table| table.source == *path)
        {
            reference_patches(doc, table, &target.name, field, new_name, &mut patches)?;
        }
        let bound = project.sources[path].binding.as_deref() == Some(&target.name);
        let has_records = doc.root.get("records").is_some();
        let expected_records = if bound
            && (has_records || project.sources[path].kind.as_deref() == Some("data"))
        {
            let records = doc.records()?;
            affected_records += records.len();
            for record in records {
                match &command {
                    Command::Type { .. } => return Err(resolution("Table operation required")),
                    Command::AddField {
                        declaration,
                        initializer,
                        ..
                    } => doc.derive_member_add(
                        &record.value,
                        &declaration.name,
                        initializer.as_ref().ok_or_else(|| {
                            precondition("explicit constant initializer required")
                        })?,
                        &mut patches,
                    )?,
                    Command::RenameField {
                        field, new_name, ..
                    } => {
                        if record.value.get(field).is_some() {
                            doc.derive_member_rename(&record.value, field, new_name, &mut patches)?;
                        }
                    }
                    Command::DropField { field, .. } => {
                        if record.value.get(field).is_some() {
                            doc.derive_member_drop(&record.value, field, &mut patches)?;
                        }
                    }
                    Command::SetFieldDeclaration { field, .. } => {
                        let field = expected[&target.name]
                            .fields
                            .iter()
                            .find(|f| f.name == *field)
                            .unwrap();
                        semantic::interpret(
                            field,
                            record.value.required(&field.name)?,
                            &project.types,
                        )
                        .map_err(|problem| precondition(format!("{path}: {}", problem.message)))?;
                    }
                }
            }
            Some(expected_records(doc, &command)?)
        } else {
            None
        };
        if !patches.is_empty() {
            let candidate = Arc::new(doc.patched(patches)?);
            // Record values/member order and declaration/key/reference semantics
            // have separate postconditions. A text patch alone proves neither.
            if let Some(mut value) = expected_records {
                // References in an inline schema are transformed in this same
                // candidate. Compare the records subtree, not its old key names.
                let Value::Mapping(root) = &mut value else {
                    unreachable!()
                };
                let records = &root.iter().find(|(name, _)| name == "records").unwrap().1;
                if !ordered_value(candidate.root.required("records")?, records) {
                    return Err(Error::new("E-MIGRATION-POSTCONDITION", path.clone()));
                }
            }
            *doc = candidate;
        }
    }
    let mut transformed = project.clone();
    let mut candidates = BTreeMap::new();
    for (path, after) in documents {
        let before = project.sources[&path].document.as_ref().unwrap();
        if before.bytes != after.bytes {
            let mut source = (*project.sources[&path]).clone();
            source.bytes = after.bytes.clone();
            source.identity = after.identity.clone();
            source.document = Some(after.clone());
            transformed.sources.insert(path.clone(), Arc::new(source));
            candidates.insert(
                path,
                Candidate {
                    before: before.clone(),
                    after,
                },
            );
        }
    }
    transformed.rebuild_declarations();
    if transformed.tables != expected || transformed.types != project.types {
        return Err(Error::new(
            "E-MIGRATION-POSTCONDITION",
            "transformed declarations differ from semantic intent",
        ));
    }
    let changed = transformed.tables[&target.name].as_ref();
    let selected_name = match &command {
        Command::Type { .. } => None,
        Command::AddField { declaration, .. } => Some(&declaration.name),
        Command::RenameField { new_name, .. } => Some(new_name),
        Command::SetFieldDeclaration { field, .. } => Some(field),
        Command::DropField { .. } => None,
    };
    if let Some(name) = selected_name {
        let field = changed
            .fields
            .iter()
            .find(|field| &field.name == name)
            .unwrap();
        if std::iter::once(&changed.primary)
            .chain(&changed.secondary)
            .any(|key| key.fields.contains(name))
            && !semantic::key_capable(field, &transformed.types)
        {
            return Err(precondition(format!(
                "{name}: key capability cannot be proved"
            )));
        }
        for table in transformed.tables.values() {
            for reference in &table.references {
                let component = (table.name == target.name && reference.fields.contains(name))
                    || (reference.target_table == target.name
                        && reference.target_fields.contains(name));
                let member_collision = table.name == target.name
                    && reference.csharp_name == semantic::public_name(name);
                if component || member_collision {
                    transformed.resolve_reference(table, reference)?;
                }
            }
        }
    }
    for problem in &transformed.declaration_problems {
        if !project.declaration_problems.iter().any(|old| {
            old.source == problem.source
                && old.code == problem.code
                && old.message == problem.message
        }) {
            return Err(Error::new(
                "E-MIGRATION-POSTCONDITION",
                format!("{}: {}", problem.source, problem.message),
            ));
        }
    }
    Ok(Plan {
        destructive: command.destructive(),
        command,
        affected_records,
        candidates,
    })
}
