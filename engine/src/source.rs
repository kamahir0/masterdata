//! Source-preserving syntax and exact occurrence addressing.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, ops::Range, sync::Arc};
use tree_sitter::{Node as SyntaxNode, Parser};
use yaml_rust2::parser::{Event, EventReceiver};

mod mapping;
mod structure;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Value {
    Text(String),
    Literal(String),
    Null,
    Sequence(Vec<Value>),
    Mapping(Vec<(String, Value)>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Plain,
    Single,
    Double,
    Literal,
    Folded,
    Block,
    Flow,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub name: String,
    pub key: Range<usize>,
    pub span: Range<usize>,
    pub value: Arc<Node>,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub span: Range<usize>,
    pub value: Arc<Node>,
}

#[derive(Clone, Debug)]
pub enum Raw {
    Scalar(String),
    Null,
    Sequence(Vec<Item>),
    Mapping(Vec<Member>),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub span: Range<usize>,
    pub style: Style,
    pub raw: Raw,
    pub safe: bool,
}

impl Node {
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.members()
            .ok()?
            .iter()
            .find(|m| m.name == key)
            .map(|m| m.value.as_ref())
    }
    pub fn text(&self) -> Result<&str> {
        match &self.raw {
            Raw::Scalar(s) => Ok(s),
            _ => Err(Error::new("E-VALUE-SHAPE", "scalar text required")),
        }
    }
    pub fn members(&self) -> Result<&[Member]> {
        match &self.raw {
            Raw::Mapping(m) => Ok(m),
            _ => Err(Error::new("E-VALUE-SHAPE", "mapping required")),
        }
    }
    pub fn items(&self) -> Result<&[Item]> {
        match &self.raw {
            Raw::Sequence(v) => Ok(v),
            _ => Err(Error::new("E-VALUE-SHAPE", "sequence required")),
        }
    }
    pub fn required(&self, key: &str) -> Result<&Node> {
        self.get(key)
            .ok_or_else(|| Error::new("E-MEMBER-MISSING", format!("missing {key}")))
    }
    pub fn value(&self) -> Value {
        match &self.raw {
            Raw::Null => Value::Null,
            Raw::Scalar(s) => Value::Text(s.clone()),
            Raw::Sequence(s) => Value::Sequence(s.iter().map(|i| i.value.value()).collect()),
            Raw::Mapping(m) => Value::Mapping(
                m.iter()
                    .map(|m| (m.name.clone(), m.value.value()))
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub bytes: Arc<str>,
    pub identity: String,
    pub root: Arc<Node>,
    pub subset_issues: Vec<(Range<usize>, String)>,
    syntax: tree_sitter::Tree,
    line_offsets: Arc<Vec<usize>>,
    pub(crate) edits: Vec<Vec<(Range<usize>, usize)>>,
}

impl Document {
    pub fn parse(bytes: impl Into<Arc<str>>) -> Result<Self> {
        Self::parse_with_tree(bytes.into(), None)
    }
    fn parse_with_tree(bytes: Arc<str>, old: Option<&tree_sitter::Tree>) -> Result<Self> {
        let _stage = crate::instrument::span("parseIndex");
        crate::instrument::count(crate::instrument::Kind::LocalParse);
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_yaml::LANGUAGE.into())
            .map_err(|e| Error::new("E-YAML-PARSER", e.to_string()))?;
        let tree = parser
            .parse(bytes.as_bytes(), old)
            .ok_or_else(|| Error::new("E-YAML-PARSE", "parse cancelled"))?;
        if tree.root_node().has_error() {
            let mut failed = tree.root_node();
            while !failed.is_error() && !failed.is_missing() {
                let mut c = failed.walk();
                let Some(child) = failed
                    .children(&mut c)
                    .find(|n| n.has_error() || n.is_missing())
                else {
                    break;
                };
                failed = child;
            }
            return Err(Error::new(
                "E-YAML-PARSE",
                format!(
                    "invalid YAML syntax at {}:{} ({})",
                    failed.start_position().row + 1,
                    failed.start_position().column + 1,
                    failed.kind()
                ),
            ));
        }
        let mut issues = Vec::new();
        let mut cursor = tree.root_node().walk();
        let docs: Vec<_> = tree
            .root_node()
            .named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .collect();
        if docs.len() != 1 {
            return Err(Error::new("E-YAML-PARSE", "one document required"));
        }
        let root = read_node(docs[0], &bytes, &mut issues)?;
        root.members()?;
        drop(cursor);
        let mut line_offsets = vec![0];
        line_offsets.extend(
            bytes
                .bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i + 1),
        );
        Ok(Self {
            identity: content_identity(bytes.as_bytes()),
            bytes,
            root: Arc::new(root),
            subset_issues: issues,
            syntax: tree,
            line_offsets: Arc::new(line_offsets),
            edits: vec![],
        })
    }

    pub fn records(&self) -> Result<&[Item]> {
        self.root.required("records")?.items()
    }

    pub fn edit_occurrence(
        &self,
        occurrence: usize,
        path: &[String],
        value: &Value,
    ) -> Result<Self> {
        let record = self
            .records()?
            .get(
                occurrence
                    .checked_sub(1)
                    .ok_or_else(|| Error::new("E-LOCATOR", "occurrence is 1-based"))?,
            )
            .ok_or_else(|| Error::new("E-LOCATOR", "occurrence missing"))?;
        let mut node = record.value.as_ref();
        for component in path {
            node = if let Raw::Sequence(s) = &node.raw {
                let i: usize = component
                    .parse()
                    .map_err(|_| Error::new("E-LOCATOR", "array occurrence required"))?;
                s.get(i)
                    .ok_or_else(|| Error::new("E-LOCATOR", "array occurrence missing"))?
                    .value
                    .as_ref()
            } else {
                node.required(component)?
            };
        }
        let mut patches = Vec::new();
        derive_patch(self, node, value, &mut patches)?;
        let candidate = self.patched(patches)?;
        let mut actual = candidate.records()?[occurrence - 1].value.as_ref();
        for component in path {
            actual = if let Raw::Sequence(items) = &actual.raw {
                items[component
                    .parse::<usize>()
                    .map_err(|_| Error::new("E-LOCATOR", "array index required"))?]
                .value
                .as_ref()
            } else {
                actual.required(component)?
            };
        }
        if !matches_value(actual, value) {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "candidate does not represent requested value",
            ));
        }
        Ok(candidate)
    }

    pub fn occurrence_value(&self, occurrence: usize, path: &[String]) -> Result<&Node> {
        let index = occurrence
            .checked_sub(1)
            .ok_or_else(|| Error::new("E-LOCATOR", "occurrence is 1-based"))?;
        let mut node = self
            .records()?
            .get(index)
            .ok_or_else(|| Error::new("E-LOCATOR", "occurrence missing"))?
            .value
            .as_ref();
        for component in path {
            node = if let Raw::Sequence(items) = &node.raw {
                let index: usize = component
                    .parse()
                    .map_err(|_| Error::new("E-LOCATOR", "array occurrence required"))?;
                items
                    .get(index)
                    .ok_or_else(|| Error::new("E-LOCATOR", "array occurrence missing"))?
                    .value
                    .as_ref()
            } else {
                node.required(component)?
            };
        }
        Ok(node)
    }

    pub fn patched(&self, mut patches: Vec<Patch>) -> Result<Self> {
        if patches.is_empty() {
            return Ok(self.clone());
        }
        patches.sort_by_key(|p| p.span.start);
        let mut last = 0;
        for patch in &patches {
            if patch.span.start < last
                || patch.span.end > self.bytes.len()
                || !self.bytes.is_char_boundary(patch.span.start)
                || !self.bytes.is_char_boundary(patch.span.end)
            {
                return Err(Error::new(
                    "E-SOURCE-UNSAFE",
                    "overlapping or invalid source range",
                ));
            }
            last = patch.span.end;
        }
        // Construct the candidate once. Replacing every cell in a large paste in
        // reverse order would repeatedly copy the tail of the same source file.
        let capacity = patches.iter().fold(self.bytes.len(), |n, p| {
            n - (p.span.end - p.span.start) + p.text.len()
        });
        let mut output = String::with_capacity(capacity);
        let mut cursor = 0;
        for patch in &patches {
            output.push_str(&self.bytes[cursor..patch.span.start]);
            output.push_str(&patch.text);
            cursor = patch.span.end;
        }
        output.push_str(&self.bytes[cursor..]);
        let mut syntax = self.syntax.clone();
        let offsets = patches
            .iter()
            .map(|p| (p.span.clone(), p.text.len()))
            .collect();
        for p in patches.into_iter().rev() {
            let start = self.point(p.span.start);
            let newlines = p.text.bytes().filter(|b| *b == b'\n').count();
            let new_end = tree_sitter::Point {
                row: start.row + newlines,
                column: if newlines == 0 {
                    start.column + p.text.len()
                } else {
                    p.text.len() - p.text.rfind('\n').unwrap() - 1
                },
            };
            syntax.edit(&tree_sitter::InputEdit {
                start_byte: p.span.start,
                old_end_byte: p.span.end,
                new_end_byte: p.span.start + p.text.len(),
                start_position: start,
                old_end_position: self.point(p.span.end),
                new_end_position: new_end,
            });
        }
        let mut candidate = Self::parse_with_tree(Arc::<str>::from(output), Some(&syntax))?;
        candidate.edits = self.edits.clone();
        candidate.edits.push(offsets);
        Ok(candidate)
    }

    pub fn newline(&self) -> &str {
        if self.bytes.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    }
    pub fn column(&self, at: usize) -> usize {
        self.point(at).column
    }
    pub fn line_start(&self, at: usize) -> usize {
        self.line_offsets[self.point(at).row]
    }
    pub fn point(&self, at: usize) -> tree_sitter::Point {
        let row = self.line_offsets.partition_point(|p| *p <= at) - 1;
        tree_sitter::Point {
            row,
            column: at - self.line_offsets[row],
        }
    }
    pub fn line_end(&self, at: usize) -> usize {
        self.bytes[at..]
            .find('\n')
            .map_or(self.bytes.len(), |x| at + x + 1)
    }
}

