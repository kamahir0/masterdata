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
    // WHY: Shared semantic preparation must hash the exact UTF-8 source that
    // was parsed, rather than consulting native filesystem state again.
    // IF REMOVED: a browser or native snapshot could validate one source and
    // hash a different later filesystem version.
    // EVIDENCE: docs/specs/runtime-hosts.md; docs/adr/0006-host-capability-composition.md
    // Regression: schema_source_hash_uses_loaded_source_content.
    pub source: String,
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
    let source = content.to_owned();
    // serde_yaml normalizes numeric-looking and legacy YAML scalar spellings
    // before typed deserialization. Keep the subset lexical gate before that
    // boundary so forbidden spellings cannot become indistinguishable from
    // canonical values (YAML-SUBSET-011, YAML-SUBSET-012, TYPE-PRIMITIVE-003).
    let content = validate_masterdata_yaml_lexemes(&path, content)?;

    let value: Value = serde_yaml::from_str(&content).map_err(|error| {
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

    Ok(LoadedDocument {
        path,
        source,
        document,
    })
}

#[derive(Default)]
struct TypeIntegerLexicalState {
    category_indent: Option<usize>,
    members_indent: Option<usize>,
    member_indent: Option<usize>,
    member_fields_indent: Option<usize>,
}

struct SourceLine<'a> {
    offset: usize,
    text: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct ParsedMappingEntry<'a> {
    is_sequence_item: bool,
    key: &'a str,
    raw_value: &'a str,
    value_start: usize,
}

#[derive(Default)]
struct FlowLexicalState {
    sequence_depth: usize,
    quote: Option<u8>,
}

#[derive(Default)]
struct MasterdataLexicalState {
    type_integer: TypeIntegerLexicalState,
    block_scalar_parent_indent: Option<usize>,
    flow: FlowLexicalState,
}

struct LexicalReplacement {
    start: usize,
    end: usize,
    replacement: String,
}

fn source_declares_type(content: &str) -> bool {
    for line in content.lines() {
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();
        if indent != 0 {
            continue;
        }
        let code = strip_yaml_comment(line);
        let Some(entry) = parse_mapping_entry(code) else {
            continue;
        };
        if !entry.is_sequence_item && yaml_key_is(entry.key, "kind") {
            let raw_value = entry.raw_value.trim();
            return raw_value == "type"
                || matches!(
                    serde_yaml::from_str::<Value>(raw_value),
                    Ok(Value::String(decoded)) if decoded == "type"
                );
        }
    }
    false
}

fn validate_masterdata_yaml_lexemes(path: &Path, content: &str) -> Result<String> {
    let type_document = source_declares_type(content);
    let lines = source_lines(content);
    let mut state = MasterdataLexicalState::default();
    let mut replacements = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        let indent = yaml_indent(line.text);
        if let Some(parent_indent) = state.block_scalar_parent_indent {
            if line.text.trim().is_empty() || indent > parent_indent {
                continue;
            }
            state.block_scalar_parent_indent = None;
        }

        let code = if state.flow.quote.is_some() {
            line.text
        } else {
            strip_yaml_comment(line.text)
        };
        if code.trim().is_empty() {
            continue;
        }

        let parsed_entry = parse_mapping_entry(code);
        let type_integer = type_document
            && update_type_integer_state(&mut state.type_integer, indent, parsed_entry.as_ref());

        if let Some(entry) = parsed_entry {
            if entry.is_sequence_item && entry.key.is_empty() {
                continue;
            }

            if is_block_scalar_indicator(entry.raw_value) {
                state.block_scalar_parent_indent = Some(indent);
                continue;
            }

            if entry.raw_value.is_empty() {
                if !has_nested_block_value(&lines, line_index, indent, !entry.is_sequence_item) {
                    return Err(yaml_lexical_error(
                        path,
                        "E-YAML-MISSING-VALUE",
                        format!("mapping entry `{}` has no explicit value", entry.key),
                        "YAML-SUBSET-017",
                    ));
                }
                continue;
            }

            let value_start = line.offset + entry.value_start;
            if entry.raw_value.trim_start().starts_with('[') {
                scan_flow_fragment(
                    path,
                    entry.raw_value,
                    value_start,
                    &mut state.flow,
                    type_integer,
                    &mut replacements,
                )?;
            } else {
                let token = entry.raw_value.trim();
                let token_start = value_start + entry.raw_value.find(token).unwrap_or(0);
                validate_plain_scalar(path, token, token_start, type_integer, &mut replacements)?;
            }
        } else if state.flow.sequence_depth > 0 {
            scan_flow_fragment(
                path,
                code,
                line.offset,
                &mut state.flow,
                false,
                &mut replacements,
            )?;
        } else if let Some((value_start, value)) = parse_block_sequence_scalar(code) {
            let token = value.trim();
            let token_start = line.offset + value_start + value.find(token).unwrap_or(0);
            if token.starts_with('[') {
                scan_flow_fragment(
                    path,
                    token,
                    token_start,
                    &mut state.flow,
                    false,
                    &mut replacements,
                )?;
            } else {
                validate_plain_scalar(path, token, token_start, false, &mut replacements)?;
            }
        }
    }

    Ok(apply_lexical_replacements(content, &mut replacements))
}

