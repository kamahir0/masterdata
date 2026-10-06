//! Saved declaration intent and exact-occurrence patches. Native Plan owns writes.
use crate::{
    Error, Result,
    migration::{self, Candidate, Plan},
    project::{Diagnostic, Project},
    semantic::{self, Field, Key, Reference, Table},
    source::{self, Document, Node, Value},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub table: String,
    pub source: String,
    pub identity: String,
    pub change: Change,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Change {
    SetFieldKey {
        occurrence: usize,
        key: String,
    },
    SetPrimaryKey {
        fields: Vec<String>,
    },
    AddSecondaryKey {
        fields: Vec<String>,
        non_unique: bool,
    },
    EditSecondaryKey {
        occurrence: usize,
        fields: Vec<String>,
        non_unique: bool,
    },
    RemoveSecondaryKey {
        occurrence: usize,
    },
    AddReference {
        declaration: ReferenceInput,
    },
    EditReference {
        occurrence: usize,
        declaration: ReferenceInput,
    },
    RemoveReference {
        occurrence: usize,
    },
}
impl Change {
    pub fn destructive(&self) -> bool {
        matches!(
            self,
            Self::RemoveSecondaryKey { .. } | Self::RemoveReference { .. }
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceInput {
    pub name: String,
    pub fields: Vec<String>,
    pub target_table: String,
    pub target_fields: Vec<String>,
    pub csharp_name: Option<String>,
}
impl ReferenceInput {
    fn resolved(&self) -> Reference {
        Reference {
            name: self.name.clone(),
            fields: self.fields.clone(),
            target_table: self.target_table.clone(),
            target_fields: self.target_fields.clone(),
            csharp_name: self
                .csharp_name
                .clone()
                .unwrap_or_else(|| format!("Get{}", semantic::public_name(&self.name))),
        }
    }
    fn value(&self) -> Value {
        let mut members = vec![
            ("name".into(), Value::Text(self.name.clone())),
            ("fields".into(), strings(&self.fields)),
            (
                "target".into(),
                Value::Mapping(vec![
                    ("table".into(), Value::Text(self.target_table.clone())),
                    ("fields".into(), strings(&self.target_fields)),
                ]),
            ),
        ];
        if let Some(name) = &self.csharp_name {
            members.push(("csharpName".into(), Value::Text(name.clone())));
        }
        Value::Mapping(members)
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceDetail {
    pub table: String,
    pub source: String,
    pub occurrence: usize,
    pub declaration: ReferenceInput,
    pub helper: String,
    pub selected_key: Option<usize>,
    pub multi: Option<bool>,
    pub optional: Option<bool>,
    pub problem: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub table: String,
    pub keys: Vec<Key>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub table: String,
    pub source: String,
    pub identity: String,
    pub fields: Vec<Field>,
    pub key_fields: Vec<String>,
    pub reference_fields: Vec<String>,
    pub primary: Key,
    pub secondary: Vec<Key>,
    pub references: Vec<ReferenceDetail>,
    pub dependents: Vec<ReferenceDetail>,
    pub targets: Vec<Target>,
    pub dependency_sources: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}
fn error(message: impl Into<String>) -> Error {
    Error::new("E-TABLE-DECLARATION", message)
}
fn target<'a>(project: &'a Project, name: &str) -> Result<&'a Arc<Table>> {
    let table = project
        .tables
        .get(name)
        .ok_or_else(|| error("saved Table declaration is unavailable"))?;
    if project
        .sources
        .values()
        .filter(|s| s.kind.as_deref() == Some("schema") && s.binding.as_deref() == Some(name))
        .count()
        != 1
    {
        return Err(error("saved Table declaration is ambiguous"));
    }
    Ok(table)
}
fn reference_detail(
    project: &Project,
    table: &Table,
    occurrence: usize,
) -> Result<ReferenceDetail> {
    let reference = &table.references[occurrence];
    let node = &project.sources[&table.source]
        .document
        .as_ref()
        .ok_or_else(|| error("schema source is unavailable"))?
        .root
        .required("references")?
        .items()?[occurrence]
        .value;
    let resolution = validate_reference(project, table, reference)
        .and_then(|_| project.resolve_reference(table, reference));
    Ok(ReferenceDetail {
        table: table.name.clone(),
        source: table.source.clone(),
        occurrence,
        declaration: ReferenceInput {
            name: reference.name.clone(),
            fields: reference.fields.clone(),
            target_table: reference.target_table.clone(),
            target_fields: reference.target_fields.clone(),
            csharp_name: node
                .get("csharpName")
                .map(|n| n.text().map(str::to_owned))
                .transpose()?,
        },
        helper: reference.csharp_name.clone(),
        selected_key: project.tables.get(&reference.target_table).and_then(|t| {
            target_keys(project, t)
                .iter()
                .position(|key| key.fields == reference.target_fields)
        }),
        multi: resolution.as_ref().ok().map(|r| r.multi),
        optional: resolution.as_ref().ok().map(|r| r.optional),
        problem: resolution.err().map(|e| e.to_string()),
    })
}
pub fn detail(project: &Project, name: &str) -> Result<Detail> {
    let table = target(project, name)?;
    let doc = project.sources[&table.source]
        .document
        .as_ref()
        .ok_or_else(|| error("schema source is unavailable"))?;
    let references = (0..table.references.len())
        .map(|i| reference_detail(project, table, i))
        .collect::<Result<Vec<_>>>()?;
    let mut dependents = Vec::new();
    let mut dependencies: BTreeSet<_> = project.dependencies(table).into_iter().collect();
    dependencies.insert(table.source.clone());
    for source_table in project.tables.values() {
        for (i, reference) in source_table.references.iter().enumerate() {
            if source_table.name != table.name && reference.target_table == table.name {
                dependents.push(reference_detail(project, source_table, i)?);
                dependencies.extend(project.dependencies(source_table));
                dependencies.insert(source_table.source.clone());
            }
        }
    }
    for reference in &table.references {
        if let Some(t) = project.tables.get(&reference.target_table) {
            dependencies.extend(project.dependencies(t));
            dependencies.insert(t.source.clone());
        }
    }
    Ok(Detail {
        table: table.name.clone(),
        source: table.source.clone(),
        identity: doc.identity.clone(),
        fields: table.fields.clone(),
        key_fields: table
            .fields
            .iter()
            .filter(|f| {
                semantic::key_capable(f, &project.types)
                    && migration::field_resolution(project, f).is_ok()
            })
            .map(|f| f.name.clone())
            .collect(),
        reference_fields: table
            .fields
            .iter()
            .filter(|f| {
                let mut required = (*f).clone();
                required.nullable = false;
                semantic::key_capable(&required, &project.types)
                    && migration::field_resolution(project, &required).is_ok()
            })
            .map(|f| f.name.clone())
            .collect(),
        primary: table.primary.clone(),
        secondary: table.secondary.clone(),
        references,
        dependents,
        targets: project
            .tables
            .values()
            .filter(|t| target(project, &t.name).is_ok())
            .map(|t| Target {
                table: t.name.clone(),
                keys: target_keys(project, t),
            })
            .collect(),
        diagnostics: project
            .declaration_problems
            .iter()
            .filter(|d| dependencies.contains(&d.source))
            .cloned()
            .collect(),
        dependency_sources: dependencies.into_iter().collect(),
    })
}
fn target_keys(project: &Project, table: &Table) -> Vec<Key> {
    std::iter::once(&table.primary)
        .chain(&table.secondary)
        .filter(|key| {
            key.fields.iter().all(|name| {
                table.fields.iter().any(|field| {
                    field.name == *name
                        && semantic::key_capable(field, &project.types)
                        && migration::field_resolution(project, field).is_ok()
                })
            })
        })
        .cloned()
        .collect()
}
fn strings(values: &[String]) -> Value {
    Value::Sequence(values.iter().cloned().map(Value::Text).collect())
}
fn key_value(fields: &[String], non_unique: bool, existing: Option<&Node>) -> Value {
    let mut values = vec![("fields".into(), strings(fields))];
    if non_unique || existing.is_some_and(|n| n.get("nonUnique").is_some()) {
        values.push(("nonUnique".into(), Value::Literal(non_unique.to_string())));
    }
    Value::Mapping(values)
}
#[derive(Clone)]
enum Position {
    Member(&'static str),
    Item(usize),
}
fn locate<'a>(doc: &'a Document, path: &[Position]) -> Result<&'a Node> {
    let mut node: &Node = &doc.root;
    for part in path {
        node = match part {
            Position::Member(name) => node.required(name)?,
            Position::Item(i) => {
                &node
                    .items()?
                    .get(*i)
                    .ok_or_else(|| error("captured declaration occurrence is missing"))?
                    .value
            }
        };
    }
    Ok(node)
}
// Reparse each localized membership edit before locating the next span. No
// serializer owns the surrounding schema, inline records or sibling trivia.
fn update(doc: &Document, path: &[Position], value: &Value) -> Result<Document> {
    let node = locate(doc, path)?;
    if let Value::Mapping(desired) = value {
        let mut after = doc.clone();
        let old = node
            .members()?
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>();
        for name in old
            .iter()
            .filter(|name| !desired.iter().any(|(n, _)| n == *name))
        {
            let mut patches = vec![];
            after.derive_member_drop(locate(&after, path)?, name, &mut patches)?;
            after = after.patched(patches)?;
        }
        for (name, child) in desired {
            let current = locate(&after, path)?;
            if current.get(name).is_some() {
                let key = current
                    .members()?
                    .iter()
                    .find(|m| m.name == *name)
                    .unwrap()
                    .name
                    .as_str();
                // Declaration syntax has a fixed vocabulary; no arbitrary YAML
                // member name becomes a frontend or persistence abstraction.
                let key = match key {
                    "name" => "name",
                    "fields" => "fields",
                    "target" => "target",
                    "table" => "table",
                    "csharpName" => "csharpName",
                    "nonUnique" => "nonUnique",
                    _ => return Err(error("unsupported declaration member")),
                };
                let mut next = path.to_vec();
                next.push(Position::Member(key));
                after = update(&after, &next, child)?;
            } else {
                let mut patches = vec![];
                after.derive_member_add(current, name, child, &mut patches)?;
                after = after.patched(patches)?;
            }
        }
        return Ok(after);
    }
    if let Value::Sequence(desired) = value {
        let mut after = doc.clone();
        while locate(&after, path)?.items()?.len() > desired.len() {
            after = after.remove_sequence(locate(&after, path)?, desired.len())?;
        }
        while locate(&after, path)?.items()?.len() < desired.len() {
            let at = locate(&after, path)?.items()?.len();
            after = after.insert_sequence(locate(&after, path)?, at, &desired[at])?;
        }
        let mut patches = vec![];
        source::derive_patch(&after, locate(&after, path)?, value, &mut patches)?;
        return after.patched(patches);
    }
    let mut patches = vec![];
    source::derive_patch(doc, node, value, &mut patches)?;
    doc.patched(patches)
}
fn insert(doc: &Document, member: &'static str, value: &Value) -> Result<Document> {
    if let Some(sequence) = doc.root.get(member) {
        return doc.insert_sequence(sequence, sequence.items()?.len(), value);
    }
    let mut patches = vec![];
    doc.derive_member_add(
        &doc.root,
        member,
        &Value::Sequence(vec![value.clone()]),
        &mut patches,
    )?;
    doc.patched(patches)
}
fn validate_reference(project: &Project, table: &Table, reference: &Reference) -> Result<()> {
    let resolved = project.resolve_reference(table, reference)?;
    for (owner, names) in [
        (table, &reference.fields),
        (
            project.tables[&resolved.target_table].as_ref(),
            &reference.target_fields,
        ),
    ] {
        for name in names {
            migration::field_resolution(
                project,
                owner
                    .fields
                    .iter()
                    .find(|field| field.name == *name)
                    .ok_or_else(|| error("Reference component is missing"))?,
            )?;
        }
    }
    Ok(())
}
pub fn derive(project: &Project, command: Command) -> Result<Plan> {
    use Position::{Item, Member};
    let table = target(project, &command.table)?;
    let before = project.sources[&table.source]
        .document
        .as_ref()
        .ok_or_else(|| error("schema source is unavailable"))?;
    if command.source != table.source || command.identity != before.identity {
        return Err(Error::new(
            "E-MIGRATION-STALE",
            "saved schema changed; reopen Table detail before reviewing this occurrence",
        ));
    }
    let mut expected = (**table).clone();
    let mut keys = BTreeSet::new();
    let after = match &command.change {
        Change::SetFieldKey { occurrence, key } => {
            let field = expected
                .fields
                .get_mut(*occurrence)
                .ok_or_else(|| error("field occurrence is missing"))?;
            field.key = key
                .parse()
                .map_err(|_| Error::new("E-SCHEMA-KEY", "non-negative integer key required"))?;
            update(
                before,
                &[Member("fields"), Item(*occurrence), Member("key")],
                &Value::Literal(key.clone()),
            )?
        }
        Change::SetPrimaryKey { fields } => {
            keys.insert(expected.primary.fields.clone());
            keys.insert(fields.clone());
            expected.primary.fields = fields.clone();
            update(
                before,
                &[Member("primaryKey"), Member("fields")],
                &strings(fields),
            )?
        }
        Change::AddSecondaryKey { fields, non_unique } => {
            keys.insert(fields.clone());
            expected.secondary.push(Key {
                fields: fields.clone(),
                non_unique: *non_unique,
            });
            insert(
                before,
                "secondaryKeys",
                &key_value(fields, *non_unique, None),
            )?
        }
        Change::EditSecondaryKey {
            occurrence,
            fields,
            non_unique,
        } => {
            let key = expected
                .secondary
                .get_mut(*occurrence)
                .ok_or_else(|| error("key occurrence is missing"))?;
            keys.insert(key.fields.clone());
            keys.insert(fields.clone());
            *key = Key {
                fields: fields.clone(),
                non_unique: *non_unique,
            };
            let path = [Member("secondaryKeys"), Item(*occurrence)];
            update(
                before,
                &path,
                &key_value(fields, *non_unique, Some(locate(before, &path)?)),
            )?
        }
        Change::RemoveSecondaryKey { occurrence } => {
            let key = expected
                .secondary
                .get(*occurrence)
                .ok_or_else(|| error("key occurrence is missing"))?;
            keys.insert(key.fields.clone());
            expected.secondary.remove(*occurrence);
            before.remove_sequence(before.root.required("secondaryKeys")?, *occurrence)?
        }
        Change::AddReference { declaration } => {
            expected.references.push(declaration.resolved());
            insert(before, "references", &declaration.value())?
        }
        Change::EditReference {
            occurrence,
            declaration,
        } => {
            *expected
                .references
                .get_mut(*occurrence)
                .ok_or_else(|| error("Reference occurrence is missing"))? = declaration.resolved();
            update(
                before,
                &[Member("references"), Item(*occurrence)],
                &declaration.value(),
            )?
        }
        Change::RemoveReference { occurrence } => {
            if expected.references.get(*occurrence).is_none() {
                return Err(error("Reference occurrence is missing"));
            }
            expected.references.remove(*occurrence);
            before.remove_sequence(before.root.required("references")?, *occurrence)?
        }
    };
    let after = Arc::new(after);
    if semantic::parse_table(&after, &table.source)? != expected {
        return Err(error("declaration differs from the captured intent"));
    }
    if !keys.is_empty() {
        for (path, source) in &project.sources {
            if source.kind.as_deref() != Some("schema") {
                continue;
            }
            let Some(doc) = &source.document else {
                continue;
            };
            let Some(references) = doc.root.get("references") else {
                continue;
            };
            for entry in references.items()? {
                if entry.value.required("target")?.required("table")?.text()? == table.name
                    && !project.tables.values().any(|t| t.source == *path)
                {
                    return Err(error(format!(
                        "{path}: incoming Reference declaration cannot be resolved"
                    )));
                }
            }
        }
    }
    let mut transformed = project.clone();
    let mut changed = (**project.sources.get(&table.source).unwrap()).clone();
    changed.bytes = after.bytes.clone();
    changed.identity = after.identity.clone();
    changed.document = Some(after.clone());
    transformed
        .sources
        .insert(table.source.clone(), Arc::new(changed));
    transformed.rebuild_declarations();
    let mut expected_tables = project.tables.clone();
    expected_tables.insert(table.name.clone(), Arc::new(expected));
    if transformed.tables != expected_tables || transformed.types != project.types {
        return Err(error("non-target declarations changed"));
    }
    for diagnostic in &transformed.declaration_problems {
        if !project.declaration_problems.iter().any(|old| {
            old.source == diagnostic.source
                && old.code == diagnostic.code
                && old.message == diagnostic.message
        }) {
            return Err(Error::new(
                "E-TABLE-DECLARATION",
                format!(
                    "{}: {}: {}",
                    diagnostic.source, diagnostic.code, diagnostic.message
                ),
            ));
        }
    }
    // Changed declarations and affected incoming References must resolve even
    // if that diagnostic already existed. Record validity is a Build concern.
    let changed = &transformed.tables[&table.name];
    match &command.change {
        Change::SetPrimaryKey { fields }
        | Change::AddSecondaryKey { fields, .. }
        | Change::EditSecondaryKey { fields, .. } => {
            for name in fields {
                let field = changed
                    .fields
                    .iter()
                    .find(|f| f.name == *name)
                    .ok_or_else(|| error("key field is missing"))?;
                migration::field_resolution(&transformed, field)?;
                if !semantic::key_capable(field, &transformed.types) {
                    return Err(error("key field is not key-capable"));
                }
            }
        }
        Change::AddReference { .. } => {
            validate_reference(&transformed, changed, changed.references.last().unwrap())?;
        }
        Change::EditReference { occurrence, .. } => {
            validate_reference(&transformed, changed, &changed.references[*occurrence])?;
        }
        _ => {}
    }
    for t in transformed.tables.values() {
        for reference in &t.references {
            if reference.target_table == table.name && keys.contains(&reference.target_fields) {
                validate_reference(&transformed, t, reference)?;
            }
        }
    }
    let candidates = if before.bytes == after.bytes {
        BTreeMap::new()
    } else {
        BTreeMap::from([(
            table.source.clone(),
            Candidate {
                before: before.clone(),
                after,
            },
        )])
    };
    Ok(Plan {
        destructive: command.change.destructive(),
        command: migration::Command::TableDeclaration { command },
        affected_records: 0,
        candidates,
    })
}
