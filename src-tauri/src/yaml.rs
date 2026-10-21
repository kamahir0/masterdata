use crate::model::{format_float, uid, Data, Source};
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Plain,
    Single,
    Double,
}

#[derive(Debug, Clone)]
pub struct AstEntry {
    pub key: String,
    pub key_style: QuoteStyle,
    pub comment_before: Vec<String>,
    pub comment_inline: Option<String>,
    pub value: AstNode,
}

#[derive(Debug, Clone)]
pub enum AstKind {
    Scalar {
        value: Data,
        style: QuoteStyle,
        raw: Option<String>,
    },
    Map(Vec<AstEntry>),
    Seq(Vec<AstNode>),
}

#[derive(Debug, Clone)]
pub struct AstNode {
    pub comment_before: Vec<String>,
    pub comment_inline: Option<String>,
    pub kind: AstKind,
}

#[derive(Debug, Clone)]
pub struct YamlDocument {
    pub leading_comments: Vec<String>,
    pub root: AstNode,
    pub trailing_comments: Vec<String>,
}

impl AstNode {
    pub fn from_data(data: &Data) -> Self {
        let kind = match data {
            Data::Null | Data::Undefined => AstKind::Scalar {
                value: Data::Null,
                style: QuoteStyle::Plain,
                raw: None,
            },
            Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Str(_) => AstKind::Scalar {
                value: data.clone(),
                style: QuoteStyle::Plain,
                raw: None,
            },
            Data::Array(items) => {
                AstKind::Seq(items.iter().map(AstNode::from_data).collect())
            }
            Data::Map(map) => {
                let mut entries = Vec::new();
                for (k, v) in map {
                    if matches!(v, Data::Undefined) {
                        continue;
                    }
                    entries.push(AstEntry {
                        key: k.clone(),
                        key_style: QuoteStyle::Plain,
                        comment_before: Vec::new(),
                        comment_inline: None,
                        value: AstNode::from_data(v),
                    });
                }
                AstKind::Map(entries)
            }
        };
        AstNode {
            comment_before: Vec::new(),
            comment_inline: None,
            kind,
        }
    }

    pub fn to_data(&self) -> Data {
        match &self.kind {
            AstKind::Scalar { value, .. } => value.clone(),
            AstKind::Seq(items) => Data::Array(items.iter().map(|i| i.to_data()).collect()),
            AstKind::Map(entries) => {
                let mut map = IndexMap::new();
                for e in entries {
                    map.insert(e.key.clone(), e.value.to_data());
                }
                Data::Map(map)
            }
        }
    }
}

pub fn parse_source(path: &str, input: &str, id: Option<String>) -> Source {
    let source_id = id.unwrap_or_else(uid);
    match parse_yaml_document(input) {
        Ok(doc) => Source {
            id: source_id,
            path: path.to_string(),
            data: doc.root.to_data(),
            raw: None,
            format_text: Some(input.to_string()),
            syntax_error: None,
        },
        Err(err) => Source {
            id: source_id,
            path: path.to_string(),
            data: Data::Null,
            raw: Some(input.to_string()),
            format_text: None,
            syntax_error: Some(err),
        },
    }
}

pub fn parse_structured_data(input: &str) -> Result<Data, String> {
    let doc = parse_yaml_document(input)
        .map_err(|e| format!("入力データの構文が不正です: {e}"))?;
    Ok(doc.root.to_data())
}

pub fn serialize(source: &Source, saved_text: Option<&str>) -> String {
    if let Some(raw) = &source.raw {
        return raw.clone();
    }
    let original = source.format_text.as_deref().or(saved_text);
    let crlf = original.map(|s| s.contains("\r\n")).unwrap_or(false);

    let doc = match original.and_then(|orig| parse_yaml_document(orig).ok()) {
        Some(mut parsed) => {
            parsed.root = sync_node(parsed.root, &source.data);
            parsed
        }
        None => YamlDocument {
            leading_comments: Vec::new(),
            root: AstNode::from_data(&source.data),
            trailing_comments: Vec::new(),
        },
    };

    let mut out = emit_document(&doc);
    if crlf {
        out = out.replace("\r\n", "\n").replace('\n', "\r\n");
    }
    out
}

fn attach_trailing_inline_comment(node: &mut AstNode, comment: String) {
    match &mut node.kind {
        AstKind::Scalar { .. } => {
            if node.comment_inline.is_none() {
                node.comment_inline = Some(comment);
            }
        }
        AstKind::Map(entries) => {
            if let Some(last) = entries.last_mut() {
                attach_trailing_inline_comment(&mut last.value, comment);
            } else if node.comment_inline.is_none() {
                node.comment_inline = Some(comment);
            }
        }
        AstKind::Seq(items) => {
            if let Some(last) = items.last_mut() {
                attach_trailing_inline_comment(last, comment);
            } else if node.comment_inline.is_none() {
                node.comment_inline = Some(comment);
            }
        }
    }
}

