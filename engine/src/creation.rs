//! Typed, deterministic construction of new source artifacts, never an existing
//! document serializer. Existing source mutations remain localized CST patches.
use crate::{
    Error, Result,
    project::Project,
    semantic::{self, Field, Key, Reference},
    source::{self, Document, Value},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    pub key: String,
    pub name: String,
    pub type_name: String,
    pub nullable: bool,
    pub array: bool,
}
impl From<Field> for Declaration {
    fn from(f: Field) -> Self {
        Self {
            key: f.key.to_string(),
            name: f.name,
            type_name: f.type_name,
            nullable: f.nullable,
            array: f.array,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "category",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Artifact {
    Folder,
    Table {
        name: String,
        csharp_name: Option<String>,
        fields: Vec<Declaration>,
        primary: Key,
        secondary: Vec<Key>,
        #[serde(default)]
        references: Vec<Reference>,
        inline: bool,
    },
    Data {
        table: String,
    },
    ValueObject {
        name: String,
        underlying: String,
        from_implicit: bool,
        to_implicit: bool,
    },
    Enum {
        name: String,
        underlying: String,
        members: Vec<(String, String)>,
    },
    Flags {
        name: String,
        underlying: String,
        members: Vec<(String, String)>,
    },
    Custom {
        name: String,
        fields: Vec<Declaration>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub root: String,
    pub path: String,
    pub artifact: Artifact,
}

impl Artifact {
    pub fn identity(&self) -> Option<&str> {
        match self {
            Self::Folder => None,
            Self::Data { table } => Some(table),
            Self::Table { name, .. }
            | Self::ValueObject { name, .. }
            | Self::Enum { name, .. }
            | Self::Flags { name, .. }
            | Self::Custom { name, .. } => Some(name),
        }
    }
    pub fn category(&self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Table { .. } => "table",
            Self::Data { .. } => "data",
            Self::ValueObject { .. } => "valueObject",
            Self::Enum { .. } => "enum",
            Self::Flags { .. } => "flags",
            Self::Custom { .. } => "custom",
        }
    }
}
fn text(s: impl Into<String>) -> Value {
    Value::Text(s.into())
}
fn boolean(b: bool) -> Value {
    Value::Literal(b.to_string())
}
fn map(items: Vec<(&str, Value)>) -> Value {
    Value::Mapping(items.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn names(items: &[String]) -> Value {
    Value::Sequence(items.iter().map(|s| text(s.clone())).collect())
}
fn fields(items: &[Declaration]) -> Value {
    Value::Sequence(
        items
            .iter()
            .map(|f| {
                map(vec![
                    ("key", Value::Literal(f.key.clone())),
                    ("name", text(&f.name)),
                    ("type", text(&f.type_name)),
                    ("nullable", boolean(f.nullable)),
                    ("array", boolean(f.array)),
                ])
            })
            .collect(),
    )
}
fn key(k: &Key, secondary: bool) -> Value {
    let mut items = vec![("fields", names(&k.fields))];
    if secondary {
        items.push(("nonUnique", boolean(k.non_unique)));
    }
    map(items)
}
pub fn document(artifact: &Artifact) -> Result<Option<Document>> {
    let value = match artifact {
        Artifact::Folder => return Ok(None),
        Artifact::Table {
            name,
            csharp_name,
            fields: fs,
            primary,
            secondary,
            references,
            inline,
        } => {
            if primary.non_unique {
                return Err(Error::new("E-KEY-SHAPE", "Primary Key cannot be nonunique"));
            }
            let mut items = vec![("kind", text("schema")), ("table", text(name))];
            if let Some(n) = csharp_name {
                items.push(("csharpName", text(n)));
            }
            items.extend([
                ("fields", fields(fs)),
                ("primaryKey", key(primary, false)),
                (
                    "secondaryKeys",
                    Value::Sequence(secondary.iter().map(|k| key(k, true)).collect()),
                ),
            ]);
            if !references.is_empty() {
                items.push((
                    "references",
                    Value::Sequence(
                        references
                            .iter()
                            .map(|r| {
                                map(vec![
                                    ("name", text(&r.name)),
                                    ("fields", names(&r.fields)),
                                    (
                                        "target",
                                        map(vec![
                                            ("table", text(&r.target_table)),
                                            ("fields", names(&r.target_fields)),
                                        ]),
                                    ),
                                    ("csharpName", text(&r.csharp_name)),
                                ])
                            })
                            .collect(),
                    ),
                ));
            }
            if *inline {
                items.push(("records", Value::Sequence(vec![])));
            }
            map(items)
        }
        Artifact::Data { table } => map(vec![
            ("kind", text("data")),
            ("table", text(table)),
            ("records", Value::Sequence(vec![])),
        ]),
        Artifact::ValueObject {
            name,
            underlying,
            from_implicit,
            to_implicit,
        } => map(vec![
            ("kind", text("type")),
            ("name", text(name)),
            (
                "valueObject",
                map(vec![
                    ("underlying", text(underlying)),
                    (
                        "conversions",
                        map(vec![
                            ("fromUnderlyingImplicit", boolean(*from_implicit)),
                            ("toUnderlyingImplicit", boolean(*to_implicit)),
                        ]),
                    ),
                ]),
            ),
        ]),
        Artifact::Enum {
            name,
            underlying,
            members,
        }
        | Artifact::Flags {
            name,
            underlying,
            members,
        } => {
            let category = if matches!(artifact, Artifact::Flags { .. }) {
                "flags"
            } else {
                "enum"
            };
            map(vec![
                ("kind", text("type")),
                ("name", text(name)),
                (
                    category,
                    map(vec![
                        ("underlying", text(underlying)),
                        (
                            "members",
                            Value::Sequence(
                                members
                                    .iter()
                                    .map(|(n, v)| {
                                        map(vec![
                                            ("name", text(n)),
                                            ("value", Value::Literal(v.clone())),
                                        ])
                                    })
                                    .collect(),
                            ),
                        ),
                    ]),
                ),
            ])
        }
        Artifact::Custom { name, fields: fs } => map(vec![
            ("kind", text("type")),
            ("name", text(name)),
            ("custom", map(vec![("fields", fields(fs))])),
        ]),
    };
    let doc = Document::parse(format!("{}\n", source::render_block(&value, 0, "\n")?))?;
    if !doc.subset_issues.is_empty() {
        return Err(Error::new(
            "E-YAML-SUBSET",
            "candidate uses unsupported source syntax",
        ));
    }
    match artifact {
        Artifact::Table { .. } => {
            semantic::parse_table(&doc, "")?;
        }
        Artifact::Data { table } => {
            if !semantic::kebab(table) {
                return Err(Error::new("E-TABLE-NAME", table));
            }
            doc.records()?;
        }
        _ => {
            semantic::parse_type(&doc)?;
        }
    }
    Ok(Some(doc))
}

/// Validation compares declarations, not existing record validity. An unrelated
/// bad record must not prevent creation; errors introduced by the candidate must.
pub fn candidate(project: &Project, request: &Request) -> Result<Option<Document>> {
    let doc = document(&request.artifact)?;
    let Some(doc) = doc else {
        return Ok(None);
    };
    let identity = request.artifact.identity().unwrap();
    let declaring_kind = match request.artifact {
        Artifact::Table { .. } | Artifact::Data { .. } => "schema",
        _ => "type",
    };
    let owners: Vec<_> = project
        .sources
        .values()
        .filter(|s| {
            s.kind.as_deref() == Some(declaring_kind)
                && s.document
                    .as_ref()
                    .and_then(|d| {
                        d.root.get(if declaring_kind == "type" {
                            "name"
                        } else {
                            "table"
                        })
                    })
                    .and_then(|n| n.text().ok())
                    == Some(identity)
        })
        .collect();
    if matches!(request.artifact, Artifact::Data { .. }) {
        if owners.len() != 1 || !project.tables.contains_key(identity) {
            return Err(Error::new(
                "E-TABLE-MISSING",
                "Data requires exactly one existing Table schema",
            ));
        }
        if project
            .declaration_problems
            .iter()
            .any(|p| p.source == owners[0].path)
        {
            return Err(Error::new(
                "E-CREATE-DEPENDENCY",
                "selected Table declaration is invalid",
            ));
        }
    } else if !owners.is_empty() {
        return Err(Error::new(
            "E-CREATE-IDENTITY",
            "declaration identity already exists",
        ));
    }
    let generated = match &request.artifact {
        Artifact::Data { .. } => None,
        Artifact::Table { .. } => Some(semantic::parse_table(&doc, &request.path)?.csharp_name),
        _ => Some(identity.to_owned()),
    };
    if generated.as_ref().is_some_and(|name| {
        project.types.contains_key(name)
            || project.tables.values().any(|t| &t.csharp_name == name)
            || matches!(name.as_str(), "MemoryDatabase")
    }) {
        return Err(Error::new(
            "E-CSHARP-COLLISION",
            "generated declaration name already exists or is reserved",
        ));
    }
    let previous: BTreeSet<_> = project
        .declaration_problems
        .iter()
        .map(|p| (p.source.clone(), p.code.clone(), p.message.clone()))
        .collect();
    let mut proposed = project.clone();
    proposed.sources.insert(
        request.path.clone(),
        Arc::new(crate::project::Source {
            path: request.path.clone(),
            physical: project.root.join(&request.path),
            bytes: doc.bytes.clone(),
            identity: doc.identity.clone(),
            kind: Some(semantic::text(&doc.root, "kind")?),
            binding: Some(identity.into()),
            document: Some(Arc::new(doc.clone())),
            error: None,
        }),
    );
    proposed.rebuild_declarations();
    let required = match &request.artifact {
        Artifact::Table { name, .. } | Artifact::Data { table: name } => proposed
            .tables
            .get(name)
            .map(|t| proposed.dependencies(t))
            .unwrap_or_default(),
        Artifact::Custom { .. } => {
            let fields = semantic::parse_type(&doc)?.1;
            let semantic::Type::Custom { fields } = fields else {
                unreachable!()
            };
            let table = semantic::Table {
                name: String::new(),
                csharp_name: String::new(),
                source: request.path.clone(),
                fields,
                primary: Key {
                    fields: vec![],
                    non_unique: false,
                },
                secondary: vec![],
                references: vec![],
            };
            proposed.dependencies(&table)
        }
        _ => vec![request.path.clone()],
    };
    if let Some(problem) = proposed.declaration_problems.iter().find(|p| {
        p.source == request.path
            || required.contains(&p.source)
            || !previous.contains(&(p.source.clone(), p.code.clone(), p.message.clone()))
    }) {
        return Err(Error::new(
            "E-CREATE-DECLARATION",
            format!("{}: {}", problem.code, problem.message),
        ));
    }
    Ok(Some(doc))
}

pub fn defaults(
    project: &Project,
    category: &str,
    path: &str,
    table: Option<&str>,
) -> Result<Artifact> {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let words: Vec<_> = stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let table_name = words
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-");
    let table_name = if semantic::kebab(&table_name) {
        table_name
    } else {
        "new-table".into()
    };
    let name: String = words.iter().map(|s| semantic::public_name(s)).collect();
    let name = if semantic::upper_identifier(&name) {
        name
    } else {
        "NewType".into()
    };
    let field = |name: &str| Declaration {
        key: "0".into(),
        name: name.into(),
        type_name: "int".into(),
        nullable: false,
        array: false,
    };
    Ok(match category {
        "folder" => Artifact::Folder,
        "table" => Artifact::Table {
            name: table_name,
            csharp_name: None,
            fields: vec![field("id")],
            primary: Key {
                fields: vec!["id".into()],
                non_unique: false,
            },
            secondary: vec![],
            references: vec![],
            inline: true,
        },
        "data" => Artifact::Data {
            table: table
                .map(str::to_owned)
                .or_else(|| {
                    (project.tables.len() == 1)
                        .then(|| project.tables.keys().next().unwrap().clone())
                })
                .unwrap_or_default(),
        },
        "valueObject" => Artifact::ValueObject {
            name: if matches!(
                name.as_str(),
                "Value" | "Equals" | "GetHashCode" | "ToString" | "CompareTo"
            ) {
                format!("{name}Id")
            } else {
                name
            },
            underlying: "int".into(),
            from_implicit: false,
            to_implicit: false,
        },
        "enum" => Artifact::Enum {
            name: name.clone(),
            underlying: "int".into(),
            members: vec![(
                if name == "None" { "Default" } else { "None" }.into(),
                "0".into(),
            )],
        },
        "flags" => Artifact::Flags {
            name: if name == "None" {
                "NoneFlags".into()
            } else {
                name.clone()
            },
            underlying: "int".into(),
            members: vec![
                ("None".into(), "0".into()),
                (
                    if name == "Enabled" {
                        "Active"
                    } else {
                        "Enabled"
                    }
                    .into(),
                    "1".into(),
                ),
            ],
        },
        "custom" => Artifact::Custom {
            name: name.clone(),
            fields: vec![field(if name == "Amount" {
                "quantity"
            } else {
                "amount"
            })],
        },
        _ => return Err(Error::new("E-CREATE-CATEGORY", "unknown artifact category")),
    })
}
pub fn choices(project: &Project) -> serde_json::Value {
    let field_types = [
        "bool", "int", "uint", "long", "ulong", "float", "double", "string",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(project.types.keys().cloned())
    .collect::<Vec<_>>();
    serde_json::json!({"fieldTypes":field_types,"valueObjectUnderlying":["int","uint","long","ulong","string"],"enumUnderlying":["int","uint","long","ulong"],"tables":project.tables.keys().collect::<Vec<_>>()})
}

pub fn suggest_field(existing: &[Declaration]) -> Result<Declaration> {
    let key = (0..=i32::MAX as u32)
        .find(|key| {
            !existing
                .iter()
                .any(|f| f.key.parse::<i128>().ok() == Some(*key as i128))
        })
        .ok_or_else(|| Error::new("E-SCHEMA-KEY", "no unused MessagePack key"))?;
    let name = (0..)
        .map(|n| {
            if n == 0 {
                "field".into()
            } else {
                format!("field{n}")
            }
        })
        .find(|name| !existing.iter().any(|f| f.name == *name))
        .unwrap();
    Ok(Declaration {
        key: key.to_string(),
        name,
        type_name: "string".into(),
        nullable: true,
        array: false,
    })
}
