use super::*;

#[derive(Clone)]
struct Node {
    start: usize,
    end: usize,
    children: Vec<Node>,
    keys: Vec<(usize, usize)>,
    indent: usize,
}
struct Locator<'a> {
    source: &'a str,
    lines: Vec<SourceLine<'a>>,
    literal: Vec<bool>,
}
impl<'a> Locator<'a> {
    fn new(source: &'a str) -> Self {
        let lines = source_lines(source);
        let literal = literal_block_scalar_content_lines(&lines);
        Self {
            source,
            lines,
            literal,
        }
    }
    fn next(&self, line: usize) -> Option<usize> {
        (line..self.lines.len())
            .find(|&i| !is_ignorable_line(self.lines[i].text) && !self.literal[i])
    }
    fn line_of(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|l| l.start <= offset)
            .saturating_sub(1)
    }
    fn node(&self, value: &Value, start: usize) -> Result<Node> {
        let line = self.line_of(start);
        let mut node = Node {
            start,
            end: start,
            children: vec![],
            keys: vec![],
            indent: start - self.lines[line].start,
        };
        match value {
            Value::Mapping(map) => {
                let mut at = start;
                for (key, value) in map {
                    let li = self.line_of(at);
                    let text = self
                        .source
                        .get(at..self.lines[li].content_end)
                        .ok_or_else(|| failure("mapping source span ambiguous"))?;
                    let colon = mapping_colon(strip_yaml_comment(text))
                        .ok_or_else(|| failure("mapping source location ambiguous"))?;
                    let raw_key = text[..colon].trim_end();
                    if decode_mapping_key(raw_key).as_deref() != key.as_str() {
                        return Err(failure("mapping source does not match canonical key"));
                    }
                    node.keys.push((at, at + raw_key.len()));
                    let value_start = at + colon + 1;
                    let remaining =
                        strip_yaml_comment(&self.source[value_start..self.lines[li].content_end])
                            .trim();
                    let child_start = if remaining.is_empty() {
                        let next = self
                            .next(li + 1)
                            .ok_or_else(|| failure("missing block value"))?;
                        self.lines[next].start + yaml_indent(self.lines[next].text)
                    } else {
                        value_start + self.source[value_start..].len()
                            - self.source[value_start..].trim_start_matches(' ').len()
                    };
                    let child = self.node(value, child_start)?;
                    node.end = child.end;
                    let next = self.next(self.line_of(child.end.saturating_sub(1)) + 1);
                    node.children.push(child);
                    if let Some(next) = next {
                        at = self.lines[next].start + node.indent;
                    }
                }
            }
            Value::Sequence(values) if self.source[start..].starts_with('[') => {
                let mut at = start + 1;
                for value in values {
                    at = self.flow_space(at);
                    let child = self.node(value, at)?;
                    at = self.flow_space(child.end);
                    node.children.push(child);
                    if self.source.as_bytes().get(at) == Some(&b',') {
                        at += 1;
                    }
                }
                at = self.flow_space(at);
                if self.source.as_bytes().get(at) != Some(&b']') {
                    return Err(failure("flow sequence location ambiguous"));
                }
                node.end = at + 1;
            }
            Value::Sequence(values) => {
                let mut at = start;
                for value in values {
                    node.keys.push((at, at + 1));
                    if self.source.as_bytes().get(at) != Some(&b'-') {
                        return Err(failure("block sequence location ambiguous"));
                    }
                    let li = self.line_of(at);
                    let content =
                        strip_yaml_comment(&self.source[at + 1..self.lines[li].content_end]);
                    let child_start = if content.trim().is_empty() {
                        let next = self
                            .next(li + 1)
                            .ok_or_else(|| failure("missing sequence value"))?;
                        self.lines[next].start + yaml_indent(self.lines[next].text)
                    } else {
                        at + 1 + content.len() - content.trim_start().len()
                    };
                    let child = self.node(value, child_start)?;
                    node.end = child.end;
                    let next = self.next(self.line_of(child.end.saturating_sub(1)) + 1);
                    node.children.push(child);
                    if let Some(next) = next {
                        at = self.lines[next].start + yaml_indent(self.lines[next].text);
                    }
                }
            }
            _ => {
                let bytes = self.source.as_bytes();
                let mut end = start;
                if matches!(bytes.get(start), Some(b'\'' | b'"')) {
                    let quote = bytes[start];
                    end += 1;
                    loop {
                        let byte = *bytes
                            .get(end)
                            .ok_or_else(|| failure("unterminated scalar"))?;
                        end += 1;
                        if byte == quote {
                            if quote == b'\'' && bytes.get(end) == Some(&quote) {
                                end += 1;
                            } else {
                                break;
                            }
                        } else if quote == b'"' && byte == b'\\' {
                            end += 1;
                        }
                    }
                } else if bytes.get(start) == Some(&b'|') {
                    end = self.lines[line].next_start;
                    let mut i = line + 1;
                    while i < self.lines.len() && self.literal[i] {
                        end = self.lines[i].next_start;
                        i += 1;
                    }
                } else {
                    while end < bytes.len() && !matches!(bytes[end], b'\n' | b'\r' | b',' | b']') {
                        if bytes[end] == b'#'
                            && (end == start || bytes[end - 1].is_ascii_whitespace())
                        {
                            break;
                        }
                        end += 1;
                    }
                    while end > start && bytes[end - 1].is_ascii_whitespace() {
                        end -= 1;
                    }
                }
                node.end = end;
            }
        }
        Ok(node)
    }
    fn flow_space(&self, mut at: usize) -> usize {
        let bytes = self.source.as_bytes();
        loop {
            while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
                at += 1;
            }
            if bytes.get(at) == Some(&b'#') {
                while bytes.get(at).is_some_and(|b| *b != b'\n') {
                    at += 1;
                }
            } else {
                return at;
            }
        }
    }
    fn append(&self, node: &Node, lines: Vec<String>) -> MigrationPatch {
        let last = self.line_of(node.end.saturating_sub(1));
        let at = self.lines[last].next_start;
        MigrationPatch {
            start: at,
            end: at,
            replacement: insertion_at(
                self.source,
                at,
                &join_rendered_lines(lines, newline_for(self.source), true),
            ),
        }
    }
    fn remove(&self, start: usize, end: usize, patches: &mut Vec<MigrationPatch>) {
        let first = self.line_of(start);
        let last = self.line_of(end.saturating_sub(1));
        for i in first..=last {
            if is_ignorable_line(self.lines[i].text) && !self.literal[i] {
                continue;
            }
            // Preserve outer comments while removing only the selected value.
            let replacement = if !self.literal[i] {
                comment_start(self.lines[i].text)
                    .map(|at| {
                        format!(
                            "{}{}{}",
                            spaces(yaml_indent(self.lines[i].text)),
                            &self.lines[i].text[at..],
                            newline_for(self.source)
                        )
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            patches.push(MigrationPatch {
                start: self.lines[i].start,
                end: self.lines[i].next_start,
                replacement,
            });
        }
    }
    fn diff(
        &self,
        node: &Node,
        old: &Value,
        new: &Value,
        patches: &mut Vec<MigrationPatch>,
    ) -> Result<()> {
        if old == new {
            return Ok(());
        }
        match (old, new) {
            (Value::Mapping(a), Value::Mapping(b)) => {
                let added: Vec<_> = b.iter().filter(|(k, _)| !a.contains_key(k)).collect();
                let removed: Vec<_> = a.iter().filter(|(k, _)| !b.contains_key(k)).collect();
                let rename = if added.len() == 1 && removed.len() == 1 && added[0].1 == removed[0].1
                {
                    Some((removed[0].0, added[0].0))
                } else {
                    None
                };
                for (i, (key, value)) in a.iter().enumerate() {
                    if let Some(next) = b.get(key) {
                        self.diff(&node.children[i], value, next, patches)?;
                    } else if let Some((_, to)) = rename.filter(|(from, _)| *from == key) {
                        let (start, end) = node.keys[i];
                        patches.push(MigrationPatch {
                            start,
                            end,
                            replacement: quoted(
                                &self.source[start..end],
                                to.as_str().ok_or_else(|| failure("non-string key"))?,
                            ),
                        });
                    } else {
                        let (start, _) = node.keys[i];
                        // A compact `- key:` owns the sequence dash. Move it to
                        // the following member when that first member is removed.
                        let li = self.line_of(start);
                        let prefix = &self.source[self.lines[li].start..start];
                        self.remove(start, node.children[i].end, patches);
                        if prefix.trim() == "-" {
                            let next = node
                                .keys
                                .get(i + 1)
                                .ok_or_else(|| {
                                    failure("cannot remove last compact mapping member")
                                })?
                                .0;
                            let line = self.line_of(next);
                            let indent = yaml_indent(self.lines[line].text);
                            patches.push(MigrationPatch {
                                start: self.lines[line].start + indent - 2,
                                end: self.lines[line].start + indent,
                                replacement: "- ".into(),
                            });
                        }
                    }
                }
                if rename.is_none() && !added.is_empty() {
                    let mut lines = vec![];
                    for (key, value) in added {
                        lines.extend(render_value_lines(
                            value,
                            node.indent,
                            Some(key.as_str().ok_or_else(|| failure("non-string key"))?),
                        )?);
                    }
                    patches.push(self.append(node, lines));
                }
            }
            (Value::Sequence(a), Value::Sequence(b)) if a.len() == b.len() => {
                for ((child, old), new) in node.children.iter().zip(a).zip(b) {
                    self.diff(child, old, new, patches)?;
                }
            }
            (Value::Sequence(a), Value::Sequence(b))
                if b.len() == a.len() + 1 && b[..a.len()] == a[..] =>
            {
                let value = b.last().expect("appended");
                let mut lines = render_value_lines(value, node.indent + 2, None)?;
                if lines.is_empty() {
                    return Err(failure("empty sequence item"));
                }
                lines[0] = format!("{}- {}", spaces(node.indent), lines[0].trim_start());
                if a.is_empty() {
                    patches.push(MigrationPatch {
                        start: node.start,
                        end: node.end,
                        replacement: format!(
                            "{}{}",
                            newline_for(self.source),
                            join_rendered_lines(lines, newline_for(self.source), false)
                        ),
                    });
                } else {
                    patches.push(self.append(node, lines));
                }
            }
            (Value::Sequence(a), Value::Sequence(b)) if a.len() == b.len() + 1 => {
                let index = (0..a.len())
                    .find(|&i| {
                        a.iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(_, v)| v)
                            .eq(b.iter())
                    })
                    .ok_or_else(|| failure("sequence deletion ambiguous"))?;
                self.remove(node.keys[index].0, node.children[index].end, patches);
            }
            _ => {
                let replacement = if let Value::String(value) = new {
                    quoted(&self.source[node.start..node.end], value)
                } else {
                    render_scalar(new)?
                };
                patches.push(MigrationPatch {
                    start: node.start,
                    end: node.end,
                    replacement,
                });
            }
        }
        Ok(())
    }
}
fn quoted(raw: &str, value: &str) -> String {
    if raw.starts_with('\'') {
        format!("'{}'", value.replace('\'', "''"))
    } else if raw.starts_with('"') {
        serde_json::to_string(value).expect("string")
    } else {
        value.into()
    }
}
fn yaml<T: serde::Serialize>(value: &T) -> Result<Value> {
    serde_yaml::to_value(value).map_err(|e| failure(e.to_string()))
}

pub(super) fn patch_document(
    before: &LoadedDocument,
    desired: &SourceDocument,
    command: &TypeMigrationCommand,
) -> Result<Vec<MigrationPatch>> {
    let old: Value = serde_yaml::from_str(&before.source).map_err(|e| failure(e.to_string()))?;
    let mut new = old.clone();
    match desired {
        SourceDocument::Data(data) => {
            new["records"] = yaml(&data.records)?;
        }
        SourceDocument::Type(ty) => {
            use TypeMigrationOperation as Op;
            match &command.operation {
                Op::SetValueObjectConversions(value) => {
                    let vo = new["valueObject"]
                        .as_mapping_mut()
                        .ok_or_else(|| failure("Value Object mapping missing"))?;
                    let key = Value::String("conversions".into());
                    if let Some(conversions) = vo.get_mut(&key) {
                        let map = conversions
                            .as_mapping_mut()
                            .ok_or_else(|| failure("conversions mapping missing"))?;
                        map.insert(
                            Value::String("fromUnderlyingImplicit".into()),
                            Value::Bool(value.from_underlying_implicit),
                        );
                        map.insert(
                            Value::String("toUnderlyingImplicit".into()),
                            Value::Bool(value.to_underlying_implicit),
                        );
                    } else {
                        vo.insert(key, yaml(value)?);
                    }
                }
                Op::AddEnumMember(member) => {
                    let cat = if ty.flags.is_some() { "flags" } else { "enum" };
                    new[cat]["members"]
                        .as_sequence_mut()
                        .ok_or_else(|| failure("members missing"))?
                        .push(yaml(member)?);
                }
                Op::RenameEnumMember { .. } | Op::DropEnumMember { .. } => {
                    let cat = if ty.flags.is_some() { "flags" } else { "enum" };
                    let seq = new[cat]["members"]
                        .as_sequence_mut()
                        .ok_or_else(|| failure("members missing"))?;
                    let index = seq
                        .iter()
                        .position(|v| v["name"].as_str() == Some(command.operation.selector()))
                        .ok_or_else(|| failure("member missing"))?;
                    if let Op::RenameEnumMember { new_name, .. } = &command.operation {
                        seq[index]["name"] = Value::String(new_name.clone());
                    } else {
                        seq.remove(index);
                    }
                }
                Op::AddCustomField { field, .. } => new["custom"]["fields"]
                    .as_sequence_mut()
                    .ok_or_else(|| failure("fields missing"))?
                    .push(yaml(field)?),
                Op::RenameCustomField { .. } | Op::DropCustomField { .. } => {
                    let seq = new["custom"]["fields"]
                        .as_sequence_mut()
                        .ok_or_else(|| failure("fields missing"))?;
                    let index = seq
                        .iter()
                        .position(|v| v["name"].as_str() == Some(command.operation.selector()))
                        .ok_or_else(|| failure("field missing"))?;
                    if let Op::RenameCustomField { new_name, .. } = &command.operation {
                        seq[index]["name"] = Value::String(new_name.clone());
                    } else {
                        seq.remove(index);
                    }
                }
            }
        }
        SourceDocument::View(_) => {}
        _ => return Err(failure("unsupported source kind")),
    }
    let locator = Locator::new(&before.source);
    let first = locator.next(0).ok_or_else(|| failure("empty source"))?;
    let node = locator.node(&old, locator.lines[first].start)?;
    let mut patches = vec![];
    locator.diff(&node, &old, &new, &mut patches)?;
    Ok(patches)
}