fn sync_node(mut node: AstNode, next: &Data) -> AstNode {
    match (&mut node.kind, next) {
        (AstKind::Map(entries), Data::Map(next_map)) => {
            if let Some(inline) = node.comment_inline.take() {
                if let Some(last) = entries.last_mut() {
                    attach_trailing_inline_comment(&mut last.value, inline);
                } else {
                    node.comment_inline = Some(inline);
                }
            }
            let mut kept = Vec::new();
            for mut entry in entries.drain(..) {
                if let Some(next_val) = next_map.get(&entry.key) {
                    if !matches!(next_val, Data::Undefined) {
                        entry.value = sync_node(entry.value, next_val);
                        kept.push(entry);
                    }
                }
            }
            for (k, v) in next_map {
                if matches!(v, Data::Undefined) {
                    continue;
                }
                if !kept.iter().any(|e| &e.key == k) {
                    kept.push(AstEntry {
                        key: k.clone(),
                        key_style: QuoteStyle::Plain,
                        comment_before: Vec::new(),
                        comment_inline: None,
                        value: AstNode::from_data(v),
                    });
                }
            }
            node.kind = AstKind::Map(kept);
            node
        }
        (AstKind::Seq(items), Data::Array(next_arr)) => {
            let old_items: Vec<AstNode> = items.drain(..).collect();
            let mut updated = Vec::with_capacity(next_arr.len());
            for (i, val) in next_arr.iter().enumerate() {
                if let Some(prev) = old_items.get(i) {
                    updated.push(sync_node(prev.clone(), val));
                } else {
                    updated.push(AstNode::from_data(val));
                }
            }
            node.kind = AstKind::Seq(updated);
            node
        }
        (
            AstKind::Scalar {
                value,
                style,
                raw,
            },
            Data::Null | Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Str(_),
        ) => {
            if value != next {
                *value = next.clone();
                *raw = None;
                if !matches!(next, Data::Str(_)) {
                    *style = QuoteStyle::Plain;
                }
            }
            node
        }
        _ => {
            let mut replacement = AstNode::from_data(next);
            replacement.comment_before = node.comment_before;
            replacement.comment_inline = node.comment_inline;
            replacement
        }
    }
}

fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.starts_with(' ') || s.ends_with(' ') || s.contains(['\n', '\r', '\t']) {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "null" | "~" | "true" | "false" | "yes" | "no" | "on" | "off" | ".inf" | "-.inf" | ".nan"
    ) {
        return true;
    }
    let first = s.chars().next().unwrap();
    if matches!(
        first,
        '-' | '?'
            | ':'
            | ','
            | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
    ) {
        return true;
    }
    if s.contains(": ")
        || s.contains(" #")
        || s.contains("\t#")
        || s.ends_with(':')
        || s.contains(['{', '}', '[', ']', ','])
    {
        return true;
    }
    if s.parse::<i128>().is_ok() || s.parse::<f64>().is_ok() {
        return true;
    }
    if s.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return true;
    }
    false
}

fn format_key(key: &str, style: QuoteStyle) -> String {
    match style {
        QuoteStyle::Double => serde_json::to_string(key).unwrap_or_else(|_| format!("\"{key}\"")),
        QuoteStyle::Single => format!("'{}'", key.replace('\'', "''")),
        QuoteStyle::Plain => {
            if needs_quoting(key) {
                serde_json::to_string(key).unwrap_or_else(|_| format!("\"{key}\""))
            } else {
                key.to_string()
            }
        }
    }
}

fn format_scalar(value: &Data, style: QuoteStyle, raw: Option<&str>) -> String {
    if let Some(r) = raw {
        return r.to_string();
    }
    match value {
        Data::Null | Data::Undefined => "null".to_string(),
        Data::Bool(b) => b.to_string(),
        Data::Int(n) => n.to_string(),
        Data::Float(f) => format_float(*f),
        Data::Str(s) => match style {
            QuoteStyle::Double => serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\"")),
            QuoteStyle::Single => format!("'{}'", s.replace('\'', "''")),
            QuoteStyle::Plain => {
                if needs_quoting(s) {
                    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
                } else {
                    s.clone()
                }
            }
        },
        Data::Array(_) => "[]".to_string(),
        Data::Map(_) => "{}".to_string(),
    }
}

fn with_inline(line: String, comment: Option<&str>) -> String {
    match comment {
        Some(c) if !c.is_empty() => format!("{line} {c}"),
        _ => line,
    }
}

