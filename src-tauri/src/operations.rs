use crate::model::{
    fields_of, tables, type_kind, type_source, type_sources, uid, Data, Field, LoadedProject,
    PathKey, SavedFile, Snapshot, Source, TableInfo,
};
use crate::schema::{
    default_value, flags_change, in_profile, map_records, map_typed_value, primitive_of,
    RecordAllocator,
};
use crate::validation::{fingerprint, validate};
use crate::yaml::{parse_structured_data, serialize};
use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStep {
    Key(String),
    Index(usize),
    Append,
}

pub struct EditSession {
    pub snapshot: Snapshot,
    pub saved: IndexMap<String, SavedFile>,
    pub saved_sources: IndexMap<String, Source>,
    pub force_dirty: HashSet<String>,
}

impl EditSession {
    pub fn new(project: &LoadedProject) -> Self {
        let mut saved = IndexMap::new();
        for f in &project.saved {
            saved.insert(f.id.clone(), f.clone());
        }
        let mut saved_sources = IndexMap::new();
        for s in &project.snapshot.sources {
            saved_sources.insert(s.id.clone(), s.clone());
        }
        Self {
            snapshot: project.snapshot.clone(),
            saved,
            saved_sources,
            force_dirty: HashSet::new(),
        }
    }

    pub fn dirty_source(&self, source: &Source) -> bool {
        if self.force_dirty.contains(&source.id) {
            return true;
        }
        let Some(base) = self.saved_sources.get(&source.id) else {
            return true;
        };
        base.path != source.path || base.data != source.data || base.raw != source.raw
    }

    pub fn files(&self) -> Vec<SavedFile> {
        self.snapshot
            .sources
            .iter()
            .map(|s| {
                let text = if self.dirty_source(s) {
                    serialize(s, self.saved.get(&s.id).map(|b| b.text.as_str()))
                } else {
                    self.saved
                        .get(&s.id)
                        .map(|b| b.text.clone())
                        .unwrap_or_else(|| serialize(s, None))
                };
                SavedFile {
                    id: s.id.clone(),
                    path: s.path.clone(),
                    text,
                }
            })
            .collect()
    }

    pub fn update(&mut self, id: &str, path: &[PathKey], value: Data) {
        for s in &mut self.snapshot.sources {
            if s.id == id {
                s.data = s.data.set_in(path, value);
                s.raw = None;
                s.syntax_error = None;
                break;
            }
        }
    }

    pub fn remove_paths(&mut self, paths: &[String]) {
        self.snapshot.sources.retain(|f| {
            !paths
                .iter()
                .any(|p| f.path == *p || f.path.starts_with(&format!("{p}/")))
        });
        self.snapshot.folders.retain(|f| {
            !paths
                .iter()
                .any(|p| f == p || f.starts_with(&format!("{p}/")))
        });
    }

    pub fn move_path(&mut self, old_path: &str, new_path: &str) -> Result<(), String> {
        if new_path.is_empty()
            || new_path == old_path
            || new_path.starts_with(&format!("{old_path}/"))
        {
            return Ok(());
        }
        if new_path.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || p.chars().any(|c| c == '\\' || (c as u32) <= 0x1f)
        }) {
            return Err("名前に使用できない文字があります。".to_string());
        }
        if self.snapshot.sources.iter().any(|f| f.path == new_path)
            || self.snapshot.folders.iter().any(|f| f == new_path)
        {
            return Err("同名の項目が存在します。".to_string());
        }
        let rename = |p: &str| -> String {
            if p == old_path {
                new_path.to_string()
            } else if let Some(rest) = p.strip_prefix(&format!("{old_path}/")) {
                format!("{new_path}/{rest}")
            } else {
                p.to_string()
            }
        };
        for f in &mut self.snapshot.sources {
            let next = rename(&f.path);
            if next != f.path {
                f.path = next;
            }
        }
        self.snapshot.folders = self.snapshot.folders.iter().map(|f| rename(f)).collect();
        Ok(())
    }

    pub fn remove_field(&mut self, id: &str, index: usize) {
        let Some(source) = self.snapshot.sources.iter().find(|s| s.id == id).cloned() else {
            return;
        };
        let fields = fields_of(&source);
        let Some(field) = fields.get(index) else {
            return;
        };
        let name = field.name.clone();
        if name.is_empty() {
            return;
        }
        let is_custom = source.data.get("custom").is_some();
        let table_name = if is_custom {
            None
        } else {
            source.data.get("table").and_then(|v| v.as_str())
        };
        let type_name = if is_custom {
            source.data.get("name").and_then(|v| v.as_str())
        } else {
            None
        };
        let mut updated_sources = map_records(&self.snapshot, table_name, type_name, |record| {
            if let Data::Map(map) = record {
                let mut next = map.clone();
                next.shift_remove(&name);
                Data::Map(next)
            } else {
                record.clone()
            }
        });
        let remaining: Vec<Data> = fields
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, f)| f.to_data())
            .collect();
        let path: Vec<PathKey> = if is_custom {
            vec![PathKey::from("custom"), PathKey::from("fields")]
        } else {
            vec![PathKey::from("fields")]
        };
        for s in &mut updated_sources {
            if s.id == id {
                s.data = s.data.set_in(&path, Data::Array(remaining.clone()));
            }
        }
        self.snapshot.sources = updated_sources;
    }

    pub fn rename_member(&mut self, id: &str, index: usize, name: &str) {
        let Some(source) = self.snapshot.sources.iter().find(|s| s.id == id).cloned() else {
            return;
        };
        let Some(kind) = type_kind(Some(&source)) else {
            return;
        };
        let Some(old) = source
            .data
            .get(kind)
            .and_then(|d| d.get("members"))
            .and_then(|m| m.get_index(index))
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
        else {
            return;
        };
        let target_type_name = source
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let snap_clone = self.snapshot.clone();
        let mut next_sources: Vec<Source> = self
            .snapshot
            .sources
            .iter()
            .map(|s| {
                let Some(records) = s.data.get("records").and_then(|v| v.as_array()) else {
                    return s.clone();
                };
                let table_name = s.data.get("table").and_then(|v| v.as_str());
                let Some(schema) = snap_clone.sources.iter().find(|f| {
                    f.data.get("kind").and_then(|v| v.as_str()) == Some("schema")
                        && f.data.get("table").and_then(|v| v.as_str()) == table_name
                }) else {
                    return s.clone();
                };
                let schema_fields = fields_of(schema);
                let mut changed = false;
                let next_records: Vec<Data> = records
                    .iter()
                    .map(|record| {
                        let Data::Map(rmap) = record else {
                            return record.clone();
                        };
                        let mut next_map = rmap.clone();
                        let mut row_changed = false;
                        for field in &schema_fields {
                            let cur_val = rmap.get(&field.name).cloned().unwrap_or(Data::Undefined);
                            if matches!(cur_val, Data::Undefined) {
                                continue;
                            }
                            let mapped = map_typed_value(
                                &snap_clone,
                                field,
                                &cur_val,
                                &target_type_name,
                                &|val| {
                                    if kind == "flags" {
                                        if let Data::Array(arr) = val {
                                            let next_arr: Vec<Data> = arr
                                                .iter()
                                                .map(|v| {
                                                    if v.as_str() == Some(old.as_str()) {
                                                        Data::Str(name.to_string())
                                                    } else {
                                                        v.clone()
                                                    }
                                                })
                                                .collect();
                                            return Data::Array(next_arr);
                                        }
                                    }
                                    if val.as_str() == Some(old.as_str()) {
                                        Data::Str(name.to_string())
                                    } else {
                                        val.clone()
                                    }
                                },
                                &HashSet::new(),
                            );
                            if mapped != cur_val {
                                next_map.insert(field.name.clone(), mapped);
                                row_changed = true;
                            }
                        }
                        if row_changed {
                            changed = true;
                            Data::Map(next_map)
                        } else {
                            record.clone()
                        }
                    })
                    .collect();
                if changed {
                    let mut ns = s.clone();
                    ns.data = ns
                        .data
                        .set_in(&[PathKey::from("records")], Data::Array(next_records));
                    ns
                } else {
                    s.clone()
                }
            })
            .collect();

        for s in &mut next_sources {
            if s.id == id {
                s.data = s.data.set_in(
                    &[
                        PathKey::from(kind),
                        PathKey::from("members"),
                        PathKey::from(index),
                        PathKey::from("name"),
                    ],
                    Data::Str(name.to_string()),
                );
            }
        }
        self.snapshot.sources = next_sources;
    }

    pub fn rename_field(&mut self, id: &str, index: usize, name: &str) -> Result<(), String> {
        let Some(source) = self.snapshot.sources.iter().find(|s| s.id == id).cloned() else {
            return Ok(());
        };
        let fields = fields_of(&source);
        let old = fields.get(index).map(|f| f.name.clone()).unwrap_or_default();
        if old == name {
            return Ok(());
        }
        let is_custom = source.data.get("custom").is_some();
        if old.is_empty() {
            let path = if is_custom {
                vec![
                    PathKey::from("custom"),
                    PathKey::from("fields"),
                    PathKey::from(index),
                    PathKey::from("name"),
                ]
            } else {
                vec![
                    PathKey::from("fields"),
                    PathKey::from(index),
                    PathKey::from("name"),
                ]
            };
            self.update(id, &path, Data::Str(name.to_string()));
            return Ok(());
        }
        if fields
            .iter()
            .enumerate()
            .any(|(i, f)| i != index && f.name == name)
        {
            return Err("同名の項目があります。".to_string());
        }

        let snap_clone = self.snapshot.clone();
        let target_type_name = source
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let target_table_name = source
            .data
            .get("table")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        fn rewrite_custom(
            snap: &Snapshot,
            target_type: &str,
            old: &str,
            name: &str,
            value: &Data,
            r#type: &str,
            field: &Field,
            seen: &HashSet<String>,
        ) -> Data {
            if value.is_null_or_undefined() {
                return value.clone();
            }
            if field.array {
                if let Data::Array(arr) = value {
                    let mut item_field = field.clone();
                    item_field.array = false;
                    return Data::Array(
                        arr.iter()
                            .map(|v| {
                                rewrite_custom(
                                    snap,
                                    target_type,
                                    old,
                                    name,
                                    v,
                                    r#type,
                                    &item_field,
                                    seen,
                                )
                            })
                            .collect(),
                    );
                }
            }
            let Data::Map(map) = value else {
                return value.clone();
            };
            let Some(def) = snap.sources.iter().find(|s| {
                s.data.get("name").and_then(|v| v.as_str()) == Some(r#type)
                    && s.data.get("custom").is_some()
            }) else {
                return value.clone();
            };
            if seen.contains(r#type) {
                return value.clone();
            }
            let mut seen_next = seen.clone();
            seen_next.insert(r#type.to_string());

            let mut next = IndexMap::new();
            for (k, v) in map {
                if r#type == target_type && k == old {
                    next.insert(name.to_string(), v.clone());
                } else {
                    next.insert(k.clone(), v.clone());
                }
            }
            for f in fields_of(def) {
                let key = if r#type == target_type && f.name == old {
                    name.to_string()
                } else {
                    f.name.clone()
                };
                if let Some(existing) = next.get(&key).cloned() {
                    let rewritten = rewrite_custom(
                        snap,
                        target_type,
                        old,
                        name,
                        &existing,
                        &f.r#type,
                        &f,
                        &seen_next,
                    );
                    next.insert(key, rewritten);
                }
            }
            Data::Map(next)
        }

        let all_tables = tables(&snap_clone);
        let next_sources: Vec<Source> = self
            .snapshot
            .sources
            .iter()
            .map(|f| {
                let mut data = f.data.clone();
                if f.id == id {
                    let path = if is_custom {
                        vec![
                            PathKey::from("custom"),
                            PathKey::from("fields"),
                            PathKey::from(index),
                            PathKey::from("name"),
                        ]
                    } else {
                        vec![
                            PathKey::from("fields"),
                            PathKey::from(index),
                            PathKey::from("name"),
                        ]
                    };
                    data = data.set_in(&path, Data::Str(name.to_string()));
                }
                if !is_custom
                    && f.data.get("table").and_then(|v| v.as_str())
                        == Some(target_table_name.as_str())
                {
                    if let Some(records) = data.get("records").and_then(|v| v.as_array()).cloned() {
                        let updated_records: Vec<Data> = records
                            .into_iter()
                            .map(|r| {
                                let Data::Map(rmap) = r else {
                                    return r;
                                };
                                if !rmap.contains_key(&old) {
                                    return Data::Map(rmap);
                                }
                                let mut nmap = IndexMap::new();
                                for (k, v) in rmap {
                                    if k == old {
                                        nmap.insert(name.to_string(), v);
                                    } else {
                                        nmap.insert(k, v);
                                    }
                                }
                                Data::Map(nmap)
                            })
                            .collect();
                        data = data.set_in(&[PathKey::from("records")], Data::Array(updated_records));
                    }
                    for key in ["primaryKey", "secondaryKeys", "references"] {
                        if let Some(entry) = data.get(key).cloned() {
                            let update_key_obj = |obj: &Data| -> Data {
                                let Some(fields_arr) = obj.get("fields").and_then(|v| v.as_array())
                                else {
                                    return obj.clone();
                                };
                                let next_fields: Vec<Data> = fields_arr
                                    .iter()
                                    .map(|n| {
                                        if n.as_str() == Some(old.as_str()) {
                                            Data::Str(name.to_string())
                                        } else {
                                            n.clone()
                                        }
                                    })
                                    .collect();
                                obj.set_in(&[PathKey::from("fields")], Data::Array(next_fields))
                            };
                            let updated_entry = match &entry {
                                Data::Array(arr) => {
                                    Data::Array(arr.iter().map(update_key_obj).collect())
                                }
                                Data::Map(_) => update_key_obj(&entry),
                                _ => entry,
                            };
                            data = data.set_in(&[PathKey::from(key)], updated_entry);
                        }
                    }
                }
                if !is_custom {
                    if let Some(refs) = data.get("references").and_then(|v| v.as_array()).cloned() {
                        let has_match = refs.iter().any(|r| {
                            r.get("target")
                                .and_then(|t| t.get("table"))
                                .and_then(|v| v.as_str())
                                == Some(target_table_name.as_str())
                                && r.get("target")
                                    .and_then(|t| t.get("fields"))
                                    .and_then(|v| v.as_array())
                                    .map(|arr| arr.iter().any(|n| n.as_str() == Some(old.as_str())))
                                    .unwrap_or(false)
                        });
                        if has_match {
                            let next_refs: Vec<Data> = refs
                                .into_iter()
                                .map(|r| {
                                    if r.get("target")
                                        .and_then(|t| t.get("table"))
                                        .and_then(|v| v.as_str())
                                        == Some(target_table_name.as_str())
                                    {
                                        if let Some(tfields) = r
                                            .get("target")
                                            .and_then(|t| t.get("fields"))
                                            .and_then(|v| v.as_array())
                                        {
                                            let nf: Vec<Data> = tfields
                                                .iter()
                                                .map(|n| {
                                                    if n.as_str() == Some(old.as_str()) {
                                                        Data::Str(name.to_string())
                                                    } else {
                                                        n.clone()
                                                    }
                                                })
                                                .collect();
                                            return r.set_in(
                                                &[PathKey::from("target"), PathKey::from("fields")],
                                                Data::Array(nf),
                                            );
                                        }
                                    }
                                    r
                                })
                                .collect();
                            data = data.set_in(&[PathKey::from("references")], Data::Array(next_refs));
                        }
                    }
                }
                if is_custom {
                    if let Some(records) = data.get("records").and_then(|v| v.as_array()).cloned() {
                        let tname = data.get("table").and_then(|v| v.as_str()).unwrap_or("");
                        if let Some(tbl) = all_tables.iter().find(|t| t.name == tname) {
                            let next_records: Vec<Data> = records
                                .into_iter()
                                .map(|r| {
                                    let Data::Map(rmap) = r else {
                                        return r;
                                    };
                                    let mut nmap = IndexMap::new();
                                    for (k, v) in rmap {
                                        if let Some(fld) = tbl.fields.iter().find(|fl| fl.name == k)
                                        {
                                            let rw = rewrite_custom(
                                                &snap_clone,
                                                &target_type_name,
                                                &old,
                                                name,
                                                &v,
                                                &fld.r#type,
                                                fld,
                                                &HashSet::new(),
                                            );
                                            nmap.insert(k, rw);
                                        } else {
                                            nmap.insert(k, v);
                                        }
                                    }
                                    Data::Map(nmap)
                                })
                                .collect();
                            data =
                                data.set_in(&[PathKey::from("records")], Data::Array(next_records));
                        }
                    }
                }
                let mut nf = f.clone();
                nf.data = data;
                nf
            })
            .collect();
        self.snapshot.sources = next_sources;
        Ok(())
    }

    pub fn rename_declaration(&mut self, id: &str, name: &str) -> Result<(), String> {
        let Some(source) = self.snapshot.sources.iter().find(|s| s.id == id).cloned() else {
            return Err("ソースの形式を先に修正してください。".to_string());
        };
        if source.syntax_error.is_some() || !source.data.is_object() {
            return Err("ソースの形式を先に修正してください。".to_string());
        }
        let is_type = source.data.get("kind").and_then(|v| v.as_str()) == Some("type");
        let key = if is_type { "name" } else { "table" };
        let old = source
            .data
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        for f in &mut self.snapshot.sources {
            let mut data = f.data.clone();
            if !is_type
                && (f.id == id
                    || (!old.is_empty()
                        && data.get("table").and_then(|v| v.as_str()) == Some(old.as_str())))
            {
                data = data.set_in(&[PathKey::from("table")], Data::Str(name.to_string()));
            }
            if is_type {
                if f.id == id {
                    data = data.set_in(&[PathKey::from("name")], Data::Str(name.to_string()));
                }
                let f_fields = fields_of(f);
                if f_fields.iter().any(|fl| fl.r#type == old) {
                    let path = if data.get("custom").is_some() {
                        vec![PathKey::from("custom"), PathKey::from("fields")]
                    } else {
                        vec![PathKey::from("fields")]
                    };
                    let updated_fields: Vec<Data> = f_fields
                        .into_iter()
                        .map(|mut fl| {
                            if fl.r#type == old {
                                fl.r#type = name.to_string();
                            }
                            fl.to_data()
                        })
                        .collect();
                    data = data.set_in(&path, Data::Array(updated_fields));
                }
            }
            if !is_type && !old.is_empty() {
                if let Some(refs) = data.get("references").and_then(|v| v.as_array()).cloned() {
                    if refs.iter().any(|r| {
                        r.get("target")
                            .and_then(|t| t.get("table"))
                            .and_then(|v| v.as_str())
                            == Some(old.as_str())
                    }) {
                        let next_refs: Vec<Data> = refs
                            .into_iter()
                            .map(|r| {
                                if r.get("target")
                                    .and_then(|t| t.get("table"))
                                    .and_then(|v| v.as_str())
                                    == Some(old.as_str())
                                {
                                    r.set_in(
                                        &[PathKey::from("target"), PathKey::from("table")],
                                        Data::Str(name.to_string()),
                                    )
                                } else {
                                    r
                                }
                            })
                            .collect();
                        data = data.set_in(&[PathKey::from("references")], Data::Array(next_refs));
                    }
                }
            }
            f.data = data;
        }
        Ok(())
    }
}

