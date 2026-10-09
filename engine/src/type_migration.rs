//! Type-specific intent and occurrence resolution, composed with the shared
//! source-set Plan/commit boundary. No filesystem writes or artifact authority.
use crate::{
    Error, Result,
    creation::Declaration,
    migration::{self, Candidate, Plan},
    project::Project,
    semantic::{self, Field, Type},
    source::{Document, Node, Patch, Raw, Value},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    SetValueObjectConversions {
        type_name: String,
        from_implicit: bool,
        to_implicit: bool,
    },
    AddEnumMember {
        type_name: String,
        name: String,
        value: String,
    },
    RenameEnumMember {
        type_name: String,
        member: String,
        new_name: String,
    },
    DropEnumMember {
        type_name: String,
        member: String,
    },
    AddCustomField {
        type_name: String,
        declaration: Declaration,
        initializer: Option<Value>,
    },
    RenameCustomField {
        type_name: String,
        field: String,
        new_name: String,
    },
    DropCustomField {
        type_name: String,
        field: String,
    },
}
impl Command {
    pub fn target(&self) -> &str {
        match self {
            Self::SetValueObjectConversions { type_name, .. }
            | Self::AddEnumMember { type_name, .. }
            | Self::RenameEnumMember { type_name, .. }
            | Self::DropEnumMember { type_name, .. }
            | Self::AddCustomField { type_name, .. }
            | Self::RenameCustomField { type_name, .. }
            | Self::DropCustomField { type_name, .. } => type_name,
        }
    }
    pub fn destructive(&self) -> bool {
        matches!(
            self,
            Self::DropEnumMember { .. } | Self::DropCustomField { .. }
        )
    }
    fn changes_values(&self) -> bool {
        !matches!(
            self,
            Self::SetValueObjectConversions { .. } | Self::AddEnumMember { .. }
        )
    }
}
fn resolution(message: impl Into<String>) -> Error {
    Error::new("E-TYPE-MIGRATION-RESOLUTION", message)
}
fn precondition(message: impl Into<String>) -> Error {
    Error::new("E-TYPE-MIGRATION-PRECONDITION", message)
}