fn emit_document(doc: &YamlDocument) -> String {
    let mut lines = Vec::new();
    for c in &doc.leading_comments {
        lines.push(c.clone());
    }
    for c in &doc.root.comment_before {
        lines.push(c.clone());
    }
    emit_node(&doc.root, 0, &mut lines);
    for c in &doc.trailing_comments {
        lines.push(c.clone());
    }
    if lines.is_empty() {
        return "null\n".to_string();
    }
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn is_empty_collection(node: &AstNode) -> Option<&'static str> {
    match &node.kind {
        AstKind::Map(entries) if entries.is_empty() => Some("{}"),
        AstKind::Seq(items) if items.is_empty() => Some("[]"),
        _ => None,
    }
}

fn emit_node(node: &AstNode, indent: usize, lines: &mut Vec<String>) {
    let pad = " ".repeat(indent);
    match &node.kind {
        AstKind::Scalar { value, style, raw } => {
            let s = format_scalar(value, *style, raw.as_deref());
            lines.push(with_inline(
                format!("{pad}{s}"),
                node.comment_inline.as_deref(),
            ));
        }
        AstKind::Map(entries) => {
            if entries.is_empty() {
                lines.push(with_inline(
                    format!("{pad}{{}}"),
                    node.comment_inline.as_deref(),
                ));
                return;
            }
            for entry in entries {
                emit_map_entry(entry, indent, None, lines);
            }
        }
        AstKind::Seq(items) => {
            if items.is_empty() {
                lines.push(with_inline(
                    format!("{pad}[]"),
                    node.comment_inline.as_deref(),
                ));
                return;
            }
            for item in items {
                emit_seq_item(item, indent, lines);
            }
        }
    }
}

fn emit_map_entry(
    entry: &AstEntry,
    indent: usize,
    first_line_prefix: Option<&str>,
    lines: &mut Vec<String>,
) {
    let pad = " ".repeat(indent);
    for c in &entry.comment_before {
        lines.push(format!("{pad}{c}"));
    }
    for c in &entry.value.comment_before {
        lines.push(format!("{pad}{c}"));
    }
    let prefix = first_line_prefix
        .map(|s| s.to_string())
        .unwrap_or_else(|| pad.clone());
    let key_str = format_key(&entry.key, entry.key_style);

    if let Some(empty_tok) = is_empty_collection(&entry.value) {
        let inline = entry
            .value
            .comment_inline
            .as_deref()
            .or(entry.comment_inline.as_deref());
        lines.push(with_inline(format!("{prefix}{key_str}: {empty_tok}"), inline));
        return;
    }

    match &entry.value.kind {
        AstKind::Scalar { value, style, raw } => {
            let val_str = format_scalar(value, *style, raw.as_deref());
            let inline = entry
                .value
                .comment_inline
                .as_deref()
                .or(entry.comment_inline.as_deref());
            lines.push(with_inline(format!("{prefix}{key_str}: {val_str}"), inline));
        }
        AstKind::Map(sub_entries) => {
            lines.push(with_inline(
                format!("{prefix}{key_str}:"),
                entry
                    .comment_inline
                    .as_deref()
                    .or(entry.value.comment_inline.as_deref()),
            ));
            for sub in sub_entries {
                emit_map_entry(sub, indent + 2, None, lines);
            }
        }
        AstKind::Seq(items) => {
            lines.push(with_inline(
                format!("{prefix}{key_str}:"),
                entry
                    .comment_inline
                    .as_deref()
                    .or(entry.value.comment_inline.as_deref()),
            ));
            for item in items {
                emit_seq_item(item, indent + 2, lines);
            }
        }
    }
}

fn emit_seq_item(item: &AstNode, indent: usize, lines: &mut Vec<String>) {
    let pad = " ".repeat(indent);
    for c in &item.comment_before {
        lines.push(format!("{pad}{c}"));
    }
    if let Some(empty_tok) = is_empty_collection(item) {
        lines.push(with_inline(
            format!("{pad}- {empty_tok}"),
            item.comment_inline.as_deref(),
        ));
        return;
    }
    match &item.kind {
        AstKind::Scalar { value, style, raw } => {
            let val_str = format_scalar(value, *style, raw.as_deref());
            lines.push(with_inline(
                format!("{pad}- {val_str}"),
                item.comment_inline.as_deref(),
            ));
        }
        AstKind::Map(entries) => {
            let first = &entries[0];
            if first.comment_before.is_empty() && first.value.comment_before.is_empty() {
                let first_prefix = format!("{pad}- ");
                emit_map_entry(first, indent + 2, Some(&first_prefix), lines);
                for rest in &entries[1..] {
                    emit_map_entry(rest, indent + 2, None, lines);
                }
            } else {
                lines.push(with_inline(
                    format!("{pad}-"),
                    item.comment_inline.as_deref(),
                ));
                for e in entries {
                    emit_map_entry(e, indent + 2, None, lines);
                }
            }
        }
        AstKind::Seq(sub_items) => {
            lines.push(with_inline(
                format!("{pad}-"),
                item.comment_inline.as_deref(),
            ));
            for sub in sub_items {
                emit_seq_item(sub, indent + 2, lines);
            }
        }
    }
}