fn source_lines(content: &str) -> Vec<SourceLine<'_>> {
    let mut lines = Vec::new();
    let mut offset = 0;
    for segment in content.split_inclusive('\n') {
        let without_newline = segment.strip_suffix('\n').unwrap_or(segment);
        let text = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        lines.push(SourceLine { offset, text });
        offset += segment.len();
    }
    if content.is_empty() {
        lines.push(SourceLine {
            offset: 0,
            text: "",
        });
    }
    lines
}

fn yaml_indent(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn parse_mapping_entry(line: &str) -> Option<ParsedMappingEntry<'_>> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let (is_sequence_item, mapping, mapping_start) = if trimmed == "-" {
        (true, "", line.len())
    } else if let Some(sequence_rest) = trimmed.strip_prefix('-') {
        if sequence_rest
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            (
                true,
                sequence_rest.trim_start(),
                leading + 1 + sequence_rest.len() - sequence_rest.trim_start().len(),
            )
        } else {
            (false, trimmed, leading)
        }
    } else {
        (false, trimmed, leading)
    };

    if mapping.is_empty() {
        return is_sequence_item.then_some(ParsedMappingEntry {
            is_sequence_item: true,
            key: "",
            raw_value: "",
            value_start: line.len(),
        });
    }

    let colon = find_mapping_colon(mapping)?;
    let key = mapping[..colon].trim();
    if key.is_empty() {
        return None;
    }
    let value = &mapping[colon + 1..];
    let raw_value = value.trim_start();
    Some(ParsedMappingEntry {
        is_sequence_item,
        key,
        raw_value,
        value_start: mapping_start + colon + 1 + value.len() - raw_value.len(),
    })
}

