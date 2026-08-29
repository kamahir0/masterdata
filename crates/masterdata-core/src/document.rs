use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{Diagnostic, ErrorKind, MasterdataError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaDocument {
    pub kind: String,
    pub table: String,
    #[serde(rename = "csharpName", default)]
    pub csharp_name: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    #[serde(rename = "reservedFields", default)]
    pub reserved_fields: Vec<ReservedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDefinition {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReservedField {
    pub id: u32,
    #[serde(rename = "formerName", default)]
    pub former_name: Option<String>,
    #[serde(rename = "formerType", default)]
    pub former_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

impl SourceDocument {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Schema(_) => "schema",
            Self::Data(_) => "data",
        }
    }

    pub fn table(&self) -> &str {
        match self {
            Self::Schema(document) => &document.table,
            Self::Data(document) => &document.table,
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
                SourceDocument::Data(_) => None,
            })
    }

    pub fn data(&self) -> impl Iterator<Item = (&PathBuf, &DataDocument)> {
        self.files
            .iter()
            .filter_map(|loaded| match &loaded.document {
                SourceDocument::Schema(_) => None,
                SourceDocument::Data(document) => Some((&loaded.path, document)),
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

    let document = match kind.as_str() {
        "schema" => serde_yaml::from_value::<SchemaDocument>(value).map(SourceDocument::Schema),
        "data" => serde_yaml::from_value::<DataDocument>(value).map(SourceDocument::Data),
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