// --- Parser ---

#[derive(Debug, Clone)]
struct LineInfo {
    indent: usize,
    content: String,
    comment: Option<String>,
}

fn split_line_comment(raw: &str) -> Result<(String, Option<String>), String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    while i < chars.len() {
        let c = chars[i];
        if in_single {
            if c == '\'' {
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if c == '"' {
            in_double = true;
            i += 1;
            continue;
        }
        if c == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
            let before: String = chars[..i].iter().collect();
            let comment: String = chars[i..].iter().collect();
            return Ok((before.trim_end().to_string(), Some(comment)));
        }
        i += 1;
    }
    if in_single || in_double {
        return Err("引用符が閉じられていません。".to_string());
    }
    Ok((raw.trim_end().to_string(), None))
}

fn parse_yaml_document(input: &str) -> Result<YamlDocument, String> {
    let normalized = input.replace("\r\n", "\n");
    // Fast check if entire document is a flow collection/scalar (e.g. JSON or multiline flow)
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return Ok(YamlDocument {
            leading_comments: Vec::new(),
            root: AstNode::from_data(&Data::Null),
            trailing_comments: Vec::new(),
        });
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let mut cursor = FlowCursor::new(trimmed);
        let node = cursor.parse_flow_node()?;
        cursor.skip_ws_and_comments();
        if !cursor.is_eof() {
            return Err("YAML/JSONの末尾に不正な文字があります。".to_string());
        }
        return Ok(YamlDocument {
            leading_comments: Vec::new(),
            root: node,
            trailing_comments: Vec::new(),
        });
    }

    let raw_lines: Vec<&str> = normalized.split('\n').collect();
    let mut leading_comments = Vec::new();
    let mut parsed_lines: Vec<LineInfo> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    let mut seen_content = false;

    let mut idx = 0;
    while idx < raw_lines.len() {
        let raw_line = raw_lines[idx];
        if raw_line.contains('\t') && raw_line.trim_start_matches(' ').starts_with('\t') {
            return Err("インデントにタブ文字は使用できません。".to_string());
        }
        let indent = raw_line.chars().take_while(|c| *c == ' ').count();
        let rest = &raw_line[indent..];
        if rest.trim().is_empty() {
            idx += 1;
            continue;
        }
        if rest.starts_with('#') {
            if !seen_content {
                leading_comments.push(rest.trim_end().to_string());
            } else {
                pending_comments.push(rest.trim_end().to_string());
            }
            idx += 1;
            continue;
        }
        if !seen_content && (rest == "---" || rest.starts_with("--- ")) {
            idx += 1;
            continue;
        }
        if rest == "..." {
            break;
        }
        seen_content = true;
        for c in pending_comments.drain(..) {
            parsed_lines.push(LineInfo {
                indent,
                content: String::new(),
                comment: Some(c),
            });
        }

        // Check if this line starts an unclosed flow collection `[` or `{` across multiple lines
        let mut combined = rest.to_string();
        let mut flow_balance = flow_bracket_balance(&combined)?;
        while flow_balance > 0 {
            idx += 1;
            if idx >= raw_lines.len() {
                return Err("コレクションの括弧が閉じられていません。".to_string());
            }
            let next_raw = raw_lines[idx].trim();
            if !next_raw.starts_with('#') {
                combined.push(' ');
                combined.push_str(next_raw);
                flow_balance = flow_bracket_balance(&combined)?;
            }
        }

        let (content, comment) = split_line_comment(&combined)?;
        parsed_lines.push(LineInfo {
            indent,
            content,
            comment,
        });
        idx += 1;
    }

    let trailing_comments = pending_comments;
    if parsed_lines.is_empty() {
        return Ok(YamlDocument {
            leading_comments,
            root: AstNode::from_data(&Data::Null),
            trailing_comments,
        });
    }

    let mut pos = 0;
    let base_indent = parsed_lines
        .iter()
        .find(|l| !l.content.is_empty())
        .map(|l| l.indent)
        .unwrap_or(0);
    let root = parse_block_node(&parsed_lines, &mut pos, base_indent)?;
    let mut final_trailing = trailing_comments;
    while pos < parsed_lines.len() {
        if parsed_lines[pos].content.is_empty() {
            if let Some(c) = &parsed_lines[pos].comment {
                final_trailing.push(c.clone());
            }
            pos += 1;
        } else {
            return Err(format!(
                "YAMLのインデントまたは構文が不正です: {}",
                parsed_lines[pos].content
            ));
        }
    }

    Ok(YamlDocument {
        leading_comments,
        root,
        trailing_comments: final_trailing,
    })
}