fn update_type_integer_state(
    state: &mut TypeIntegerLexicalState,
    indent: usize,
    entry: Option<&ParsedMappingEntry<'_>>,
) -> bool {
    if let Some(members_indent) = state.members_indent {
        let is_members_item = entry.is_some_and(|entry| entry.is_sequence_item);
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

    let Some(entry) = entry else {
        return false;
    };

    if yaml_key_is(entry.key, "enum") || yaml_key_is(entry.key, "flags") {
        state.category_indent = Some(indent);
        state.members_indent = None;
        state.member_indent = None;
        state.member_fields_indent = None;
        return false;
    }

    if yaml_key_is(entry.key, "members")
        && state
            .category_indent
            .is_some_and(|category_indent| indent > category_indent)
    {
        state.members_indent = Some(indent);
        state.member_indent = None;
        state.member_fields_indent = None;
        return false;
    }

    let Some(members_indent) = state.members_indent else {
        return false;
    };
    if indent < members_indent || (indent == members_indent && !entry.is_sequence_item) {
        return false;
    }

    if entry.is_sequence_item && entry.key.is_empty() {
        state.member_indent = Some(indent);
        state.member_fields_indent = None;
        return false;
    }

    if entry.is_sequence_item {
        state.member_indent = Some(indent);
        state.member_fields_indent = None;
    } else if state
        .member_indent
        .is_none_or(|member_indent| indent < member_indent)
    {
        return false;
    }

    if !entry.is_sequence_item {
        match state.member_fields_indent {
            Some(member_fields_indent) if indent != member_fields_indent => return false,
            Some(_) => {}
            None => state.member_fields_indent = Some(indent),
        }
    }

    yaml_key_is(entry.key, "value")
}

fn has_nested_block_value(
    lines: &[SourceLine<'_>],
    line_index: usize,
    parent_indent: usize,
    allow_indentless_sequence: bool,
) -> bool {
    for line in lines.iter().skip(line_index + 1) {
        let code = strip_yaml_comment(line.text);
        if code.trim().is_empty() {
            continue;
        }
        let child_indent = yaml_indent(line.text);
        return child_indent > parent_indent
            || (allow_indentless_sequence
                && child_indent == parent_indent
                && is_block_sequence_marker(code));
    }
    false
}

fn is_block_sequence_marker(value: &str) -> bool {
    let value = value.trim_start();
    value == "-"
        || value
            .strip_prefix('-')
            .is_some_and(|rest| rest.as_bytes().first().is_some_and(u8::is_ascii_whitespace))
}

fn parse_block_sequence_scalar(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let leading = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix('-')?;
    if !rest.is_empty()
        && !rest
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        return None;
    }
    let value = rest.trim_start();
    if value.is_empty() || find_mapping_colon(value).is_some() {
        return None;
    }
    Some((leading + 1 + rest.len() - value.len(), value))
}

fn scan_flow_fragment(
    path: &Path,
    fragment: &str,
    absolute_start: usize,
    state: &mut FlowLexicalState,
    type_integer: bool,
    replacements: &mut Vec<LexicalReplacement>,
) -> Result<()> {
    let mut token_start = None;
    let bytes = fragment.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if let Some(quote) = state.quote {
            if quote == b'\'' {
                if bytes[index] == b'\'' && bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else if bytes[index] == b'\'' {
                    state.quote = None;
                    index += 1;
                } else {
                    index += 1;
                }
            } else if bytes[index] == b'\\' {
                index += 2;
            } else if bytes[index] == b'"' {
                state.quote = None;
                index += 1;
            } else {
                index += 1;
            }
            continue;
        }

        match bytes[index] {
            b'\'' | b'"' => {
                token_start = None;
                state.quote = Some(bytes[index]);
                index += 1;
            }
            b'[' => {
                finish_flow_token(
                    path,
                    fragment,
                    absolute_start,
                    &mut token_start,
                    index,
                    type_integer,
                    replacements,
                )?;
                state.sequence_depth += 1;
                index += 1;
            }
            b']' => {
                finish_flow_token(
                    path,
                    fragment,
                    absolute_start,
                    &mut token_start,
                    index,
                    type_integer,
                    replacements,
                )?;
                state.sequence_depth = state.sequence_depth.saturating_sub(1);
                index += 1;
            }
            b',' | b'{' | b'}' => {
                finish_flow_token(
                    path,
                    fragment,
                    absolute_start,
                    &mut token_start,
                    index,
                    type_integer,
                    replacements,
                )?;
                index += 1;
            }
            b':' if bytes
                .get(index + 1)
                .is_none_or(|next| next.is_ascii_whitespace() || *next == b']') =>
            {
                finish_flow_token(
                    path,
                    fragment,
                    absolute_start,
                    &mut token_start,
                    index,
                    type_integer,
                    replacements,
                )?;
                index += 1;
            }
            b'#' if index == 0 || bytes[index - 1].is_ascii_whitespace() => {
                finish_flow_token(
                    path,
                    fragment,
                    absolute_start,
                    &mut token_start,
                    index,
                    type_integer,
                    replacements,
                )?;
                break;
            }
            byte if !byte.is_ascii_whitespace() && token_start.is_none() => {
                token_start = Some(index);
                index += 1;
            }
            _ => index += 1,
        }
    }

    finish_flow_token(
        path,
        fragment,
        absolute_start,
        &mut token_start,
        bytes.len(),
        type_integer,
        replacements,
    )
}

fn finish_flow_token(
    path: &Path,
    fragment: &str,
    absolute_start: usize,
    token_start: &mut Option<usize>,
    end: usize,
    type_integer: bool,
    replacements: &mut Vec<LexicalReplacement>,
) -> Result<()> {
    let Some(start) = token_start.take() else {
        return Ok(());
    };
    let raw = &fragment[start..end];
    let token = raw.trim();
    if token.is_empty() {
        return Ok(());
    }
    let token_start = absolute_start + start + raw.find(token).unwrap_or(0);
    validate_plain_scalar(path, token, token_start, type_integer, replacements)
}

