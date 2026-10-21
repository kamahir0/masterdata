use crate::model::{
    fields_of, selected, type_kind, type_source, Data, Field, Snapshot, Source, TableInfo,
    PRIMITIVES,
};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

pub fn primitive_of(snapshot: &Snapshot, field: &Field) -> String {
    if let Some(ts) = type_source(snapshot, &field.r#type) {
        if let Some(vo) = ts.data.get("valueObject") {
            if let Some(u) = vo.get("underlying").and_then(|v| v.as_str()) {
                return u.to_string();
            }
        }
    }
    field.r#type.clone()
}

pub fn default_value(snapshot: &Snapshot, field: &Field, seen: &HashSet<String>) -> Data {
    if field.nullable {
        return Data::Null;
    }
    if field.array {
        return Data::Array(Vec::new());
    }
    if field.r#type == "bool" {
        return Data::Bool(false);
    }
    if field.r#type == "string" {
        return Data::Str(String::new());
    }
    if PRIMITIVES.contains(&field.r#type.as_str()) {
        if field.r#type == "float" || field.r#type == "double" {
            return Data::Float(0.0);
        }
        return Data::Int(0);
    }
    if seen.contains(&field.r#type) {
        return Data::Null;
    }
    let Some(t_src) = type_source(snapshot, &field.r#type) else {
        return Data::Null;
    };
    let Some(kind) = type_kind(Some(&t_src)) else {
        return Data::Null;
    };
    let Some(definition) = t_src.data.get(kind) else {
        return Data::Null;
    };
    if kind == "valueObject" {
        let underlying = definition
            .get("underlying")
            .and_then(|v| v.as_str())
            .unwrap_or("int")
            .to_string();
        return default_value(
            snapshot,
            &Field {
                name: String::new(),
                r#type: underlying,
                key: None,
                array: false,
                nullable: false,
            },
            seen,
        );
    }
    if kind == "enum" {
        let members = definition
            .get("members")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if let Some(zero_member) = members
            .iter()
            .find(|m| m.get("value").map(|v| v.text()).as_deref() == Some("0"))
        {
            if let Some(name) = zero_member.get("name").and_then(|v| v.as_str()) {
                return Data::Str(name.to_string());
            }
        }
        if let Some(first) = members.first().and_then(|m| m.get("name")).and_then(|v| v.as_str()) {
            return Data::Str(first.to_string());
        }
        return Data::Str(String::new());
    }
    if kind == "flags" {
        return Data::Array(vec![Data::Str("None".to_string())]);
    }
    let mut next_seen = seen.clone();
    next_seen.insert(field.r#type.clone());
    let mut map = IndexMap::new();
    for f in fields_of(&t_src) {
        map.insert(f.name.clone(), default_value(snapshot, &f, &next_seen));
    }
    Data::Map(map)
}

pub fn flags_change(current: &[String], names: &[String]) -> Vec<String> {
    if names.iter().any(|n| n == "None") && !current.iter().any(|n| n == "None") {
        return vec!["None".to_string()];
    }
    let bits: Vec<String> = names.iter().filter(|n| n.as_str() != "None").cloned().collect();
    if bits.is_empty() {
        vec!["None".to_string()]
    } else {
        bits
    }
}

pub fn in_profile(snapshot: &Snapshot, name: &str) -> Snapshot {
    let profile = snapshot.config.profiles.get(name);
    let sources = snapshot
        .sources
        .iter()
        .map(|s| {
            if let Some(records) = s.data.get("records").and_then(|v| v.as_array()) {
                let filtered: Vec<Data> = records
                    .iter()
                    .filter(|r| selected(r, profile))
                    .cloned()
                    .collect();
                let mut next_data = s.data.clone();
                if let Some(map) = next_data.as_map_mut() {
                    map.insert("records".to_string(), Data::Array(filtered));
                }
                let mut next_source = s.clone();
                next_source.data = next_data;
                next_source
            } else {
                s.clone()
            }
        })
        .collect();
    Snapshot {
        sources,
        folders: snapshot.folders.clone(),
        config: snapshot.config.clone(),
    }
}

pub struct RecordAllocator<'a> {
    snapshot: &'a Snapshot,
    table: &'a TableInfo,
    keys: Vec<Vec<String>>,
    indexes: Vec<HashSet<String>>,
    key_names: Vec<String>,
    used: HashMap<String, HashSet<String>>,
    maxima: HashMap<String, i128>,
    counters: HashMap<String, usize>,
}

fn signature(key: &[String], row: &Data) -> String {
    let parts: Vec<String> = key
        .iter()
        .map(|n| row.get(n).map(|v| v.text()).unwrap_or_default())
        .collect();
    serde_json::to_string(&parts).unwrap_or_default()
}