fn flow_bracket_balance(s: &str) -> Result<i32, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut balance = 0i32;
    while i < chars.len() {
        let c = chars[i];
        if in_single {
            if c == '\'' {
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_single = true;
        } else if c == '"' {
            in_double = true;
        } else if c == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
            break;
        } else if c == '[' || c == '{' {
            balance += 1;
        } else if c == ']' || c == '}' {
            balance -= 1;
            if balance < 0 {
                return Err("閉じ括弧が対応していません。".to_string());
            }
        }
        i += 1;
    }
    if in_single || in_double {
        return Err("引用符が閉じられていません。".to_string());
    }
    Ok(balance)
}

fn collect_comments(lines: &[LineInfo], pos: &mut usize, min_indent: usize) -> Vec<String> {
    let mut comments = Vec::new();
    while *pos < lines.len() && lines[*pos].content.is_empty() {
        if lines[*pos].indent < min_indent {
            break;
        }
        if let Some(c) = &lines[*pos].comment {
            comments.push(c.clone());
        }
        *pos += 1;
    }
    comments
}

fn is_seq_line(content: &str) -> bool {
    content == "-" || content.starts_with("- ")
}

fn split_key_value(content: &str) -> Result<Option<(String, QuoteStyle, String)>, String> {
    let chars: Vec<char> = content.chars().collect();
    if chars.is_empty() || chars[0] == '[' || chars[0] == '{' {
        return Ok(None);
    }
    if chars[0] == '"' || chars[0] == '\'' {
        let mut cursor = FlowCursor::new(content);
        let (key, style) = cursor.parse_key_string()?;
        cursor.skip_spaces();
        if cursor.peek() == Some(':') {
            cursor.advance();
            if cursor.is_eof() || cursor.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                cursor.skip_spaces();
                let rest = cursor.remaining().to_string();
                return Ok(Some((key, style, rest)));
            }
        }
        return Ok(None);
    }
    for i in 0..chars.len() {
        if chars[i] == ':' && (i + 1 == chars.len() || chars[i + 1].is_whitespace()) {
            let key_raw: String = chars[..i].iter().collect();
            let key = key_raw.trim().to_string();
            if key.is_empty() {
                return Err("キー名が空です。".to_string());
            }
            let rest: String = chars[i + 1..].iter().collect();
            return Ok(Some((key, QuoteStyle::Plain, rest.trim().to_string())));
        }
    }
    Ok(None)
}

fn parse_block_node(
    lines: &[LineInfo],
    pos: &mut usize,
    expected_indent: usize,
) -> Result<AstNode, String> {
    let leading = collect_comments(lines, pos, expected_indent);
    if *pos >= lines.len() {
        let mut n = AstNode::from_data(&Data::Null);
        n.comment_before = leading;
        return Ok(n);
    }
    let line = &lines[*pos];
    if line.indent != expected_indent {
        return Err(format!(
            "インデントが不正です（期待: {}, 実際: {}）: {}",
            expected_indent, line.indent, line.content
        ));
    }

    if is_seq_line(&line.content) {
        let seq = parse_block_seq(lines, pos, expected_indent, leading)?;
        return Ok(seq);
    }

    if split_key_value(&line.content)?.is_some() {
        let map = parse_block_map(lines, pos, expected_indent, leading, None)?;
        return Ok(map);
    }

    let inline_comment = line.comment.clone();
    let content = line.content.clone();
    *pos += 1;
    let mut node = parse_inline_value(&content)?;
    node.comment_before = leading;
    node.comment_inline = inline_comment;
    Ok(node)
}

fn parse_block_seq(
    lines: &[LineInfo],
    pos: &mut usize,
    seq_indent: usize,
    mut first_item_comments: Vec<String>,
) -> Result<AstNode, String> {
    let mut items = Vec::new();
    while *pos < lines.len() {
        let mut item_comments = std::mem::take(&mut first_item_comments);
        item_comments.extend(collect_comments(lines, pos, seq_indent));
        if *pos >= lines.len() {
            break;
        }
        let line = &lines[*pos];
        if line.indent < seq_indent {
            break;
        }
        if line.indent > seq_indent {
            return Err(format!("リスト要素のインデントが不正です: {}", line.content));
        }
        if !is_seq_line(&line.content) {
            break;
        }

        let after_dash = if line.content == "-" {
            ""
        } else {
            line.content[2..].trim()
        };
        let line_comment = line.comment.clone();
        *pos += 1;

        if after_dash.is_empty() {
            let child_comments = collect_comments(lines, pos, seq_indent + 1);
            if *pos < lines.len() && lines[*pos].indent > seq_indent {
                let child_indent = lines[*pos].indent;
                let mut item_node = parse_block_node(lines, pos, child_indent)?;
                let mut combined = item_comments;
                combined.extend(child_comments);
                item_node.comment_before = combined;
                if item_node.comment_inline.is_none() {
                    item_node.comment_inline = line_comment;
                }
                items.push(item_node);
            } else {
                let mut item_node = AstNode::from_data(&Data::Null);
                item_node.comment_before = item_comments;
                item_node.comment_inline = line_comment;
                items.push(item_node);
            }
            continue;
        }

        if let Some((k, k_style, v_rest)) = split_key_value(after_dash)? {
            let first_entry =
                parse_map_entry_from_parts(lines, pos, seq_indent + 2, k, k_style, &v_rest, line_comment)?;
            let map_node =
                parse_block_map(lines, pos, seq_indent + 2, item_comments, Some(first_entry))?;
            items.push(map_node);
        } else {
            let mut item_node = parse_inline_value(after_dash)?;
            item_node.comment_before = item_comments;
            item_node.comment_inline = line_comment;
            items.push(item_node);
        }
    }

    Ok(AstNode {
        comment_before: Vec::new(),
        comment_inline: None,
        kind: AstKind::Seq(items),
    })
}