fn validate_plain_scalar(
    path: &Path,
    token: &str,
    token_start: usize,
    type_integer: bool,
    replacements: &mut Vec<LexicalReplacement>,
) -> Result<()> {
    if token.is_empty() || is_quoted_scalar(token) {
        return Ok(());
    }

    if type_integer && (looks_like_numeric_scalar(token) || is_nonfinite_float(token)) {
        if !is_masterdata_integer_lexeme(token) {
            return Err(yaml_lexical_error(
                path,
                "E-YAML-INVALID-INTEGER",
                format!(
                    "Enum/Flags member integer `{token}` does not match the Masterdata YAML integer grammar"
                ),
                "YAML-SUBSET-011",
            ));
        }
        return Ok(());
    }

    if token == "~" {
        return Err(yaml_lexical_error(
            path,
            "E-YAML-INVALID-NULL",
            "`~` is not supported as a Masterdata null literal",
            "YAML-SUBSET-010",
        ));
    }

    if let Some(replacement) = noncanonical_plain_string(token) {
        replacements.push(LexicalReplacement {
            start: token_start,
            end: token_start + token.len(),
            replacement,
        });
        return Ok(());
    }

    if token == "true" || token == "false" || token == "null" {
        return Ok(());
    }

    if is_nonfinite_float(token) || looks_like_numeric_scalar(token) {
        if is_masterdata_integer_lexeme(token) || is_masterdata_float_lexeme(token) {
            return Ok(());
        }
        let (code, requirement, category) = if is_float_like_scalar(token) {
            ("E-YAML-INVALID-FLOAT", "YAML-SUBSET-012", "floating-point")
        } else {
            ("E-YAML-INVALID-INTEGER", "YAML-SUBSET-011", "integer")
        };
        return Err(yaml_lexical_error(
            path,
            code,
            format!("{category} scalar `{token}` is not supported by the Masterdata YAML subset"),
            requirement,
        ));
    }

    Ok(())
}

fn noncanonical_plain_string(token: &str) -> Option<String> {
    let lower = token.to_ascii_lowercase();
    if !matches!(
        lower.as_str(),
        "true" | "false" | "null" | "yes" | "no" | "on" | "off"
    ) {
        return None;
    }
    if matches!(token, "true" | "false" | "null") {
        return None;
    }
    Some(format!("'{token}'"))
}

fn yaml_lexical_error(
    path: &Path,
    code: &str,
    message: impl Into<String>,
    requirement: &str,
) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Parse, message)
        .with_source(path.to_path_buf())
        .with_related_requirement(requirement)
}

fn apply_lexical_replacements(content: &str, replacements: &mut Vec<LexicalReplacement>) -> String {
    if replacements.is_empty() {
        return content.to_owned();
    }
    replacements.sort_unstable_by_key(|replacement| replacement.start);
    let mut normalized = content.to_owned();
    for replacement in replacements.drain(..).rev() {
        normalized.replace_range(replacement.start..replacement.end, &replacement.replacement);
    }
    normalized
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
    let unsigned = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    if unsigned.is_empty() {
        return false;
    }
    if is_nonfinite_float(value) {
        return true;
    }
    match unsigned.as_bytes().first() {
        Some(b'.') => unsigned.as_bytes().get(1).is_some_and(u8::is_ascii_digit),
        Some(byte) if byte.is_ascii_digit() => {
            if unsigned.bytes().all(|byte| byte.is_ascii_digit()) {
                return true;
            }
            if unsigned.len() > 1
                && unsigned.starts_with('0')
                && matches!(
                    unsigned.as_bytes()[1],
                    b'x' | b'X' | b'o' | b'O' | b'b' | b'B'
                )
            {
                return true;
            }
            if let Some(index) = unsigned.find(['e', 'E']) {
                let mantissa = &unsigned[..index];
                let exponent = &unsigned[index + 1..];
                let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
                return !mantissa.is_empty()
                    && mantissa
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'_'))
                    && mantissa.bytes().any(|byte| byte.is_ascii_digit())
                    && !exponent.is_empty()
                    && exponent.bytes().all(|byte| byte.is_ascii_digit());
            }
            unsigned
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'_'))
        }
        _ => false,
    }
}

fn is_masterdata_float_lexeme(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    if value.is_empty() || value.starts_with('+') {
        return false;
    }

    let (mantissa, exponent) = match value.find(['e', 'E']) {
        Some(index) if value[index + 1..].find(['e', 'E']).is_none() => {
            (&value[..index], Some(&value[index + 1..]))
        }
        Some(_) => return false,
        None => (value, None),
    };

    if let Some(exponent) = exponent {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !exponent.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }

    match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            !whole.is_empty()
                && !fraction.is_empty()
                && whole.bytes().all(|byte| byte.is_ascii_digit())
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => exponent.is_some() && mantissa.bytes().all(|byte| byte.is_ascii_digit()),
    }
}

fn is_float_like_scalar(value: &str) -> bool {
    if is_nonfinite_float(value) || value.contains('.') {
        return true;
    }
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    let Some(index) = unsigned.find(['e', 'E']) else {
        return false;
    };
    unsigned[..index].bytes().all(|byte| byte.is_ascii_digit())
}

fn is_nonfinite_float(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "nan"
            | "+nan"
            | "-nan"
            | "infinity"
            | "+infinity"
            | "-infinity"
            | ".nan"
            | "+.nan"
            | "-.nan"
            | ".inf"
            | "+.inf"
            | "-.inf"
            | "inf"
            | "+inf"
            | "-inf"
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