pub fn content_identity(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug)]
pub struct Patch {
    pub span: Range<usize>,
    pub text: String,
}

fn children(n: SyntaxNode<'_>) -> Vec<SyntaxNode<'_>> {
    let mut c = n.walk();
    n.named_children(&mut c)
        .filter(|n| n.kind() != "comment")
        .collect()
}

fn read_node(
    n: SyntaxNode<'_>,
    bytes: &str,
    issues: &mut Vec<(Range<usize>, String)>,
) -> Result<Node> {
    let span = n.byte_range();
    let spelling = &bytes[span.clone()];
    let mut safe = true;
    let (style, raw) = match n.kind() {
        "document" | "block_node" | "flow_node" | "block_sequence_item" => {
            for child in (0..n.child_count()).filter_map(|i| n.child(i)) {
                if matches!(
                    child.kind(),
                    "---" | "..." | "yaml_directive" | "tag_directive" | "reserved_directive"
                ) {
                    return Err(Error::new(
                        "E-YAML-SUBSET",
                        "document markers and directives are unsupported",
                    ));
                }
            }
            let cs = children(n);
            if cs.len() != 1 {
                return Err(Error::new(
                    "E-YAML-SUBSET",
                    "tags / anchors / ambiguous node unsupported",
                ));
            }
            return read_node(cs[0], bytes, issues);
        }
        "block_mapping" | "flow_mapping" => {
            let mut members = Vec::new();
            let mut names = HashSet::new();
            for pair in children(n) {
                if !matches!(pair.kind(), "block_mapping_pair" | "flow_pair") {
                    return Err(Error::new(
                        "E-YAML-SUBSET",
                        "explicit mapping pair required",
                    ));
                }
                let key = pair
                    .child_by_field_name("key")
                    .ok_or_else(|| Error::new("E-YAML-SUBSET", "missing mapping key"))?;
                let value = pair
                    .child_by_field_name("value")
                    .ok_or_else(|| Error::new("E-YAML-SUBSET", "implicit null unsupported"))?;
                let kn = read_node(key, bytes, issues)?;
                let name = kn.text()?.to_owned();
                if kn.style == Style::Plain
                    && (matches!(name.as_str(), "true" | "false" | "null" | "~")
                        || name.parse::<f64>().is_ok())
                {
                    return Err(Error::new("E-YAML-KEY", "mapping key must be string"));
                }
                if name == "<<" || !names.insert(name.clone()) {
                    return Err(Error::new("E-YAML-KEY", "duplicate / merge mapping key"));
                }
                members.push(Member {
                    name,
                    key: kn.span,
                    span: pair.byte_range(),
                    value: Arc::new(read_node(value, bytes, issues)?),
                });
            }
            (
                if n.kind() == "flow_mapping" {
                    Style::Flow
                } else {
                    Style::Block
                },
                Raw::Mapping(members),
            )
        }
        "block_sequence" | "flow_sequence" => {
            let mut items = Vec::new();
            for item in children(n) {
                items.push(Item {
                    span: item.byte_range(),
                    value: Arc::new(read_node(item, bytes, issues)?),
                });
            }
            (
                if n.kind() == "flow_sequence" {
                    Style::Flow
                } else {
                    Style::Block
                },
                Raw::Sequence(items),
            )
        }
        "plain_scalar" => {
            if spelling == "~" {
                issues.push((span.clone(), "unsupported null shorthand".into()));
                safe = false;
            }
            let text = if spelling.contains('\n') {
                decode_scalar(spelling)?
            } else {
                spelling.to_owned()
            };
            (
                Style::Plain,
                if spelling == "null" {
                    Raw::Null
                } else {
                    Raw::Scalar(text)
                },
            )
        }
        "single_quote_scalar" | "double_quote_scalar" => (
            if n.kind() == "single_quote_scalar" {
                Style::Single
            } else {
                Style::Double
            },
            Raw::Scalar(decode_scalar(spelling)?),
        ),
        "block_scalar" => {
            let indicator = spelling
                .lines()
                .next()
                .unwrap_or("")
                .split('#')
                .next()
                .unwrap_or("")
                .trim();
            if indicator != "|" {
                issues.push((span.clone(), "unsupported block scalar indicator".into()));
                safe = false;
            }
            let style = if indicator.starts_with('>') {
                Style::Folded
            } else {
                Style::Literal
            };
            (style, Raw::Scalar(decode_scalar(&format!("{spelling}\n"))?))
        }
        _ => {
            return Err(Error::new(
                "E-YAML-SUBSET",
                format!("unsupported {}", n.kind()),
            ));
        }
    };
    Ok(Node {
        span,
        style,
        raw,
        safe,
    })
}

fn decode_scalar(spelling: &str) -> Result<String> {
    #[derive(Default)]
    struct Sink {
        scalars: Vec<String>,
    }
    impl EventReceiver for Sink {
        fn on_event(&mut self, e: Event) {
            if let Event::Scalar(s, _, _, _) = e {
                self.scalars.push(s);
            }
        }
    }
    let mut sink = Sink::default();
    yaml_rust2::parser::Parser::new_from_str(spelling)
        .load(&mut sink, false)
        .map_err(|e| Error::new("E-YAML-PARSE", e.to_string()))?;
    if sink.scalars.len() != 1 {
        return Err(Error::new("E-YAML-SCALAR", "one scalar required"));
    }
    Ok(sink.scalars.remove(0))
}

pub fn derive_patch(
    doc: &Document,
    node: &Node,
    desired: &Value,
    patches: &mut Vec<Patch>,
) -> Result<()> {
    if !node.safe {
        return Err(Error::new(
            "E-SOURCE-UNSAFE",
            "target syntax cannot be safely edited",
        ));
    }
    match (&node.raw, desired) {
        (Raw::Null, Value::Null) => return Ok(()),
        (Raw::Scalar(old), Value::Text(new) | Value::Literal(new)) if old == new => return Ok(()),
        (Raw::Mapping(old), Value::Mapping(new)) => {
            let names: HashSet<_> = new.iter().map(|(n, _)| n).collect();
            if names.len() != new.len() {
                return Err(Error::new("E-SOURCE-UNSAFE", "duplicate requested member"));
            }
            if old.len() != new.len() || old.iter().any(|m| !names.contains(&m.name)) {
                return Err(Error::new(
                    "E-SOURCE-UNSAFE",
                    "mapping membership change requires a localized structural operation",
                ));
            }
            for member in old {
                derive_patch(
                    doc,
                    &member.value,
                    &new.iter().find(|(n, _)| n == &member.name).unwrap().1,
                    patches,
                )?;
            }
            return Ok(());
        }
        (Raw::Sequence(old), Value::Sequence(new)) if old.len() == new.len() => {
            for (a, b) in old.iter().zip(new) {
                derive_patch(doc, &a.value, b, patches)?;
            }
            return Ok(());
        }
        (Raw::Mapping(_) | Raw::Sequence(_), Value::Mapping(_) | Value::Sequence(_)) => {
            return Err(Error::new(
                "E-SOURCE-UNSAFE",
                "structural edit requires exact item ranges",
            ));
        }
        _ => {}
    }
    let line = &doc.bytes[doc.line_start(node.span.start)..node.span.start];
    let indent = if node.style == Style::Literal {
        doc.bytes[node.span.clone()]
            .lines()
            .skip(1)
            .find(|s| !s.trim().is_empty())
            .map(|s| {
                s.chars()
                    .take_while(|c| *c == ' ')
                    .count()
                    .saturating_sub(2)
            })
            .unwrap_or_else(|| line.chars().take_while(|c| *c == ' ').count())
    } else {
        doc.column(node.span.start)
    };
    let nl = doc.newline();
    let text = match desired {
        Value::Text(s) => render_text(s, node.style, indent, nl)?,
        Value::Literal(s) => render_literal(s)?,
        Value::Null => "null".into(),
        Value::Sequence(s) if s.is_empty() => "[]".into(),
        Value::Mapping(_) | Value::Sequence(_) => {
            return structure::derive_patch_materialization(doc, node, desired, patches);
        }
    };
    let span = node.span.clone();
    patches.push(Patch { span, text });
    Ok(())
}

fn plain_text_safe(s: &str) -> bool {
    !s.is_empty()
        && !matches!(s, "null" | "~")
        && s.trim() == s
        && !s.contains(['\n', '\r', '\t', '{', '}', '[', ']', ','])
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.starts_with([
            '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`', '-', '?', ':',
        ])
}

fn render_text(s: &str, style: Style, column: usize, nl: &str) -> Result<String> {
    Ok(match style {
        Style::Single if !s.contains(['\n', '\r']) => format!("'{}'", s.replace('\'', "''")),
        Style::Double => serde_json::to_string(s).unwrap(),
        Style::Literal if s.ends_with('\n') && !s.ends_with("\n\n") && !s.contains('\r') => {
            let padding = " ".repeat(column + 2);
            format!(
                "|{nl}{}",
                s.trim_end_matches('\n')
                    .split('\n')
                    .map(|l| format!("{padding}{l}"))
                    .collect::<Vec<_>>()
                    .join(nl)
            )
        }
        _ if plain_text_safe(s) => s.into(),
        _ => serde_json::to_string(s).unwrap(),
    })
}

pub fn render_block(value: &Value, indentation: usize, nl: &str) -> Result<String> {
    let pad = " ".repeat(indentation);
    match value {
        Value::Mapping(m) => {
            if m.is_empty() {
                return Ok(format!("{pad}{{}}"));
            }
            m.iter()
                .map(|(name, v)| {
                    let name = if plain_text_safe(name) {
                        name.to_owned()
                    } else {
                        serde_json::to_string(name).unwrap()
                    };
                    if matches!(v, Value::Mapping(_) | Value::Sequence(_))
                        && !matches!(v, Value::Sequence(s) if s.is_empty())
                    {
                        Ok(format!(
                            "{pad}{name}:{nl}{}",
                            render_block(v, indentation + 2, nl)?
                        ))
                    } else {
                        Ok(format!("{pad}{name}:{}", inline_render(v, nl)?))
                    }
                })
                .collect::<Result<Vec<_>>>()
                .map(|l| l.join(nl))
        }
        Value::Sequence(s) => s
            .iter()
            .map(|v| {
                let nested = render_block(v, indentation + 2, nl)?;
                Ok(format!(
                    "{pad}- {}",
                    nested.strip_prefix(&format!("{pad}  ")).unwrap_or(&nested)
                ))
            })
            .collect::<Result<Vec<_>>>()
            .map(|l| l.join(nl)),
        _ => Ok(format!("{pad}{}", inline_render(value, nl)?.trim_start())),
    }
}

fn inline_render(value: &Value, nl: &str) -> Result<String> {
    Ok(format!(
        " {}",
        match value {
            Value::Null => "null".into(),
            Value::Text(s) => render_text(s, Style::Plain, 0, nl)?,
            Value::Literal(s) => render_literal(s)?,
            Value::Sequence(s) if s.is_empty() => "[]".into(),
            _ =>
                return Err(Error::new(
                    "E-SOURCE-UNSAFE",
                    "compound value must use block syntax"
                )),
        }
    ))
}

fn render_literal(s: &str) -> Result<String> {
    if s.contains(['\n', '\r']) {
        return Err(Error::new(
            "E-SOURCE-UNSAFE",
            "literal must stay on one line",
        ));
    }
    if matches!(s, "true" | "false")
        || crate::semantic::integer_grammar(s)
        || crate::semantic::float_grammar(s)
    {
        return Ok(s.into());
    }
    let parsed = Document::parse(format!("value: {s}\n"))?;
    let value = parsed.root.required("value")?;
    if parsed.root.members()?.len() != 1
        || value.text().ok() != Some(s)
        || !value.safe
        || value.style != Style::Plain
    {
        return Err(Error::new(
            "E-SOURCE-UNSAFE",
            "literal must be a plain scalar",
        ));
    }
    Ok(s.to_owned())
}

pub fn matches_value(node: &Node, value: &Value) -> bool {
    match (&node.raw, value) {
        (Raw::Null, Value::Null) => true,
        (Raw::Scalar(a), Value::Text(b) | Value::Literal(b)) => a == b,
        (Raw::Sequence(a), Value::Sequence(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| matches_value(&a.value, b))
        }
        (Raw::Mapping(a), Value::Mapping(b)) => {
            a.len() == b.len()
                && a.iter().all(|m| {
                    b.iter()
                        .find(|(n, _)| n == &m.name)
                        .is_some_and(|(_, v)| matches_value(&m.value, v))
                })
        }
        _ => false,
    }
}