fn parse_map_entry_from_parts(
    lines: &[LineInfo],
    pos: &mut usize,
    parent_indent: usize,
    key: String,
    key_style: QuoteStyle,
    v_rest: &str,
    line_comment: Option<String>,
) -> Result<AstEntry, String> {
    if !v_rest.is_empty() {
        let mut val_node = parse_inline_value(v_rest)?;
        val_node.comment_inline = line_comment;
        Ok(AstEntry {
            key,
            key_style,
            comment_before: Vec::new(),
            comment_inline: None,
            value: val_node,
        })
    } else {
        let child_comments = collect_comments(lines, pos, parent_indent);
        if *pos < lines.len() {
            let next_indent = lines[*pos].indent;
            if next_indent > parent_indent
                || (next_indent == parent_indent && is_seq_line(&lines[*pos].content))
            {
                let mut val_node = parse_block_node(lines, pos, next_indent)?;
                let mut combined = child_comments;
                combined.extend(val_node.comment_before);
                val_node.comment_before = combined;
                return Ok(AstEntry {
                    key,
                    key_style,
                    comment_before: Vec::new(),
                    comment_inline: line_comment,
                    value: val_node,
                });
            }
        }
        let mut val_node = AstNode::from_data(&Data::Null);
        val_node.comment_before = child_comments;
        val_node.comment_inline = line_comment;
        Ok(AstEntry {
            key,
            key_style,
            comment_before: Vec::new(),
            comment_inline: None,
            value: val_node,
        })
    }
}

fn parse_block_map(
    lines: &[LineInfo],
    pos: &mut usize,
    map_indent: usize,
    mut first_entry_comments: Vec<String>,
    initial_entry: Option<AstEntry>,
) -> Result<AstNode, String> {
    let mut entries: Vec<AstEntry> = Vec::new();
    let mut keys_seen = std::collections::HashSet::new();

    if let Some(mut first) = initial_entry {
        first.comment_before = std::mem::take(&mut first_entry_comments);
        keys_seen.insert(first.key.clone());
        entries.push(first);
    }

    while *pos < lines.len() {
        let save_pos = *pos;
        let mut entry_comments = std::mem::take(&mut first_entry_comments);
        entry_comments.extend(collect_comments(lines, pos, map_indent));
        if *pos >= lines.len() {
            *pos = save_pos;
            break;
        }
        let line = &lines[*pos];
        if line.indent < map_indent {
            *pos = save_pos;
            break;
        }
        if line.indent > map_indent {
            return Err(format!("マップのインデントが不正です: {}", line.content));
        }
        let Some((key, key_style, v_rest)) = split_key_value(&line.content)? else {
            return Err(format!("キーと値の形式が不正です: {}", line.content));
        };
        if !keys_seen.insert(key.clone()) {
            return Err(format!("キー「{key}」が重複しています。"));
        }
        let line_comment = line.comment.clone();
        *pos += 1;
        let mut entry =
            parse_map_entry_from_parts(lines, pos, map_indent, key, key_style, &v_rest, line_comment)?;
        entry.comment_before = entry_comments;
        entries.push(entry);
    }

    Ok(AstNode {
        comment_before: Vec::new(),
        comment_inline: None,
        kind: AstKind::Map(entries),
    })
}

fn parse_inline_value(text: &str) -> Result<AstNode, String> {
    let trimmed = text.trim();
    if trimmed.starts_with('*') || trimmed.starts_with('&') {
        return Err("YAMLのアンカー・エイリアスはサポートしていません。".to_string());
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') || trimmed.starts_with('"') || trimmed.starts_with('\'') {
        let mut cursor = FlowCursor::new(trimmed);
        let node = cursor.parse_flow_node()?;
        cursor.skip_spaces();
        if !cursor.is_eof() {
            return Err(format!("値の末尾に不正な文字があります: {trimmed}"));
        }
        return Ok(node);
    }
    if trimmed.contains('[') || trimmed.contains('{') || trimmed.contains(']') || trimmed.contains('}') {
        // Unquoted brackets inside plain scalar that look like broken flow syntax
        if trimmed.starts_with('[') || trimmed.starts_with('{') {
            return Err(format!("不正なコレクション構文です: {trimmed}"));
        }
    }
    Ok(parse_plain_scalar(trimmed))
}

