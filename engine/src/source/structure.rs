use super::*;

impl Document {
    pub(crate) fn map_anchor(&self, mut offset: usize) -> usize {
        for edits in &self.edits {
            let mut delta: isize = 0;
            for (span, length) in edits {
                if offset <= span.start {
                    break;
                }
                if offset < span.end {
                    offset = span.start;
                    break;
                }
                delta += *length as isize - (span.end - span.start) as isize;
            }
            offset = offset.saturating_add_signed(delta);
        }
        offset
    }

    pub fn insert_owned_item(
        &self,
        sequence: &Node,
        at: usize,
        position: usize,
        source: &str,
    ) -> Result<Self> {
        let items = sequence.items()?;
        if at > items.len() || !self.bytes.is_char_boundary(position) {
            return Err(unsafe_structure("stale owned item position"));
        }
        if !items.is_empty() && sequence.style != Style::Block {
            return Err(unsafe_structure("stored record requires block sequence"));
        }
        let lower = if at > 0 {
            self.item_range(sequence, at - 1, true)?.end
        } else {
            let records = self
                .root
                .members()?
                .iter()
                .find(|m| m.name == "records")
                .ok_or_else(|| unsafe_structure("record container missing"))?;
            self.line_end(records.key.end)
        };
        let upper = if at < items.len() {
            self.item_range(sequence, at, true)?.start
        } else {
            self.root
                .members()?
                .iter()
                .find(|m| m.key.start > sequence.span.end)
                .map_or(self.bytes.len(), |m| self.line_start(m.key.start))
        };
        if position < lower || position > upper || self.line_start(position) != position {
            return Err(unsafe_structure(
                "owned record anchor is outside its current gap",
            ));
        }
        let mut patches = vec![];
        if items.is_empty() {
            let empty = self.line_start(sequence.span.start)..self.line_end(sequence.span.end);
            if !self.bytes[empty.start..sequence.span.start]
                .trim()
                .is_empty()
            {
                return Err(unsafe_structure(
                    "empty restore requires an owned block slot",
                ));
            }
            if position == empty.start {
                return self.patched(vec![Patch {
                    span: empty,
                    text: source.into(),
                }]);
            }
            if position > empty.start && position < empty.end {
                return Err(unsafe_structure("record anchor overlaps empty slot"));
            }
            patches.push(Patch {
                span: empty,
                text: String::new(),
            });
        }
        let mut text = source.to_owned();
        if position < self.bytes.len() && !text.ends_with('\n') {
            text.push_str(self.newline());
        }
        patches.push(Patch {
            span: position..position,
            text,
        });
        self.patched(patches)
    }
    pub fn item_range(&self, sequence: &Node, index: usize, newline: bool) -> Result<Range<usize>> {
        if sequence.style != Style::Block {
            return Err(unsafe_structure("block item required"));
        }
        let item = sequence
            .items()?
            .get(index)
            .ok_or_else(|| unsafe_structure("item missing"))?;
        let start = self.line_start(item.span.start);
        if self.bytes[start..item.span.start]
            .bytes()
            .any(|b| b != b' ')
        {
            return Err(unsafe_structure("ambiguous item prefix"));
        }
        let semantic_end = end_of_value(&item.value);
        let mut end = if semantic_end > start && self.bytes.as_bytes()[semantic_end - 1] == b'\n' {
            semantic_end
        } else {
            self.line_end(semantic_end)
        };
        if !newline && self.bytes[..end].ends_with('\n') {
            end -= 1;
            if self.bytes[..end].ends_with('\r') {
                end -= 1;
            }
        }
        if let Some(next) = sequence.items()?.get(index + 1)
            && end > self.line_start(next.span.start)
        {
            return Err(unsafe_structure("overlapping item ownership"));
        }
        Ok(start..end)
    }

