use super::*;
use crate::source::Node;
use serde::Deserialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorNode {
    pub path: Vec<String>,
    pub label: String,
    pub shape: Option<Shape>,
    pub kind: String,
    pub display: String,
    pub input: Option<String>,
    pub valid: bool,
    pub editable: bool,
    pub reason: Option<String>,
    pub problem: Option<semantic::ValueProblem>,
    pub children: Vec<EditorNode>,
    pub total_children: usize,
    pub start: usize,
    pub selected_members: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorProjection {
    pub source: String,
    pub row: String,
    pub revision: u64,
    pub generation: u64,
    pub node: EditorNode,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Operation {
    Text {
        text: String,
    },
    Null,
    Materialize,
    Enum {
        symbol: String,
    },
    Flag {
        symbol: String,
        enabled: bool,
    },
    Add,
    Remove {
        item: String,
    },
    Reorder {
        items: Vec<String>,
    },
    Move {
        item: String,
        before: Option<String>,
    },
    Place {
        item: String,
        index: usize,
    },
    Nudge {
        item: String,
        delta: isize,
    },
}
struct Resolved {
    node: Arc<Node>,
    shape: Shape,
    actual: Vec<String>,
    key: Vec<String>,
}
impl Workspace {
    pub fn complex_view(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
        value_path: &[String],
        window: std::ops::Range<usize>,
    ) -> Result<EditorProjection> {
        let start = window.start;
        let count = window.end.saturating_sub(start);
        let (table, types) = self.authoring_context(path, revision, generation, false)?;
        let d = self.drafts.get_mut(path).unwrap();
        let resolved = resolve(d, &table, &types, row, value_path)?;
        let mut root = descriptor(
            value_path.to_vec(),
            resolved
                .actual
                .last()
                .map(|label| {
                    label
                        .parse::<usize>()
                        .map(|index| (index + 1).to_string())
                        .unwrap_or_else(|_| label.clone())
                })
                .unwrap_or_default(),
            Some(&resolved.shape),
            Some(&resolved.node),
            &types,
        );
        root.start = start;
        if self.recovery_required || d.outcome == Some(crate::native::Outcome::OutcomeUnknown) {
            root.editable = false;
            root.reason = Some(
                if self.recovery_required {
                    "Recovery Required"
                } else {
                    "Outcome Unknown: source observation required"
                }
                .into(),
            );
        }
        if resolved.shape.array || resolved.shape.category == "flags" {
            if let Ok(items) = resolved.node.items() {
                let ids = element_ids(d, &resolved.key, &resolved.node)?;
                root.total_children = items.len();
                let mut element = resolved.shape.clone();
                element.array = false;
                element.nullable = false;
                for (i, item) in items.iter().enumerate().skip(start).take(count.min(64)) {
                    let mut p = value_path.to_vec();
                    p.push(ids[i].clone());
                    let mut child = descriptor(
                        p,
                        (i + 1).to_string(),
                        Some(&element),
                        Some(&item.value),
                        &types,
                    );
                    if resolved.shape.category == "flags" {
                        child.editable = false;
                        child.valid = item
                            .value
                            .text()
                            .is_ok_and(|s| resolved.shape.members.iter().any(|m| m == s));
                        child.reason =
                            Some("Flags member operations use the parent control".into());
                    }
                    root.children.push(child);
                }
                if resolved.shape.category == "flags" {
                    root.selected_members = items
                        .iter()
                        .filter_map(|i| i.value.text().ok())
                        .filter(|s| resolved.shape.members.iter().any(|m| m == s))
                        .map(str::to_owned)
                        .collect();
                }
            }
        } else if resolved.shape.category == "custom"
            && let Ok(members) = resolved.node.members()
        {
            let mut children = resolved
                .shape
                .fields
                .iter()
                .map(|(f, s)| {
                    (
                        f.name.clone(),
                        Some(s),
                        members
                            .iter()
                            .find(|m| m.name == f.name)
                            .map(|m| m.value.as_ref()),
                    )
                })
                .collect::<Vec<_>>();
            children.extend(
                members
                    .iter()
                    .filter(|m| !resolved.shape.fields.iter().any(|(f, _)| f.name == m.name))
                    .map(|m| (m.name.clone(), None, Some(m.value.as_ref()))),
            );
            root.total_children = children.len();
            for (name, shape, node) in children.into_iter().skip(start).take(count.min(64)) {
                let mut p = value_path.to_vec();
                p.push(name.clone());
                root.children.push(descriptor(p, name, shape, node, &types));
            }
        }
        Ok(EditorProjection {
            source: path.into(),
            row: row.into(),
            revision: d.revision,
            generation: self.generation,
            node: root,
        })
    }
    pub fn complex_operation(
        &mut self,
        path: &str,
        revision: u64,
        generation: u64,
        row: &str,
        value_path: &[String],
        operation: Operation,
    ) -> Result<bool> {
        let (table, types) = self.batch_context(path, revision, generation)?;
        let d = self.drafts.get_mut(path).unwrap();
        let target = resolve(d, &table, &types, row, value_path)?;
        if !target.node.safe {
            return Err(Error::new(
                "E-SOURCE-UNSAFE",
                "target source representation is unsafe",
            ));
        }
        let record = d.active_index(row)?;
        let old_ids = if target.node.items().is_ok() {
            Some(element_ids(d, &target.key, &target.node)?)
        } else {
            None
        };
        let mut ids = old_ids.clone();
        let candidate = match operation {
            Operation::Text { text } => {
                if target.shape.array
                    || matches!(target.shape.category.as_str(), "custom" | "flags")
                {
                    return Err(Error::new("E-VALUE-SHAPE", "scalar leaf required"));
                }
                d.document.edit_occurrence(
                    record + 1,
                    &target.actual,
                    &semantic::authoring_input(&target.shape, &text),
                )?
            }
            Operation::Null => {
                if !target.shape.nullable {
                    return Err(Error::new("E-NULLABILITY", "Nullable value required"));
                }
                d.document
                    .edit_occurrence(record + 1, &target.actual, &Value::Null)?
            }
            Operation::Materialize => {
                if (target.shape.array || target.shape.category == "flags")
                    && target.node.items().is_ok()
                    || target.shape.category == "custom" && target.node.members().is_ok()
                {
                    return Ok(false);
                }
                let value = if target.shape.array || target.shape.category == "flags" {
                    ids = Some(vec![]);
                    Value::Sequence(vec![])
                } else if target.shape.category == "custom" {
                    Value::Mapping(
                        target
                            .shape
                            .fields
                            .iter()
                            .map(|(f, _)| (f.name.clone(), Value::Null))
                            .collect(),
                    )
                } else {
                    return Err(Error::new(
                        "E-VALUE-INPUT",
                        "enter a scalar value explicitly",
                    ));
                };
                d.document
                    .edit_occurrence(record + 1, &target.actual, &value)?
            }
            Operation::Enum { symbol } => {
                if target.shape.category != "enum"
                    || target.shape.array
                    || !target.shape.members.contains(&symbol)
                {
                    return Err(Error::new("E-ENUM-MEMBER", "declared Enum member required"));
                }
                d.document
                    .edit_occurrence(record + 1, &target.actual, &Value::Text(symbol))?
            }
            Operation::Flag { symbol, enabled } => {
                if target.shape.category != "flags"
                    || target.shape.array
                    || !target.shape.members.contains(&symbol)
                {
                    return Err(Error::new(
                        "E-FLAGS-MEMBER",
                        "declared Flags member required",
                    ));
                }
                let items = target.node.items()?;
                let selected = items
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.value.text().is_ok_and(|s| s == symbol))
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if selected.is_empty() == !enabled {
                    return Ok(false);
                }
                if enabled {
                    let candidate = d.document.insert_sequence(
                        &target.node,
                        items.len(),
                        &Value::Text(symbol),
                    )?;
                    let next = ids.as_mut().unwrap();
                    next.push(new_element_id(d, &target.key, next.len()));
                    candidate
                } else {
                    let mut candidate = (*d.document).clone();
                    for at in selected.into_iter().rev() {
                        let node = candidate.occurrence_value(record + 1, &target.actual)?;
                        candidate = candidate.remove_sequence(node, at)?;
                        ids.as_mut().unwrap().remove(at);
                    }
                    candidate
                }
            }
            Operation::Add => {
                if !target.shape.array {
                    return Err(Error::new("E-VALUE-SHAPE", "Array required"));
                }
                let at = target.node.items()?.len();
                let candidate = d.document.insert_sequence(&target.node, at, &Value::Null)?;
                ids.as_mut()
                    .unwrap()
                    .push(new_element_id(d, &target.key, at));
                candidate
            }
            Operation::Remove { item } => {
                if !target.shape.array && target.shape.category != "flags" {
                    return Err(Error::new("E-VALUE-SHAPE", "sequence required"));
                }
                let at = ids
                    .as_ref()
                    .unwrap()
                    .iter()
                    .position(|id| id == &item)
                    .ok_or_else(|| Error::new("E-LOCATOR-STALE", "Array occurrence missing"))?;
                let candidate = d.document.remove_sequence(&target.node, at)?;
                ids.as_mut().unwrap().remove(at);
                candidate
            }
            Operation::Reorder { items } => {
                if !target.shape.array {
                    return Err(Error::new("E-VALUE-SHAPE", "Array required"));
                }
                let positions = super::records::permutation(ids.as_ref().unwrap(), &items)?;
                let candidate = d.document.reorder_sequence(&target.node, &positions)?;
                ids = Some(items);
                candidate
            }
            Operation::Move { item, before } => {
                if !target.shape.array {
                    return Err(Error::new("E-VALUE-SHAPE", "Array required"));
                }
                let old = ids.as_ref().unwrap();
                if !old.contains(&item) {
                    return Err(Error::new("E-LOCATOR-STALE", "Array occurrence missing"));
                }
                if before.as_ref() == Some(&item) {
                    return Ok(false);
                }
                let mut order = old
                    .iter()
                    .filter(|s| *s != &item)
                    .cloned()
                    .collect::<Vec<_>>();
                let at = before
                    .as_ref()
                    .map(|b| {
                        order.iter().position(|s| s == b).ok_or_else(|| {
                            Error::new("E-LOCATOR-STALE", "Array destination missing")
                        })
                    })
                    .transpose()?
                    .unwrap_or(order.len());
                order.insert(at, item);
                let positions = super::records::permutation(old, &order)?;
                let candidate = d.document.reorder_sequence(&target.node, &positions)?;
                ids = Some(order);
                candidate
            }
            Operation::Nudge { item, delta } => {
                if !target.shape.array || !matches!(delta, -1 | 1) {
                    return Err(Error::new(
                        "E-REORDER-POSITION",
                        "Array neighbouring position required",
                    ));
                }
                let old = ids.as_ref().unwrap();
                let at = old
                    .iter()
                    .position(|id| id == &item)
                    .ok_or_else(|| Error::new("E-LOCATOR-STALE", "Array occurrence missing"))?;
                let to = at as isize + delta;
                if to < 0 || to >= old.len() as isize {
                    return Ok(false);
                }
                let mut order = old.clone();
                order.swap(at, to as usize);
                let positions = super::records::permutation(old, &order)?;
                let candidate = d.document.reorder_sequence(&target.node, &positions)?;
                ids = Some(order);
                candidate
            }
            Operation::Place { item, index } => {
                if !target.shape.array {
                    return Err(Error::new("E-VALUE-SHAPE", "Array required"));
                }
                let old = ids.as_ref().unwrap();
                let at = old
                    .iter()
                    .position(|id| id == &item)
                    .ok_or_else(|| Error::new("E-LOCATOR-STALE", "Array occurrence missing"))?;
                if index >= old.len() {
                    return Err(Error::new(
                        "E-REORDER-POSITION",
                        "Array destination outside source",
                    ));
                }
                if at == index {
                    return Ok(false);
                }
                let mut order = old.clone();
                order.remove(at);
                order.insert(index, item);
                let positions = super::records::permutation(old, &order)?;
                let candidate = d.document.reorder_sequence(&target.node, &positions)?;
                ids = Some(order);
                candidate
            }
        };
        // The source primitives already reparse their localized candidates. Check
        // record membership as well: a nested operation cannot change its source rows.
        if candidate.records()?.len() != d.document.records()?.len() {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "complex operation changed record membership",
            ));
        }
        if let Some(ids) = &ids
            && candidate
                .occurrence_value(record + 1, &target.actual)?
                .items()
                .is_ok_and(|s| s.len() != ids.len())
        {
            return Err(Error::new(
                "E-SOURCE-POSTCONDITION",
                "Array occurrence count differs",
            ));
        }
        let changed = d.apply(candidate, d.row_ids.clone());
        if changed {
            if let Some(ids) = ids {
                Arc::make_mut(&mut d.arrays).insert(target.key, ids);
            }
            self.generation += 1;
            self.diagnostics_pending = true;
        }
        Ok(changed)
    }
}
fn descriptor(
    path: Vec<String>,
    label: String,
    shape: Option<&Shape>,
    node: Option<&Node>,
    types: &semantic::Types,
) -> EditorNode {
    let result = shape.zip(node).map(|(s, n)| {
        semantic::interpret(
            &Field {
                key: 0,
                name: label.clone(),
                type_name: s.type_name.clone(),
                nullable: s.nullable,
                array: s.array,
            },
            n,
            types,
        )
    });
    let problem = result.as_ref().and_then(|r| r.as_ref().err()).cloned();
    let reason = if shape.is_none() {
        Some("unknown source member; preserved until an explicit structural repair".into())
    } else if node.is_none() {
        Some("source member missing".into())
    } else if node.is_some_and(|n| !n.safe) {
        Some("unsafe source representation".into())
    } else {
        None
    };
    EditorNode {
        path,
        label,
        shape: shape.cloned(),
        kind: match node.map(|n| &n.raw) {
            None => "missing",
            Some(Raw::Null) => "null",
            Some(Raw::Scalar(_)) => "scalar",
            Some(Raw::Mapping(_)) => "mapping",
            Some(Raw::Sequence(_)) => "sequence",
        }
        .into(),
        display: node.map(display).unwrap_or_else(|| "(missing)".into()),
        input: node.and_then(|n| n.text().ok()).map(str::to_owned),
        valid: result.is_some_and(|r| r.is_ok()),
        editable: reason.is_none(),
        reason,
        problem,
        children: vec![],
        total_children: 0,
        start: 0,
        selected_members: vec![],
    }
}
fn resolve(
    d: &mut Draft,
    table: &Table,
    types: &semantic::Types,
    row: &str,
    path: &[String],
) -> Result<Resolved> {
    let first = path
        .first()
        .ok_or_else(|| Error::new("E-LOCATOR", "field path required"))?;
    let f = table
        .fields
        .iter()
        .find(|f| &f.name == first)
        .ok_or_else(|| Error::new("E-FIELD-MISSING", first))?;
    let mut shape = semantic::shape(f, types)?;
    let at = d.active_index(row)?;
    let mut node = d.document.records()?[at]
        .value
        .members()?
        .iter()
        .find(|m| &m.name == first)
        .map(|m| m.value.clone())
        .ok_or_else(|| Error::new("E-FIELD-MISSING", first))?;
    let mut actual = vec![first.clone()];
    let mut key = vec![row.into(), first.clone()];
    for component in path.iter().skip(1) {
        if shape.array {
            let ids = element_ids(d, &key, &node)?;
            let index = ids.iter().position(|s| s == component).ok_or_else(|| {
                Error::new(
                    "E-LOCATOR-STALE",
                    "exact Array occurrence identity required",
                )
            })?;
            node = node.items()?[index].value.clone();
            actual.push(index.to_string());
            shape.array = false;
            shape.nullable = false;
        } else if shape.category == "custom" {
            shape = shape
                .fields
                .iter()
                .find(|(f, _)| &f.name == component)
                .map(|(_, s)| s.clone())
                .ok_or_else(|| Error::new("E-FIELD-MISSING", component))?;
            node = node
                .members()?
                .iter()
                .find(|m| &m.name == component)
                .map(|m| m.value.clone())
                .ok_or_else(|| Error::new("E-FIELD-MISSING", component))?;
            actual.push(component.clone());
        } else {
            return Err(Error::new("E-LOCATOR", "path descends into a scalar"));
        }
        key.push(component.clone());
    }
    Ok(Resolved {
        node,
        shape,
        actual,
        key,
    })
}
pub(super) fn diagnostic_path(
    d: &mut Draft,
    table: &Table,
    types: &semantic::Types,
    row: &str,
    path: &[String],
) -> Result<(Vec<String>, usize)> {
    let field = table
        .fields
        .iter()
        .find(|f| Some(&f.name) == path.first())
        .ok_or_else(|| Error::new("E-FIELD-MISSING", "diagnostic field missing"))?;
    let mut shape = Some(semantic::shape(field, types)?);
    let mut node = d.document.records()?[d.active_index(row)?]
        .value
        .members()?
        .iter()
        .find(|m| m.name == field.name)
        .map(|m| m.value.clone());
    let mut result = vec![field.name.clone()];
    let mut key = vec![row.to_owned(), field.name.clone()];
    let mut start = 0;
    for component in path.iter().skip(1) {
        let current = node
            .as_ref()
            .ok_or_else(|| Error::new("E-LOCATOR-STALE", "diagnostic parent missing"))?;
        let next = if matches!(current.raw, Raw::Sequence(_)) {
            let index = component
                .parse::<usize>()
                .map_err(|_| Error::new("E-LOCATOR-STALE", "diagnostic element index invalid"))?;
            let ids = element_ids(d, &key, current)?;
            let id = ids
                .get(index)
                .ok_or_else(|| Error::new("E-LOCATOR-STALE", "diagnostic element missing"))?
                .clone();
            start = index / 64 * 64;
            shape = shape.map(|mut s| {
                s.array = false;
                s.nullable = false;
                s
            });
            node = Some(current.items()?[index].value.clone());
            id
        } else {
            let members = current.members()?;
            let fields = shape.as_ref().map(|s| s.fields.as_slice()).unwrap_or(&[]);
            let known = fields.iter().position(|(f, _)| &f.name == component);
            let index = known
                .or_else(|| {
                    members
                        .iter()
                        .filter(|m| !fields.iter().any(|(f, _)| f.name == m.name))
                        .position(|m| &m.name == component)
                        .map(|i| fields.len() + i)
                })
                .ok_or_else(|| Error::new("E-LOCATOR-STALE", "diagnostic member missing"))?;
            start = index / 64 * 64;
            shape = fields
                .iter()
                .find(|(f, _)| &f.name == component)
                .map(|(_, s)| s.clone());
            node = members
                .iter()
                .find(|m| &m.name == component)
                .map(|m| m.value.clone());
            component.clone()
        };
        key.push(next.clone());
        result.push(next);
    }
    Ok((result, start))
}
fn new_element_id(d: &Draft, key: &[String], ordinal: usize) -> String {
    format!(
        "element:{}:{}:{}:{}",
        d.base.content,
        d.revision + 1,
        crate::source::content_identity(serde_json::to_string(key).unwrap().as_bytes()),
        ordinal
    )
}
fn element_ids(d: &mut Draft, key: &[String], node: &Node) -> Result<Vec<String>> {
    let len = node.items()?.len();
    if let Some(ids) = d.arrays.get(key)
        && ids.len() == len
    {
        return Ok(ids.clone());
    }
    let ids = (0..len)
        .map(|i| new_element_id(d, key, i))
        .collect::<Vec<_>>();
    Arc::make_mut(&mut d.arrays).insert(key.to_vec(), ids.clone());
    Ok(ids)
}