impl<'a> RecordAllocator<'a> {
    pub fn new(snapshot: &'a Snapshot, table: &'a TableInfo) -> Self {
        let rows: Vec<&Data> = table
            .sources
            .iter()
            .filter_map(|s| s.data.get("records").and_then(|v| v.as_array()))
            .flatten()
            .collect();

        let mut keys: Vec<Vec<String>> = Vec::new();
        if let Some(pk) = table
            .source
            .data
            .get("primaryKey")
            .and_then(|v| v.get("fields"))
            .and_then(|v| v.as_array())
        {
            let pk_fields: Vec<String> = pk.iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
            if !pk_fields.is_empty() {
                keys.push(pk_fields);
            }
        }
        if let Some(sks) = table
            .source
            .data
            .get("secondaryKeys")
            .and_then(|v| v.as_array())
        {
            for sk in sks {
                let non_unique = sk.get("nonUnique").and_then(|v| v.as_bool()).unwrap_or(false);
                if non_unique {
                    continue;
                }
                if let Some(f_arr) = sk.get("fields").and_then(|v| v.as_array()) {
                    let sk_fields: Vec<String> = f_arr
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect();
                    if !sk_fields.is_empty() {
                        keys.push(sk_fields);
                    }
                }
            }
        }

        let indexes: Vec<HashSet<String>> = keys
            .iter()
            .map(|k| rows.iter().map(|r| signature(k, r)).collect())
            .collect();

        let mut key_names = Vec::new();
        for k in &keys {
            for name in k {
                if !key_names.contains(name) {
                    key_names.push(name.clone());
                }
            }
        }

        let mut used: HashMap<String, HashSet<String>> = HashMap::new();
        let mut maxima: HashMap<String, i128> = HashMap::new();
        for name in &key_names {
            let mut set = HashSet::new();
            let mut max = 0i128;
            for r in &rows {
                let val = r.get(name).unwrap_or(&Data::Undefined);
                set.insert(val.text());
                if let Some(n) = val.parse_bigint() {
                    if n > max {
                        max = n;
                    }
                }
            }
            used.insert(name.clone(), set);
            maxima.insert(name.clone(), max);
        }

        Self {
            snapshot,
            table,
            keys,
            indexes,
            key_names,
            used,
            maxima,
            counters: HashMap::new(),
        }
    }

    pub fn allocate(&mut self, template: Option<&Data>) -> Result<Data, String> {
        let mut result = match template {
            Some(t) => t.clone(),
            None => {
                let seen = HashSet::new();
                let mut map = IndexMap::new();
                for f in &self.table.fields {
                    map.insert(f.name.clone(), default_value(self.snapshot, f, &seen));
                }
                Data::Map(map)
            }
        };

        for attempt in 0..10000 {
            let conflict = self
                .keys
                .iter()
                .enumerate()
                .find(|(i, key)| self.indexes[*i].contains(&signature(key, &result)))
                .map(|(i, _)| i);

            let Some(conflict_idx) = conflict else {
                for (i, key) in self.keys.iter().enumerate() {
                    self.indexes[i].insert(signature(key, &result));
                }
                for name in &self.key_names {
                    let v = result.get(name).cloned().unwrap_or(Data::Undefined);
                    if let Some(u) = self.used.get_mut(name) {
                        u.insert(v.text());
                    }
                    if let Some(n) = v.parse_bigint() {
                        let cur_max = self.maxima.get(name).copied().unwrap_or(0);
                        if n > cur_max {
                            self.maxima.insert(name.clone(), n);
                        }
                    }
                }
                return Ok(result);
            };

            let components: Vec<&Field> = self.keys[conflict_idx]
                .iter()
                .rev()
                .filter_map(|name| self.table.fields.iter().find(|f| &f.name == name))
                .filter(|f| !f.array && !f.nullable)
                .collect();

            let field = components
                .iter()
                .copied()
                .find(|f| {
                    matches!(
                        primitive_of(self.snapshot, f).as_str(),
                        "int" | "uint" | "long" | "ulong" | "string"
                    )
                })
                .or_else(|| components.first().copied())
                .ok_or_else(|| {
                    "一意なキーを作成できません。キーの設定を確認してください。".to_string()
                })?;

            let primitive = primitive_of(self.snapshot, field);
            let values = self.used.get(&field.name).unwrap();
            if primitive == "string" {
                let mut i = self.counters.get(&field.name).copied().unwrap_or(0);
                let prefix = template
                    .and_then(|t| t.get(&field.name))
                    .map(|v| v.text())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "new".to_string());
                let candidate = loop {
                    i += 1;
                    let c = format!("{prefix}-{i}");
                    if !values.contains(&c) {
                        break c;
                    }
                };
                self.counters.insert(field.name.clone(), i);
                if let Some(m) = result.as_map_mut() {
                    m.insert(field.name.clone(), Data::Str(candidate));
                }
            } else if matches!(primitive.as_str(), "int" | "uint" | "long" | "ulong") {
                let limit: i128 = match primitive.as_str() {
                    "int" => 2147483647,
                    "uint" => 4294967295,
                    "long" => 9223372036854775807,
                    _ => 18446744073709551615,
                };
                let mut candidate = self.maxima.get(&field.name).copied().unwrap_or(0) + 1;
                if candidate > limit {
                    candidate = 0;
                    while values.contains(&candidate.to_string())
                        && candidate <= (values.len() as i128)
                    {
                        candidate += 1;
                    }
                }
                if candidate > limit || values.contains(&candidate.to_string()) {
                    return Err("キーの空きを割り当てられません。".to_string());
                }
                if let Some(m) = result.as_map_mut() {
                    m.insert(field.name.clone(), Data::Int(candidate));
                }
            } else {
                let members = type_source(self.snapshot, &field.r#type)
                    .and_then(|s| s.data.get("enum").cloned())
                    .and_then(|e| e.get("members").cloned())
                    .and_then(|m| m.as_array().cloned())
                    .unwrap_or_default();
                if attempt >= members.len() {
                    return Err("このEnumの一意な値は使い切られています。".to_string());
                }
                let m_name = members[attempt]
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(m) = result.as_map_mut() {
                    m.insert(field.name.clone(), Data::Str(m_name));
                }
            }
        }
        Err("一意なキー候補を割り当てられませんでした。".to_string())
    }
}

