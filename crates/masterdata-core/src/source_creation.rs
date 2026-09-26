//! Pure typed source construction. Filesystem ownership remains in the host.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceCreation {
    Folder,
    /// High-level intent used by the desktop Explorer.  The concrete
    /// declaration is deliberately expanded here, rather than in a GUI
    /// adapter, so starter defaults and identity validation remain shared
    /// domain semantics.
    Starter {
        kind: StarterKind,
        #[serde(default)]
        table: Option<String>,
    },
    Table {
        table: String,
        #[serde(rename = "inlineRecords", default)]
        inline_records: bool,
        #[serde(rename = "csharpName")]
        csharp_name: Option<String>,
        fields: Vec<FieldDefinition>,
        #[serde(rename = "primaryKey")]
        primary_key: PrimaryKeyDefinition,
        #[serde(rename = "secondaryKeys", default)]
        secondary_keys: Vec<SecondaryKeyDefinition>,
    },
    Data {
        table: String,
    },
    ValueObject {
        name: String,
        underlying: String,
        conversions: ConversionDefinition,
    },
    Enum {
        name: String,
        underlying: String,
        members: Vec<CreationMember>,
    },
    Flags {
        name: String,
        underlying: String,
        members: Vec<CreationMember>,
    },
    CustomType {
        name: String,
        fields: Vec<TypeFieldDefinition>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StarterKind {
    Table,
    Data,
    ValueObject,
    Enum,
    Flags,
    CustomType,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationMember {
    pub name: String,
    pub value: String,
}
#[derive(Debug, Clone)]
pub struct SourceCreationPlan {
    pub source: String,
    pub document: LoadedDocument,
    pub resolution: ProjectDocuments,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationChoices {
    pub field_types: Vec<String>,
    pub tables: Vec<String>,
    pub value_object_underlyings: Vec<String>,
    pub enum_underlyings: Vec<String>,
}
pub fn creation_choices(documents: &ProjectDocuments) -> CreationChoices {
    let primitives = [
        PrimitiveType::Bool,
        PrimitiveType::Int,
        PrimitiveType::UInt,
        PrimitiveType::Long,
        PrimitiveType::ULong,
        PrimitiveType::Float,
        PrimitiveType::Double,
        PrimitiveType::String,
    ];
    let mut field_types = primitives
        .iter()
        .map(|p| p.name().to_owned())
        .collect::<Vec<_>>();
    field_types.extend(
        documents
            .types()
            .map(|(_, ty)| ty.name.clone())
            .collect::<BTreeSet<_>>(),
    );
    CreationChoices {
        field_types,
        tables: documents
            .schemas()
            .filter(|(_, schema)| {
                documents
                    .schemas()
                    .filter(|(_, other)| other.table == schema.table)
                    .count()
                    == 1
            })
            .map(|(_, schema)| schema.table.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        value_object_underlyings: primitives
            .iter()
            .filter(|p| p.is_key_compatible())
            .map(|p| p.name().to_owned())
            .collect(),
        enum_underlyings: primitives
            .iter()
            .filter(|p| p.is_integer())
            .map(|p| p.name().to_owned())
            .collect(),
    }
}

pub fn prepare_source_creation(
    documents: &ProjectDocuments,
    path: &Path,
    request: &SourceCreation,
) -> Result<SourceCreationPlan> {
    let document = match request {
        SourceCreation::Folder => {
            return Err(creation_error(
                "E-SOURCE-CREATE-CATEGORY",
                "folder has no source document",
                path,
            ));
        }
        SourceCreation::Starter { kind, table } => {
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    creation_error(
                        "E-SOURCE-CREATE-IDENTITY",
                        "starter filename must have a valid identity stem",
                        path,
                    )
                })?;
            let expanded = starter_request(kind, stem, table.as_deref(), path)?;
            return prepare_source_creation(documents, path, &expanded);
        }
        SourceCreation::Table {
            table,
            inline_records,
            csharp_name,
            fields,
            primary_key,
            secondary_keys,
        } => SourceDocument::Schema(SchemaDocument {
            kind: "schema".into(),
            table: table.clone(),
            csharp_name: csharp_name.clone(),
            fields: fields.clone(),
            primary_key: Some(primary_key.clone()),
            secondary_keys: secondary_keys.clone(),
            references: Vec::new(),
            records: inline_records.then(Vec::new),
        }),
        SourceCreation::Data { table } => {
            if documents
                .schemas()
                .filter(|(_, schema)| schema.table == *table)
                .count()
                != 1
            {
                return Err(creation_error(
                    "E-SOURCE-CREATE-TABLE",
                    "Data creation requires exactly one existing Table schema",
                    path,
                ));
            }
            SourceDocument::Data(DataDocument {
                kind: "data".into(),
                table: table.clone(),
                records: Vec::new(),
            })
        }
        _ => {
            let mut ty = TypeDocument {
                kind: "type".into(),
                name: String::new(),
                value_object: None,
                custom: None,
                enum_definition: None,
                flags: None,
            };
            match request {
                SourceCreation::ValueObject {
                    name,
                    underlying,
                    conversions,
                } => {
                    ty.name = name.clone();
                    ty.value_object = Some(ValueObjectDefinition {
                        underlying: underlying.clone(),
                        conversions: conversions.clone(),
                    });
                }
                SourceCreation::CustomType { name, fields } => {
                    ty.name = name.clone();
                    ty.custom = Some(CustomTypeDefinition {
                        fields: fields.clone(),
                    });
                }
                SourceCreation::Enum {
                    name,
                    underlying,
                    members,
                }
                | SourceCreation::Flags {
                    name,
                    underlying,
                    members,
                } => {
                    ty.name = name.clone();
                    let members = members
                        .iter()
                        .enumerate()
                        .map(|(index, member)| {
                            member
                                .value
                                .parse::<i128>()
                                .map(|value| EnumMember {
                                    name: member.name.clone(),
                                    value: IntegerLiteral(value),
                                })
                                .map_err(|_| {
                                    let mut error = creation_error(
                                        "E-SOURCE-CREATE-MEMBER",
                                        "member value must be an explicit integer",
                                        path,
                                    );
                                    error.diagnostic.schema_path =
                                        Some(format!("members[{index}].value"));
                                    error
                                })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    if matches!(request, SourceCreation::Enum { .. }) {
                        ty.enum_definition = Some(EnumDefinition {
                            underlying: underlying.clone(),
                            members,
                        });
                    } else {
                        ty.flags = Some(FlagsDefinition {
                            underlying: underlying.clone(),
                            members,
                        });
                    }
                }
                _ => unreachable!(),
            }
            SourceDocument::Type(ty)
        }
    };
    for existing in &documents.files {
        let collision = match (&document, &existing.document) {
            (SourceDocument::Schema(a), SourceDocument::Schema(b)) => a.table == b.table,
            (SourceDocument::Type(a), SourceDocument::Type(b)) => a.name == b.name,
            _ => false,
        };
        if collision {
            return Err(creation_error(
                "E-SOURCE-CREATE-IDENTITY",
                "declaration identity already exists in this Project",
                path,
            ));
        }
    }
    let mut value = match &document {
        SourceDocument::Schema(d) => serde_yaml::to_value(d),
        SourceDocument::Data(d) => serde_yaml::to_value(d),
        SourceDocument::Type(d) => serde_yaml::to_value(d),
    }
    .map_err(|error| creation_error("E-SOURCE-CREATE-RENDER", error.to_string(), path))?;
    if let Some(mapping) = value.as_mapping_mut() {
        mapping.retain(|_, value| !value.is_null());
    }
    let source = serde_yaml::to_string(&value)
        .map_err(|error| creation_error("E-SOURCE-CREATE-RENDER", error.to_string(), path))?;
    let loaded = parse_yaml_document(path.to_path_buf(), &source)?;
    if loaded.document != document {
        return Err(creation_error(
            "E-SOURCE-CREATE-ROUNDTRIP",
            "rendered declaration differs from the request",
            path,
        ));
    }
    let resolution = creation_resolution(documents, &loaded);
    let report = validate_documents(&resolution);
    if let Some(diagnostic) = report.diagnostics.into_iter().next() {
        return Err(MasterdataError {
            diagnostic: Box::new(diagnostic),
        });
    }
    Ok(SourceCreationPlan {
        source,
        document: loaded,
        resolution,
    })
}

fn starter_request(
    kind: &StarterKind,
    stem: &str,
    table: Option<&str>,
    path: &Path,
) -> Result<SourceCreation> {
    Ok(match kind {
        StarterKind::Table => SourceCreation::Table {
            table: starter_table_name(stem, path)?,
            inline_records: true,
            csharp_name: None,
            fields: vec![FieldDefinition {
                key: 0,
                name: "id".to_owned(),
                type_name: "int".to_owned(),
                nullable: false,
                array: false,
            }],
            primary_key: PrimaryKeyDefinition {
                fields: vec!["id".to_owned()],
            },
            secondary_keys: Vec::new(),
        },
        StarterKind::Data => SourceCreation::Data {
            table: table
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    creation_error(
                        "E-SOURCE-CREATE-TABLE",
                        "Data creation requires an explicit existing Table selection",
                        path,
                    )
                })?
                .to_owned(),
        },
        StarterKind::ValueObject => SourceCreation::ValueObject {
            name: starter_type_name(stem, path)?,
            underlying: "int".to_owned(),
            conversions: ConversionDefinition::default(),
        },
        StarterKind::Enum => SourceCreation::Enum {
            name: starter_type_name(stem, path)?,
            underlying: "int".to_owned(),
            members: vec![CreationMember {
                name: "Default".to_owned(),
                value: "0".to_owned(),
            }],
        },
        StarterKind::Flags => SourceCreation::Flags {
            name: starter_type_name(stem, path)?,
            underlying: "int".to_owned(),
            members: vec![CreationMember {
                name: "None".to_owned(),
                value: "0".to_owned(),
            }],
        },
        StarterKind::CustomType => SourceCreation::CustomType {
            name: starter_type_name(stem, path)?,
            fields: vec![TypeFieldDefinition {
                key: 0,
                name: "value".to_owned(),
                type_name: "int".to_owned(),
                nullable: false,
                array: false,
            }],
        },
    })
}

fn starter_segments(stem: &str) -> Vec<String> {
    stem.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

fn starter_table_name(stem: &str, path: &Path) -> Result<String> {
    let mut segments = starter_segments(stem);
    if segments.is_empty() {
        return Err(creation_error(
            "E-SOURCE-CREATE-IDENTITY",
            "starter filename must contain an ASCII identity segment",
            path,
        ));
    }
    if segments[0].as_bytes()[0].is_ascii_digit() {
        segments[0].insert_str(0, "table");
    }
    Ok(segments.join("-"))
}

fn starter_type_name(stem: &str, path: &Path) -> Result<String> {
    let segments = starter_segments(stem);
    if segments.is_empty() {
        return Err(creation_error(
            "E-SOURCE-CREATE-IDENTITY",
            "starter filename must contain an ASCII identity segment",
            path,
        ));
    }
    let mut result = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index == 0 && segment.as_bytes()[0].is_ascii_digit() {
            result.push_str("Type");
        }
        let mut characters = segment.chars();
        if let Some(first) = characters.next() {
            result.push(first.to_ascii_uppercase());
            result.extend(characters);
        }
    }
    Ok(result)
}

fn creation_resolution(
    documents: &ProjectDocuments,
    candidate: &LoadedDocument,
) -> ProjectDocuments {
    // WHY: only the new declaration and the symbols it uses gate creation.
    // Project-wide validation would block creation on unrelated existing errors.
    // Keep every matching dependency declaration so duplicate symbols still fail.
    // EVIDENCE: SOURCE-CREATE-010; docs/specs/source-creation.md.
    let mut files = vec![candidate.clone()];
    if let SourceDocument::Data(data) = &candidate.document {
        files.extend(documents.files.iter().filter(|file| matches!(&file.document, SourceDocument::Schema(schema) if schema.table == data.table)).cloned());
    }
    let mut pending = BTreeSet::new();
    for file in &files {
        referenced_types(&file.document, &mut pending);
    }
    let mut visited = BTreeSet::new();
    if let SourceDocument::Type(ty) = &candidate.document {
        visited.insert(ty.name.clone());
    }
    while let Some(name) = pending.pop_first() {
        if PrimitiveType::parse(&name).is_some() || !visited.insert(name.clone()) {
            continue;
        }
        for file in documents
            .files
            .iter()
            .filter(|file| file.document.type_name() == Some(&name))
        {
            referenced_types(&file.document, &mut pending);
            files.push(file.clone());
        }
    }
    ProjectDocuments { files }
}
fn referenced_types(document: &SourceDocument, names: &mut BTreeSet<String>) {
    match document {
        SourceDocument::Schema(schema) => {
            names.extend(schema.fields.iter().map(|field| field.type_name.clone()))
        }
        SourceDocument::Type(ty) => {
            if let Some(custom) = &ty.custom {
                names.extend(custom.fields.iter().map(|field| field.type_name.clone()));
            }
        }
        _ => {}
    }
}
pub(crate) fn creation_error(
    code: &str,
    message: impl Into<String>,
    path: &Path,
) -> MasterdataError {
    let mut error =
        MasterdataError::new(code, ErrorKind::Validation, message).with_source(PathBuf::from(path));
    error
        .diagnostic
        .related_requirements
        .push("SOURCE-CREATE-010".into());
    error
}
