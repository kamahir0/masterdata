use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{ErrorKind, MasterdataError, Result};

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
    if source_declares_type(content) {
        validate_type_integer_scalar_lexemes(&path, content)?;
    }

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

#[derive(Default)]
struct TypeIntegerLexicalState {
    category_indent: Option<usize>,
    members_indent: Option<usize>,
    member_indent: Option<usize>,
    member_fields_indent: Option<usize>,
    block_scalar_parent_indent: Option<usize>,
}

fn source_declares_type(content: &str) -> bool {
    for line in content.lines() {
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();
        if indent != 0 {
            continue;
        }
        let code = strip_yaml_comment(line);
        let Some((is_sequence_item, key, raw_value)) = parse_mapping_entry(code) else {
            continue;
        };
        if !is_sequence_item && yaml_key_is(key, "kind") {
            let raw_value = raw_value.trim();
            return raw_value == "type"
                || matches!(
                    serde_yaml::from_str::<Value>(raw_value),
                    Ok(Value::String(decoded)) if decoded == "type"
                );
        }
    }
    false
}

/// Validate the source spelling of Enum/Flags member integer values before
/// serde_yaml turns YAML numeric-looking scalars into `Value::Number`.
///
/// This is intentionally a narrow lexical gate for the one Type System path
/// where the source spelling is part of the approved contract. It recognizes
/// the surrounding block structure needed to reach `enum/flags.members` but
/// does not attempt to parse YAML generally. Quoted values, comments, and
/// block scalar contents are therefore left to the normal YAML parser and
/// typed shape validation.
fn validate_type_integer_scalar_lexemes(path: &Path, content: &str) -> Result<()> {
    let mut state = TypeIntegerLexicalState::default();

    for line in content.lines() {
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();

        if let Some(parent_indent) = state.block_scalar_parent_indent {
            if line.trim().is_empty() || indent > parent_indent {
                continue;
            }
            state.block_scalar_parent_indent = None;
        }

        let code = strip_yaml_comment(line);
        let trimmed = code.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed_entry = parse_mapping_entry(code);
        if let Some(members_indent) = state.members_indent {
            let is_members_item = parsed_entry
                .as_ref()
                .is_some_and(|(is_sequence_item, _, _)| *is_sequence_item);
            if indent < members_indent || (indent == members_indent && !is_members_item) {
                state.members_indent = None;
                state.member_indent = None;
                state.member_fields_indent = None;
            }
        }
        if let Some(member_indent) = state.member_indent
            && indent < member_indent
        {
            state.member_indent = None;
            state.member_fields_indent = None;
        }

        let Some((is_sequence_item, key, raw_value)) = parsed_entry else {
            continue;
        };

        if is_block_scalar_indicator(raw_value) {
            state.block_scalar_parent_indent = Some(indent);
        }

        if yaml_key_is(key, "enum") || yaml_key_is(key, "flags") {
            state.category_indent = Some(indent);
            state.members_indent = None;
            state.member_indent = None;
            state.member_fields_indent = None;
            continue;
        }

        if yaml_key_is(key, "members")
            && state
                .category_indent
                .is_some_and(|category_indent| indent > category_indent)
        {
            state.members_indent = Some(indent);
            state.member_indent = None;
            state.member_fields_indent = None;
            continue;
        }

        let Some(members_indent) = state.members_indent else {
            continue;
        };

        if indent < members_indent || (indent == members_indent && !is_sequence_item) {
            continue;
        }

        if is_sequence_item_marker(trimmed) {
            state.member_indent = Some(indent);
            state.member_fields_indent = None;
            continue;
        }

        if is_sequence_item {
            state.member_indent = Some(indent);
            state.member_fields_indent = None;
        } else if state
            .member_indent
            .is_none_or(|member_indent| indent < member_indent)
        {
            continue;
        }

        if !is_sequence_item {
            match state.member_fields_indent {
                Some(member_fields_indent) if indent != member_fields_indent => continue,
                Some(_) => {}
                None => state.member_fields_indent = Some(indent),
            }
        }

        if !yaml_key_is(key, "value") {
            continue;
        }

        if is_block_scalar_indicator(raw_value) || is_quoted_scalar(raw_value) {
            continue;
        }

        let raw_value = strip_yaml_comment(raw_value).trim();
        if looks_like_numeric_scalar(raw_value) && !is_masterdata_integer_lexeme(raw_value) {
            return Err(MasterdataError::new(
                "E-YAML-INVALID-INTEGER",
                ErrorKind::Parse,
                format!(
                    "Enum/Flags member integer `{raw_value}` does not match the Masterdata YAML integer grammar"
                ),
            )
            .with_source(path.to_path_buf())
            .with_related_requirement("YAML-SUBSET-011"));
        }
    }

    Ok(())
}

fn parse_mapping_entry(line: &str) -> Option<(bool, &str, &str)> {
    let trimmed = line.trim_start();
    let (is_sequence_item, mapping) = if trimmed == "-" {
        (true, "")
    } else if let Some(sequence_rest) = trimmed.strip_prefix('-') {
        if sequence_rest
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            (true, sequence_rest.trim_start())
        } else {
            (false, trimmed)
        }
    } else {
        (false, trimmed)
    };

    if mapping.is_empty() {
        return is_sequence_item.then_some((true, "", ""));
    }

    let colon = find_mapping_colon(mapping)?;
    let key = mapping[..colon].trim();
    if key.is_empty() {
        return None;
    }
    Some((is_sequence_item, key, mapping[colon + 1..].trim_start()))
}

fn find_mapping_colon(value: &str) -> Option<usize> {
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

fn is_sequence_item_marker(value: &str) -> bool {
    value == "-"
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
            None => match bytes[index] {
                b'\'' | b'"' => {
                    quote = Some(bytes[index]);
                    index += 1;
                }
                b'#' if index == 0 || bytes[index - 1].is_ascii_whitespace() => {
                    return &value[..index];
                }
                _ => index += 1,
            },
            _ => unreachable!(),
        }
    }
    value
}

fn yaml_key_is(key: &str, expected: &str) -> bool {
    if key == expected {
        return true;
    }
    matches!(
        serde_yaml::from_str::<Value>(key),
        Ok(Value::String(decoded)) if decoded == expected
    )
}

fn is_block_scalar_indicator(value: &str) -> bool {
    value.trim_start().starts_with(['|', '>'])
}

fn is_quoted_scalar(value: &str) -> bool {
    matches!(value.trim_start().as_bytes().first(), Some(b'\'' | b'"'))
}

fn looks_like_numeric_scalar(value: &str) -> bool {
    matches!(
        value.as_bytes().first(),
        Some(b'+' | b'-' | b'.' | b'0'..=b'9')
    )
}

fn is_masterdata_integer_lexeme(value: &str) -> bool {
    if value == "0" || value == "-0" {
        return true;
    }

    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && digits.as_bytes()[0].is_ascii_digit()
        && digits.as_bytes()[0] != b'0'
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}
