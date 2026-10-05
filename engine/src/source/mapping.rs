use super::*;

fn unsafe_member(message: &str) -> Error {
    Error::new("E-SOURCE-UNSAFE", message)
}
impl Document {
    fn mapping_commas(&self, mapping: &Node) -> Result<Vec<Range<usize>>> {
        let syntax = self
            .syntax
            .root_node()
            .descendant_for_byte_range(mapping.span.start, mapping.span.end)
            .ok_or_else(|| unsafe_member("mapping syntax missing"))?;
        if syntax.kind() != "flow_mapping" || syntax.byte_range() != mapping.span {
            return Err(unsafe_member("flow mapping ownership is ambiguous"));
        }
        Ok((0..syntax.child_count())
            .filter_map(|i| syntax.child(i))
            .filter(|node| node.kind() == ",")
            .map(|node| node.byte_range())
            .collect())
    }
    pub fn derive_member_rename(
        &self,
        mapping: &Node,
        old: &str,
        new: &str,
        patches: &mut Vec<Patch>,
    ) -> Result<()> {
        let members = mapping.members()?;
        if !mapping.safe
            || members
                .iter()
                .any(|member| member.name == new && old != new)
        {
            return Err(unsafe_member("unsafe mapping or member collision"));
        }
        let member = members
            .iter()
            .find(|member| member.name == old)
            .ok_or_else(|| unsafe_member("member not found"))?;
        if old == new {
            return Ok(());
        }
        let spelling = &self.bytes[member.key.clone()];
        let style = if spelling.starts_with('\'') {
            Style::Single
        } else if spelling.starts_with('"') {
            Style::Double
        } else {
            Style::Plain
        };
        patches.push(Patch {
            span: member.key.clone(),
            text: render_text(new, style, 0, self.newline())?,
        });
        Ok(())
    }
    pub fn derive_member_add(
        &self,
        mapping: &Node,
        name: &str,
        value: &Value,
        patches: &mut Vec<Patch>,
    ) -> Result<()> {
        let members = mapping.members()?;
        if !mapping.safe || members.iter().any(|member| member.name == name) {
            return Err(unsafe_member("unsafe mapping or member collision"));
        }
        let key = render_text(name, Style::Plain, 0, self.newline())?;
        if mapping.style == Style::Flow {
            let commas = self.mapping_commas(mapping)?;
            let trailing = members
                .last()
                .is_some_and(|last| commas.iter().any(|comma| comma.start >= last.span.end));
            // Existing flow syntax is preserved. A new compound mapping cannot
            // be written as block syntax inside it, so fail closed instead of
            // materializing a flow mapping or rewriting the surrounding record.
            let scalar = inline_render(value, self.newline())?;
            let text = format!(
                "{}{key}:{scalar}",
                if members.is_empty() || trailing {
                    ""
                } else {
                    ", "
                }
            );
            patches.push(Patch {
                span: mapping.span.end - 1..mapping.span.end - 1,
                text,
            });
            return Ok(());
        }
        let last = members
            .last()
            .ok_or_else(|| unsafe_member("block mapping member required"))?;
        let end = self.member_line_end(last);
        let indent = self.column(members[0].key.start);
        let rendered = render_block(
            &Value::Mapping(vec![(name.into(), value.clone())]),
            indent,
            self.newline(),
        )?;
        let prefix = if end > 0 && self.bytes.as_bytes()[end - 1] != b'\n' {
            self.newline()
        } else {
            ""
        };
        let suffix = if end < self.bytes.len() || self.bytes.ends_with('\n') {
            self.newline()
        } else {
            ""
        };
        patches.push(Patch {
            span: end..end,
            text: format!("{prefix}{rendered}{suffix}"),
        });
        Ok(())
    }
    pub fn derive_member_drop(
        &self,
        mapping: &Node,
        name: &str,
        patches: &mut Vec<Patch>,
    ) -> Result<()> {
        let members = mapping.members()?;
        if !mapping.safe {
            return Err(unsafe_member("unsafe mapping"));
        }
        let index = members
            .iter()
            .position(|member| member.name == name)
            .ok_or_else(|| unsafe_member("member not found"))?;
        let member = &members[index];
        if mapping.style == Style::Flow {
            let commas = self.mapping_commas(mapping)?;
            let comma = if let Some(next) = members.get(index + 1) {
                commas
                    .iter()
                    .find(|comma| comma.start >= member.span.end && comma.end <= next.span.start)
            } else if index > 0 {
                commas.iter().find(|comma| {
                    comma.start >= members[index - 1].span.end && comma.end <= member.span.start
                })
            } else {
                commas.first()
            };
            patches.push(Patch {
                span: member.span.clone(),
                text: String::new(),
            });
            if let Some(comma) = comma {
                patches.push(Patch {
                    span: comma.clone(),
                    text: String::new(),
                });
            }
            return Ok(());
        }
        let start = self.line_start(member.key.start);
        let prefix = &self.bytes[start..member.key.start];
        let end = self.member_line_end(member);
        if prefix.trim() == "-" {
            let next = members
                .get(index + 1)
                .ok_or_else(|| unsafe_member("removing the only inline block member is unsafe"))?;
            let next_start = self.line_start(next.key.start);
            if end > next_start || !self.bytes[next_start..next.key.start].trim().is_empty() {
                return Err(unsafe_member("next member has ambiguous ownership"));
            }
            // The sequence dash belongs to the record, not its first field.
            // Transfer it to the next field rather than deleting the record.
            patches.push(Patch {
                span: next_start..next.key.start,
                text: prefix.into(),
            });
        } else if !prefix.trim().is_empty() || members.len() == 1 {
            return Err(unsafe_member("block member ownership is ambiguous"));
        }
        patches.push(Patch {
            span: start..end,
            text: self.preserved_member_trivia(start..end, self.column(member.key.start)),
        });
        Ok(())
    }
    fn preserved_member_trivia(&self, range: Range<usize>, indent: usize) -> String {
        fn comments(node: SyntaxNode<'_>, range: &Range<usize>, out: &mut Vec<Range<usize>>) {
            if node.end_byte() <= range.start || node.start_byte() >= range.end {
                return;
            }
            if node.kind() == "comment" {
                if node.start_byte() >= range.start && node.end_byte() <= range.end {
                    out.push(node.byte_range());
                }
                return;
            }
            for child in (0..node.child_count()).filter_map(|i| node.child(i)) {
                comments(child, range, out);
            }
        }
        // A '#' inside a block scalar is value text. Only CST comment nodes
        // survive field removal; lexical scanning would corrupt that boundary.
        let mut owned_comments = Vec::new();
        comments(self.syntax.root_node(), &range, &mut owned_comments);
        let mut retained = String::new();
        let mut at = range.start;
        while at < range.end {
            let end = self.line_end(at).min(range.end);
            let line = &self.bytes[at..end];
            if line.trim().is_empty() {
                retained.push_str(line);
            } else if let Some(comment) = owned_comments
                .iter()
                .find(|c| c.start >= at && c.start < end)
            {
                let prefix = &self.bytes[at..comment.start];
                if prefix.trim().is_empty() {
                    retained.push_str(prefix);
                } else if at == range.start {
                    retained.push_str(&" ".repeat(indent));
                } else {
                    retained.push_str(&prefix[..prefix.len() - prefix.trim_start().len()]);
                }
                retained.push_str(&self.bytes[comment.start..end]);
            }
            at = end;
        }
        retained
    }
    fn member_line_end(&self, member: &Member) -> usize {
        fn value_end(node: &Node) -> usize {
            if node.style == Style::Flow {
                return node.span.end;
            }
            match &node.raw {
                Raw::Mapping(members) => members
                    .last()
                    .map_or(node.span.end, |member| value_end(&member.value)),
                Raw::Sequence(items) => items
                    .last()
                    .map_or(node.span.end, |item| value_end(&item.value)),
                _ => node.span.end,
            }
        }
        let end = value_end(&member.value);
        if end > 0 && self.bytes.as_bytes()[end - 1] == b'\n' {
            end
        } else {
            self.line_end(end)
        }
    }
}