fn parse_plain_scalar(trimmed: &str) -> AstNode {
    if trimmed.is_empty() || trimmed == "null" || trimmed == "~" {
        return AstNode {
            comment_before: Vec::new(),
            comment_inline: None,
            kind: AstKind::Scalar {
                value: Data::Null,
                style: QuoteStyle::Plain,
                raw: None,
            },
        };
    }
    if trimmed == "true" {
        return AstNode {
            comment_before: Vec::new(),
            comment_inline: None,
            kind: AstKind::Scalar {
                value: Data::Bool(true),
                style: QuoteStyle::Plain,
                raw: None,
            },
        };
    }
    if trimmed == "false" {
        return AstNode {
            comment_before: Vec::new(),
            comment_inline: None,
            kind: AstKind::Scalar {
                value: Data::Bool(false),
                style: QuoteStyle::Plain,
                raw: None,
            },
        };
    }
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        if let Ok(n) = i128::from_str_radix(hex, 16) {
            return AstNode {
                comment_before: Vec::new(),
                comment_inline: None,
                kind: AstKind::Scalar {
                    value: Data::Int(n),
                    style: QuoteStyle::Plain,
                    raw: None,
                },
            };
        }
    }
    // Plain integer: optional '-', followed by digits (no leading zero unless "0")
    let is_int = {
        let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
        !digits.is_empty()
            && digits.chars().all(|c| c.is_ascii_digit())
            && (digits == "0" || !digits.starts_with('0'))
    };
    if is_int {
        if let Ok(n) = trimmed.parse::<i128>() {
            return AstNode {
                comment_before: Vec::new(),
                comment_inline: None,
                kind: AstKind::Scalar {
                    value: Data::Int(n),
                    style: QuoteStyle::Plain,
                    raw: None,
                },
            };
        }
    }
    // Plain float
    let is_float_candidate = {
        let s = trimmed.strip_prefix('-').or_else(|| trimmed.strip_prefix('+')).unwrap_or(trimmed);
        !s.is_empty()
            && (s.contains('.') || s.contains('e') || s.contains('E'))
            && s.chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
    };
    if is_float_candidate {
        if let Ok(f) = trimmed.parse::<f64>() {
            if f.is_finite() {
                return AstNode {
                    comment_before: Vec::new(),
                    comment_inline: None,
                    kind: AstKind::Scalar {
                        value: Data::Float(f),
                        style: QuoteStyle::Plain,
                        raw: None,
                    },
                };
            }
        }
    }

    AstNode {
        comment_before: Vec::new(),
        comment_inline: None,
        kind: AstKind::Scalar {
            value: Data::Str(trimmed.to_string()),
            style: QuoteStyle::Plain,
            raw: None,
        },
    }
}

