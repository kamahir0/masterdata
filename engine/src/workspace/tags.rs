//! Record metadata authoring, independent of domain columns and Build selection.
use super::*;
use crate::source::{Node, Style};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TagOperation {
    Add { text: String },
    Remove { index: usize },
    Replace { index: usize, text: String },
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEntry {
    pub index: usize,
    pub text: String,
    pub valid: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagView {
    pub source: String,
    pub row: String,
    pub revision: u64,
    pub generation: u64,
    pub entries: Vec<TagEntry>,
    pub start: usize,
    pub total: usize,
    pub editable: bool,
    pub reason: Option<String>,
    pub known: Vec<String>,
    pub partial: bool,
}

fn string_entries(record: &Node) -> Result<Vec<String>> {
    if !record.safe {
        return Err(Error::new(
            "E-SOURCE-UNSAFE",
            "record location is ambiguous",
        ));
    }
    record.members()?;
    let Some(tags) = record.get("$tags") else {
        return Ok(vec![]);
    };
    if !tags.safe {
        return Err(Error::new("E-SOURCE-UNSAFE", "Tag location is ambiguous"));
    }
    tags.items()?
        .iter()
        .map(|item| {
            if !item.value.safe {
                return Err(Error::new(
                    "E-SOURCE-UNSAFE",
                    "Tag entry location is ambiguous",
                ));
            }
            item.value.text().map(str::to_owned)
        })
        .collect()
}
fn editable_entries(record: &Node) -> Result<Vec<String>> {
    let entries = string_entries(record)?;
    if record.get("$tags").is_none() && record.style != Style::Block {
        return Err(Error::new(
            "E-SOURCE-UNSAFE",
            "adding Tags requires a block record mapping",
        ));
    }
    Ok(entries)
}

impl Workspace {
    pub fn tag_view(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
        start: usize,
    ) -> Result<TagView> {
        self.authoring_context(path, revision, generation, false)?;
        let d = &self.drafts[path];
        let at = d.active_index(row)?;
        let record = d.document.occurrence_value(at + 1, &[])?;
        let entries = editable_entries(record);
        let reason = entries.as_ref().err().map(|e| match e.code {
            "E-VALUE-SHAPE" => "Tags must be a sequence of strings; edit this source externally to repair its shape".into(),
            _ => e.message.clone(),
        });
        let entries = entries.unwrap_or_default();
        let total = entries.len();
        let start = start.min(total.saturating_sub(1));
        let mut counts = BTreeMap::new();
        for tag in &entries {
            *counts.entry(tag).or_insert(0usize) += 1;
        }
        let entries = entries
            .iter()
            .enumerate()
            .skip(start)
            .take(64)
            .map(|(index, text)| {
                let reason = if !semantic::kebab(text) {
                    Some("Use lowercase kebab-case".into())
                } else if counts[text] > 1 {
                    Some("Duplicate Tag".into())
                } else {
                    None
                };
                TagEntry {
                    index,
                    text: text.clone(),
                    valid: reason.is_none(),
                    reason,
                }
            })
            .collect();
        // Discovery is contextual, not on the selection critical path. Include
        // only already loaded sources and saved profile tags, never a registry
        // or a broad filesystem/YAML refresh. Bound autocomplete transport.
        let mut known = BTreeSet::new();
        let mut partial = !self.unavailable.is_empty();
        for (source_path, source) in &self.read.sources {
            if self.unavailable.contains_key(source_path) {
                continue;
            }
            let document = self
                .drafts
                .get(source_path)
                .map(|d| &d.document)
                .or(source.document.as_ref());
            let Some(document) = document else {
                partial = true;
                continue;
            };
            if let Ok(records) = document.records() {
                for record in records {
                    match string_entries(&record.value) {
                        Ok(tags) => {
                            for tag in tags {
                                known.insert(tag);
                            }
                        }
                        Err(_) => partial = true,
                    }
                }
            }
        }
        for profile in self.read.config.build.profiles.values() {
            known.extend(
                profile
                    .include_tags
                    .iter()
                    .chain(&profile.exclude_tags)
                    .cloned(),
            );
        }
        partial |= known.len() > 100;
        Ok(TagView {
            source: path.into(),
            row: row.into(),
            revision: d.revision,
            generation: self.generation,
            entries,
            start,
            total,
            editable: reason.is_none(),
            reason,
            known: known.into_iter().take(100).collect(),
            partial,
        })
    }

    pub fn edit_tag(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
        operation: TagOperation,
    ) -> Result<bool> {
        self.authoring_context(path, revision, generation, true)?;
        let d = &self.drafts[path];
        let at = d.active_index(row)?;
        let record = d.document.occurrence_value(at + 1, &[])?;
        let mut expected = editable_entries(record)?;
        let absent = record.get("$tags").is_none();
        let mut restore_absent = false;
        let mut owned_prefix = false;
        let candidate = match operation {
            TagOperation::Add { text } => {
                let value = Value::Text(text.clone());
                let candidate = if let Some(tags) = record.get("$tags") {
                    d.document.insert_sequence(tags, expected.len(), &value)?
                } else {
                    let mut patches = vec![];
                    d.document.derive_member_add(
                        record,
                        "$tags",
                        &Value::Sequence(vec![value]),
                        &mut patches,
                    )?;
                    owned_prefix = patches.iter().any(|p| {
                        p.span.start == d.document.bytes.len()
                            && !d.document.bytes.ends_with('\n')
                            && p.text.starts_with(d.document.newline())
                    });
                    d.document.patched(patches)?
                };
                expected.push(text);
                candidate
            }
            TagOperation::Replace { index, text } => {
                let entry = expected
                    .get_mut(index)
                    .ok_or_else(|| Error::new("E-LOCATOR-STALE", "Tag entry missing"))?;
                *entry = text.clone();
                d.document.edit_occurrence(
                    at + 1,
                    &["$tags".into(), index.to_string()],
                    &Value::Text(text),
                )?
            }
            TagOperation::Remove { index } => {
                if index >= expected.len() {
                    return Err(Error::new("E-LOCATOR-STALE", "Tag entry missing"));
                }
                expected.remove(index);
                restore_absent = expected.is_empty() && d.tag_origins.contains_key(row);
                if restore_absent {
                    let mut patches = vec![];
                    d.document
                        .derive_member_drop(record, "$tags", &mut patches)?;
                    // The first metadata addition at a no-newline EOF owns the
                    // delimiter it introduced. Remove it only with that final,
                    // uncommented property; unrelated trivia remains untouched.
                    if d.tag_origins[row] && !d.document.bytes.ends_with('\n') {
                        for patch in &mut patches {
                            if patch.span.end == d.document.bytes.len()
                                && patch.text.is_empty()
                                && patch.span.start >= d.document.newline().len()
                                && d.document.bytes[..patch.span.start]
                                    .ends_with(d.document.newline())
                            {
                                patch.span.start -= d.document.newline().len();
                            }
                        }
                    }
                    d.document.patched(patches)?
                } else {
                    d.document
                        .remove_sequence(record.required("$tags")?, index)?
                }
            }
        };
        let actual = candidate.occurrence_value(at + 1, &[])?;
        if record
            .members()?
            .iter()
            .filter(|m| m.name != "$tags")
            .any(|member| {
                !actual
                    .get(&member.name)
                    .is_some_and(|node| crate::source::matches_value(node, &member.value.value()))
            })
        {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "Tag edit altered a domain value",
            ));
        }
        if string_entries(actual)? != expected || (restore_absent && actual.get("$tags").is_some())
        {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "Tag candidate differs from exact entries",
            ));
        }
        let d = self.drafts.get_mut(path).unwrap();
        let changed = d.apply(candidate, d.row_ids.clone());
        if changed {
            if absent {
                Arc::make_mut(&mut d.tag_origins).insert(row.into(), owned_prefix);
            }
            if restore_absent {
                Arc::make_mut(&mut d.tag_origins).remove(row);
            }
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }
}
