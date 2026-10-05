//! Schema-directed interpretation shared by Desktop, CLI, migration and delivery.
use crate::{
    Error, Result,
    source::{Document, Node, Raw, Style, Value},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub key: u32,
    pub name: String,
    pub type_name: String,
    pub nullable: bool,
    pub array: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    pub fields: Vec<String>,
    pub non_unique: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub name: String,
    pub fields: Vec<String>,
    pub target_table: String,
    pub target_fields: Vec<String>,
    pub csharp_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,
    pub csharp_name: String,
    pub source: String,
    pub fields: Vec<Field>,
    pub primary: Key,
    pub secondary: Vec<Key>,
    pub references: Vec<Reference>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "category", rename_all = "camelCase")]
pub enum Type {
    ValueObject {
        underlying: String,
        from_implicit: bool,
        to_implicit: bool,
    },
    Custom {
        fields: Vec<Field>,
    },
    Enum {
        underlying: String,
        flags: bool,
        members: Vec<(String, String)>,
    },
}

pub type Types = BTreeMap<String, Type>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Typed {
    Null,
    Boolean(bool),
    Integer(String),
    Float(String),
    Text(String),
    Enum(String),
    Flags(Vec<String>),
    Mapping(Vec<(String, Typed)>),
    Array(Vec<Typed>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueProblem {
    pub code: String,
    pub message: String,
    pub path: Vec<String>,
}

impl ValueProblem {
    fn new(message: impl Into<String>) -> Self {
        Self {
            code: "E-VALUE".into(),
            message: message.into(),
            path: vec![],
        }
    }
    fn at(mut self, path: impl Into<String>) -> Self {
        self.path.insert(0, path.into());
        self
    }
}
type ValueResult<T> = std::result::Result<T, ValueProblem>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    pub type_name: String,
    pub category: String,
    pub nullable: bool,
    pub array: bool,
    pub underlying: Option<String>,
    pub members: Vec<String>,
    pub fields: Vec<(Field, Shape)>,
}

pub fn shape(field: &Field, types: &Types) -> Result<Shape> {
    fn build(f: &Field, types: &Types, visiting: &mut BTreeSet<String>) -> Result<Shape> {
        let mut s = Shape {
            type_name: f.type_name.clone(),
            category: "primitive".into(),
            nullable: f.nullable,
            array: f.array,
            underlying: None,
            members: vec![],
            fields: vec![],
        };
        if primitive(&f.type_name) {
            return Ok(s);
        }
        if !visiting.insert(f.type_name.clone()) {
            return Err(Error::new("E-TYPE-CYCLE", f.type_name.clone()));
        }
        match types
            .get(&f.type_name)
            .ok_or_else(|| Error::new("E-TYPE-UNRESOLVED", f.type_name.clone()))?
        {
            Type::ValueObject { underlying, .. } => {
                s.category = "valueObject".into();
                s.underlying = Some(underlying.clone());
            }
            Type::Enum {
                underlying,
                flags,
                members,
            } => {
                s.category = if *flags { "flags" } else { "enum" }.into();
                s.underlying = Some(underlying.clone());
                s.members = members.iter().map(|(n, _)| n.clone()).collect();
            }
            Type::Custom { fields } => {
                s.category = "custom".into();
                s.fields = fields
                    .iter()
                    .map(|f| Ok((f.clone(), build(f, types, visiting)?)))
                    .collect::<Result<_>>()?;
            }
        }
        visiting.remove(&f.type_name);
        Ok(s)
    }
    build(field, types, &mut BTreeSet::new())
}

pub fn primitive(s: &str) -> bool {
    matches!(
        s,
        "bool" | "int" | "uint" | "long" | "ulong" | "float" | "double" | "string"
    )
}
pub fn integral(s: &str) -> bool {
    matches!(s, "int" | "uint" | "long" | "ulong")
}
pub fn integer_grammar(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    s == "0"
        || (s
            .as_bytes()
            .first()
            .is_some_and(|c| (b'1'..=b'9').contains(c))
            && s.bytes().all(|b| b.is_ascii_digit()))
}
pub fn float_grammar(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let parts: Vec<_> = s.split(['e', 'E']).collect();
    if parts.len() > 2 {
        return false;
    }
    let exponent = parts
        .get(1)
        .map(|p| p.strip_prefix(['+', '-']).unwrap_or(p));
    if exponent.is_some_and(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    match parts[0].split_once('.') {
        Some((a, b)) => digits(a) && digits(b),
        None => parts.len() == 2 && digits(parts[0]),
    }
}
pub fn integer_value(s: &str, target: &str) -> ValueResult<i128> {
    if !integer_grammar(s) {
        return Err(ValueProblem::new("base-10 integer grammar required"));
    }
    let n = s
        .parse::<i128>()
        .map_err(|_| ValueProblem::new("integer outside representable range"))?;
    let (min, max) = match target {
        "int" => (i32::MIN as i128, i32::MAX as i128),
        "uint" => (0, u32::MAX as i128),
        "long" => (i64::MIN as i128, i64::MAX as i128),
        "ulong" => (0, u64::MAX as i128),
        _ => return Err(ValueProblem::new("unknown integer type")),
    };
    if n < min || n > max {
        return Err(ValueProblem::new(format!("outside {target} range")));
    }
    Ok(n)
}

pub fn interpret(field: &Field, node: &Node, types: &Types) -> ValueResult<Typed> {
    if matches!(node.raw, Raw::Null) {
        return if field.nullable {
            Ok(Typed::Null)
        } else {
            Err(ValueProblem::new("non-null value required"))
        };
    }
    if !node.safe {
        return Err(ValueProblem::new("unsupported source representation"));
    }
    if field.array {
        let Raw::Sequence(items) = &node.raw else {
            return Err(ValueProblem::new("Array requires a sequence"));
        };
        return items
            .iter()
            .enumerate()
            .map(|(i, n)| {
                interpret_base(&field.type_name, &n.value, types, 0)
                    .map_err(|e| e.at(i.to_string()))
            })
            .collect::<ValueResult<Vec<_>>>()
            .map(Typed::Array);
    }
    interpret_base(&field.type_name, node, types, 0)
}

fn interpret_base(target: &str, node: &Node, types: &Types, depth: usize) -> ValueResult<Typed> {
    if depth > types.len() + 1 {
        return Err(ValueProblem::new("type resolution depth exceeded"));
    }
    if !node.safe {
        return Err(ValueProblem::new("unsupported source representation"));
    }
    if primitive(target) {
        let Raw::Scalar(s) = &node.raw else {
            return Err(ValueProblem::new("scalar required"));
        };
        return match target {
            "string" => Ok(Typed::Text(s.clone())),
            "bool" => match s.as_str() {
                "true" => Ok(Typed::Boolean(true)),
                "false" => Ok(Typed::Boolean(false)),
                _ => Err(ValueProblem::new("true or false required")),
            },
            "int" | "uint" | "long" | "ulong" => {
                integer_value(s, target).map(|n| Typed::Integer(n.to_string()))
            }
            "float" | "double" => {
                if !float_grammar(s) {
                    return Err(ValueProblem::new("fraction or exponent required"));
                }
                let valid = if target == "float" {
                    s.parse::<f32>().is_ok_and(f32::is_finite)
                } else {
                    s.parse::<f64>().is_ok_and(f64::is_finite)
                };
                if valid {
                    Ok(Typed::Float(s.clone()))
                } else {
                    Err(ValueProblem::new("finite floating value required"))
                }
            }
            _ => unreachable!(),
        };
    }
    match types
        .get(target)
        .ok_or_else(|| ValueProblem::new(format!("unresolved type {target}")))?
    {
        Type::ValueObject { underlying, .. } => interpret_base(underlying, node, types, depth + 1),
        Type::Enum {
            flags: false,
            members,
            ..
        } => {
            let name = node
                .text()
                .map_err(|_| ValueProblem::new("symbolic enum member required"))?;
            if members.iter().any(|(n, _)| n == name) {
                Ok(Typed::Enum(name.into()))
            } else {
                Err(ValueProblem::new(format!("unknown {target} member {name}")))
            }
        }
        Type::Enum {
            flags: true,
            members,
            ..
        } => {
            let items = node
                .items()
                .map_err(|_| ValueProblem::new("Flags require a member sequence"))?;
            let mut values = BTreeSet::new();
            for (i, n) in items.iter().enumerate() {
                let name = n.value.text().map_err(|_| {
                    ValueProblem::new("symbolic Flags member required").at(i.to_string())
                })?;
                if !members.iter().any(|(n, _)| n == name) || !values.insert(name.to_owned()) {
                    return Err(ValueProblem::new(format!(
                        "unknown / duplicate Flags member {name}"
                    ))
                    .at(i.to_string()));
                }
            }
            if values.is_empty() || (values.contains("None") && values.len() != 1) {
                return Err(ValueProblem::new(
                    "None must appear alone; Flags cannot be empty",
                ));
            }
            Ok(Typed::Flags(values.into_iter().collect()))
        }
        Type::Custom { fields } => {
            let mapping = node
                .members()
                .map_err(|_| ValueProblem::new("Custom requires a mapping"))?;
            if let Some(m) = mapping
                .iter()
                .find(|m| !fields.iter().any(|f| f.name == m.name))
            {
                return Err(ValueProblem::new("unknown Custom member").at(m.name.clone()));
            }
            let mut output = Vec::new();
            for f in fields {
                let n = node
                    .get(&f.name)
                    .ok_or_else(|| ValueProblem::new("field entry required").at(f.name.clone()))?;
                let v = if matches!(n.raw, Raw::Null) && f.nullable {
                    Typed::Null
                } else if f.array {
                    let items = n.items().map_err(|_| {
                        ValueProblem::new("Array requires sequence").at(f.name.clone())
                    })?;
                    Typed::Array(
                        items
                            .iter()
                            .enumerate()
                            .map(|(i, n)| {
                                interpret_base(&f.type_name, &n.value, types, depth + 1)
                                    .map_err(|e| e.at(i.to_string()).at(f.name.clone()))
                            })
                            .collect::<ValueResult<_>>()?,
                    )
                } else {
                    interpret_base(&f.type_name, n, types, depth + 1)
                        .map_err(|e| e.at(f.name.clone()))?
                };
                output.push((f.name.clone(), v));
            }
            Ok(Typed::Mapping(output))
        }
    }
}

pub fn authoring_input(shape: &Shape, text: &str) -> Value {
    let string = shape.type_name == "string"
        || shape.underlying.as_deref() == Some("string")
        || shape.category == "enum";
    if !string && (integer_grammar(text) || float_grammar(text) || matches!(text, "true" | "false"))
    {
        Value::Literal(text.into())
    } else {
        Value::Text(text.into())
    }
}

pub fn text(node: &Node, key: &str) -> Result<String> {
    Ok(node.required(key)?.text()?.into())
}
fn optional_text(node: &Node, key: &str) -> Result<Option<String>> {
    node.get(key)
        .map(|n| n.text().map(str::to_owned))
        .transpose()
}
pub fn bool_option(node: &Node, key: &str) -> Result<bool> {
    let Some(n) = node.get(key) else {
        return Ok(false);
    };
    if n.style != Style::Plain {
        return Err(Error::new(
            "E-SCHEMA-OPTION",
            format!("{key} must be unquoted boolean"),
        ));
    }
    match n.text()? {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(Error::new(
            "E-SCHEMA-OPTION",
            format!("{key} must be boolean"),
        )),
    }
}
pub fn reject_unknown(n: &Node, known: &[&str]) -> Result<()> {
    if let Some(m) = n
        .members()?
        .iter()
        .find(|m| !known.contains(&m.name.as_str()))
    {
        return Err(Error::new(
            "E-SCHEMA-MEMBER",
            format!("unknown member {}", m.name),
        ));
    }
    Ok(())
}
pub fn lower_identifier(s: &str) -> bool {
    s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && s.bytes().all(|b| b.is_ascii_alphanumeric())
}
pub fn upper_identifier(s: &str) -> bool {
    s.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub fn csharp_identifier(s: &str) -> bool {
    s.as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !crate::project::csharp_keyword(s)
}
pub fn kebab(s: &str) -> bool {
    let mut parts = s.split('-');
    let first = parts.next().unwrap_or("");
    first.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && first
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && parts.all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}
pub fn public_name(s: &str) -> String {
    let mut bytes = s.as_bytes().to_vec();
    if let Some(b) = bytes.first_mut() {
        b.make_ascii_uppercase();
    }
    String::from_utf8(bytes).unwrap()
}

pub fn derive_field_type_patch(
    doc: &Document,
    field: &Node,
    type_name: &str,
    patches: &mut Vec<crate::source::Patch>,
) -> Result<()> {
    if !primitive(type_name) && !upper_identifier(type_name) {
        return Err(Error::new("E-TYPE-NAME", type_name));
    }
    crate::source::derive_patch(
        doc,
        field.required("type")?,
        &Value::Text(type_name.into()),
        patches,
    )
}
pub fn key_capable(f: &Field, types: &Types) -> bool {
    !f.nullable
        && !f.array
        && (integral(&f.type_name)
            || f.type_name == "string"
            || matches!(
                types.get(&f.type_name),
                Some(Type::ValueObject { .. } | Type::Enum { flags: false, .. })
            ))
}

pub fn fields(node: &Node) -> Result<Vec<Field>> {
    let mut names = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut fields = Vec::new();
    for item in node.items()? {
        let n = &item.value;
        reject_unknown(n, &["key", "name", "type", "nullable", "array"])?;
        let key_node = n.required("key")?;
        if key_node.style != Style::Plain || !integer_grammar(key_node.text()?) {
            return Err(Error::new(
                "E-SCHEMA-KEY",
                "unquoted non-negative integer key required",
            ));
        }
        let key = key_node.text()?.parse::<i128>().map_err(|_| {
            Error::new(
                "E-SCHEMA-KEY",
                "key exceeds native MessagePack KeyAttribute range",
            )
        })?;
        if !(0..=i32::MAX as i128).contains(&key) {
            return Err(Error::new(
                "E-SCHEMA-KEY",
                "key exceeds native MessagePack KeyAttribute range",
            ));
        }
        let f = Field {
            key: key as u32,
            name: text(n, "name")?,
            type_name: text(n, "type")?,
            nullable: bool_option(n, "nullable")?,
            array: bool_option(n, "array")?,
        };
        if !lower_identifier(&f.name)
            || crate::project::csharp_keyword(&f.name)
            || !names.insert(f.name.clone())
            || !keys.insert(f.key)
            || (f.nullable && f.array)
            || f.type_name.contains(['?', '[', ']'])
        {
            return Err(Error::new(
                "E-SCHEMA-FIELD",
                "invalid name/key/modifier or duplicate field",
            ));
        }
        fields.push(f);
    }
    if fields.is_empty() {
        return Err(Error::new("E-SCHEMA-FIELD", "at least one field required"));
    }
    Ok(fields)
}

fn string_sequence(node: &Node) -> Result<Vec<String>> {
    node.items()?
        .iter()
        .map(|i| i.value.text().map(str::to_owned))
        .collect()
}

pub fn parse_table(doc: &Document, source: &str) -> Result<Table> {
    let n = &doc.root;
    reject_unknown(
        n,
        &[
            "kind",
            "table",
            "csharpName",
            "fields",
            "primaryKey",
            "secondaryKeys",
            "references",
            "records",
        ],
    )?;
    let name = text(n, "table")?;
    if !kebab(&name) {
        return Err(Error::new("E-TABLE-NAME", name));
    }
    let csharp_name = optional_text(n, "csharpName")?
        .unwrap_or_else(|| name.split('-').map(public_name).collect());
    if !csharp_identifier(&csharp_name) {
        return Err(Error::new("E-CSHARP-NAME", csharp_name));
    }
    let fields = fields(n.required("fields")?)?;
    let parse_key = |n: &Node, secondary: bool| -> Result<Key> {
        reject_unknown(
            n,
            if secondary {
                &["fields", "nonUnique"]
            } else {
                &["fields"]
            },
        )?;
        let k = Key {
            fields: string_sequence(n.required("fields")?)?,
            non_unique: bool_option(n, "nonUnique")?,
        };
        if k.fields.is_empty()
            || k.fields.iter().collect::<BTreeSet<_>>().len() != k.fields.len()
            || k.fields
                .iter()
                .any(|n| !fields.iter().any(|f| &f.name == n))
        {
            return Err(Error::new(
                "E-KEY-SHAPE",
                "non-empty unique resolved key components required",
            ));
        }
        Ok(k)
    };
    let primary = parse_key(n.required("primaryKey")?, false)?;
    let secondary = n
        .get("secondaryKeys")
        .map(|n| {
            n.items()?
                .iter()
                .map(|i| parse_key(&i.value, true))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut key_names = BTreeSet::new();
    key_names.insert(primary.fields.clone());
    let mut query_names = BTreeSet::new();
    query_names.insert(query_name(&primary.fields));
    for k in &secondary {
        if !key_names.insert(k.fields.clone()) || !query_names.insert(query_name(&k.fields)) {
            return Err(Error::new(
                "E-KEY-COLLISION",
                "duplicate key or generated query",
            ));
        }
    }
    let references = n
        .get("references")
        .map(|n| {
            n.items()?
                .iter()
                .map(|item| {
                    let n = &item.value;
                    reject_unknown(n, &["name", "fields", "target", "csharpName"])?;
                    let name = text(n, "name")?;
                    let csharp_name = optional_text(n, "csharpName")?
                        .unwrap_or_else(|| format!("Get{}", public_name(&name)));
                    let target = n.required("target")?;
                    reject_unknown(target, &["table", "fields"])?;
                    if !lower_identifier(&name) || !csharp_identifier(&csharp_name) {
                        return Err(Error::new("E-REFERENCE-NAME", name));
                    }
                    Ok(Reference {
                        name,
                        fields: string_sequence(n.required("fields")?)?,
                        target_table: text(target, "table")?,
                        target_fields: string_sequence(target.required("fields")?)?,
                        csharp_name,
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut ref_names = BTreeSet::new();
    for r in &references {
        if !ref_names.insert(r.name.clone()) {
            return Err(Error::new("E-REFERENCE-NAME", "duplicate Reference"));
        }
    }
    Ok(Table {
        name,
        csharp_name,
        source: source.into(),
        fields,
        primary,
        secondary,
        references,
    })
}
pub fn query_name(fields: &[String]) -> String {
    fields
        .iter()
        .map(|s| public_name(s))
        .collect::<Vec<_>>()
        .join("And")
}

pub fn parse_type(doc: &Document) -> Result<(String, Type)> {
    let n = &doc.root;
    reject_unknown(
        n,
        &["kind", "name", "valueObject", "custom", "enum", "flags"],
    )?;
    let name = text(n, "name")?;
    if !upper_identifier(&name) {
        return Err(Error::new("E-TYPE-NAME", name));
    }
    let categories: Vec<_> = ["valueObject", "custom", "enum", "flags"]
        .into_iter()
        .filter(|c| n.get(c).is_some())
        .collect();
    if categories.len() != 1 {
        return Err(Error::new(
            "E-TYPE-CATEGORY",
            "exactly one category required",
        ));
    }
    let kind = categories[0];
    let body = n.required(kind)?;
    let t = match kind {
        "valueObject" => {
            reject_unknown(body, &["underlying", "conversions"])?;
            let underlying = text(body, "underlying")?;
            if !integral(&underlying) && underlying != "string" {
                return Err(Error::new("E-VO-UNDERLYING", underlying));
            }
            let (from_implicit, to_implicit) = if let Some(c) = body.get("conversions") {
                reject_unknown(c, &["fromUnderlyingImplicit", "toUnderlyingImplicit"])?;
                (
                    bool_option(c, "fromUnderlyingImplicit")?,
                    bool_option(c, "toUnderlyingImplicit")?,
                )
            } else {
                (false, false)
            };
            Type::ValueObject {
                underlying,
                from_implicit,
                to_implicit,
            }
        }
        "custom" => {
            reject_unknown(body, &["fields"])?;
            Type::Custom {
                fields: fields(body.required("fields")?)?,
            }
        }
        _ => {
            reject_unknown(body, &["underlying", "members"])?;
            let underlying = text(body, "underlying")?;
            if !integral(&underlying) {
                return Err(Error::new("E-ENUM-UNDERLYING", underlying));
            }
            let flags = kind == "flags";
            let mut names = BTreeSet::new();
            let mut numbers = BTreeSet::new();
            let mut members = Vec::new();
            for item in body.required("members")?.items()? {
                let n = &item.value;
                reject_unknown(n, &["name", "value"])?;
                let member = text(n, "name")?;
                let value = n.required("value")?;
                if value.style != Style::Plain {
                    return Err(Error::new(
                        "E-ENUM-VALUE",
                        "unquoted integer declaration required",
                    ));
                }
                let value = integer_value(value.text()?, &underlying)
                    .map_err(|e| Error::new("E-ENUM-VALUE", e.message))?;
                if !upper_identifier(&member)
                    || member == name
                    || !names.insert(member.clone())
                    || !numbers.insert(value)
                {
                    return Err(Error::new("E-ENUM-MEMBER", "invalid or duplicate member"));
                }
                if flags {
                    let mask = if matches!(underlying.as_str(), "int" | "uint") {
                        value as u32 as u64
                    } else {
                        value as u64
                    };
                    if (mask == 0 && member != "None")
                        || (mask != 0 && (member == "None" || mask.count_ones() != 1))
                    {
                        return Err(Error::new(
                            "E-FLAGS-MEMBER",
                            "None=0 and atomic bit members required",
                        ));
                    }
                }
                members.push((member, value.to_string()));
            }
            if members.is_empty() || (flags && !names.contains("None")) {
                return Err(Error::new("E-ENUM-MEMBER", "required members missing"));
            }
            Type::Enum {
                underlying,
                flags,
                members,
            }
        }
    };
    Ok((name, t))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyPart {
    Number(i128),
    Ordinal(Vec<u16>),
}

pub fn key_part(value: &Typed, f: &Field, types: &Types) -> Result<KeyPart> {
    match value {
        Typed::Integer(n) => {
            Ok(KeyPart::Number(n.parse().map_err(|_| {
                Error::new("E-KEY-VALUE", "integer key malformed")
            })?))
        }
        Typed::Text(s) => Ok(KeyPart::Ordinal(s.encode_utf16().collect())),
        Typed::Enum(n) => {
            let Some(Type::Enum { members, .. }) = types.get(&f.type_name) else {
                return Err(Error::new("E-KEY-VALUE", "enum type missing"));
            };
            let value = &members
                .iter()
                .find(|(s, _)| s == n)
                .ok_or_else(|| Error::new("E-KEY-VALUE", "enum member missing"))?
                .1;
            Ok(KeyPart::Number(value.parse().unwrap()))
        }
        _ => Err(Error::new("E-KEY-VALUE", "key-compatible scalar required")),
    }
}