struct FlowCursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> FlowCursor<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_spaces(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.peek() == Some('#') {
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn parse_flow_node(&mut self) -> Result<AstNode, String> {
        self.skip_ws_and_comments();
        match self.peek() {
            None => Err("予期しない終端です。".to_string()),
            Some('{') => self.parse_flow_map(),
            Some('[') => self.parse_flow_seq(),
            Some('"') => {
                let s = self.parse_double_quoted()?;
                Ok(AstNode {
                    comment_before: Vec::new(),
                    comment_inline: None,
                    kind: AstKind::Scalar {
                        value: Data::Str(s),
                        style: QuoteStyle::Double,
                        raw: None,
                    },
                })
            }
            Some('\'') => {
                let s = self.parse_single_quoted()?;
                Ok(AstNode {
                    comment_before: Vec::new(),
                    comment_inline: None,
                    kind: AstKind::Scalar {
                        value: Data::Str(s),
                        style: QuoteStyle::Single,
                        raw: None,
                    },
                })
            }
            Some('*') | Some('&') => {
                Err("YAMLのアンカー・エイリアスはサポートしていません。".to_string())
            }
            Some(_) => {
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if matches!(c, ',' | ']' | '}' | ':' | '\n' | '\r') {
                        if c == ':' {
                            let after = &self.input[self.pos + 1..];
                            if after.is_empty()
                                || after.starts_with(char::is_whitespace)
                                || after.starts_with([',', ']', '}'])
                            {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if c == '#' && self.pos > start {
                        let prev = self.input[..self.pos].chars().last().unwrap_or(' ');
                        if prev.is_whitespace() {
                            break;
                        }
                    }
                    self.advance();
                }
                let token = self.input[start..self.pos].trim();
                if token.is_empty() {
                    return Err(format!("不正なトークンです: {}", self.remaining()));
                }
                Ok(parse_plain_scalar(token))
            }
        }
    }

    fn parse_flow_seq(&mut self) -> Result<AstNode, String> {
        self.advance(); // '['
        let mut items = Vec::new();
        loop {
            self.skip_ws_and_comments();
            match self.peek() {
                None => return Err("配列の ']' が閉じられていません。".to_string()),
                Some(']') => {
                    self.advance();
                    break;
                }
                _ => {
                    let item = self.parse_flow_node()?;
                    items.push(item);
                    self.skip_ws_and_comments();
                    match self.peek() {
                        Some(',') => {
                            self.advance();
                        }
                        Some(']') => {}
                        _ => return Err("配列の要素間に ',' または ']' が必要です。".to_string()),
                    }
                }
            }
        }
        Ok(AstNode {
            comment_before: Vec::new(),
            comment_inline: None,
            kind: AstKind::Seq(items),
        })
    }

    fn parse_flow_map(&mut self) -> Result<AstNode, String> {
        self.advance(); // '{'
        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();
        loop {
            self.skip_ws_and_comments();
            match self.peek() {
                None => return Err("マップの '}' が閉じられていません。".to_string()),
                Some('}') => {
                    self.advance();
                    break;
                }
                _ => {
                    let (key, key_style) = self.parse_key_string()?;
                    if !seen.insert(key.clone()) {
                        return Err(format!("キー「{key}」が重複しています。"));
                    }
                    self.skip_ws_and_comments();
                    if self.peek() != Some(':') {
                        return Err(format!("キー「{key}」の後に ':' が必要です。"));
                    }
                    self.advance();
                    self.skip_ws_and_comments();
                    let value = if matches!(self.peek(), Some(',' | '}')) {
                        AstNode::from_data(&Data::Null)
                    } else {
                        self.parse_flow_node()?
                    };
                    entries.push(AstEntry {
                        key,
                        key_style,
                        comment_before: Vec::new(),
                        comment_inline: None,
                        value,
                    });
                    self.skip_ws_and_comments();
                    match self.peek() {
                        Some(',') => {
                            self.advance();
                        }
                        Some('}') => {}
                        _ => return Err("マップの要素間に ',' または '}' が必要です。".to_string()),
                    }
                }
            }
        }
        Ok(AstNode {
            comment_before: Vec::new(),
            comment_inline: None,
            kind: AstKind::Map(entries),
        })
    }

    fn parse_key_string(&mut self) -> Result<(String, QuoteStyle), String> {
        self.skip_ws_and_comments();
        match self.peek() {
            Some('"') => Ok((self.parse_double_quoted()?, QuoteStyle::Double)),
            Some('\'') => Ok((self.parse_single_quoted()?, QuoteStyle::Single)),
            Some(_) => {
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if matches!(c, ':' | ',' | '}' | ']' | '\n' | '\r') {
                        break;
                    }
                    self.advance();
                }
                let k = self.input[start..self.pos].trim().to_string();
                if k.is_empty() {
                    return Err("キー名が空です。".to_string());
                }
                Ok((k, QuoteStyle::Plain))
            }
            None => Err("キー名がありません。".to_string()),
        }
    }

    fn parse_double_quoted(&mut self) -> Result<String, String> {
        self.advance(); // '"'
        let mut out = String::new();
        while let Some(c) = self.advance() {
            if c == '"' {
                return Ok(out);
            }
            if c == '\\' {
                let esc = self
                    .advance()
                    .ok_or_else(|| "エスケープシーケンスが不正です。".to_string())?;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000c}'),
                    '0' => out.push('\0'),
                    'u' => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            hex.push(
                                self.advance()
                                    .ok_or_else(|| "Unicodeエスケープが不正です。".to_string())?,
                            );
                        }
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| "Unicodeエスケープが不正です。".to_string())?;
                        let ch = char::from_u32(code)
                            .ok_or_else(|| "Unicodeスカラ値が不正です。".to_string())?;
                        out.push(ch);
                    }
                    other => out.push(other),
                }
            } else {
                out.push(c);
            }
        }
        Err("二重引用符が閉じられていません。".to_string())
    }

    fn parse_single_quoted(&mut self) -> Result<String, String> {
        self.advance(); // '\''
        let mut out = String::new();
        while let Some(c) = self.advance() {
            if c == '\'' {
                if self.peek() == Some('\'') {
                    self.advance();
                    out.push('\'');
                } else {
                    return Ok(out);
                }
            } else {
                out.push(c);
            }
        }
        Err("単一引用符が閉じられていません。".to_string())
    }
}
