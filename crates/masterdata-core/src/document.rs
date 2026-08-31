use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{Diagnostic, ErrorKind, MasterdataError, Result};

/// A table schema document. `key` is deliberately a MessagePack key, not a
/// logical field identity; the latter was retired by specification change
/// 0003. Type modifiers are represented as fields rather than being encoded
/// in `type_name` so the resolver can share one semantic path for tables and
/// Custom Types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SchemaDocument {
    pub kind: String,
    pub table: String,
    #[serde(rename = "csharpName", default)]
    pub csharp_name: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    #[serde(rename = "primaryKey", default)]
    pub primary_key: Option<PrimaryKeyDefinition>,
    #[serde(rename = "secondaryKeys", default)]
    pub secondary_keys: Vec<SecondaryKeyDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FieldDefinition {
    pub key: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub array: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrimaryKeyDefinition {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SecondaryKeyDefinition {
    pub fields: Vec<String>,
    #[serde(rename = "nonUnique", default)]
    pub non_unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypeDocument {
    pub kind: String,
    pub name: String,
    #[serde(rename = "valueObject", default)]
    pub value_object: Option<ValueObjectDefinition>,
    #[serde(default)]
    pub custom: Option<CustomTypeDefinition>,
    #[serde(rename = "enum", default)]
    pub enum_definition: Option<EnumDefinition>,
    #[serde(default)]
    pub flags: Option<FlagsDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValueObjectDefinition {
    pub underlying: String,
    #[serde(default)]
    pub conversions: ConversionDefinition,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConversionDefinition {
    #[serde(rename = "fromUnderlyingImplicit", default)]
    pub from_underlying_implicit: bool,
    #[serde(rename = "toUnderlyingImplicit", default)]
    pub to_underlying_implicit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomTypeDefinition {
    pub fields: Vec<TypeFieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypeFieldDefinition {
    pub key: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub array: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnumDefinition {
    pub underlying: String,
    pub members: Vec<EnumMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlagsDefinition {
    pub underlying: String,
    pub members: Vec<EnumMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnumMember {
    pub name: String,
    pub value: IntegerLiteral,
}

/// Enum values are kept as a signed wide integer after YAML deserialization.
/// The declared underlying type later applies the fixed-width range check;
/// retaining the YAML number as a typed integer avoids silently accepting
/// floating-point or string values as enum numbers.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntegerLiteral(pub i128);

impl<'de> Deserialize<'de> for IntegerLiteral {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let integer = match value {
            Value::Number(number) => number
                .as_i64()
                .map(i128::from)
                .or_else(|| number.as_u64().map(i128::from)),
            _ => None,
        };
        integer
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("enum member value must be an integer scalar"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DataDocument {
    pub kind: String,
    pub table: String,
    #[serde(default)]
    pub records: Vec<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceDocument {
    Schema(SchemaDocument),
    Data(DataDocument),
    Type(TypeDocument),
}

impl SourceDocument {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Schema(_) => "schema",
            Self::Data(_) => "data",
            Self::Type(_) => "type",
        }
    }

    pub fn table_identity(&self) -> Option<&str> {
        match self {
            Self::Schema(document) => Some(&document.table),
            Self::Data(document) => Some(&document.table),
            Self::Type(_) => None,
        }
    }

    pub fn type_name(&self) -> Option<&str> {
        match self {
            Self::Type(document) => Some(&document.name),
            Self::Schema(_) | Self::Data(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDocument {
    pub path: PathBuf,
    pub document: SourceDocument,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectDocuments {
    pub files: Vec<LoadedDocument>,
}

impl ProjectDocuments {
    pub fn schemas(&self) -> impl Iterator<Item = (&PathBuf, &SchemaDocument)> {
        self.files
            .iter()
            .filter_map(|loaded| match &loaded.document {
                SourceDocument::Schema(document) => Some((&loaded.path, document)),
                SourceDocument::Data(_) | SourceDocument::Type(_) => None,
            })
    }

    pub fn data(&self) -> impl Iterator<Item = (&PathBuf, &DataDocument)> {
        self.files
            .iter()
            .filter_map(|loaded| match &loaded.document {
                SourceDocument::Data(document) => Some((&loaded.path, document)),
                SourceDocument::Schema(_) | SourceDocument::Type(_) => None,
            })
    }

    pub fn types(&self) -> impl Iterator<Item = (&PathBuf, &TypeDocument)> {
        self.files
            .iter()
            .filter_map(|loaded| match &loaded.document {
                SourceDocument::Type(document) => Some((&loaded.path, document)),
                SourceDocument::Schema(_) | SourceDocument::Data(_) => None,
            })
    }
}

pub fn parse_yaml_document(path: PathBuf, content: &str) -> Result<LoadedDocument> {
    let value: Value = serde_yaml::from_str(content).map_err(|error| {
        MasterdataError::new(
            "E-YAML-PARSE",
            ErrorKind::Parse,
            format!("could not parse YAML: {error}"),
        )
        .with_source(path.clone())
    })?;

    let kind = value
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String("kind".to_owned())))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            MasterdataError::new(
                "E-YAML-MISSING-KIND",
                ErrorKind::Parse,
                "YAML document must declare a string `kind` field",
            )
            .with_source(path.clone())
        })?;

    // Dispatching through a typed struct is intentional: type declarations
    // must not remain a stringly-typed map whose category or modifier meaning
    // is invented later by code generation.
    let document = match kind.as_str() {
        "schema" => serde_yaml::from_value::<SchemaDocument>(value).map(SourceDocument::Schema),
        "data" => serde_yaml::from_value::<DataDocument>(value).map(SourceDocument::Data),
        "type" => serde_yaml::from_value::<TypeDocument>(value).map(SourceDocument::Type),
        other => {
            return Err(MasterdataError::new(
                "E-YAML-UNKNOWN-KIND",
                ErrorKind::Parse,
                format!("unsupported YAML document kind `{other}`"),
            )
            .with_source(path));
        }
    }
    .map_err(|error| {
        MasterdataError::new(
            "E-YAML-SHAPE",
            ErrorKind::Parse,
            format!("document does not match the `{kind}` shape: {error}"),
        )
        .with_source(path.clone())
    })?;

    Ok(LoadedDocument { path, document })
}

pub fn schema_diagnostic(code: &str, message: impl Into<String>, path: &Path) -> Diagnostic {
    Diagnostic::new(code, ErrorKind::Validation, message).with_source(path.to_path_buf())
}