pub fn map_typed_value<F>(
    snapshot: &Snapshot,
    field: &Field,
    value: &Data,
    target: &str,
    modify: &F,
    seen: &HashSet<String>,
) -> Data
where
    F: Fn(&Data) -> Data,
{
    if value.is_null_or_undefined() {
        return value.clone();
    }
    if field.array {
        let Some(arr) = value.as_array() else {
            return value.clone();
        };
        let mut item_field = field.clone();
        item_field.array = false;
        let next: Vec<Data> = arr
            .iter()
            .map(|v| map_typed_value(snapshot, &item_field, v, target, modify, seen))
            .collect();
        if next != *arr {
            return Data::Array(next);
        }
        return value.clone();
    }
    if field.r#type == target {
        return modify(value);
    }
    let Some(t_src) = type_source(snapshot, &field.r#type) else {
        return value.clone();
    };
    if t_src.data.get("custom").is_none() || !value.is_object() || seen.contains(&field.r#type) {
        return value.clone();
    }
    let mut next_seen = seen.clone();
    next_seen.insert(field.r#type.clone());
    let mut next_map = value.as_map().unwrap().clone();
    let mut changed = false;
    for child in fields_of(&t_src) {
        let current_val = next_map.get(&child.name).cloned().unwrap_or(Data::Undefined);
        let updated = map_typed_value(snapshot, &child, &current_val, target, modify, &next_seen);
        if updated != current_val {
            next_map.insert(child.name, updated);
            changed = true;
        }
    }
    if changed {
        Data::Map(next_map)
    } else {
        value.clone()
    }
}

pub fn map_records<F>(
    snapshot: &Snapshot,
    table_name: Option<&str>,
    type_name: Option<&str>,
    modify: F,
) -> Vec<Source>
where
    F: Fn(&Data) -> Data,
{
    snapshot
        .sources
        .iter()
        .map(|source| {
            let Some(records) = source.data.get("records").and_then(|v| v.as_array()) else {
                return source.clone();
            };
            let src_table = source.data.get("table").and_then(|v| v.as_str()).unwrap_or("");
            let schema = snapshot.sources.iter().find(|s| {
                s.data.get("kind").and_then(|v| v.as_str()) == Some("schema")
                    && s.data.get("table").and_then(|v| v.as_str()) == Some(src_table)
            });
            let mut any_changed = false;
            let next_records: Vec<Data> = records
                .iter()
                .map(|record| {
                    if !record.is_object() {
                        return record.clone();
                    }
                    if let Some(t_name) = table_name {
                        if src_table == t_name {
                            let updated = modify(record);
                            if &updated != record {
                                any_changed = true;
                            }
                            return updated;
                        }
                    }
                    let (Some(target_type), Some(schema_src)) = (type_name, schema) else {
                        return record.clone();
                    };
                    let mut next_row = record.as_map().unwrap().clone();
                    let mut row_changed = false;
                    let seen = HashSet::new();
                    for field in fields_of(schema_src) {
                        let cur = next_row.get(&field.name).cloned().unwrap_or(Data::Undefined);
                        let changed =
                            map_typed_value(snapshot, &field, &cur, target_type, &modify, &seen);
                        if changed != cur {
                            next_row.insert(field.name, changed);
                            row_changed = true;
                        }
                    }
                    if row_changed {
                        any_changed = true;
                        Data::Map(next_row)
                    } else {
                        record.clone()
                    }
                })
                .collect();

            if any_changed {
                let mut next_source = source.clone();
                if let Some(m) = next_source.data.as_map_mut() {
                    m.insert("records".to_string(), Data::Array(next_records));
                }
                next_source
            } else {
                source.clone()
            }
        })
        .collect()
}