    pub fn insert_sequence(&self, sequence: &Node, at: usize, value: &Value) -> Result<Self> {
        let items = sequence.items()?;
        if at > items.len() {
            return Err(unsafe_structure("insert position outside sequence"));
        }
        let mut expected = items
            .iter()
            .map(|item| item.value.value())
            .collect::<Vec<_>>();
        expected.insert(at, value.clone());
        if items.is_empty() {
            let mut patches = vec![];
            derive_patch_materialization(
                self,
                sequence,
                &Value::Sequence(vec![value.clone()]),
                &mut patches,
            )?;
            return self.sequence_candidate(sequence, patches, expected);
        }
        if sequence.style == Style::Flow {
            let rendered = inline_render(value, self.newline())?
                .trim_start()
                .to_owned();
            let (position, text) = if at < items.len() {
                (items[at].span.start, format!("{rendered}, "))
            } else {
                let trailing = self
                    .flow_commas(sequence)?
                    .iter()
                    .any(|r| r.start >= items.last().unwrap().span.end);
                (
                    sequence.span.end - 1,
                    format!("{}{rendered}", if trailing { "" } else { ", " }),
                )
            };
            return self.sequence_candidate(
                sequence,
                vec![Patch {
                    span: position..position,
                    text,
                }],
                expected,
            );
        }
        let indent = self.column(items[0].span.start);
        let mut rendered = render_block(
            &Value::Sequence(vec![value.clone()]),
            indent,
            self.newline(),
        )?;
        let position = if at < items.len() {
            self.item_range(sequence, at, true)?.start
        } else {
            self.item_range(sequence, items.len() - 1, true)?.end
        };
        if position > 0 && !self.bytes[..position].ends_with('\n') {
            rendered.insert_str(0, self.newline());
        }
        rendered.push_str(self.newline());
        self.sequence_candidate(
            sequence,
            vec![Patch {
                span: position..position,
                text: rendered,
            }],
            expected,
        )
    }

    pub fn remove_sequence(&self, sequence: &Node, at: usize) -> Result<Self> {
        let items = sequence.items()?;
        let item = items
            .get(at)
            .ok_or_else(|| unsafe_structure("remove occurrence missing"))?;
        let expected = items
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != at)
            .map(|(_, item)| item.value.value())
            .collect::<Vec<_>>();
        if sequence.style == Style::Flow {
            let mut patches = vec![Patch {
                span: item.span.clone(),
                text: String::new(),
            }];
            let commas = self.flow_commas(sequence)?;
            let comma = if at + 1 < items.len() {
                commas
                    .iter()
                    .find(|r| r.start >= item.span.end && r.end <= items[at + 1].span.start)
            } else if at > 0 {
                commas
                    .iter()
                    .find(|r| r.start >= items[at - 1].span.end && r.end <= item.span.start)
            } else {
                commas.first()
            };
            if let Some(comma) = comma {
                patches.push(Patch {
                    span: comma.clone(),
                    text: String::new(),
                });
            }
            return self.sequence_candidate(sequence, patches, expected);
        }
        let span = self.item_range(sequence, at, true)?;
        let text = if items.len() == 1 {
            format!(
                "{}[]{}",
                " ".repeat(self.column(item.span.start)),
                self.newline()
            )
        } else {
            String::new()
        };
        self.sequence_candidate(sequence, vec![Patch { span, text }], expected)
    }

    pub fn reorder_sequence(&self, sequence: &Node, order: &[usize]) -> Result<Self> {
        let items = sequence.items()?;
        if order.len() != items.len()
            || order.iter().copied().collect::<HashSet<_>>().len() != items.len()
            || order.iter().any(|i| *i >= items.len())
        {
            return Err(unsafe_structure("exact occurrence permutation required"));
        }
        let ranges = (0..items.len())
            .map(|i| {
                if sequence.style == Style::Flow {
                    Ok(items[i].span.clone())
                } else {
                    self.item_range(sequence, i, false)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let patches = order
            .iter()
            .enumerate()
            .filter(|(at, source)| *at != **source)
            .map(|(at, source)| Patch {
                span: ranges[at].clone(),
                text: self.bytes[ranges[*source].clone()].to_owned(),
            })
            .collect();
        self.sequence_candidate(
            sequence,
            patches,
            order
                .iter()
                .map(|index| items[*index].value.value())
                .collect(),
        )
    }

    fn sequence_candidate(
        &self,
        sequence: &Node,
        patches: Vec<Patch>,
        expected: Vec<Value>,
    ) -> Result<Self> {
        let mut route = vec![];
        if !node_route(&self.root, sequence, &mut route) {
            return Err(unsafe_structure(
                "sequence belongs to another source generation",
            ));
        }
        let candidate = self.patched(patches)?;
        let mut actual = candidate.root.as_ref();
        for step in route {
            actual = match step {
                Route::Member(name) => actual.required(&name)?,
                Route::Item(index) => {
                    &actual
                        .items()?
                        .get(index)
                        .ok_or_else(|| unsafe_structure("sequence container moved"))?
                        .value
                }
            };
        }
        if !matches_value(actual, &Value::Sequence(expected)) {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "sequence candidate differs from exact occurrence operation",
            ));
        }
        Ok(candidate)
    }

    fn flow_commas(&self, sequence: &Node) -> Result<Vec<Range<usize>>> {
        let syntax = self
            .syntax
            .root_node()
            .descendant_for_byte_range(sequence.span.start, sequence.span.end)
            .ok_or_else(|| unsafe_structure("flow syntax missing"))?;
        if syntax.kind() != "flow_sequence" || syntax.byte_range() != sequence.span {
            return Err(unsafe_structure("flow container is ambiguous"));
        }
        Ok((0..syntax.child_count())
            .filter_map(|i| syntax.child(i))
            .filter(|n| n.kind() == ",")
            .map(|n| n.byte_range())
            .collect())
    }
}