struct Closure {
    source: String,
    // Reverse reachability is classification, not a second type system. Actual
    // declaration/field/Reference rules remain owned by the shared resolver.
    related: BTreeSet<String>,
    tables: BTreeSet<String>,
}
fn field_names(node: &Node) -> Result<Vec<String>> {
    node.items()?
        .iter()
        .map(|entry| Ok(entry.value.required("type")?.text()?.to_owned()))
        .collect()
}
fn classify(project: &Project, target: &str) -> Result<Closure> {
    let mut declarations: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut schemas: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut schema_fields = BTreeMap::new();
    for (path, source) in &project.sources {
        let doc = source
            .document
            .as_ref()
            .ok_or_else(|| resolution(format!("{path}: source cannot be classified")))?;
        match source.kind.as_deref() {
            Some("type") => {
                let name = doc.root.required("name")?.text()?.to_owned();
                let categories: Vec<_> = ["valueObject", "enum", "flags", "custom"]
                    .into_iter()
                    .filter(|category| doc.root.get(category).is_some())
                    .collect();
                if categories.len() != 1 {
                    return Err(resolution(format!(
                        "{path}: type category cannot be classified"
                    )));
                }
                let dependencies = if categories[0] == "custom" {
                    field_names(doc.root.required("custom")?.required("fields")?)?
                } else {
                    vec![]
                };
                declarations
                    .entry(name.clone())
                    .or_default()
                    .push(path.clone());
                children.entry(name).or_default().extend(dependencies);
            }
            Some("schema") => {
                let name = doc.root.required("table")?.text()?.to_owned();
                schemas.entry(name).or_default().push(path.clone());
                schema_fields.insert(path.clone(), field_names(doc.root.required("fields")?)?);
            }
            Some("data") => {
                doc.root.required("table")?.text()?;
            }
            _ => return Err(resolution(format!("{path}: unknown source kind"))),
        }
    }
    let paths = declarations
        .get(target)
        .filter(|paths| paths.len() == 1)
        .ok_or_else(|| resolution("logical type declaration is missing or ambiguous"))?;
    if !project.types.contains_key(target) {
        return Err(resolution("target type declaration cannot be resolved"));
    }
    let mut related = BTreeSet::from([target.to_owned()]);
    loop {
        let previous = related.len();
        for (name, dependencies) in &children {
            if dependencies.iter().any(|name| related.contains(name)) {
                related.insert(name.clone());
            }
        }
        if related.len() == previous {
            break;
        }
    }
    for name in &related {
        if declarations.get(name).is_none_or(|paths| paths.len() != 1)
            || !project.types.contains_key(name)
        {
            return Err(resolution(format!(
                "{name}: dependent declaration cannot be resolved"
            )));
        }
    }
    let mut tables = BTreeSet::new();
    for (name, paths) in &schemas {
        if paths.iter().any(|path| {
            schema_fields[path]
                .iter()
                .any(|name| related.contains(name))
        }) {
            if paths.len() != 1 || !project.tables.contains_key(name) {
                return Err(resolution(format!(
                    "{name}: dependent Table cannot be resolved"
                )));
            }
            tables.insert(name.clone());
        }
    }
    // An unparseable Reference source can still point at an affected nominal
    // key. Its primitive fields alone do not prove it is outside the closure.
    let components = tables
        .iter()
        .map(|name| {
            (
                name.clone(),
                project.tables[name]
                    .fields
                    .iter()
                    .filter(|field| related.contains(&field.type_name))
                    .map(|field| field.name.clone())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (table, paths) in &schemas {
        for path in paths {
            if project.tables.values().any(|table| table.source == *path) {
                continue;
            }
            let doc = project.sources[path].document.as_ref().unwrap();
            if let Some(references) = doc.root.get("references") {
                for entry in references.items()? {
                    let source_fields = entry.value.required("fields")?.items()?;
                    let target = entry.value.required("target")?;
                    let target_table = target.required("table")?.text()?;
                    let target_fields = target.required("fields")?.items()?;
                    let source_related = source_fields
                        .iter()
                        .map(|field| field.value.text())
                        .collect::<Result<Vec<_>>>()?
                        .iter()
                        .any(|field| {
                            components
                                .get(table)
                                .is_some_and(|names| names.contains(*field))
                        });
                    let target_related = target_fields
                        .iter()
                        .map(|field| field.value.text())
                        .collect::<Result<Vec<_>>>()?
                        .iter()
                        .any(|field| {
                            components
                                .get(target_table)
                                .is_some_and(|names| names.contains(*field))
                        });
                    if source_related || target_related {
                        return Err(resolution(format!(
                            "{path}: affected Reference source cannot be resolved"
                        )));
                    }
                }
            }
        }
    }
    for (path, source) in &project.sources {
        if source.kind.as_deref() == Some("data") {
            let binding = source
                .document
                .as_ref()
                .unwrap()
                .root
                .required("table")?
                .text()?;
            if !schemas.contains_key(binding) {
                return Err(resolution(format!(
                    "{path}: data binding cannot be classified"
                )));
            }
        }
    }
    Ok(Closure {
        source: paths[0].clone(),
        related,
        tables,
    })
}

fn expected_type(project: &Project, command: &Command) -> Result<Type> {
    let mut expected = project.types[command.target()].clone();
    match (&mut expected, command) {
        (
            Type::ValueObject {
                from_implicit,
                to_implicit,
                ..
            },
            Command::SetValueObjectConversions {
                from_implicit: from,
                to_implicit: to,
                ..
            },
        ) => {
            *from_implicit = *from;
            *to_implicit = *to;
        }
        (
            Type::Enum {
                underlying,
                members,
                ..
            },
            Command::AddEnumMember { name, value, .. },
        ) => {
            let number = semantic::integer_value(value, underlying)
                .map_err(|error| precondition(error.message))?;
            members.push((name.clone(), number.to_string()));
        }
        (
            Type::Enum { flags, members, .. },
            Command::RenameEnumMember {
                member, new_name, ..
            },
        ) => {
            if *flags && member == "None" {
                return Err(precondition("Flags None=0 cannot be renamed"));
            }
            let current = members
                .iter_mut()
                .find(|(name, _)| name == member)
                .ok_or_else(|| precondition("member not found"))?;
            current.0 = new_name.clone();
        }
        (Type::Enum { flags, members, .. }, Command::DropEnumMember { member, .. }) => {
            if *flags && member == "None" {
                return Err(precondition("Flags None=0 cannot be dropped"));
            }
            if !members.iter().any(|(name, _)| name == member) {
                return Err(precondition("member not found"));
            }
            members.retain(|(name, _)| name != member);
        }
        (Type::Custom { fields }, Command::AddCustomField { declaration, .. }) => {
            fields.push(migration::field_declaration(declaration, &project.types)?);
        }
        (
            Type::Custom { fields },
            Command::RenameCustomField {
                field, new_name, ..
            },
        ) => {
            fields
                .iter_mut()
                .find(|f| f.name == *field)
                .ok_or_else(|| precondition("field not found"))?
                .name = new_name.clone();
        }
        (Type::Custom { fields }, Command::DropCustomField { field, .. }) => {
            if !fields.iter().any(|f| f.name == *field) {
                return Err(precondition("field not found"));
            }
            fields.retain(|f| f.name != *field);
        }
        _ => {
            return Err(precondition(
                "operation does not match the resolved type category",
            ));
        }
    }
    Ok(expected)
}
fn declaration_candidate(doc: &Document, current: &Type, command: &Command) -> Result<Document> {
    let mut patches = vec![];
    match (current, command) {
        (
            Type::ValueObject {
                from_implicit: from,
                to_implicit: to,
                ..
            },
            Command::SetValueObjectConversions {
                from_implicit,
                to_implicit,
                ..
            },
        ) => {
            let body = doc.root.required("valueObject")?;
            if let Some(conversions) = body.get("conversions") {
                for (name, value) in [
                    ("fromUnderlyingImplicit", *from_implicit),
                    ("toUnderlyingImplicit", *to_implicit),
                ] {
                    let value = Value::Literal(value.to_string());
                    if let Some(node) = conversions.get(name) {
                        migration::scalar_patch(doc, node, &value, &mut patches)?;
                    } else {
                        doc.derive_member_add(conversions, name, &value, &mut patches)?;
                    }
                }
            } else if from != from_implicit || to != to_implicit {
                doc.derive_member_add(
                    body,
                    "conversions",
                    &Value::Mapping(vec![
                        (
                            "fromUnderlyingImplicit".into(),
                            Value::Literal(from_implicit.to_string()),
                        ),
                        (
                            "toUnderlyingImplicit".into(),
                            Value::Literal(to_implicit.to_string()),
                        ),
                    ]),
                    &mut patches,
                )?;
            }
        }
        (Type::Enum { flags, members, .. }, command) => {
            let sequence = doc
                .root
                .required(if *flags { "flags" } else { "enum" })?
                .required("members")?;
            return match command {
                Command::AddEnumMember { name, value, .. } => doc.insert_sequence(
                    sequence,
                    members.len(),
                    &Value::Mapping(vec![
                        ("name".into(), Value::Text(name.clone())),
                        ("value".into(), Value::Literal(value.clone())),
                    ]),
                ),
                Command::RenameEnumMember {
                    member, new_name, ..
                } => {
                    let index = members
                        .iter()
                        .position(|(name, _)| name == member)
                        .ok_or_else(|| precondition("member not found"))?;
                    migration::scalar_patch(
                        doc,
                        sequence.items()?[index].value.required("name")?,
                        &Value::Text(new_name.clone()),
                        &mut patches,
                    )?;
                    doc.patched(patches)
                }
                Command::DropEnumMember { member, .. } => doc.remove_sequence(
                    sequence,
                    members
                        .iter()
                        .position(|(name, _)| name == member)
                        .ok_or_else(|| precondition("member not found"))?,
                ),
                _ => Err(precondition("Enum operation required")),
            };
        }
        (Type::Custom { fields }, command) => {
            let sequence = doc.root.required("custom")?.required("fields")?;
            return match command {
                Command::AddCustomField { declaration, .. } => doc.insert_sequence(
                    sequence,
                    fields.len(),
                    &migration::declaration_value(declaration),
                ),
                Command::RenameCustomField {
                    field, new_name, ..
                } => {
                    let index = fields
                        .iter()
                        .position(|f| f.name == *field)
                        .ok_or_else(|| precondition("field not found"))?;
                    migration::scalar_patch(
                        doc,
                        sequence.items()?[index].value.required("name")?,
                        &Value::Text(new_name.clone()),
                        &mut patches,
                    )?;
                    doc.patched(patches)
                }
                Command::DropCustomField { field, .. } => doc.remove_sequence(
                    sequence,
                    fields
                        .iter()
                        .position(|f| f.name == *field)
                        .ok_or_else(|| precondition("field not found"))?,
                ),
                _ => Err(precondition("Custom Type operation required")),
            };
        }
        _ => return Err(precondition("Value Object operation required")),
    }
    doc.patched(patches)
}

struct Values<'a> {
    project: &'a Project,
    closure: &'a Closure,
    command: &'a Command,
    doc: &'a Document,
    patches: Vec<Patch>,
    count: usize,
}
impl Values<'_> {
    fn field(&mut self, field: &Field, node: &Node) -> Result<Value> {
        if !self.closure.related.contains(&field.type_name) {
            return Ok(node.value());
        }
        if !node.safe {
            return Err(resolution(
                "dependent value has unsupported source representation",
            ));
        }
        if matches!(node.raw, Raw::Null) && !field.array {
            return Ok(Value::Null);
        }
        if field.array {
            return Ok(Value::Sequence(
                node.items()?
                    .iter()
                    .map(|item| self.base(&field.type_name, &item.value))
                    .collect::<Result<_>>()?,
            ));
        }
        self.base(&field.type_name, node)
    }
    fn symbol(&mut self, node: &Node) -> Result<Value> {
        if !node.safe {
            return Err(resolution("symbolic occurrence is unsafe"));
        }
        let name = node.text()?;
        match self.command {
            Command::RenameEnumMember {
                member, new_name, ..
            } if name == member => {
                if member != new_name {
                    migration::scalar_patch(
                        self.doc,
                        node,
                        &Value::Text(new_name.clone()),
                        &mut self.patches,
                    )?;
                    self.count += 1;
                }
                Ok(Value::Text(new_name.clone()))
            }
            Command::DropEnumMember { member, .. } if name == member => Err(precondition(
                "member is used by an existing value occurrence",
            )),
            _ => Ok(node.value()),
        }
    }
    fn base(&mut self, name: &str, node: &Node) -> Result<Value> {
        if !node.safe {
            return Err(resolution("dependent occurrence is unsafe"));
        }
        match self
            .project
            .types
            .get(name)
            .ok_or_else(|| resolution(format!("{name}: dependent type cannot be resolved")))?
        {
            Type::Enum { flags, .. } => {
                if name != self.command.target() {
                    return Ok(node.value());
                }
                if *flags {
                    Ok(Value::Sequence(
                        node.items()?
                            .iter()
                            .map(|item| self.symbol(&item.value))
                            .collect::<Result<_>>()?,
                    ))
                } else {
                    self.symbol(node)
                }
            }
            Type::Custom { fields } => {
                let target = name == self.command.target();
                let mut expected = vec![];
                for member in node.members()? {
                    if target
                        && matches!(self.command, Command::DropCustomField { field, .. } if field == &member.name)
                    {
                        self.doc
                            .derive_member_drop(node, &member.name, &mut self.patches)?;
                        self.count += 1;
                        continue;
                    }
                    let value = if let Some(field) =
                        fields.iter().find(|field| field.name == member.name)
                    {
                        self.field(field, &member.value)?
                    } else {
                        member.value.value()
                    };
                    let mut key = member.name.clone();
                    if target
                        && let Command::RenameCustomField {
                            field, new_name, ..
                        } = self.command
                        && field == &member.name
                    {
                        self.doc
                            .derive_member_rename(node, field, new_name, &mut self.patches)?;
                        key = new_name.clone();
                        if field != new_name {
                            self.count += 1;
                        }
                    }
                    expected.push((key, value));
                }
                if target
                    && let Command::AddCustomField {
                        declaration,
                        initializer,
                        ..
                    } = self.command
                {
                    let value = initializer.as_ref().ok_or_else(|| {
                        precondition(
                            "explicit constant initializer required for existing Custom values",
                        )
                    })?;
                    self.doc.derive_member_add(
                        node,
                        &declaration.name,
                        value,
                        &mut self.patches,
                    )?;
                    expected.push((declaration.name.clone(), value.clone()));
                    self.count += 1;
                }
                Ok(Value::Mapping(expected))
            }
            Type::ValueObject { .. } => Ok(node.value()),
        }
    }
    fn records(&mut self, table: &semantic::Table) -> Result<Value> {
        let mut expected = vec![];
        for record in self.doc.records()? {
            let mut members = vec![];
            for member in record.value.members()? {
                let value = if let Some(field) = table.fields.iter().find(|f| f.name == member.name)
                {
                    self.field(field, &member.value)?
                } else {
                    member.value.value()
                };
                members.push((member.name.clone(), value));
            }
            expected.push(Value::Mapping(members));
        }
        Ok(Value::Sequence(expected))
    }
}

pub fn derive(project: &Project, command: Command) -> Result<Plan> {
    let closure = classify(project, command.target())?;
    let expected = expected_type(project, &command)?;
    let before = project.sources[&closure.source].document.as_ref().unwrap();
    let after = Arc::new(declaration_candidate(
        before,
        &project.types[command.target()],
        &command,
    )?);
    let (name, parsed) = semantic::parse_type(&after)?;
    if name != command.target() || parsed != expected {
        return Err(Error::new(
            "E-TYPE-MIGRATION-POSTCONDITION",
            "type declaration differs from semantic intent",
        ));
    }
    let mut expected_types = (*project.types).clone();
    expected_types.insert(command.target().into(), expected);
    let mut transformed = project.clone();
    let mut candidates = BTreeMap::new();
    let mut add_candidate = |path: &str, after: Arc<Document>| {
        let source = &project.sources[path];
        let before = source.document.as_ref().unwrap();
        if before.bytes != after.bytes {
            let mut changed = (**source).clone();
            changed.bytes = after.bytes.clone();
            changed.identity = after.identity.clone();
            changed.document = Some(after.clone());
            transformed.sources.insert(path.into(), Arc::new(changed));
            candidates.insert(
                path.into(),
                Candidate {
                    before: before.clone(),
                    after,
                },
            );
        }
    };
    add_candidate(&closure.source, after);
    if let Command::AddCustomField {
        declaration,
        initializer: Some(value),
        ..
    } = &command
    {
        let field = migration::field_declaration(declaration, &expected_types)?;
        migration::constant(&field, value, &expected_types)?;
    }
    let mut count = 0;
    if command.changes_values() {
        for (path, source) in &project.sources {
            let Some(table) = source
                .binding
                .as_ref()
                .filter(|name| closure.tables.contains(*name))
                .and_then(|name| project.tables.get(name))
            else {
                continue;
            };
            let doc = source.document.as_ref().unwrap();
            if source.kind.as_deref() == Some("schema") && doc.root.get("records").is_none() {
                continue;
            }
            let mut values = Values {
                project,
                closure: &closure,
                command: &command,
                doc,
                patches: vec![],
                count: 0,
            };
            let expected = values.records(table)?;
            count += values.count;
            if !values.patches.is_empty() {
                let after = Arc::new(doc.patched(values.patches)?);
                if !migration::ordered_value(after.root.required("records")?, &expected) {
                    return Err(Error::new(
                        "E-TYPE-MIGRATION-POSTCONDITION",
                        format!("{path}: transformed values or member order differ"),
                    ));
                }
                add_candidate(path, after);
            }
        }
    }
    transformed.rebuild_declarations();
    if *transformed.types != expected_types || transformed.tables != project.tables {
        return Err(Error::new(
            "E-TYPE-MIGRATION-POSTCONDITION",
            "transformed declarations differ from semantic intent",
        ));
    }
    migration::field_resolution(
        &transformed,
        &Field {
            key: 0,
            name: "value".into(),
            type_name: command.target().into(),
            nullable: false,
            array: false,
        },
    )?;
    for problem in &transformed.declaration_problems {
        if problem.source == closure.source
            || !project.declaration_problems.iter().any(|old| {
                old.source == problem.source
                    && old.code == problem.code
                    && old.message == problem.message
            })
        {
            return Err(Error::new(
                "E-TYPE-MIGRATION-POSTCONDITION",
                format!("{}: {}", problem.source, problem.message),
            ));
        }
    }
    for table in transformed.tables.values() {
        for reference in &table.references {
            let target = transformed.tables.get(&reference.target_table);
            let component = table.fields.iter().any(|field| {
                reference.fields.contains(&field.name) && closure.related.contains(&field.type_name)
            }) || target.is_some_and(|target| {
                target.fields.iter().any(|field| {
                    reference.target_fields.contains(&field.name)
                        && closure.related.contains(&field.type_name)
                })
            });
            if component {
                transformed.resolve_reference(table, reference)?;
            }
        }
    }
    Ok(Plan {
        destructive: command.destructive(),
        command: migration::Command::Type { command },
        affected_records: count,
        candidates,
    })
}