pub fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn normalize_source_path(input: &str) -> String {
    let trimmed = input.trim();
    let without_prefix = trimmed.strip_prefix("sources/").unwrap_or(trimmed);
    let lower = without_prefix.to_ascii_lowercase();
    if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        without_prefix.to_string()
    } else {
        format!("{without_prefix}.yaml")
    }
}

pub fn ensure_source_editable(source: &Source) -> Result<(), String> {
    if source.syntax_error.is_some() || !source.data.is_object() {
        return Err(format!(
            "対象ファイルにYAML構文エラーがあるため変更できません: {}",
            source.path
        ));
    }
    Ok(())
}

pub fn ensure_table_editable(snapshot: &Snapshot, table_name: &str) -> Result<(), String> {
    let escaped = regex::escape(table_name);
    let pattern = Regex::new(&format!(r#"\btable:\s*['"]?{escaped}['"]?\b"#)).unwrap();
    for s in &snapshot.sources {
        if s.syntax_error.is_some() {
            if let Some(raw) = &s.raw {
                if pattern.is_match(raw) {
                    return Err(format!(
                        "テーブル「{table_name}」の関連ファイルにYAML構文エラーがあるため変更できません: {}",
                        s.path
                    ));
                }
            }
        }
        let kind = s.data.get("kind").and_then(|v| v.as_str());
        if matches!(kind, Some("schema") | Some("data"))
            && s.data.get("table").map(|v| v.text()).as_deref() == Some(table_name)
        {
            ensure_source_editable(s)?;
        }
    }
    Ok(())
}

pub fn ensure_token_editable(snapshot: &Snapshot, token: &str) -> Result<(), String> {
    let escaped = regex::escape(token);
    let pattern = Regex::new(&format!(r"\b{escaped}\b")).unwrap();
    for s in &snapshot.sources {
        if s.syntax_error.is_some() {
            if let Some(raw) = &s.raw {
                if pattern.is_match(raw) {
                    return Err(format!(
                        "関連する可能性のあるファイルにYAML構文エラーがあるため一括変更を中止しました: {}",
                        s.path
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn find_table(snapshot: &Snapshot, name: &str) -> Result<TableInfo, String> {
    let table = tables(snapshot)
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("テーブル「{name}」が見つかりません。"))?;
    ensure_source_editable(&table.source)?;
    Ok(table)
}

pub fn find_type(snapshot: &Snapshot, name: &str) -> Result<Source, String> {
    let source =
        type_source(snapshot, name).ok_or_else(|| format!("型「{name}」が見つかりません。"))?;
    ensure_source_editable(&source)?;
    Ok(source)
}

pub fn next_flag_bit(source: &Source) -> Result<i128, String> {
    let def = source.data.get("flags");
    let members = def
        .and_then(|d| d.get("members"))
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    let mut used = HashSet::new();
    for m in &members {
        if let Some(v) = m.get("value").and_then(|v| v.parse_bigint()) {
            used.insert(v);
        }
    }
    let underlying = def
        .and_then(|d| d.get("underlying"))
        .and_then(|u| u.as_str())
        .unwrap_or("uint");
    let width: u32 = if matches!(underlying, "long" | "ulong") {
        64
    } else {
        32
    };
    let signed = matches!(underlying, "int" | "long");
    let max_bit = if signed { width - 1 } else { width };
    for bit in 0..max_bit {
        let candidate = 1i128 << bit;
        if !used.contains(&candidate) {
            return Ok(candidate);
        }
    }
    let name = source.data.get("name").map(|v| v.text()).unwrap_or_default();
    Err(format!("Flags「{name}」の空きビットがありません。"))
}

pub fn next_enum_value(source: &Source) -> i128 {
    let members = source
        .data
        .get("enum")
        .and_then(|d| d.get("members"))
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    if members.is_empty() {
        return 0;
    }
    let mut max = -1i128;
    for m in &members {
        if let Some(v) = m.get("value").and_then(|v| v.parse_bigint()) {
            if v > max {
                max = v;
            }
        }
    }
    max + 1
}

pub fn format_problems(snapshot: &Snapshot, profile: &str) -> Result<Vec<Data>, String> {
    if !profile.is_empty() && !snapshot.config.profiles.contains_key(profile) {
        return Err(format!("プロファイル「{profile}」が見つかりません。"));
    }
    let raw_problems = validate(snapshot, profile);
    let mut by_id = IndexMap::new();
    for s in &snapshot.sources {
        by_id.insert(s.id.clone(), s);
    }
    let all_tables = tables(snapshot);

    let out = raw_problems
        .into_iter()
        .map(|p| {
            let src = by_id.get(&p.source).copied();
            let table_name = src.and_then(|s| {
                let kind = s.data.get("kind").and_then(|v| v.as_str());
                if matches!(kind, Some("schema") | Some("data")) {
                    let t = s.data.get("table").map(|v| v.text()).unwrap_or_default();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t)
                    }
                } else {
                    None
                }
            });
            let mut record_pk: Option<Data> = None;
            if let (Some(tname), Some(s)) = (&table_name, src) {
                if p.path.first() == Some(&PathKey::Key("records".to_string())) {
                    if let Some(PathKey::Index(row_idx)) = p.path.get(1) {
                        if let Some(records) = s.data.get("records").and_then(|v| v.as_array()) {
                            if let Some(row) = records.get(*row_idx) {
                                let pk_fields: Vec<String> = all_tables
                                    .iter()
                                    .find(|t| &t.name == tname)
                                    .and_then(|t| t.source.data.get("primaryKey"))
                                    .and_then(|pk| pk.get("fields"))
                                    .and_then(|f| f.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|x| x.as_str().map(str::to_string))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                if row.is_object() && !pk_fields.is_empty() {
                                    if pk_fields.len() == 1 {
                                        record_pk = row.get(&pk_fields[0]).cloned();
                                    } else {
                                        let mut pk_map = IndexMap::new();
                                        for k in pk_fields {
                                            pk_map.insert(
                                                k.clone(),
                                                row.get(&k).cloned().unwrap_or(Data::Null),
                                            );
                                        }
                                        record_pk = Some(Data::Map(pk_map));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut item = IndexMap::new();
            item.insert(
                "file".to_string(),
                Data::Str(match src {
                    Some(s) => format!("sources/{}", s.path),
                    None => p.source.clone(),
                }),
            );
            item.insert(
                "source".to_string(),
                Data::Str(src.map(|s| s.path.clone()).unwrap_or(p.source.clone())),
            );
            if let Some(tn) = table_name {
                item.insert("table".to_string(), Data::Str(tn));
            }
            if let Some(s) = src {
                if s.data.get("kind").and_then(|v| v.as_str()) == Some("type") {
                    if let Some(tname) = s.data.get("name").and_then(|v| v.as_str()) {
                        if !tname.is_empty() {
                            item.insert("type".to_string(), Data::Str(tname.to_string()));
                        }
                    }
                }
            }
            if let Some(rpk) = record_pk {
                if !matches!(rpk, Data::Undefined) {
                    item.insert("recordPk".to_string(), rpk);
                }
            }
            let path_data = Data::Array(
                p.path
                    .into_iter()
                    .map(|k| match k {
                        PathKey::Key(s) => Data::Str(s),
                        PathKey::Index(i) => Data::Int(i as i128),
                    })
                    .collect(),
            );
            item.insert("path".to_string(), path_data);
            item.insert("code".to_string(), Data::Str(p.code));
            item.insert("message".to_string(), Data::Str(p.message));
            Data::Map(item)
        })
        .collect();
    Ok(out)
}

fn int_limit(prim: &str) -> Option<i128> {
    match prim {
        "int" => Some(2147483647),
        "uint" => Some(4294967295),
        "long" => Some(9223372036854775807),
        "ulong" => Some(18446744073709551615),
        _ => None,
    }
}

fn seed_omitted_primary_keys(
    snapshot: &Snapshot,
    table: &TableInfo,
    rows: &[Data],
    record: &mut IndexMap<String, Data>,
    omitted_fields: &[String],
) {
    for name in omitted_fields {
        let Some(field) = table.fields.iter().find(|f| &f.name == name) else {
            continue;
        };
        if field.array || field.nullable {
            continue;
        }
        let prim = primitive_of(snapshot, field);
        let Some(limit) = int_limit(&prim) else {
            continue;
        };
        let mut max = 0i128;
        for r in rows {
            if let Some(n) = r.get(name).and_then(|v| v.parse_bigint()) {
                if n > max {
                    max = n;
                }
            }
        }
        let candidate = max.saturating_add(1);
        if candidate <= limit {
            record.insert(name.clone(), Data::Int(candidate));
        }
    }
}

pub fn inspect_project(snapshot: &Snapshot, target: Option<&str>) -> Result<Data, String> {
    let all_tables = tables(snapshot);
    let all_types = type_sources(snapshot);
    let problems = format_problems(snapshot, "")?;
    let syntax_errors: Vec<Data> = snapshot
        .sources
        .iter()
        .filter_map(|s| {
            s.syntax_error.as_ref().map(|msg| {
                let mut m = IndexMap::new();
                m.insert("path".to_string(), Data::Str(s.path.clone()));
                m.insert("message".to_string(), Data::Str(msg.clone()));
                Data::Map(m)
            })
        })
        .collect();

    let summarize_table = |t: &TableInfo| -> IndexMap<String, Data> {
        let pk_fields: Vec<String> = t
            .source
            .data
            .get("primaryKey")
            .and_then(|pk| pk.get("fields"))
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let rows: Vec<Data> = t
            .sources
            .iter()
            .flat_map(|s| {
                s.data
                    .get("records")
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();
        let mut template = IndexMap::new();
        for f in &t.fields {
            template.insert(f.name.clone(), default_value(snapshot, f, &HashSet::new()));
        }
        seed_omitted_primary_keys(snapshot, t, &rows, &mut template, &pk_fields);
        let mut allocator = RecordAllocator::new(snapshot, t);
        let next_primary_key = match allocator.allocate(Some(&Data::Map(template))) {
            Ok(candidate) => {
                if pk_fields.len() == 1 {
                    candidate.get(&pk_fields[0]).cloned().unwrap_or(Data::Null)
                } else if pk_fields.len() > 1 {
                    let mut m = IndexMap::new();
                    for k in &pk_fields {
                        m.insert(k.clone(), candidate.get(k).cloned().unwrap_or(Data::Null));
                    }
                    Data::Map(m)
                } else {
                    Data::Null
                }
            }
            Err(_) => Data::Null,
        };

        let mut total_records = 0i128;
        let data_files: Vec<Data> = t
            .sources
            .iter()
            .map(|s| {
                let count = s
                    .data
                    .get("records")
                    .and_then(|r| r.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0) as i128;
                total_records += count;
                let mut df = IndexMap::new();
                df.insert("path".to_string(), Data::Str(s.path.clone()));
                df.insert(
                    "kind".to_string(),
                    Data::Str(
                        s.data
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or("data")
                            .to_string(),
                    ),
                );
                df.insert("recordCount".to_string(), Data::Int(count));
                Data::Map(df)
            })
            .collect();

        let fields_data: Vec<Data> = t
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut fm = IndexMap::new();
                fm.insert("name".to_string(), Data::Str(f.name.clone()));
                fm.insert("type".to_string(), Data::Str(f.r#type.clone()));
                fm.insert("key".to_string(), Data::Int(f.key.unwrap_or(i as i64) as i128));
                fm.insert("array".to_string(), Data::Bool(f.array));
                fm.insert("nullable".to_string(), Data::Bool(f.nullable));
                Data::Map(fm)
            })
            .collect();

        let secondary_keys: Vec<Data> = t
            .source
            .data
            .get("secondaryKeys")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|sk| {
                        let mut skm = IndexMap::new();
                        let sfields = sk
                            .get("fields")
                            .and_then(|f| f.as_array())
                            .cloned()
                            .unwrap_or_default();
                        skm.insert("fields".to_string(), Data::Array(sfields));
                        skm.insert(
                            "nonUnique".to_string(),
                            Data::Bool(
                                sk.get("nonUnique")
                                    .and_then(|b| b.as_bool())
                                    .unwrap_or(false),
                            ),
                        );
                        Data::Map(skm)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let references: Vec<Data> = t
            .source
            .data
            .get("references")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|r| {
                        let mut rm = IndexMap::new();
                        rm.insert(
                            "name".to_string(),
                            r.get("name").cloned().unwrap_or(Data::Null),
                        );
                        rm.insert(
                            "csharpName".to_string(),
                            r.get("csharpName").cloned().unwrap_or(Data::Null),
                        );
                        rm.insert(
                            "fields".to_string(),
                            Data::Array(
                                r.get("fields")
                                    .and_then(|f| f.as_array())
                                    .cloned()
                                    .unwrap_or_default(),
                            ),
                        );
                        let mut target_m = IndexMap::new();
                        target_m.insert(
                            "table".to_string(),
                            r.get("target")
                                .and_then(|tg| tg.get("table"))
                                .cloned()
                                .unwrap_or(Data::Null),
                        );
                        target_m.insert(
                            "fields".to_string(),
                            Data::Array(
                                r.get("target")
                                    .and_then(|tg| tg.get("fields"))
                                    .and_then(|f| f.as_array())
                                    .cloned()
                                    .unwrap_or_default(),
                            ),
                        );
                        rm.insert("target".to_string(), Data::Map(target_m));
                        Data::Map(rm)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut tm = IndexMap::new();
        tm.insert("name".to_string(), Data::Str(t.name.clone()));
        tm.insert(
            "csharpName".to_string(),
            t.source
                .data
                .get("csharpName")
                .cloned()
                .unwrap_or(Data::Null),
        );
        tm.insert("path".to_string(), Data::Str(t.source.path.clone()));
        tm.insert("fields".to_string(), Data::Array(fields_data));
        tm.insert(
            "primaryKey".to_string(),
            Data::Array(pk_fields.into_iter().map(Data::Str).collect()),
        );
        tm.insert("secondaryKeys".to_string(), Data::Array(secondary_keys));
        tm.insert("references".to_string(), Data::Array(references));
        tm.insert("recordCount".to_string(), Data::Int(total_records));
        tm.insert("dataFiles".to_string(), Data::Array(data_files));
        tm.insert(
            "requiresExplicitSourceOnAdd".to_string(),
            Data::Bool(t.sources.len() > 1),
        );
        tm.insert("nextPrimaryKey".to_string(), next_primary_key);
        tm
    };

    let summarize_type = |s: &Source| -> IndexMap<String, Data> {
        let kind = type_kind(Some(s)).unwrap_or("unknown");
        let def = s.data.get(kind);
        let mut tm = IndexMap::new();
        tm.insert(
            "name".to_string(),
            Data::Str(
                s.data
                    .get("name")
                    .map(|v| v.text())
                    .unwrap_or_default(),
            ),
        );
        tm.insert("kind".to_string(), Data::Str(kind.to_string()));
        tm.insert("path".to_string(), Data::Str(s.path.clone()));
        if let Some(u) = def.and_then(|d| d.get("underlying")).and_then(|v| v.as_str()) {
            tm.insert("underlying".to_string(), Data::Str(u.to_string()));
        }
        if matches!(kind, "enum" | "flags") {
            let members: Vec<Data> = def
                .and_then(|d| d.get("members"))
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|m| {
                            let mut mm = IndexMap::new();
                            mm.insert(
                                "name".to_string(),
                                m.get("name").cloned().unwrap_or(Data::Null),
                            );
                            mm.insert(
                                "value".to_string(),
                                m.get("value").cloned().unwrap_or(Data::Null),
                            );
                            Data::Map(mm)
                        })
                        .collect()
                })
                .unwrap_or_default();
            tm.insert("members".to_string(), Data::Array(members));
        }
        if kind == "flags" {
            let next_val = next_flag_bit(s).map(Data::Int).unwrap_or(Data::Null);
            tm.insert("nextFlagValue".to_string(), next_val);
        }
        if kind == "custom" {
            let fields_data: Vec<Data> = fields_of(s)
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let mut fm = IndexMap::new();
                    fm.insert("name".to_string(), Data::Str(f.name.clone()));
                    fm.insert("type".to_string(), Data::Str(f.r#type.clone()));
                    fm.insert(
                        "key".to_string(),
                        Data::Int(f.key.unwrap_or(i as i64) as i128),
                    );
                    fm.insert("array".to_string(), Data::Bool(f.array));
                    fm.insert("nullable".to_string(), Data::Bool(f.nullable));
                    Data::Map(fm)
                })
                .collect();
            tm.insert("fields".to_string(), Data::Array(fields_data));
        }
        tm
    };

    if let Some(target_str) = target {
        let normalized = target_str.strip_prefix("sources/").unwrap_or(target_str);
        if let Some(table) = all_tables
            .iter()
            .find(|t| t.name == target_str || t.source.path == normalized)
        {
            let pk_fields: Vec<String> = table
                .source
                .data
                .get("primaryKey")
                .and_then(|pk| pk.get("fields"))
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let rows: Vec<Data> = table
                .sources
                .iter()
                .flat_map(|s| {
                    s.data
                        .get("records")
                        .and_then(|r| r.as_array())
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();
            let pk_values: Vec<Data> = rows
                .iter()
                .take(200)
                .map(|r| {
                    if pk_fields.len() == 1 {
                        r.get(&pk_fields[0]).cloned().unwrap_or(Data::Null)
                    } else {
                        let mut m = IndexMap::new();
                        for k in &pk_fields {
                            m.insert(k.clone(), r.get(k).cloned().unwrap_or(Data::Null));
                        }
                        Data::Map(m)
                    }
                })
                .collect();
            let referenced_by: Vec<Data> = all_tables
                .iter()
                .flat_map(|other| {
                    other
                        .source
                        .data
                        .get("references")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|r| {
                            r.get("target")
                                .and_then(|tg| tg.get("table"))
                                .and_then(|v| v.as_str())
                                == Some(table.name.as_str())
                        })
                        .map(|r| {
                            let mut rm = IndexMap::new();
                            rm.insert("table".to_string(), Data::Str(other.name.clone()));
                            rm.insert(
                                "reference".to_string(),
                                r.get("name").cloned().unwrap_or(Data::Null),
                            );
                            rm.insert(
                                "fields".to_string(),
                                Data::Array(
                                    r.get("fields")
                                        .and_then(|f| f.as_array())
                                        .cloned()
                                        .unwrap_or_default(),
                                ),
                            );
                            rm.insert(
                                "targetFields".to_string(),
                                Data::Array(
                                    r.get("target")
                                        .and_then(|tg| tg.get("fields"))
                                        .and_then(|f| f.as_array())
                                        .cloned()
                                        .unwrap_or_default(),
                                ),
                            );
                            Data::Map(rm)
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            let mut out = IndexMap::new();
            out.insert("kind".to_string(), Data::Str("table".to_string()));
            for (k, v) in summarize_table(table) {
                out.insert(k, v);
            }
            out.insert("referencedBy".to_string(), Data::Array(referenced_by));
            out.insert("primaryKeyValues".to_string(), Data::Array(pk_values));
            let table_problems: Vec<Data> = problems
                .iter()
                .filter(|p| p.get("table").and_then(|v| v.as_str()) == Some(table.name.as_str()))
                .cloned()
                .collect();
            out.insert("problems".to_string(), Data::Array(table_problems));
            out.insert(
                "totalProjectProblems".to_string(),
                Data::Int(problems.len() as i128),
            );
            return Ok(Data::Map(out));
        }

        if let Some(ty) = all_types.iter().find(|s| {
            s.data.get("name").map(|v| v.text()).as_deref() == Some(target_str)
                || s.path == normalized
        }) {
            let type_name = ty.data.get("name").map(|v| v.text()).unwrap_or_default();
            let used_by_tables: Vec<Data> = all_tables
                .iter()
                .filter(|t| t.fields.iter().any(|f| f.r#type == type_name))
                .map(|t| {
                    let mut m = IndexMap::new();
                    m.insert("table".to_string(), Data::Str(t.name.clone()));
                    m.insert(
                        "fields".to_string(),
                        Data::Array(
                            t.fields
                                .iter()
                                .filter(|f| f.r#type == type_name)
                                .map(|f| Data::Str(f.name.clone()))
                                .collect(),
                        ),
                    );
                    Data::Map(m)
                })
                .collect();
            let used_by_types: Vec<Data> = all_types
                .iter()
                .filter(|s| {
                    s.data.get("custom").is_some()
                        && fields_of(s).iter().any(|f| f.r#type == type_name)
                })
                .map(|s| {
                    let mut m = IndexMap::new();
                    m.insert(
                        "type".to_string(),
                        Data::Str(s.data.get("name").map(|v| v.text()).unwrap_or_default()),
                    );
                    m.insert(
                        "fields".to_string(),
                        Data::Array(
                            fields_of(s)
                                .into_iter()
                                .filter(|f| f.r#type == type_name)
                                .map(|f| Data::Str(f.name))
                                .collect(),
                        ),
                    );
                    Data::Map(m)
                })
                .collect();
            let mut out = summarize_type(ty);
            out.insert("usedByTables".to_string(), Data::Array(used_by_tables));
            out.insert("usedByTypes".to_string(), Data::Array(used_by_types));
            let type_problems: Vec<Data> = problems
                .iter()
                .filter(|p| {
                    p.get("type").and_then(|v| v.as_str()) == Some(type_name.as_str())
                        || p.get("source").and_then(|v| v.as_str()) == Some(ty.path.as_str())
                })
                .cloned()
                .collect();
            out.insert("problems".to_string(), Data::Array(type_problems));
            out.insert(
                "totalProjectProblems".to_string(),
                Data::Int(problems.len() as i128),
            );
            return Ok(Data::Map(out));
        }
        return Err(format!(
            "指定されたテーブルまたは型「{target_str}」が見つかりません。"
        ));
    }

    let mut out = IndexMap::new();
    out.insert(
        "config".to_string(),
        crate::wire::config_to_data(&snapshot.config),
    );
    out.insert(
        "tables".to_string(),
        Data::Array(
            all_tables
                .iter()
                .map(|t| Data::Map(summarize_table(t)))
                .collect(),
        ),
    );
    out.insert(
        "types".to_string(),
        Data::Array(
            all_types
                .iter()
                .map(|s| Data::Map(summarize_type(s)))
                .collect(),
        ),
    );
    let mut prob_summary = IndexMap::new();
    prob_summary.insert("total".to_string(), Data::Int(problems.len() as i128));
    prob_summary.insert(
        "syntaxErrorCount".to_string(),
        Data::Int(syntax_errors.len() as i128),
    );
    prob_summary.insert("syntaxErrors".to_string(), Data::Array(syntax_errors));
    out.insert("problems".to_string(), Data::Map(prob_summary));
    Ok(Data::Map(out))
}

pub fn parse_field_path(raw: &str) -> Result<Vec<PathStep>, String> {
    let mut steps = Vec::new();
    let re = Regex::new(r"([^.\[\]]+)|\[(\d+|\+)\]").unwrap();
    for cap in re.captures_iter(raw) {
        if let Some(m) = cap.get(1) {
            steps.push(PathStep::Key(m.as_str().to_string()));
        } else if let Some(m) = cap.get(2) {
            if m.as_str() == "+" {
                steps.push(PathStep::Append);
            } else {
                let idx = m
                    .as_str()
                    .parse::<usize>()
                    .map_err(|_| format!("フィールドパスが不正です: {raw}"))?;
                steps.push(PathStep::Index(idx));
            }
        }
    }
    let normalized = raw.replace('.', "");
    let check: String = steps
        .iter()
        .map(|s| match s {
            PathStep::Key(k) => k.clone(),
            PathStep::Index(i) => format!("[{i}]"),
            PathStep::Append => "[+]".to_string(),
        })
        .collect();
    if steps.is_empty() || normalized != check {
        return Err(format!("フィールドパスが不正です: {raw}"));
    }
    Ok(steps)
}

pub fn resolve_field_at_path(
    snapshot: &Snapshot,
    table_fields: &[Field],
    steps: &[PathStep],
) -> Option<Field> {
    let mut current: Option<Field> = None;
    let mut fields: Option<Vec<Field>> = Some(table_fields.to_vec());
    for step in steps {
        match step {
            PathStep::Index(_) | PathStep::Append => {
                let cur = current.as_ref()?;
                if !cur.array {
                    return None;
                }
                let mut item_field = cur.clone();
                item_field.array = false;
                item_field.nullable = false;
                let ty = type_source(snapshot, &item_field.r#type);
                fields = if ty.as_ref().and_then(|s| s.data.get("custom")).is_some() {
                    ty.as_ref().map(fields_of)
                } else {
                    None
                };
                current = Some(item_field);
            }
            PathStep::Key(k) => {
                let flist = fields.as_ref()?;
                let found = flist.iter().find(|f| &f.name == k)?.clone();
                if !found.array {
                    let ty = type_source(snapshot, &found.r#type);
                    fields = if ty.as_ref().and_then(|s| s.data.get("custom")).is_some() {
                        ty.as_ref().map(fields_of)
                    } else {
                        None
                    };
                } else {
                    fields = None;
                }
                current = Some(found);
            }
        }
    }
    current
}

pub fn coerce_scalar_or_structured(
    snapshot: &Snapshot,
    field: Option<&Field>,
    raw: &Data,
) -> Result<Data, String> {
    let Data::Str(raw_str) = raw else {
        if let Data::Float(f) = raw {
            if f.is_finite() && f.fract() == 0.0 {
                if let Some(fld) = field {
                    let prim = primitive_of(snapshot, fld);
                    if matches!(prim.as_str(), "int" | "uint" | "long" | "ulong") {
                        return Ok(Data::Int(*f as i128));
                    }
                }
            }
        }
        if let Data::Array(arr) = raw {
            if let Some(fld) = field {
                if !fld.array {
                    let ty = type_source(snapshot, &fld.r#type);
                    if type_kind(ty.as_ref()) == Some("flags") {
                        let names: Vec<String> = arr.iter().map(|x| x.text()).collect();
                        return Ok(Data::Array(
                            flags_change(&[], &names).into_iter().map(Data::Str).collect(),
                        ));
                    }
                } else {
                    let mut item_field = fld.clone();
                    item_field.array = false;
                    item_field.nullable = false;
                    let mut out = Vec::with_capacity(arr.len());
                    for item in arr {
                        out.push(coerce_scalar_or_structured(
                            snapshot,
                            Some(&item_field),
                            item,
                        )?);
                    }
                    return Ok(Data::Array(out));
                }
            }
        }
        if let Data::Map(map) = raw {
            if let Some(fld) = field {
                if !fld.array {
                    let ty = type_source(snapshot, &fld.r#type);
                    if let Some(ty_src) = ty.as_ref() {
                        if ty_src.data.get("custom").is_some() {
                            let sub_fields = fields_of(ty_src);
                            let mut next = map.clone();
                            for sf in &sub_fields {
                                if let Some(existing) = next.get(&sf.name).cloned() {
                                    let coerced = coerce_scalar_or_structured(
                                        snapshot,
                                        Some(sf),
                                        &existing,
                                    )?;
                                    next.insert(sf.name.clone(), coerced);
                                }
                            }
                            return Ok(Data::Map(next));
                        }
                    }
                }
            }
        }
        return Ok(raw.clone());
    };

    let trimmed = raw_str.trim();
    if let Some(fld) = field {
        if fld.nullable && trimmed == "null" {
            return Ok(Data::Null);
        }
        if fld.array {
            let mut item_field = fld.clone();
            item_field.array = false;
            item_field.nullable = false;
            if trimmed.starts_with('[') {
                let parsed = parse_structured_data(trimmed)?;
                let Data::Array(arr) = parsed else {
                    return Err(format!("配列形式で指定してください: {raw_str}"));
                };
                let mut out = Vec::with_capacity(arr.len());
                for item in &arr {
                    out.push(coerce_scalar_or_structured(
                        snapshot,
                        Some(&item_field),
                        item,
                    )?);
                }
                return Ok(Data::Array(out));
            }
            if trimmed.is_empty() {
                return Ok(Data::Array(Vec::new()));
            }
            let items = split_csv(trimmed);
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(coerce_scalar_or_structured(
                    snapshot,
                    Some(&item_field),
                    &Data::Str(item),
                )?);
            }
            return Ok(Data::Array(out));
        }

        let ty = type_source(snapshot, &fld.r#type);
        let kind = type_kind(ty.as_ref());
        let prim = primitive_of(snapshot, fld);
        if prim == "string" || kind == Some("enum") {
            if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            {
                let parsed = parse_structured_data(trimmed)?;
                return Ok(Data::Str(parsed.text()));
            }
            return Ok(Data::Str(raw_str.clone()));
        }
        if prim == "bool" {
            if trimmed == "true" {
                return Ok(Data::Bool(true));
            }
            if trimmed == "false" {
                return Ok(Data::Bool(false));
            }
            return Err(format!("真偽値（true / false）を指定してください: {raw_str}"));
        }
        if matches!(prim.as_str(), "int" | "uint" | "long" | "ulong") {
            let n = trimmed
                .parse::<i128>()
                .map_err(|_| format!("整数を指定してください: {raw_str}"))?;
            return Ok(Data::Int(n));
        }
        if matches!(prim.as_str(), "float" | "double") {
            let f = trimmed
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .ok_or_else(|| format!("数値を指定してください: {raw_str}"))?;
            return Ok(Data::Float(f));
        }
        if kind == Some("flags") {
            if trimmed.starts_with('[') {
                let parsed = parse_structured_data(trimmed)?;
                let Data::Array(arr) = parsed else {
                    return Err(format!(
                        "Flagsは配列またはカンマ区切りで指定してください: {raw_str}"
                    ));
                };
                let names: Vec<String> = arr.iter().map(|x| x.text()).collect();
                return Ok(Data::Array(
                    flags_change(&[], &names).into_iter().map(Data::Str).collect(),
                ));
            }
            let names = split_csv(trimmed);
            return Ok(Data::Array(
                flags_change(&[], &names).into_iter().map(Data::Str).collect(),
            ));
        }
        if kind == Some("custom") {
            let parsed = parse_structured_data(trimmed)?;
            if !parsed.is_object() {
                return Err(format!(
                    "構造体はJSON/YAMLオブジェクトで指定してください: {raw_str}"
                ));
            }
            return coerce_scalar_or_structured(snapshot, Some(fld), &parsed);
        }
    }

    if trimmed == "null" {
        return Ok(Data::Null);
    }
    if trimmed == "true" {
        return Ok(Data::Bool(true));
    }
    if trimmed == "false" {
        return Ok(Data::Bool(false));
    }
    if let Ok(n) = trimmed.parse::<i128>() {
        return Ok(Data::Int(n));
    }
    if trimmed.contains('.') {
        if let Ok(f) = trimmed.parse::<f64>() {
            if f.is_finite() {
                return Ok(Data::Float(f));
            }
        }
    }
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return parse_structured_data(trimmed);
    }
    Ok(Data::Str(raw_str.clone()))
}

pub fn set_by_path(root: &Data, steps: &[PathStep], value: Data) -> Result<Data, String> {
    if steps.is_empty() {
        return Ok(value);
    }
    let head = &steps[0];
    let tail = &steps[1..];
    match head {
        PathStep::Append => {
            let mut arr = match root {
                Data::Array(a) => a.clone(),
                _ => Vec::new(),
            };
            arr.push(set_by_path(&Data::Undefined, tail, value)?);
            Ok(Data::Array(arr))
        }
        PathStep::Index(idx) => {
            let mut arr = match root {
                Data::Array(a) => a.clone(),
                _ => Vec::new(),
            };
            if *idx > arr.len() {
                return Err(format!(
                    "配列インデックス [{idx}] が範囲外です（長さ {}）。",
                    arr.len()
                ));
            }
            let current = arr.get(*idx).cloned().unwrap_or(Data::Undefined);
            let updated = set_by_path(&current, tail, value)?;
            if *idx == arr.len() {
                arr.push(updated);
            } else {
                arr[*idx] = updated;
            }
            Ok(Data::Array(arr))
        }
        PathStep::Key(k) => {
            let mut map = match root {
                Data::Map(m) => m.clone(),
                _ => IndexMap::new(),
            };
            let current = map.get(k).cloned().unwrap_or(Data::Undefined);
            let updated = set_by_path(&current, tail, value)?;
            map.insert(k.clone(), updated);
            Ok(Data::Map(map))
        }
    }
}

pub fn get_by_path<'a>(root: &'a Data, steps: &[PathStep]) -> Option<&'a Data> {
    let mut cur = root;
    for step in steps {
        match step {
            PathStep::Append => return None,
            PathStep::Index(i) => {
                cur = cur.get_index(*i)?;
            }
            PathStep::Key(k) => {
                cur = cur.get(k)?;
            }
        }
    }
    Some(cur)
}

pub fn parse_assignments(
    snapshot: &Snapshot,
    table_fields: &[Field],
    tokens: &[String],
) -> Result<Vec<(Vec<PathStep>, Data)>, String> {
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        let Some(eq) = token.find('=') else {
            return Err(format!("\"field=value\" の形式で指定してください: {token}"));
        };
        if eq == 0 {
            return Err(format!("\"field=value\" の形式で指定してください: {token}"));
        }
        let raw_path = token[..eq].trim();
        let raw_value = &token[eq + 1..];
        let steps = parse_field_path(raw_path)?;
        if steps.first() == Some(&PathStep::Key("$tags".to_string())) {
            let tags = if raw_value.trim().starts_with('[') {
                let parsed = parse_structured_data(raw_value.trim())?;
                parsed
                    .as_array()
                    .map(|arr| arr.iter().map(|x| Data::Str(x.text())).collect())
                    .unwrap_or_default()
            } else {
                split_csv(raw_value).into_iter().map(Data::Str).collect()
            };
            out.push((steps, Data::Array(tags)));
            continue;
        }
        let target_field = resolve_field_at_path(snapshot, table_fields, &steps);
        let coerced = coerce_scalar_or_structured(
            snapshot,
            target_field.as_ref(),
            &Data::Str(raw_value.to_string()),
        )?;
        out.push((steps, coerced));
    }
    Ok(out)
}

pub fn parse_pk_selector(
    snapshot: &Snapshot,
    table_fields: &[Field],
    pk_fields: &[String],
    raw_pk: &Data,
) -> Result<IndexMap<String, String>, String> {
    if pk_fields.is_empty() {
        return Err("テーブルに主キーが設定されていません。".to_string());
    }
    if let Data::Map(map) = raw_pk {
        let mut out = IndexMap::new();
        for name in pk_fields {
            let Some(val) = map.get(name) else {
                return Err(format!("複合主キーの項目「{name}」を指定してください。"));
            };
            out.insert(name.clone(), val.text());
        }
        return Ok(out);
    }
    let s = raw_pk.text();
    let trimmed = s.trim();
    if trimmed.starts_with('{') {
        let obj = parse_structured_data(trimmed)?;
        return parse_pk_selector(snapshot, table_fields, pk_fields, &obj);
    }
    if pk_fields.len() == 1 && !trimmed.contains('=') {
        let field = table_fields.iter().find(|f| f.name == pk_fields[0]);
        let coerced =
            coerce_scalar_or_structured(snapshot, field, &Data::Str(trimmed.to_string()))?;
        let mut out = IndexMap::new();
        out.insert(pk_fields[0].clone(), coerced.text());
        return Ok(out);
    }
    let parts = split_csv(trimmed);
    let mut map = IndexMap::new();
    for part in parts {
        let Some(eq) = part.find('=') else {
            return Err(format!(
                "複合主キーは \"key1=val1,key2=val2\" またはJSONで指定してください: {trimmed}"
            ));
        };
        if eq == 0 {
            return Err(format!(
                "複合主キーは \"key1=val1,key2=val2\" またはJSONで指定してください: {trimmed}"
            ));
        }
        let k = part[..eq].trim();
        let v = part[eq + 1..].trim();
        let field = table_fields.iter().find(|f| f.name == k);
        let coerced = coerce_scalar_or_structured(snapshot, field, &Data::Str(v.to_string()))?;
        map.insert(k.to_string(), coerced.text());
    }
    for name in pk_fields {
        if !map.contains_key(name) {
            return Err(format!("複合主キーの項目「{name}」が不足しています。"));
        }
    }
    Ok(map)
}

pub fn matches_pk(row: &Data, pk_fields: &[String], expected: &IndexMap<String, String>) -> bool {
    let Data::Map(map) = row else {
        return false;
    };
    pk_fields.iter().all(|name| {
        let actual = map.get(name).map(|v| v.text()).unwrap_or_default();
        expected.get(name) == Some(&actual)
    })
}

fn compare_scalar(actual: Option<&Data>, op: &str, rhs_raw: &str) -> Result<bool, String> {
    if op == "in" {
        let candidates = split_csv(rhs_raw);
        let Some(act) = actual else {
            return Ok(false);
        };
        if let Data::Array(arr) = act {
            return Ok(arr.iter().any(|item| candidates.contains(&item.text())));
        }
        return Ok(candidates.contains(&act.text()));
    }
    if op == "~=" {
        let re = Regex::new(rhs_raw).map_err(|e| format!("正規表現が不正です: {e}"))?;
        let Some(act) = actual else {
            return Ok(false);
        };
        if let Data::Array(arr) = act {
            return Ok(arr.iter().any(|item| re.is_match(&item.text())));
        }
        return Ok(re.is_match(&act.text()));
    }
    if let Some(Data::Array(arr)) = actual {
        if op == "=" || op == "==" {
            return Ok(arr.iter().any(|x| x.text() == rhs_raw)
                || Data::Array(arr.clone()).to_json_value().to_string() == rhs_raw);
        }
        if op == "!=" {
            return Ok(!arr.iter().any(|x| x.text() == rhs_raw));
        }
    }
    let Some(act) = actual else {
        return Ok(match op {
            "=" | "==" => rhs_raw == "null",
            "!=" => rhs_raw != "null",
            _ => false,
        });
    };
    if act.is_null_or_undefined() {
        return Ok(match op {
            "=" | "==" => rhs_raw == "null",
            "!=" => rhs_raw != "null",
            _ => false,
        });
    }
    if let Some(left_num) = act.as_i128() {
        if let Ok(right_num) = rhs_raw.trim().parse::<i128>() {
            return Ok(match op {
                "=" | "==" => left_num == right_num,
                "!=" => left_num != right_num,
                ">" => left_num > right_num,
                ">=" => left_num >= right_num,
                "<" => left_num < right_num,
                "<=" => left_num <= right_num,
                _ => false,
            });
        }
    }
    if let Data::Float(left_f) = act {
        if let Ok(right_f) = rhs_raw.trim().parse::<f64>() {
            return Ok(match op {
                "=" | "==" => *left_f == right_f,
                "!=" => *left_f != right_f,
                ">" => *left_f > right_f,
                ">=" => *left_f >= right_f,
                "<" => *left_f < right_f,
                "<=" => *left_f <= right_f,
                _ => false,
            });
        }
    }
    let left_str = act.text();
    let right_str = rhs_raw
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string();
    Ok(match op {
        "=" | "==" => left_str == right_str,
        "!=" => left_str != right_str,
        ">" => left_str > right_str,
        ">=" => left_str >= right_str,
        "<" => left_str < right_str,
        "<=" => left_str <= right_str,
        _ => false,
    })
}

pub fn matches_where(row: &Data, expressions: &[String]) -> Result<bool, String> {
    if !row.is_object() {
        return Ok(false);
    }
    let re =
        Regex::new(r"^([A-Za-z0-9_$.\[\]]+)\s*(!=|>=|<=|~=|==|=|>|<|\s+in\s+)\s*(.*)$").unwrap();
    for expr in expressions {
        let Some(caps) = re.captures(expr) else {
            return Err(format!(
                "--where 条件の形式が不正です（例: \"rarity=SSR\", \"price>=100\"）: {expr}"
            ));
        };
        let raw_path = caps.get(1).unwrap().as_str().trim();
        let op = caps.get(2).unwrap().as_str().trim();
        let rhs = caps.get(3).unwrap().as_str().trim();
        let steps = parse_field_path(raw_path)?;
        let actual = get_by_path(row, &steps);
        if !compare_scalar(actual, op, rhs)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub struct QueryOptions {
    pub pk: Option<Data>,
    pub r#where: Vec<String>,
    pub source: Option<String>,
    pub profile: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub fn query_records(
    snapshot: &Snapshot,
    table_name: &str,
    options: &QueryOptions,
) -> Result<Data, String> {
    let active_snapshot = if let Some(prof) = &options.profile {
        if !prof.is_empty() {
            if !snapshot.config.profiles.contains_key(prof) {
                return Err(format!("プロファイル「{prof}」が見つかりません。"));
            }
            in_profile(snapshot, prof)
        } else {
            snapshot.clone()
        }
    } else {
        snapshot.clone()
    };
    let table = find_table(&active_snapshot, table_name)?;
    let pk_fields: Vec<String> = table
        .source
        .data
        .get("primaryKey")
        .and_then(|pk| pk.get("fields"))
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let pk_expected = if let Some(pk) = &options.pk {
        Some(parse_pk_selector(
            &active_snapshot,
            &table.fields,
            &pk_fields,
            pk,
        )?)
    } else {
        None
    };
    let source_filter = options.source.as_deref().map(normalize_source_path);
    if let Some(sf) = &source_filter {
        if !table.sources.iter().any(|s| &s.path == sf) {
            return Err(format!(
                "テーブル「{table_name}」のデータファイル「{sf}」が見つかりません。"
            ));
        }
    }

    let mut matched = Vec::new();
    for src in &table.sources {
        if let Some(sf) = &source_filter {
            if &src.path != sf {
                continue;
            }
        }
        ensure_source_editable(src)?;
        let records = src
            .data
            .get("records")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        for (index, record) in records.into_iter().enumerate() {
            if let Some(expected) = &pk_expected {
                if !matches_pk(&record, &pk_fields, expected) {
                    continue;
                }
            }
            if !options.r#where.is_empty() && !matches_where(&record, &options.r#where)? {
                continue;
            }
            let mut item = IndexMap::new();
            item.insert("source".to_string(), Data::Str(src.path.clone()));
            item.insert("index".to_string(), Data::Int(index as i128));
            item.insert("record".to_string(), record);
            matched.push(Data::Map(item));
        }
    }

    let total = matched.len();
    let offset = options.offset.unwrap_or(0);
    let limit = options.limit.unwrap_or(total);
    let sliced: Vec<Data> = matched.into_iter().skip(offset).take(limit).collect();

    let mut out = IndexMap::new();
    out.insert("table".to_string(), Data::Str(table_name.to_string()));
    out.insert("totalMatched".to_string(), Data::Int(total as i128));
    out.insert("records".to_string(), Data::Array(sliced));
    Ok(Data::Map(out))
}

fn next_field_key(fields: &[Field]) -> i64 {
    fields
        .iter()
        .enumerate()
        .map(|(i, f)| f.key.unwrap_or(i as i64))
        .max()
        .unwrap_or(-1)
        + 1
}

fn ensure_unique_source_path(snapshot: &Snapshot, path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || p.chars().any(|c| c == '\\' || (c as u32) <= 0x1f)
        })
    {
        return Err(format!("ファイルパスが不正です: {path}"));
    }
    if snapshot.sources.iter().any(|s| s.path == path) {
        return Err(format!("ファイル「sources/{path}」は既に存在します。"));
    }
    Ok(())
}

fn add_folders_for_path(folders: &[String], rel_path: &str) -> Vec<String> {
    let mut set: HashSet<String> = folders.iter().cloned().collect();
    let parts: Vec<&str> = rel_path.split('/').collect();
    if parts.len() > 1 {
        for i in 1..parts.len() {
            set.insert(parts[..i].join("/"));
        }
    }
    let mut out: Vec<String> = set.into_iter().collect();
    out.sort();
    out
}

fn backfill_defined_fields(
    snapshot: &Snapshot,
    fields: &[Field],
    value: &Data,
    seen: &HashSet<String>,
) -> Data {
    let Data::Map(map) = value else {
        return value.clone();
    };
    let mut changed = false;
    let mut next = map.clone();
    for field in fields {
        if !next.contains_key(&field.name) {
            next.insert(
                field.name.clone(),
                default_value(snapshot, field, &HashSet::new()),
            );
            changed = true;
            continue;
        }
        if let Some(ty) = type_source(snapshot, &field.r#type) {
            if ty.data.get("custom").is_some() && !seen.contains(&field.r#type) {
                let mut next_seen = seen.clone();
                next_seen.insert(field.r#type.clone());
                let custom_fields = fields_of(&ty);
                let existing = next.get(&field.name).unwrap().clone();
                if field.array {
                    if let Data::Array(arr) = &existing {
                        let mapped: Vec<Data> = arr
                            .iter()
                            .map(|item| {
                                backfill_defined_fields(snapshot, &custom_fields, item, &next_seen)
                            })
                            .collect();
                        if &mapped != arr {
                            next.insert(field.name.clone(), Data::Array(mapped));
                            changed = true;
                        }
                    }
                } else if existing.is_object() {
                    let updated =
                        backfill_defined_fields(snapshot, &custom_fields, &existing, &next_seen);
                    if updated != existing {
                        next.insert(field.name.clone(), updated);
                        changed = true;
                    }
                }
            }
        }
    }
    if changed {
        Data::Map(next)
    } else {
        value.clone()
    }
}

fn strings_from_data(val: Option<&Data>) -> Vec<String> {
    match val {
        Some(Data::Array(arr)) => arr.iter().map(|x| x.text()).collect(),
        Some(Data::Str(s)) => split_csv(s),
        _ => Vec::new(),
    }
}

pub fn apply_operation(session: &mut EditSession, op: &Data) -> Result<Data, String> {
    let snapshot = session.snapshot.clone();
    let kind = op.get("op").and_then(|v| v.as_str()).unwrap_or("");

    match kind {
        "config.set" => {
            let mut next = snapshot.config.clone();
            if let Some(name_val) = op.get("name") {
                if !name_val.is_null_or_undefined() {
                    let s = name_val.as_str().unwrap_or("").trim();
                    if s.is_empty() {
                        return Err("プロジェクト名は空でない文字列を指定してください。".to_string());
                    }
                    next.name = s.to_string();
                }
            }
            if let Some(cs) = op.get("publishCsharp").and_then(|v| v.as_str()) {
                next.publish.csharp = cs.to_string();
            }
            if let Some(bin) = op.get("publishBinary").and_then(|v| v.as_str()) {
                next.publish.binary = bin.to_string();
            }
            session.snapshot.config = next;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert(
                "config".to_string(),
                crate::wire::config_to_data(&session.snapshot.config),
            );
            Ok(Data::Map(out))
        }

        "config.profile.set" => {
            let name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if name.is_empty() {
                return Err("プロファイル名を指定してください。".to_string());
            }
            let current = snapshot.config.profiles.get(&name).cloned().unwrap_or(
                crate::model::Profile {
                    include: Vec::new(),
                    exclude: Vec::new(),
                },
            );
            let include = if op.get("include").is_some() {
                strings_from_data(op.get("include"))
            } else {
                current.include
            };
            let exclude = if op.get("exclude").is_some() {
                strings_from_data(op.get("exclude"))
            } else {
                current.exclude
            };
            let prof = crate::model::Profile { include, exclude };
            session
                .snapshot
                .config
                .profiles
                .insert(name.clone(), prof.clone());
            let mut def = IndexMap::new();
            def.insert(
                "include".to_string(),
                Data::Array(prof.include.into_iter().map(Data::Str).collect()),
            );
            def.insert(
                "exclude".to_string(),
                Data::Array(prof.exclude.into_iter().map(Data::Str).collect()),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("profile".to_string(), Data::Str(name));
            out.insert("definition".to_string(), Data::Map(def));
            Ok(Data::Map(out))
        }

        "config.profile.remove" => {
            let name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if name.is_empty() || !snapshot.config.profiles.contains_key(&name) {
                return Err(format!("プロファイル「{name}」が見つかりません。"));
            }
            session.snapshot.config.profiles.shift_remove(&name);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("removed".to_string(), Data::Str(name));
            Ok(Data::Map(out))
        }

        "table.create" => {
            let name = op
                .get("name")
                .or_else(|| op.get("table"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if name.is_empty() {
                return Err("テーブル名を指定してください。".to_string());
            }
            if tables(&snapshot).iter().any(|t| t.name == name) {
                return Err(format!("テーブル「{name}」は既に存在します。"));
            }
            let rel_path = normalize_source_path(
                op.get("path")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&format!("{name}.yaml")),
            );
            ensure_unique_source_path(&snapshot, &rel_path)?;
            let raw_fields = op
                .get("fields")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_else(|| {
                    let mut m = IndexMap::new();
                    m.insert("key".to_string(), Data::Int(0));
                    m.insert("name".to_string(), Data::Str("id".to_string()));
                    m.insert("type".to_string(), Data::Str("int".to_string()));
                    vec![Data::Map(m)]
                });
            let fields: Vec<Field> = raw_fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    if let Data::Str(s) = f {
                        let mut parts = s.splitn(2, ':');
                        let fname = parts.next().unwrap_or("").trim().to_string();
                        let ftype = parts.next().unwrap_or("int").trim().to_string();
                        Field {
                            key: Some(i as i64),
                            name: fname,
                            r#type: ftype,
                            array: false,
                            nullable: false,
                        }
                    } else {
                        Field {
                            key: Some(
                                f.get("key")
                                    .and_then(|v| v.as_i128())
                                    .unwrap_or(i as i128) as i64,
                            ),
                            name: f
                                .get("name")
                                .map(|v| v.text())
                                .unwrap_or_default(),
                            r#type: f
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("int")
                                .to_string(),
                            array: f.get("array").and_then(|v| v.as_bool()).unwrap_or(false),
                            nullable: f
                                .get("nullable")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        }
                    }
                })
                .collect();
            let pk_fields = {
                let pks = strings_from_data(op.get("pk"));
                if pks.is_empty() {
                    vec![fields
                        .first()
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| "id".to_string())]
                } else {
                    pks
                }
            };
            let mut data = IndexMap::new();
            data.insert("kind".to_string(), Data::Str("schema".to_string()));
            data.insert("table".to_string(), Data::Str(name.clone()));
            if let Some(cs) = op
                .get("csharpName")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                data.insert("csharpName".to_string(), Data::Str(cs.to_string()));
            }
            data.insert(
                "fields".to_string(),
                Data::Array(fields.iter().map(|f| f.to_data()).collect()),
            );
            let mut pk_map = IndexMap::new();
            pk_map.insert(
                "fields".to_string(),
                Data::Array(pk_fields.into_iter().map(Data::Str).collect()),
            );
            data.insert("primaryKey".to_string(), Data::Map(pk_map));
            data.insert("records".to_string(), Data::Array(Vec::new()));

            session.snapshot.folders = add_folders_for_path(&session.snapshot.folders, &rel_path);
            session.snapshot.sources.push(Source {
                id: uid(),
                path: rel_path.clone(),
                data: Data::Map(data),
                raw: None,
                format_text: None,
                syntax_error: None,
            });
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(name));
            out.insert("path".to_string(), Data::Str(rel_path));
            Ok(Data::Map(out))
        }

        "table.delete" => {
            let name = op
                .get("name")
                .or_else(|| op.get("table"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_table_editable(&snapshot, &name)?;
            let table = find_table(&snapshot, &name)?;
            let paths: Vec<String> = table.sources.iter().map(|s| s.path.clone()).collect();
            session.remove_paths(&paths);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(name));
            out.insert(
                "removedFiles".to_string(),
                Data::Array(paths.into_iter().map(Data::Str).collect()),
            );
            Ok(Data::Map(out))
        }

        "table.dataFile.create" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            find_table(&snapshot, &table_name)?;
            let raw_path = op
                .get("path")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "作成するデータファイルのパスを指定してください。".to_string())?;
            let rel_path = normalize_source_path(raw_path);
            ensure_unique_source_path(&snapshot, &rel_path)?;
            let mut data = IndexMap::new();
            data.insert("kind".to_string(), Data::Str("data".to_string()));
            data.insert("table".to_string(), Data::Str(table_name.clone()));
            data.insert("records".to_string(), Data::Array(Vec::new()));
            session.snapshot.folders = add_folders_for_path(&session.snapshot.folders, &rel_path);
            session.snapshot.sources.push(Source {
                id: uid(),
                path: rel_path.clone(),
                data: Data::Map(data),
                raw: None,
                format_text: None,
                syntax_error: None,
            });
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("path".to_string(), Data::Str(rel_path));
            Ok(Data::Map(out))
        }

        "table.column.add" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_table_editable(&snapshot, &table_name)?;
            let table = find_table(&snapshot, &table_name)?;
            let col_name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let col_type = op
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if col_name.is_empty() || col_type.is_empty() {
                return Err("追加する列名と型を指定してください。".to_string());
            }
            if table.fields.iter().any(|f| f.name == col_name) {
                return Err(format!("列「{col_name}」は既に存在します。"));
            }
            let key = op
                .get("key")
                .and_then(|v| v.as_i128())
                .map(|k| k as i64)
                .unwrap_or_else(|| next_field_key(&table.fields));
            let field = Field {
                key: Some(key),
                name: col_name.clone(),
                r#type: col_type,
                array: op.get("array").and_then(|v| v.as_bool()).unwrap_or(false),
                nullable: op
                    .get("nullable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            };
            let mut next_fields: Vec<Data> = table.fields.iter().map(|f| f.to_data()).collect();
            next_fields.push(field.to_data());

            let def_val = default_value(&snapshot, &field, &HashSet::new());
            let mut updated_sources =
                map_records(&snapshot, Some(&table_name), None, |record| {
                    if let Data::Map(rmap) = record {
                        if rmap.contains_key(&col_name) {
                            record.clone()
                        } else {
                            let mut next = rmap.clone();
                            next.insert(col_name.clone(), def_val.clone());
                            Data::Map(next)
                        }
                    } else {
                        record.clone()
                    }
                });
            for s in &mut updated_sources {
                if s.id == table.source.id {
                    s.data = s
                        .data
                        .set_in(&[PathKey::from("fields")], Data::Array(next_fields.clone()));
                }
            }
            session.snapshot.sources = updated_sources;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("field".to_string(), field.to_data());
            Ok(Data::Map(out))
        }

        "table.column.set" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let col_name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let index = table
                .fields
                .iter()
                .position(|f| f.name == col_name)
                .ok_or_else(|| format!("テーブル「{table_name}」に列「{col_name}」がありません。"))?;
            let mut updated = table.fields[index].clone();
            if let Some(t) = op.get("type").and_then(|v| v.as_str()) {
                updated.r#type = t.to_string();
            }
            if let Some(k) = op.get("key").and_then(|v| v.as_i128()) {
                updated.key = Some(k as i64);
            }
            if let Some(a) = op.get("array").and_then(|v| v.as_bool()) {
                updated.array = a;
            }
            if let Some(n) = op.get("nullable").and_then(|v| v.as_bool()) {
                updated.nullable = n;
            }
            session.update(
                &table.source.id,
                &[PathKey::from("fields"), PathKey::from(index)],
                updated.to_data(),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("field".to_string(), updated.to_data());
            Ok(Data::Map(out))
        }

        "table.column.remove" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_table_editable(&snapshot, &table_name)?;
            let table = find_table(&snapshot, &table_name)?;
            let col_name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let index = table
                .fields
                .iter()
                .position(|f| f.name == col_name)
                .ok_or_else(|| format!("テーブル「{table_name}」に列「{col_name}」がありません。"))?;
            session.remove_field(&table.source.id, index);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("removed".to_string(), Data::Str(col_name));
            Ok(Data::Map(out))
        }

        "table.column.rename" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_table_editable(&snapshot, &table_name)?;
            let table = find_table(&snapshot, &table_name)?;
            let old_name = op
                .get("old")
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let new_name = op
                .get("new")
                .or_else(|| op.get("newName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &old_name)?;
            let index = table
                .fields
                .iter()
                .position(|f| f.name == old_name)
                .ok_or_else(|| format!("テーブル「{table_name}」に列「{old_name}」がありません。"))?;
            session.rename_field(&table.source.id, index, &new_name)?;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("from".to_string(), Data::Str(old_name));
            out.insert("to".to_string(), Data::Str(new_name));
            Ok(Data::Map(out))
        }

        "table.column.move" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let col_name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let from = table
                .fields
                .iter()
                .position(|f| f.name == col_name)
                .ok_or_else(|| format!("テーブル「{table_name}」に列「{col_name}」がありません。"))?;
            let to_raw = op.get("to").and_then(|v| v.as_i128()).unwrap_or(-1);
            if to_raw < 0 || (to_raw as usize) >= table.fields.len() {
                return Err(format!(
                    "移動先インデックスが範囲外です: {}",
                    op.get("to").map(|v| v.text()).unwrap_or_default()
                ));
            }
            let to = to_raw as usize;
            let mut fields_with_keys: Vec<Data> = table
                .fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let mut fc = f.clone();
                    fc.key = Some(f.key.unwrap_or(i as i64));
                    fc.to_data()
                })
                .collect();
            let moved = fields_with_keys.remove(from);
            fields_with_keys.insert(to, moved);
            session.update(
                &table.source.id,
                &[PathKey::from("fields")],
                Data::Array(fields_with_keys),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("column".to_string(), Data::Str(col_name));
            out.insert("from".to_string(), Data::Int(from as i128));
            out.insert("to".to_string(), Data::Int(to as i128));
            Ok(Data::Map(out))
        }

        "table.key.setPk" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let fields = strings_from_data(op.get("fields"));
            if fields.is_empty() {
                return Err("主キーの項目を指定してください。".to_string());
            }
            let mut pk_map = IndexMap::new();
            pk_map.insert(
                "fields".to_string(),
                Data::Array(fields.iter().cloned().map(Data::Str).collect()),
            );
            session.update(
                &table.source.id,
                &[PathKey::from("primaryKey")],
                Data::Map(pk_map),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert(
                "primaryKey".to_string(),
                Data::Array(fields.into_iter().map(Data::Str).collect()),
            );
            Ok(Data::Map(out))
        }

        "table.key.addSk" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let fields = strings_from_data(op.get("fields"));
            if fields.is_empty() {
                return Err("副キーの項目を指定してください。".to_string());
            }
            let mut current = table
                .source
                .data
                .get("secondaryKeys")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut entry = IndexMap::new();
            entry.insert(
                "fields".to_string(),
                Data::Array(fields.into_iter().map(Data::Str).collect()),
            );
            if op.get("nonUnique").and_then(|v| v.as_bool()).unwrap_or(false) {
                entry.insert("nonUnique".to_string(), Data::Bool(true));
            }
            let entry_data = Data::Map(entry);
            current.push(entry_data.clone());
            session.update(
                &table.source.id,
                &[PathKey::from("secondaryKeys")],
                Data::Array(current),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("secondaryKey".to_string(), entry_data);
            Ok(Data::Map(out))
        }

        "table.key.removeSk" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let current = table
                .source
                .data
                .get("secondaryKeys")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let idx_raw = op.get("index").and_then(|v| v.as_i128()).unwrap_or(-1);
            if idx_raw < 0 || (idx_raw as usize) >= current.len() {
                return Err(format!(
                    "副キーの番号が範囲外です: {}",
                    op.get("index").map(|v| v.text()).unwrap_or_default()
                ));
            }
            let idx = idx_raw as usize;
            let next: Vec<Data> = current
                .into_iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, v)| v)
                .collect();
            session.update(
                &table.source.id,
                &[PathKey::from("secondaryKeys")],
                Data::Array(next),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("removedIndex".to_string(), Data::Int(idx as i128));
            Ok(Data::Map(out))
        }

        "table.ref.add" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let ref_name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let own_fields = strings_from_data(op.get("fields"));
            let target_table = op
                .get("targetTable")
                .or_else(|| op.get("target").and_then(|t| t.get("table")))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let target_fields = strings_from_data(
                op.get("targetFields")
                    .or_else(|| op.get("target").and_then(|t| t.get("fields"))),
            );
            if ref_name.is_empty()
                || own_fields.is_empty()
                || target_table.is_empty()
                || target_fields.is_empty()
            {
                return Err(
                    "参照名、自テーブル項目、参照先テーブル、参照先項目をすべて指定してください。"
                        .to_string(),
                );
            }
            let mut current = table
                .source
                .data
                .get("references")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if current
                .iter()
                .any(|r| r.get("name").and_then(|v| v.as_str()) == Some(ref_name.as_str()))
            {
                return Err(format!("参照「{ref_name}」は既に存在します。"));
            }
            let mut entry = IndexMap::new();
            entry.insert("name".to_string(), Data::Str(ref_name));
            if let Some(cs) = op
                .get("csharpName")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                entry.insert("csharpName".to_string(), Data::Str(cs.to_string()));
            }
            entry.insert(
                "fields".to_string(),
                Data::Array(own_fields.into_iter().map(Data::Str).collect()),
            );
            let mut target_map = IndexMap::new();
            target_map.insert("table".to_string(), Data::Str(target_table));
            target_map.insert(
                "fields".to_string(),
                Data::Array(target_fields.into_iter().map(Data::Str).collect()),
            );
            entry.insert("target".to_string(), Data::Map(target_map));
            let entry_data = Data::Map(entry);
            current.push(entry_data.clone());
            session.update(
                &table.source.id,
                &[PathKey::from("references")],
                Data::Array(current),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("reference".to_string(), entry_data);
            Ok(Data::Map(out))
        }

        "table.ref.remove" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let ref_name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let current = table
                .source
                .data
                .get("references")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if !current
                .iter()
                .any(|r| r.get("name").and_then(|v| v.as_str()) == Some(ref_name.as_str()))
            {
                return Err(format!("参照「{ref_name}」が見つかりません。"));
            }
            let next: Vec<Data> = current
                .into_iter()
                .filter(|r| r.get("name").and_then(|v| v.as_str()) != Some(ref_name.as_str()))
                .collect();
            session.update(
                &table.source.id,
                &[PathKey::from("references")],
                Data::Array(next),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("removed".to_string(), Data::Str(ref_name));
            Ok(Data::Map(out))
        }

        "type.create" => {
            let raw_kind = op
                .get("kind")
                .or_else(|| op.get("typeKind"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let normalized_kind = if raw_kind == "value-object" {
                "valueObject"
            } else {
                raw_kind
            };
            if !matches!(
                normalized_kind,
                "enum" | "flags" | "custom" | "valueObject"
            ) {
                return Err(format!(
                    "型の種類は enum / flags / custom / valueObject を指定してください: {raw_kind}"
                ));
            }
            let name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if name.is_empty() {
                return Err("型名を指定してください。".to_string());
            }
            if type_source(&snapshot, &name).is_some() {
                return Err(format!("型「{name}」は既に存在します。"));
            }
            let rel_path = normalize_source_path(
                op.get("path")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&format!("{name}.yaml")),
            );
            ensure_unique_source_path(&snapshot, &rel_path)?;

            let definition = match normalized_kind {
                "valueObject" => {
                    let mut m = IndexMap::new();
                    m.insert(
                        "underlying".to_string(),
                        Data::Str(
                            op.get("underlying")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .unwrap_or("int")
                                .to_string(),
                        ),
                    );
                    Data::Map(m)
                }
                "custom" => {
                    let raw_fields = op
                        .get("fields")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let fields: Vec<Data> = raw_fields
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            if let Data::Str(s) = f {
                                let mut parts = s.splitn(2, ':');
                                let fname = parts.next().unwrap_or("").trim().to_string();
                                let ftype = parts.next().unwrap_or("int").trim().to_string();
                                Field {
                                    key: Some(i as i64),
                                    name: fname,
                                    r#type: ftype,
                                    array: false,
                                    nullable: false,
                                }
                                .to_data()
                            } else {
                                Field {
                                    key: Some(
                                        f.get("key")
                                            .and_then(|v| v.as_i128())
                                            .unwrap_or(i as i128)
                                            as i64,
                                    ),
                                    name: f.get("name").map(|v| v.text()).unwrap_or_default(),
                                    r#type: f
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("int")
                                        .to_string(),
                                    array: f
                                        .get("array")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    nullable: f
                                        .get("nullable")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                }
                                .to_data()
                            }
                        })
                        .collect();
                    let mut m = IndexMap::new();
                    m.insert("fields".to_string(), Data::Array(fields));
                    Data::Map(m)
                }
                "flags" => {
                    let underlying = op
                        .get("underlying")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or("uint")
                        .to_string();
                    let input_members: Vec<Data> = match op.get("members") {
                        Some(Data::Array(arr)) => arr.clone(),
                        Some(Data::Str(s)) => split_csv(s).into_iter().map(Data::Str).collect(),
                        _ => Vec::new(),
                    };
                    let mut none_m = IndexMap::new();
                    none_m.insert("name".to_string(), Data::Str("None".to_string()));
                    none_m.insert("value".to_string(), Data::Int(0));
                    let mut members = vec![Data::Map(none_m)];
                    let mut bit = 0u32;
                    for item in input_members {
                        if let Data::Str(s) = item {
                            let mut parts = s.splitn(2, '=');
                            let m_name = parts.next().unwrap_or("").trim();
                            if m_name == "None" {
                                continue;
                            }
                            let val = if let Some(v_str) = parts.next() {
                                v_str
                                    .trim()
                                    .parse::<i128>()
                                    .map_err(|_| format!("不正な整数値です: {v_str}"))?
                            } else {
                                let v = 1i128 << bit;
                                bit += 1;
                                v
                            };
                            let mut mm = IndexMap::new();
                            mm.insert("name".to_string(), Data::Str(m_name.to_string()));
                            mm.insert("value".to_string(), Data::Int(val));
                            members.push(Data::Map(mm));
                        } else if item.is_object() {
                            let m_name = item.get("name").map(|v| v.text()).unwrap_or_default();
                            if m_name == "None" {
                                continue;
                            }
                            let val = if let Some(v) =
                                item.get("value").and_then(|v| v.parse_bigint())
                            {
                                v
                            } else {
                                let v = 1i128 << bit;
                                bit += 1;
                                v
                            };
                            let mut mm = IndexMap::new();
                            mm.insert("name".to_string(), Data::Str(m_name));
                            mm.insert("value".to_string(), Data::Int(val));
                            members.push(Data::Map(mm));
                        }
                    }
                    let mut m = IndexMap::new();
                    m.insert("underlying".to_string(), Data::Str(underlying));
                    m.insert("members".to_string(), Data::Array(members));
                    Data::Map(m)
                }
                _ => {
                    let underlying = op
                        .get("underlying")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or("int")
                        .to_string();
                    let input_members: Vec<Data> = match op.get("members") {
                        Some(Data::Array(arr)) => arr.clone(),
                        Some(Data::Str(s)) => split_csv(s).into_iter().map(Data::Str).collect(),
                        _ => vec![Data::Str("NewMember".to_string())],
                    };
                    let mut members = Vec::with_capacity(input_members.len());
                    for (i, item) in input_members.into_iter().enumerate() {
                        if let Data::Str(s) = item {
                            let mut parts = s.splitn(2, '=');
                            let m_name = parts.next().unwrap_or("").trim().to_string();
                            let val = if let Some(v_str) = parts.next() {
                                v_str
                                    .trim()
                                    .parse::<i128>()
                                    .map_err(|_| format!("不正な整数値です: {v_str}"))?
                            } else {
                                i as i128
                            };
                            let mut mm = IndexMap::new();
                            mm.insert("name".to_string(), Data::Str(m_name));
                            mm.insert("value".to_string(), Data::Int(val));
                            members.push(Data::Map(mm));
                        } else {
                            let m_name = item.get("name").map(|v| v.text()).unwrap_or_default();
                            let val = item
                                .get("value")
                                .and_then(|v| v.parse_bigint())
                                .unwrap_or(i as i128);
                            let mut mm = IndexMap::new();
                            mm.insert("name".to_string(), Data::Str(m_name));
                            mm.insert("value".to_string(), Data::Int(val));
                            members.push(Data::Map(mm));
                        }
                    }
                    let mut m = IndexMap::new();
                    m.insert("underlying".to_string(), Data::Str(underlying));
                    m.insert("members".to_string(), Data::Array(members));
                    Data::Map(m)
                }
            };

            let mut data = IndexMap::new();
            data.insert("kind".to_string(), Data::Str("type".to_string()));
            data.insert("name".to_string(), Data::Str(name.clone()));
            data.insert(normalized_kind.to_string(), definition);

            session.snapshot.folders = add_folders_for_path(&session.snapshot.folders, &rel_path);
            session.snapshot.sources.push(Source {
                id: uid(),
                path: rel_path.clone(),
                data: Data::Map(data),
                raw: None,
                format_text: None,
                syntax_error: None,
            });
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(name));
            out.insert(
                "kindName".to_string(),
                Data::Str(normalized_kind.to_string()),
            );
            out.insert("path".to_string(), Data::Str(rel_path));
            Ok(Data::Map(out))
        }

        "type.delete" => {
            let name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &name)?;
            session.remove_paths(&[source.path.clone()]);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("removed".to_string(), Data::Str(name));
            out.insert("path".to_string(), Data::Str(source.path));
            Ok(Data::Map(out))
        }

        "type.setUnderlying" => {
            let name = op
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let underlying = op
                .get("underlying")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &name)?;
            let Some(tkind) = type_kind(Some(&source)) else {
                return Err(format!("型「{name}」には基底型（underlying）を設定できません。"));
            };
            if tkind == "custom" {
                return Err(format!("型「{name}」には基底型（underlying）を設定できません。"));
            }
            session.update(
                &source.id,
                &[PathKey::from(tkind), PathKey::from("underlying")],
                Data::Str(underlying.clone()),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(name));
            out.insert("underlying".to_string(), Data::Str(underlying));
            Ok(Data::Map(out))
        }

        "type.member.add" => {
            let type_name = op
                .get("type")
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let member_name = op
                .get("member")
                .or_else(|| op.get("memberName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &type_name)?;
            let tkind = type_kind(Some(&source)).unwrap_or("");
            if !matches!(tkind, "enum" | "flags") {
                return Err(format!("型「{type_name}」はEnumまたはFlagsではありません。"));
            }
            let mut members = source
                .data
                .get(tkind)
                .and_then(|d| d.get("members"))
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            if members
                .iter()
                .any(|m| m.get("name").and_then(|v| v.as_str()) == Some(member_name.as_str()))
            {
                return Err(format!("候補「{member_name}」は既に存在します。"));
            }
            let value = if let Some(v) = op.get("value").filter(|v| !v.is_null_or_undefined()) {
                v.parse_bigint()
                    .ok_or_else(|| format!("不正な整数値です: {}", v.text()))?
            } else if tkind == "flags" {
                next_flag_bit(&source)?
            } else {
                next_enum_value(&source)
            };
            let mut entry = IndexMap::new();
            entry.insert("name".to_string(), Data::Str(member_name));
            entry.insert("value".to_string(), Data::Int(value));
            let entry_data = Data::Map(entry);
            members.push(entry_data.clone());
            session.update(
                &source.id,
                &[PathKey::from(tkind), PathKey::from("members")],
                Data::Array(members),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("member".to_string(), entry_data);
            Ok(Data::Map(out))
        }

        "type.member.set" => {
            let type_name = op
                .get("type")
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let member_name = op
                .get("member")
                .or_else(|| op.get("memberName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &type_name)?;
            let tkind = type_kind(Some(&source)).unwrap_or("");
            if !matches!(tkind, "enum" | "flags") {
                return Err(format!("型「{type_name}」はEnumまたはFlagsではありません。"));
            }
            let members = source
                .data
                .get(tkind)
                .and_then(|d| d.get("members"))
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let index = members
                .iter()
                .position(|m| m.get("name").and_then(|v| v.as_str()) == Some(member_name.as_str()))
                .ok_or_else(|| format!("候補「{member_name}」が見つかりません。"))?;
            let val_raw = op
                .get("value")
                .filter(|v| !v.is_null_or_undefined())
                .ok_or_else(|| "変更する値（--value）を指定してください。".to_string())?;
            let value = val_raw
                .parse_bigint()
                .ok_or_else(|| format!("不正な整数値です: {}", val_raw.text()))?;
            if tkind == "flags" && member_name == "None" && value != 0 {
                return Err("Flagsの予約候補「None」の値は0から変更できません。".to_string());
            }
            session.update(
                &source.id,
                &[
                    PathKey::from(tkind),
                    PathKey::from("members"),
                    PathKey::from(index),
                    PathKey::from("value"),
                ],
                Data::Int(value),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("member".to_string(), Data::Str(member_name));
            out.insert("value".to_string(), Data::Int(value));
            Ok(Data::Map(out))
        }

        "type.member.remove" => {
            let type_name = op
                .get("type")
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let member_name = op
                .get("member")
                .or_else(|| op.get("memberName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &type_name)?;
            let tkind = type_kind(Some(&source)).unwrap_or("");
            if !matches!(tkind, "enum" | "flags") {
                return Err(format!("型「{type_name}」はEnumまたはFlagsではありません。"));
            }
            if tkind == "flags" && member_name == "None" {
                return Err("Flagsの予約候補「None」は削除できません。".to_string());
            }
            let members = source
                .data
                .get(tkind)
                .and_then(|d| d.get("members"))
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let index = members
                .iter()
                .position(|m| m.get("name").and_then(|v| v.as_str()) == Some(member_name.as_str()))
                .ok_or_else(|| format!("候補「{member_name}」が見つかりません。"))?;
            let next: Vec<Data> = members
                .into_iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, v)| v)
                .collect();
            session.update(
                &source.id,
                &[PathKey::from(tkind), PathKey::from("members")],
                Data::Array(next),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("removed".to_string(), Data::Str(member_name));
            Ok(Data::Map(out))
        }

        "type.member.rename" | "rename.member" => {
            let type_name = op
                .get("type")
                .or_else(|| op.get("target"))
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let old_name = op
                .get("old")
                .or_else(|| op.get("member"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let new_name = op
                .get("new")
                .or_else(|| op.get("newName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &old_name)?;
            let source = find_type(&snapshot, &type_name)?;
            let tkind = type_kind(Some(&source)).unwrap_or("");
            if !matches!(tkind, "enum" | "flags") {
                return Err(format!("型「{type_name}」はEnumまたはFlagsではありません。"));
            }
            if tkind == "flags" && old_name == "None" {
                return Err("Flagsの予約候補「None」は名前変更できません。".to_string());
            }
            let members = source
                .data
                .get(tkind)
                .and_then(|d| d.get("members"))
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let index = members
                .iter()
                .position(|m| m.get("name").and_then(|v| v.as_str()) == Some(old_name.as_str()))
                .ok_or_else(|| format!("候補「{old_name}」が見つかりません。"))?;
            if members.iter().enumerate().any(|(i, m)| {
                i != index && m.get("name").and_then(|v| v.as_str()) == Some(new_name.as_str())
            }) {
                return Err(format!("候補「{new_name}」は既に存在します。"));
            }
            session.rename_member(&source.id, index, &new_name);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("from".to_string(), Data::Str(old_name));
            out.insert("to".to_string(), Data::Str(new_name));
            Ok(Data::Map(out))
        }

        "type.member.move" => {
            let type_name = op
                .get("type")
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let member_name = op
                .get("member")
                .or_else(|| op.get("memberName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &type_name)?;
            let tkind = type_kind(Some(&source)).unwrap_or("");
            if !matches!(tkind, "enum" | "flags") {
                return Err(format!("型「{type_name}」はEnumまたはFlagsではありません。"));
            }
            let mut members = source
                .data
                .get(tkind)
                .and_then(|d| d.get("members"))
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let from = members
                .iter()
                .position(|m| m.get("name").and_then(|v| v.as_str()) == Some(member_name.as_str()))
                .ok_or_else(|| format!("候補「{member_name}」が見つかりません。"))?;
            let to_raw = op.get("to").and_then(|v| v.as_i128()).unwrap_or(-1);
            if to_raw < 0 || (to_raw as usize) >= members.len() {
                return Err(format!(
                    "移動先インデックスが範囲外です: {}",
                    op.get("to").map(|v| v.text()).unwrap_or_default()
                ));
            }
            let to = to_raw as usize;
            if tkind == "flags" && (member_name == "None" || to == 0) {
                return Err("Flagsの予約候補「None」は先頭（0番目）に固定されます。".to_string());
            }
            let moved = members.remove(from);
            members.insert(to, moved);
            session.update(
                &source.id,
                &[PathKey::from(tkind), PathKey::from("members")],
                Data::Array(members),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("member".to_string(), Data::Str(member_name));
            out.insert("from".to_string(), Data::Int(from as i128));
            out.insert("to".to_string(), Data::Int(to as i128));
            Ok(Data::Map(out))
        }

        "type.field.add" => {
            let type_name = op
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let field_name = op
                .get("name")
                .or_else(|| op.get("field"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let field_type = op
                .get("fieldType")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &type_name)?;
            let source = find_type(&snapshot, &type_name)?;
            if type_kind(Some(&source)) != Some("custom") {
                return Err(format!("型「{type_name}」はCustom構造体ではありません。"));
            }
            let fields = fields_of(&source);
            if fields.iter().any(|f| f.name == field_name) {
                return Err(format!("フィールド「{field_name}」は既に存在します。"));
            }
            let key = op
                .get("key")
                .and_then(|v| v.as_i128())
                .map(|k| k as i64)
                .unwrap_or_else(|| next_field_key(&fields));
            let field = Field {
                key: Some(key),
                name: field_name.clone(),
                r#type: field_type,
                array: op.get("array").and_then(|v| v.as_bool()).unwrap_or(false),
                nullable: op
                    .get("nullable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            };
            let mut next_fields: Vec<Data> = fields.iter().map(|f| f.to_data()).collect();
            next_fields.push(field.to_data());
            let def_val = default_value(&snapshot, &field, &HashSet::new());
            let mut updated_sources =
                map_records(&snapshot, None, Some(&type_name), |val| {
                    if let Data::Map(vmap) = val {
                        if vmap.contains_key(&field_name) {
                            val.clone()
                        } else {
                            let mut next = vmap.clone();
                            next.insert(field_name.clone(), def_val.clone());
                            Data::Map(next)
                        }
                    } else {
                        val.clone()
                    }
                });
            for s in &mut updated_sources {
                if s.id == source.id {
                    s.data = s.data.set_in(
                        &[PathKey::from("custom"), PathKey::from("fields")],
                        Data::Array(next_fields.clone()),
                    );
                }
            }
            session.snapshot.sources = updated_sources;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("field".to_string(), field.to_data());
            Ok(Data::Map(out))
        }

        "type.field.set" => {
            let type_name = op
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let field_name = op
                .get("name")
                .or_else(|| op.get("field"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &type_name)?;
            if type_kind(Some(&source)) != Some("custom") {
                return Err(format!("型「{type_name}」はCustom構造体ではありません。"));
            }
            let fields = fields_of(&source);
            let index = fields
                .iter()
                .position(|f| f.name == field_name)
                .ok_or_else(|| format!("フィールド「{field_name}」が見つかりません。"))?;
            let mut updated = fields[index].clone();
            if let Some(ft) = op.get("fieldType").and_then(|v| v.as_str()) {
                updated.r#type = ft.to_string();
            }
            if let Some(k) = op.get("key").and_then(|v| v.as_i128()) {
                updated.key = Some(k as i64);
            }
            if let Some(a) = op.get("array").and_then(|v| v.as_bool()) {
                updated.array = a;
            }
            if let Some(n) = op.get("nullable").and_then(|v| v.as_bool()) {
                updated.nullable = n;
            }
            session.update(
                &source.id,
                &[
                    PathKey::from("custom"),
                    PathKey::from("fields"),
                    PathKey::from(index),
                ],
                updated.to_data(),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("field".to_string(), updated.to_data());
            Ok(Data::Map(out))
        }

        "type.field.remove" => {
            let type_name = op
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let field_name = op
                .get("name")
                .or_else(|| op.get("field"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &type_name)?;
            let source = find_type(&snapshot, &type_name)?;
            if type_kind(Some(&source)) != Some("custom") {
                return Err(format!("型「{type_name}」はCustom構造体ではありません。"));
            }
            let fields = fields_of(&source);
            let index = fields
                .iter()
                .position(|f| f.name == field_name)
                .ok_or_else(|| format!("フィールド「{field_name}」が見つかりません。"))?;
            session.remove_field(&source.id, index);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("removed".to_string(), Data::Str(field_name));
            Ok(Data::Map(out))
        }

        "type.field.rename" => {
            let type_name = op
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let old_name = op
                .get("old")
                .or_else(|| op.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let new_name = op
                .get("new")
                .or_else(|| op.get("newName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &type_name)?;
            ensure_token_editable(&snapshot, &old_name)?;
            let source = find_type(&snapshot, &type_name)?;
            if type_kind(Some(&source)) != Some("custom") {
                return Err(format!("型「{type_name}」はCustom構造体ではありません。"));
            }
            let fields = fields_of(&source);
            let index = fields
                .iter()
                .position(|f| f.name == old_name)
                .ok_or_else(|| format!("フィールド「{old_name}」が見つかりません。"))?;
            session.rename_field(&source.id, index, &new_name)?;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("from".to_string(), Data::Str(old_name));
            out.insert("to".to_string(), Data::Str(new_name));
            Ok(Data::Map(out))
        }

        "type.field.move" => {
            let type_name = op
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let field_name = op
                .get("name")
                .or_else(|| op.get("field"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let source = find_type(&snapshot, &type_name)?;
            if type_kind(Some(&source)) != Some("custom") {
                return Err(format!("型「{type_name}」はCustom構造体ではありません。"));
            }
            let fields = fields_of(&source);
            let from = fields
                .iter()
                .position(|f| f.name == field_name)
                .ok_or_else(|| format!("フィールド「{field_name}」が見つかりません。"))?;
            let to_raw = op.get("to").and_then(|v| v.as_i128()).unwrap_or(-1);
            if to_raw < 0 || (to_raw as usize) >= fields.len() {
                return Err(format!(
                    "移動先インデックスが範囲外です: {}",
                    op.get("to").map(|v| v.text()).unwrap_or_default()
                ));
            }
            let to = to_raw as usize;
            let mut fields_with_keys: Vec<Data> = fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let mut fc = f.clone();
                    fc.key = Some(f.key.unwrap_or(i as i64));
                    fc.to_data()
                })
                .collect();
            let moved = fields_with_keys.remove(from);
            fields_with_keys.insert(to, moved);
            session.update(
                &source.id,
                &[PathKey::from("custom"), PathKey::from("fields")],
                Data::Array(fields_with_keys),
            );
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("type".to_string(), Data::Str(type_name));
            out.insert("field".to_string(), Data::Str(field_name));
            out.insert("from".to_string(), Data::Int(from as i128));
            out.insert("to".to_string(), Data::Int(to as i128));
            Ok(Data::Map(out))
        }

        "rename.table" => {
            let old_name = op
                .get("old")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let new_name = op
                .get("new")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_table_editable(&snapshot, &old_name)?;
            ensure_token_editable(&snapshot, &old_name)?;
            let table = find_table(&snapshot, &old_name)?;
            if tables(&snapshot)
                .iter()
                .any(|t| t.name == new_name && t.source.id != table.source.id)
            {
                return Err(format!("テーブル「{new_name}」は既に存在します。"));
            }
            session.rename_declaration(&table.source.id, &new_name)?;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("from".to_string(), Data::Str(old_name));
            out.insert("to".to_string(), Data::Str(new_name));
            Ok(Data::Map(out))
        }

        "rename.type" => {
            let old_name = op
                .get("old")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let new_name = op
                .get("new")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &old_name)?;
            let source = find_type(&snapshot, &old_name)?;
            if let Some(existing) = type_source(&snapshot, &new_name) {
                if existing.id != source.id {
                    return Err(format!("型「{new_name}」は既に存在します。"));
                }
            }
            session.rename_declaration(&source.id, &new_name)?;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("from".to_string(), Data::Str(old_name));
            out.insert("to".to_string(), Data::Str(new_name));
            Ok(Data::Map(out))
        }

        "rename.field" => {
            let target_name = op
                .get("target")
                .or_else(|| op.get("table"))
                .or_else(|| op.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let old_name = op
                .get("old")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let new_name = op
                .get("new")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_token_editable(&snapshot, &old_name)?;
            if let Some(table) = tables(&snapshot).into_iter().find(|t| t.name == target_name) {
                ensure_table_editable(&snapshot, &target_name)?;
                let index = table
                    .fields
                    .iter()
                    .position(|f| f.name == old_name)
                    .ok_or_else(|| {
                        format!("テーブル「{target_name}」に列「{old_name}」がありません。")
                    })?;
                session.rename_field(&table.source.id, index, &new_name)?;
                let mut out = IndexMap::new();
                out.insert("op".to_string(), Data::Str(kind.to_string()));
                out.insert("target".to_string(), Data::Str(target_name));
                out.insert("from".to_string(), Data::Str(old_name));
                out.insert("to".to_string(), Data::Str(new_name));
                return Ok(Data::Map(out));
            }
            let ty = find_type(&snapshot, &target_name)?;
            if type_kind(Some(&ty)) != Some("custom") {
                return Err(format!(
                    "型「{target_name}」はフィールドを持つ構造体ではありません。"
                ));
            }
            let fields = fields_of(&ty);
            let index = fields
                .iter()
                .position(|f| f.name == old_name)
                .ok_or_else(|| {
                    format!("構造体「{target_name}」にフィールド「{old_name}」がありません。")
                })?;
            session.rename_field(&ty.id, index, &new_name)?;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("target".to_string(), Data::Str(target_name));
            out.insert("from".to_string(), Data::Str(old_name));
            out.insert("to".to_string(), Data::Str(new_name));
            Ok(Data::Map(out))
        }

        "source.move" => {
            let from = normalize_source_path(
                op.get("from")
                    .or_else(|| op.get("old"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            );
            let to = normalize_source_path(
                op.get("to")
                    .or_else(|| op.get("new"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            );
            if !snapshot.sources.iter().any(|s| s.path == from) {
                return Err(format!("ファイル「sources/{from}」が見つかりません。"));
            }
            session.snapshot.folders = add_folders_for_path(&session.snapshot.folders, &to);
            session.move_path(&from, &to)?;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("from".to_string(), Data::Str(from));
            out.insert("to".to_string(), Data::Str(to));
            Ok(Data::Map(out))
        }

        "source.delete" => {
            let target = normalize_source_path(
                op.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            );
            if !snapshot
                .sources
                .iter()
                .any(|s| s.path == target || s.path.starts_with(&format!("{target}/")))
            {
                return Err(format!("対象「sources/{target}」が見つかりません。"));
            }
            session.remove_paths(&[target.clone()]);
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("removed".to_string(), Data::Str(target));
            Ok(Data::Map(out))
        }

        "record.add" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            ensure_table_editable(&snapshot, &table_name)?;
            let table = find_table(&snapshot, &table_name)?;
            let target_source = if let Some(src_opt) = op
                .get("source")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                let rel = normalize_source_path(src_opt);
                table
                    .sources
                    .iter()
                    .find(|s| s.path == rel)
                    .cloned()
                    .ok_or_else(|| {
                        let candidates: Vec<String> =
                            table.sources.iter().map(|s| s.path.clone()).collect();
                        format!(
                            "テーブル「{table_name}」のデータファイル「{rel}」が見つかりません。候補: {}",
                            candidates.join(", ")
                        )
                    })?
            } else {
                if table.sources.len() > 1 {
                    let candidates: Vec<String> =
                        table.sources.iter().map(|s| s.path.clone()).collect();
                    return Err(format!(
                        "テーブル「{table_name}」は複数のデータファイルに分かれています。追加先を --source で指定してください（候補: {}）。",
                        candidates.join(", ")
                    ));
                }
                table
                    .sources
                    .first()
                    .cloned()
                    .unwrap_or_else(|| table.source.clone())
            };
            ensure_source_editable(&target_source)?;

            let input_records: Vec<Data> = if let Some(Data::Array(arr)) = op.get("records") {
                arr.clone()
            } else {
                vec![op
                    .get("values")
                    .cloned()
                    .unwrap_or_else(|| Data::Map(IndexMap::new()))]
            };

            let pk_fields: Vec<String> = table
                .source
                .data
                .get("primaryKey")
                .and_then(|pk| pk.get("fields"))
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let existing_rows: Vec<Data> = table
                .sources
                .iter()
                .flat_map(|s| {
                    s.data
                        .get("records")
                        .and_then(|r| r.as_array())
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();
            let mut existing_pk_set: HashSet<String> = if pk_fields.is_empty() {
                HashSet::new()
            } else {
                existing_rows
                    .iter()
                    .map(|r| {
                        let vals: Vec<Data> = pk_fields
                            .iter()
                            .map(|k| r.get(k).cloned().unwrap_or(Data::Undefined))
                            .collect();
                        fingerprint(&Data::Array(vals))
                    })
                    .collect()
            };

            let mut allocator = RecordAllocator::new(&snapshot, &table);
            let mut added: Vec<Data> = Vec::new();
            let auto_key = op.get("autoKey").and_then(|v| v.as_bool()).unwrap_or(false);

            let parsed_assignments = if let Some(Data::Array(assign_arr)) = op.get("assignments") {
                let mut list = Vec::new();
                for a in assign_arr {
                    if let Some(p_str) = a.get("pathStr").and_then(|v| v.as_str()) {
                        let steps = parse_field_path(p_str)?;
                        let val = a.get("value").cloned().unwrap_or(Data::Null);
                        list.push((steps, val));
                    }
                }
                list
            } else {
                Vec::new()
            };

            for raw_input in input_records {
                let mut base_map = IndexMap::new();
                for f in &table.fields {
                    base_map.insert(f.name.clone(), default_value(&snapshot, f, &HashSet::new()));
                }
                let mut base_record = Data::Map(base_map);

                let explicit_pk_provided = !pk_fields.is_empty()
                    && raw_input.is_object()
                    && pk_fields.iter().all(|k| {
                        raw_input
                            .get(k)
                            .map(|v| !matches!(v, Data::Undefined))
                            .unwrap_or(false)
                    });

                if let Data::Map(rmap) = &raw_input {
                    for (raw_key, raw_val) in rmap {
                        if raw_key == "$tags" {
                            let tags = strings_from_data(Some(raw_val));
                            base_record = base_record.set_in(
                                &[PathKey::from("$tags")],
                                Data::Array(tags.into_iter().map(Data::Str).collect()),
                            );
                            continue;
                        }
                        let steps = parse_field_path(raw_key)?;
                        let target_field = resolve_field_at_path(&snapshot, &table.fields, &steps);
                        let coerced = coerce_scalar_or_structured(
                            &snapshot,
                            target_field.as_ref(),
                            raw_val,
                        )?;
                        base_record = set_by_path(&base_record, &steps, coerced)?;
                    }
                }

                for (steps, val) in &parsed_assignments {
                    base_record = set_by_path(&base_record, steps, val.clone())?;
                }

                let mentions_pk = !pk_fields.is_empty()
                    && pk_fields.iter().all(|k| {
                        raw_input
                            .get(k)
                            .map(|v| !matches!(v, Data::Undefined))
                            .unwrap_or(false)
                            || parsed_assignments
                                .iter()
                                .any(|(steps, _)| steps.len() == 1 && steps[0] == PathStep::Key(k.clone()))
                    });

                if let Some(tags_val) = op.get("tags") {
                    let tags = strings_from_data(Some(tags_val));
                    if !tags.is_empty() {
                        base_record = base_record.set_in(
                            &[PathKey::from("$tags")],
                            Data::Array(tags.into_iter().map(Data::Str).collect()),
                        );
                    }
                }

                let tags_backup = base_record.get("$tags").cloned();

                if (explicit_pk_provided || mentions_pk) && !auto_key {
                    let vals: Vec<Data> = pk_fields
                        .iter()
                        .map(|k| base_record.get(k).cloned().unwrap_or(Data::Undefined))
                        .collect();
                    let sig = fingerprint(&Data::Array(vals));
                    if existing_pk_set.contains(&sig) {
                        let desc: Vec<String> = pk_fields
                            .iter()
                            .map(|k| {
                                format!(
                                    "{k}={}",
                                    base_record.get(k).map(|v| v.text()).unwrap_or_default()
                                )
                            })
                            .collect();
                        return Err(format!(
                            "指定された主キー（{}）は既に存在します。",
                            desc.join(", ")
                        ));
                    }
                    existing_pk_set.insert(sig);
                    let mut allocated = allocator.allocate(Some(&base_record))?;
                    if let Some(tb) = tags_backup {
                        allocated = allocated.set_in(&[PathKey::from("$tags")], tb);
                    }
                    added.push(allocated);
                } else {
                    let omitted_pk: Vec<String> = pk_fields
                        .iter()
                        .filter(|k| {
                            let in_raw = raw_input
                                .get(k)
                                .map(|v| !matches!(v, Data::Undefined))
                                .unwrap_or(false);
                            let in_assign = parsed_assignments.iter().any(|(steps, _)| {
                                steps.len() == 1 && steps[0] == PathStep::Key((*k).clone())
                            });
                            !in_raw && !in_assign
                        })
                        .cloned()
                        .collect();
                    let mut all_rows = existing_rows.clone();
                    all_rows.extend(added.iter().cloned());
                    if let Data::Map(bmap) = &mut base_record {
                        seed_omitted_primary_keys(
                            &snapshot,
                            &table,
                            &all_rows,
                            bmap,
                            &omitted_pk,
                        );
                    }
                    let mut allocated = allocator.allocate(Some(&base_record))?;
                    if let Some(tb) = tags_backup {
                        allocated = allocated.set_in(&[PathKey::from("$tags")], tb);
                    }
                    let vals: Vec<Data> = pk_fields
                        .iter()
                        .map(|k| allocated.get(k).cloned().unwrap_or(Data::Undefined))
                        .collect();
                    existing_pk_set.insert(fingerprint(&Data::Array(vals)));
                    added.push(allocated);
                }
            }

            let mut current_records = target_source
                .data
                .get("records")
                .and_then(|r| r.as_array())
                .cloned()
                .unwrap_or_default();
            current_records.extend(added.iter().cloned());
            session.update(
                &target_source.id,
                &[PathKey::from("records")],
                Data::Array(current_records),
            );

            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("source".to_string(), Data::Str(target_source.path));
            out.insert("addedCount".to_string(), Data::Int(added.len() as i128));
            out.insert("records".to_string(), Data::Array(added));
            Ok(Data::Map(out))
        }

        "record.set" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let pk_fields: Vec<String> = table
                .source
                .data
                .get("primaryKey")
                .and_then(|pk| pk.get("fields"))
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let where_list: Vec<String> = match op.get("where") {
                Some(Data::Array(arr)) => arr.iter().map(|x| x.text()).collect(),
                Some(Data::Str(s)) if !s.is_empty() => vec![s.clone()],
                _ => Vec::new(),
            };
            let has_pk = op.get("pk").map(|v| !v.is_null_or_undefined()).unwrap_or(false);
            let all = op.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
            if !has_pk && where_list.is_empty() && !all {
                return Err(
                    "更新対象を --pk または --where（全件なら --all）で指定してください。"
                        .to_string(),
                );
            }
            let pk_expected = if has_pk {
                Some(parse_pk_selector(
                    &snapshot,
                    &table.fields,
                    &pk_fields,
                    op.get("pk").unwrap(),
                )?)
            } else {
                None
            };
            let source_filter = op
                .get("source")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(normalize_source_path);

            let parsed_assignments = if let Some(Data::Array(assign_arr)) = op.get("assignments") {
                let mut list = Vec::new();
                for a in assign_arr {
                    if let Some(p_str) = a.get("pathStr").and_then(|v| v.as_str()) {
                        let steps = parse_field_path(p_str)?;
                        let val = a.get("value").cloned().unwrap_or(Data::Null);
                        list.push((steps, val));
                    }
                }
                list
            } else {
                Vec::new()
            };

            let mut updated_count = 0i128;
            let mut next_sources = Vec::with_capacity(snapshot.sources.len());
            for src in &snapshot.sources {
                if !table.sources.iter().any(|ts| ts.id == src.id) {
                    next_sources.push(src.clone());
                    continue;
                }
                if let Some(sf) = &source_filter {
                    if &src.path != sf {
                        next_sources.push(src.clone());
                        continue;
                    }
                }
                ensure_source_editable(src)?;
                let records = src
                    .data
                    .get("records")
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default();
                let mut source_changed = false;
                let mut next_records = Vec::with_capacity(records.len());
                for row in records {
                    if !row.is_object() {
                        next_records.push(row);
                        continue;
                    }
                    if let Some(exp) = &pk_expected {
                        if !matches_pk(&row, &pk_fields, exp) {
                            next_records.push(row);
                            continue;
                        }
                    }
                    if !where_list.is_empty() && !matches_where(&row, &where_list)? {
                        next_records.push(row);
                        continue;
                    }

                    let mut next_row = row.clone();
                    if let Some(Data::Map(set_map)) = op.get("set") {
                        for (raw_key, raw_val) in set_map {
                            let steps = parse_field_path(raw_key)?;
                            let target_field =
                                resolve_field_at_path(&snapshot, &table.fields, &steps);
                            let coerced = coerce_scalar_or_structured(
                                &snapshot,
                                target_field.as_ref(),
                                raw_val,
                            )?;
                            next_row = set_by_path(&next_row, &steps, coerced)?;
                        }
                    }
                    for (steps, val) in &parsed_assignments {
                        next_row = set_by_path(&next_row, steps, val.clone())?;
                    }
                    if let Some(tags_val) = op.get("tags") {
                        if !matches!(tags_val, Data::Undefined) {
                            let tags = strings_from_data(Some(tags_val));
                            if tags.is_empty() {
                                next_row =
                                    next_row.set_in(&[PathKey::from("$tags")], Data::Undefined);
                            } else {
                                next_row = next_row.set_in(
                                    &[PathKey::from("$tags")],
                                    Data::Array(tags.into_iter().map(Data::Str).collect()),
                                );
                            }
                        }
                    }
                    if let Some(add_val) = op.get("addTags") {
                        let add = strings_from_data(Some(add_val));
                        let mut existing = strings_from_data(next_row.get("$tags"));
                        for t in add {
                            if !existing.contains(&t) {
                                existing.push(t);
                            }
                        }
                        next_row = next_row.set_in(
                            &[PathKey::from("$tags")],
                            Data::Array(existing.into_iter().map(Data::Str).collect()),
                        );
                    }
                    if let Some(rem_val) = op.get("removeTags") {
                        let rem: HashSet<String> =
                            strings_from_data(Some(rem_val)).into_iter().collect();
                        let existing = strings_from_data(next_row.get("$tags"));
                        let filtered: Vec<String> =
                            existing.into_iter().filter(|t| !rem.contains(t)).collect();
                        if filtered.is_empty() {
                            next_row =
                                next_row.set_in(&[PathKey::from("$tags")], Data::Undefined);
                        } else {
                            next_row = next_row.set_in(
                                &[PathKey::from("$tags")],
                                Data::Array(filtered.into_iter().map(Data::Str).collect()),
                            );
                        }
                    }

                    updated_count += 1;
                    source_changed = true;
                    next_records.push(next_row);
                }
                if source_changed {
                    let mut ns = src.clone();
                    ns.data = ns
                        .data
                        .set_in(&[PathKey::from("records")], Data::Array(next_records));
                    next_sources.push(ns);
                } else {
                    next_sources.push(src.clone());
                }
            }

            if updated_count == 0 {
                return Err("指定された条件に一致するレコードが見つかりません。".to_string());
            }
            session.snapshot.sources = next_sources;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("updatedCount".to_string(), Data::Int(updated_count));
            Ok(Data::Map(out))
        }

        "record.delete" => {
            let table_name = op
                .get("table")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let table = find_table(&snapshot, &table_name)?;
            let pk_fields: Vec<String> = table
                .source
                .data
                .get("primaryKey")
                .and_then(|pk| pk.get("fields"))
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let where_list: Vec<String> = match op.get("where") {
                Some(Data::Array(arr)) => arr.iter().map(|x| x.text()).collect(),
                Some(Data::Str(s)) if !s.is_empty() => vec![s.clone()],
                _ => Vec::new(),
            };
            let has_pk = op.get("pk").map(|v| !v.is_null_or_undefined()).unwrap_or(false);
            let all = op.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
            if !has_pk && where_list.is_empty() && !all {
                return Err(
                    "削除対象を --pk、--where、または --all で指定してください。".to_string(),
                );
            }
            let pk_expected = if has_pk {
                Some(parse_pk_selector(
                    &snapshot,
                    &table.fields,
                    &pk_fields,
                    op.get("pk").unwrap(),
                )?)
            } else {
                None
            };
            let source_filter = op
                .get("source")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(normalize_source_path);

            let mut deleted_count = 0i128;
            let mut next_sources = Vec::with_capacity(snapshot.sources.len());
            for src in &snapshot.sources {
                if !table.sources.iter().any(|ts| ts.id == src.id) {
                    next_sources.push(src.clone());
                    continue;
                }
                if let Some(sf) = &source_filter {
                    if &src.path != sf {
                        next_sources.push(src.clone());
                        continue;
                    }
                }
                ensure_source_editable(src)?;
                let records = src
                    .data
                    .get("records")
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default();
                let orig_len = records.len();
                let mut remaining = Vec::with_capacity(orig_len);
                for row in records {
                    if !row.is_object() {
                        remaining.push(row);
                        continue;
                    }
                    if let Some(exp) = &pk_expected {
                        if !matches_pk(&row, &pk_fields, exp) {
                            remaining.push(row);
                            continue;
                        }
                    }
                    if !where_list.is_empty() && !matches_where(&row, &where_list)? {
                        remaining.push(row);
                        continue;
                    }
                    deleted_count += 1;
                }
                if remaining.len() != orig_len {
                    let mut ns = src.clone();
                    ns.data = ns
                        .data
                        .set_in(&[PathKey::from("records")], Data::Array(remaining));
                    next_sources.push(ns);
                } else {
                    next_sources.push(src.clone());
                }
            }

            if deleted_count == 0 {
                return Err("削除対象のレコードが見つかりません。".to_string());
            }
            session.snapshot.sources = next_sources;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("table".to_string(), Data::Str(table_name));
            out.insert("deletedCount".to_string(), Data::Int(deleted_count));
            Ok(Data::Map(out))
        }

        "fix" => {
            let target_table = op
                .get("table")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            if let Some(tt) = &target_table {
                ensure_table_editable(&snapshot, tt)?;
                find_table(&snapshot, tt)?;
            }
            let mut touched_files = 0i128;
            let mut table_map = IndexMap::new();
            for t in tables(&snapshot) {
                table_map.insert(t.name.clone(), t);
            }
            let mut next_sources = Vec::with_capacity(snapshot.sources.len());
            for src in &snapshot.sources {
                if src.syntax_error.is_some() || !src.data.is_object() {
                    next_sources.push(src.clone());
                    continue;
                }
                if let Some(tt) = &target_table {
                    let skind = src.data.get("kind").and_then(|v| v.as_str());
                    let stable = src.data.get("table").map(|v| v.text()).unwrap_or_default();
                    if !(matches!(skind, Some("schema") | Some("data")) && &stable == tt) {
                        next_sources.push(src.clone());
                        continue;
                    }
                }

                let mut data = src.data.clone();
                let mut data_changed = false;
                if data.get("kind").and_then(|v| v.as_str()) == Some("schema") {
                    if let Some(fields_arr) = data.get("fields").and_then(|v| v.as_array()).cloned()
                    {
                        let mut f_changed = false;
                        let fixed: Vec<Data> = fields_arr
                            .into_iter()
                            .enumerate()
                            .map(|(i, f)| {
                                if let Data::Map(fmap) = &f {
                                    if !fmap.contains_key("key") {
                                        f_changed = true;
                                        let mut nm = IndexMap::new();
                                        nm.insert("key".to_string(), Data::Int(i as i128));
                                        for (k, v) in fmap {
                                            nm.insert(k.clone(), v.clone());
                                        }
                                        return Data::Map(nm);
                                    }
                                }
                                f
                            })
                            .collect();
                        if f_changed {
                            data = data.set_in(&[PathKey::from("fields")], Data::Array(fixed));
                            data_changed = true;
                        }
                    }
                }
                if data.get("kind").and_then(|v| v.as_str()) == Some("type") {
                    if let Some(fields_arr) = data
                        .get("custom")
                        .and_then(|c| c.get("fields"))
                        .and_then(|v| v.as_array())
                        .cloned()
                    {
                        let mut f_changed = false;
                        let fixed: Vec<Data> = fields_arr
                            .into_iter()
                            .enumerate()
                            .map(|(i, f)| {
                                if let Data::Map(fmap) = &f {
                                    if !fmap.contains_key("key") {
                                        f_changed = true;
                                        let mut nm = IndexMap::new();
                                        nm.insert("key".to_string(), Data::Int(i as i128));
                                        for (k, v) in fmap {
                                            nm.insert(k.clone(), v.clone());
                                        }
                                        return Data::Map(nm);
                                    }
                                }
                                f
                            })
                            .collect();
                        if f_changed {
                            data = data.set_in(
                                &[PathKey::from("custom"), PathKey::from("fields")],
                                Data::Array(fixed),
                            );
                            data_changed = true;
                        }
                    }
                }
                if matches!(
                    data.get("kind").and_then(|v| v.as_str()),
                    Some("schema") | Some("data")
                ) {
                    if let Some(records_arr) =
                        data.get("records").and_then(|v| v.as_array()).cloned()
                    {
                        let tname = data.get("table").map(|v| v.text()).unwrap_or_default();
                        if let Some(tbl) = table_map.get(&tname) {
                            let mut r_changed = false;
                            let next_records: Vec<Data> = records_arr
                                .into_iter()
                                .map(|row| {
                                    let updated = backfill_defined_fields(
                                        &snapshot,
                                        &tbl.fields,
                                        &row,
                                        &HashSet::new(),
                                    );
                                    if updated != row {
                                        r_changed = true;
                                    }
                                    updated
                                })
                                .collect();
                            if r_changed {
                                data = data.set_in(
                                    &[PathKey::from("records")],
                                    Data::Array(next_records),
                                );
                                data_changed = true;
                            }
                        }
                    }
                }

                let mut candidate = src.clone();
                candidate.data = data;
                let formatted = serialize(&candidate, None);
                if data_changed || src.format_text.as_deref() != Some(formatted.as_str()) {
                    touched_files += 1;
                    session.force_dirty.insert(candidate.id.clone());
                    next_sources.push(candidate);
                } else {
                    next_sources.push(src.clone());
                }
            }
            session.snapshot.sources = next_sources;
            let mut out = IndexMap::new();
            out.insert("op".to_string(), Data::Str(kind.to_string()));
            out.insert("touchedFiles".to_string(), Data::Int(touched_files));
            Ok(Data::Map(out))
        }

        _ => Err(format!("未知の操作です: {kind}")),
    }
}