enum Route {
    Member(String),
    Item(usize),
}
fn node_route(node: &Node, target: &Node, route: &mut Vec<Route>) -> bool {
    if std::ptr::eq(node, target) {
        return true;
    }
    if node.span.start > target.span.start || node.span.end < target.span.end {
        return false;
    }
    match &node.raw {
        Raw::Mapping(members) => {
            for member in members {
                route.push(Route::Member(member.name.clone()));
                if node_route(&member.value, target, route) {
                    return true;
                }
                route.pop();
            }
        }
        Raw::Sequence(items) => {
            let index = items.partition_point(|item| item.value.span.end < target.span.start);
            if let Some(item) = items.get(index) {
                route.push(Route::Item(index));
                if node_route(&item.value, target, route) {
                    return true;
                }
                route.pop();
            }
        }
        _ => {}
    }
    false
}

fn end_of_value(node: &Node) -> usize {
    if node.style == Style::Flow {
        return node.span.end;
    }
    match &node.raw {
        Raw::Mapping(m) => m.last().map_or(node.span.end, |m| end_of_value(&m.value)),
        Raw::Sequence(s) => s.last().map_or(node.span.end, |i| end_of_value(&i.value)),
        _ => node.span.end,
    }
}

pub(super) fn derive_patch_materialization(
    doc: &Document,
    node: &Node,
    desired: &Value,
    patches: &mut Vec<Patch>,
) -> Result<()> {
    let prefix = &doc.bytes[doc.line_start(node.span.start)..node.span.start];
    if prefix.contains('{') || prefix.contains('[') {
        return Err(unsafe_structure("new block value inside flow mapping"));
    }
    let own_line = prefix.trim().is_empty();
    let indent = prefix.bytes().take_while(|b| *b == b' ').count() + if own_line { 0 } else { 2 };
    let end = doc.line_end(node.span.end);
    let suffix = &doc.bytes[node.span.end..end];
    let block = render_block(desired, indent, doc.newline())?;
    let mut span = node.span.clone();
    while span.start > doc.line_start(span.start) && doc.bytes.as_bytes()[span.start - 1] == b' ' {
        span.start -= 1;
    }
    if own_line {
        if suffix.trim_start().starts_with('#') {
            return Err(unsafe_structure(
                "commented standalone empty value cannot be materialized safely",
            ));
        }
        patches.push(Patch { span, text: block });
    } else if suffix.trim_start().starts_with('#') {
        // Keep the container's inline comment on its original line.
        patches.push(Patch {
            span,
            text: String::new(),
        });
        patches.push(Patch {
            span: end..end,
            text: format!(
                "{}{block}{}",
                if suffix.ends_with('\n') {
                    ""
                } else {
                    doc.newline()
                },
                doc.newline()
            ),
        });
    } else {
        patches.push(Patch {
            span,
            text: format!("{}{block}", doc.newline()),
        });
    }
    Ok(())
}
fn unsafe_structure(message: &str) -> Error {
    Error::new("E-SOURCE-UNSAFE", message)
}
