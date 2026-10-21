use crate::model::{
    BuildResult, Config, Data, LoadedProject, PathKey, Profile, PublishConfig, SaveRequest,
    SaveResult, SavedFile, Snapshot, Source,
};
use indexmap::IndexMap;
use serde_json::{json, Value};

pub fn encode_data(value: &Data) -> String {
    let mut integers: Vec<Value> = Vec::new();
    let data = walk_encode(value, &[], &mut integers);
    json!([data, integers]).to_string()
}

fn walk_encode(value: &Data, path: &[PathKey], integers: &mut Vec<Value>) -> Value {
    match value {
        Data::Null => Value::Null,
        Data::Undefined => {
            integers.push(json!([path_to_json(path), "", "undefined"]));
            Value::Null
        }
        Data::Bool(b) => Value::Bool(*b),
        Data::Int(n) => {
            integers.push(json!([path_to_json(path), n.to_string()]));
            Value::Null
        }
        Data::Float(f) => {
            if !f.is_finite() || (*f == 0.0 && f.is_sign_negative()) {
                let repr = if *f == 0.0 && f.is_sign_negative() {
                    "-0".to_string()
                } else if f.is_nan() {
                    "NaN".to_string()
                } else if *f == f64::INFINITY {
                    "Infinity".to_string()
                } else {
                    "-Infinity".to_string()
                };
                integers.push(json!([path_to_json(path), repr, "number"]));
                Value::Null
            } else if f.fract() == 0.0 && *f >= (i64::MIN as f64) && *f <= (i64::MAX as f64) {
                json!(*f as i64)
            } else {
                serde_json::Number::from_f64(*f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
        }
        Data::Str(s) => Value::String(s.clone()),
        Data::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(PathKey::Index(i));
                out.push(walk_encode(item, &child_path, integers));
            }
            Value::Array(out)
        }
        Data::Map(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let mut child_path = path.to_vec();
                child_path.push(PathKey::Key(k.clone()));
                out.insert(k.clone(), walk_encode(v, &child_path, integers));
            }
            Value::Object(out)
        }
    }
}

fn path_to_json(path: &[PathKey]) -> Vec<Value> {
    path.iter()
        .map(|k| match k {
            PathKey::Key(s) => Value::String(s.clone()),
            PathKey::Index(i) => json!(*i),
        })
        .collect()
}

pub fn decode_data(text: &str) -> Result<Data, String> {
    let parsed: Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid wire payload: {e}"))?;
    let pair = parsed
        .as_array()
        .ok_or_else(|| "Wire payload must be a 2-element array".to_string())?;
    if pair.len() < 2 {
        return Err("Wire payload must contain [data, integers]".to_string());
    }
    let mut data = Data::from_json_value(&pair[0], false);
    if let Some(integers) = pair[1].as_array() {
        for entry in integers {
            let Some(tuple) = entry.as_array() else {
                continue;
            };
            if tuple.len() < 2 {
                continue;
            }
            let Some(raw_path) = tuple[0].as_array() else {
                continue;
            };
            let path: Vec<PathKey> = raw_path
                .iter()
                .filter_map(|p| {
                    if let Some(s) = p.as_str() {
                        Some(PathKey::Key(s.to_string()))
                    } else {
                        p.as_u64().map(|i| PathKey::Index(i as usize))
                    }
                })
                .collect();
            let val_str = tuple[1].as_str().unwrap_or("");
            let kind = tuple.get(2).and_then(|v| v.as_str());
            let restored = match kind {
                Some("undefined") => Data::Undefined,
                Some("number") => match val_str {
                    "-0" => Data::Float(-0.0),
                    "NaN" => Data::Float(f64::NAN),
                    "Infinity" => Data::Float(f64::INFINITY),
                    "-Infinity" => Data::Float(f64::NEG_INFINITY),
                    other => Data::Float(other.parse::<f64>().unwrap_or(f64::NAN)),
                },
                _ => match val_str.parse::<i128>() {
                    Ok(n) => Data::Int(n),
                    Err(_) => Data::Str(val_str.to_string()),
                },
            };
            if path.is_empty() {
                data = restored;
            } else {
                data = set_in_preserving_undefined(&data, &path, restored);
            }
        }
    }
    Ok(data)
}

fn set_in_preserving_undefined(root: &Data, path: &[PathKey], next: Data) -> Data {
    if path.is_empty() {
        return next;
    }
    match &path[0] {
        PathKey::Index(idx) => {
            let mut arr = match root {
                Data::Array(a) => a.clone(),
                _ => Vec::new(),
            };
            if *idx >= arr.len() {
                arr.resize(*idx + 1, Data::Null);
            }
            let child = arr.get(*idx).cloned().unwrap_or(Data::Null);
            arr[*idx] = set_in_preserving_undefined(&child, &path[1..], next);
            Data::Array(arr)
        }
        PathKey::Key(k) => {
            let mut map = match root {
                Data::Map(m) => m.clone(),
                _ => IndexMap::new(),
            };
            let child = map.get(k).cloned().unwrap_or(Data::Null);
            let updated = set_in_preserving_undefined(&child, &path[1..], next);
            map.insert(k.clone(), updated);
            Data::Map(map)
        }
    }
}

pub fn config_to_data(config: &Config) -> Data {
    let mut profiles = IndexMap::new();
    for (name, p) in &config.profiles {
        let mut pm = IndexMap::new();
        pm.insert(
            "include".to_string(),
            Data::Array(p.include.iter().cloned().map(Data::Str).collect()),
        );
        pm.insert(
            "exclude".to_string(),
            Data::Array(p.exclude.iter().cloned().map(Data::Str).collect()),
        );
        profiles.insert(name.clone(), Data::Map(pm));
    }
    let mut publish = IndexMap::new();
    publish.insert("csharp".to_string(), Data::Str(config.publish.csharp.clone()));
    publish.insert("binary".to_string(), Data::Str(config.publish.binary.clone()));

    let mut map = IndexMap::new();
    map.insert("name".to_string(), Data::Str(config.name.clone()));
    map.insert("profiles".to_string(), Data::Map(profiles));
    map.insert("publish".to_string(), Data::Map(publish));
    Data::Map(map)
}

pub fn config_from_data(data: &Data) -> Result<Config, String> {
    let name = data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut profiles = IndexMap::new();
    if let Some(pmap) = data.get("profiles").and_then(|v| v.as_map()) {
        for (k, v) in pmap {
            let include = v
                .get("include")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let exclude = v
                .get("exclude")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            profiles.insert(k.clone(), Profile { include, exclude });
        }
    }
    let publish_data = data.get("publish");
    let csharp = publish_data
        .and_then(|p| p.get("csharp"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let binary = publish_data
        .and_then(|p| p.get("binary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(Config {
        name,
        profiles,
        publish: PublishConfig { csharp, binary },
    })
}

pub fn publish_config_from_data(data: &Data) -> PublishConfig {
    let csharp = data
        .get("csharp")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let binary = data
        .get("binary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    PublishConfig { csharp, binary }
}

pub fn source_to_data(source: &Source) -> Data {
    let mut map = IndexMap::new();
    map.insert("id".to_string(), Data::Str(source.id.clone()));
    map.insert("path".to_string(), Data::Str(source.path.clone()));
    map.insert("data".to_string(), source.data.clone());
    if let Some(raw) = &source.raw {
        map.insert("raw".to_string(), Data::Str(raw.clone()));
    }
    if let Some(ft) = &source.format_text {
        map.insert("formatText".to_string(), Data::Str(ft.clone()));
    }
    if let Some(err) = &source.syntax_error {
        map.insert("syntaxError".to_string(), Data::Str(err.clone()));
    }
    Data::Map(map)
}

pub fn source_from_data(data: &Data) -> Result<Source, String> {
    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let path = data
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let inner = data.get("data").cloned().unwrap_or(Data::Null);
    let raw = data
        .get("raw")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let format_text = data
        .get("formatText")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let syntax_error = data
        .get("syntaxError")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(Source {
        id,
        path,
        data: inner,
        raw,
        format_text,
        syntax_error,
    })
}

pub fn snapshot_to_data(snapshot: &Snapshot) -> Data {
    let mut map = IndexMap::new();
    map.insert(
        "sources".to_string(),
        Data::Array(snapshot.sources.iter().map(source_to_data).collect()),
    );
    map.insert(
        "folders".to_string(),
        Data::Array(snapshot.folders.iter().cloned().map(Data::Str).collect()),
    );
    map.insert("config".to_string(), config_to_data(&snapshot.config));
    Data::Map(map)
}

pub fn snapshot_from_data(data: &Data) -> Result<Snapshot, String> {
    let sources = data
        .get("sources")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| source_from_data(s).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let folders = data
        .get("folders")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| f.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let config = config_from_data(data.get("config").unwrap_or(&Data::Null))?;
    Ok(Snapshot {
        sources,
        folders,
        config,
    })
}

pub fn saved_file_to_data(file: &SavedFile) -> Data {
    let mut map = IndexMap::new();
    map.insert("id".to_string(), Data::Str(file.id.clone()));
    map.insert("path".to_string(), Data::Str(file.path.clone()));
    map.insert("text".to_string(), Data::Str(file.text.clone()));
    Data::Map(map)
}

pub fn saved_file_from_data(data: &Data) -> SavedFile {
    SavedFile {
        id: data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        path: data
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        text: data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

pub fn loaded_project_to_data(project: &LoadedProject) -> Data {
    let mut map = IndexMap::new();
    map.insert("root".to_string(), Data::Str(project.root.clone()));
    map.insert("token".to_string(), Data::Str(project.token.clone()));
    map.insert("snapshot".to_string(), snapshot_to_data(&project.snapshot));
    map.insert(
        "saved".to_string(),
        Data::Array(project.saved.iter().map(saved_file_to_data).collect()),
    );
    map.insert(
        "configText".to_string(),
        Data::Str(project.config_text.clone()),
    );
    Data::Map(map)
}

pub fn save_request_from_data(data: &Data) -> Result<SaveRequest, String> {
    let token = data
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let files = data
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(saved_file_from_data).collect())
        .unwrap_or_default();
    let folders = data
        .get("folders")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| f.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let config = config_from_data(data.get("config").unwrap_or(&Data::Null))?;
    Ok(SaveRequest {
        token,
        files,
        folders,
        config,
    })
}

pub fn save_result_to_data(result: &SaveResult) -> Data {
    let mut map = IndexMap::new();
    map.insert(
        "files".to_string(),
        Data::Array(result.files.iter().map(saved_file_to_data).collect()),
    );
    map.insert(
        "configText".to_string(),
        Data::Str(result.config_text.clone()),
    );
    Data::Map(map)
}

pub fn build_result_to_data(result: &BuildResult) -> Data {
    let mut map = IndexMap::new();
    map.insert("directory".to_string(), Data::Str(result.directory.clone()));
    map.insert(
        "revision".to_string(),
        Data::Float(result.revision as f64),
    );
    map.insert("profile".to_string(), Data::Str(result.profile.clone()));
    map.insert("tables".to_string(), Data::Float(result.tables as f64));
    map.insert("records".to_string(), Data::Float(result.records as f64));
    map.insert("log".to_string(), Data::Str(result.log.clone()));
    map.insert(
        "files".to_string(),
        Data::Array(result.files.iter().cloned().map(Data::Str).collect()),
    );
    let mut hashes = IndexMap::new();
    for (k, v) in &result.hashes {
        hashes.insert(k.clone(), Data::Str(v.clone()));
    }
    map.insert("hashes".to_string(), Data::Map(hashes));
    Data::Map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_preserves_uint64_and_special_numbers() {
        let mut nested = IndexMap::new();
        nested.insert("value".to_string(), Data::Int(2));
        let mut map = IndexMap::new();
        map.insert("n".to_string(), Data::Int(18446744073709551615));
        map.insert("min".to_string(), Data::Int(-9223372036854775808));
        map.insert(
            "array".to_string(),
            Data::Array(vec![Data::Int(1), Data::Null]),
        );
        map.insert("__bigint".to_string(), Data::Str("user-data".to_string()));
        map.insert("nested".to_string(), Data::Map(nested));
        let val = Data::Map(map);
        let encoded = encode_data(&val);
        let decoded = decode_data(&encoded).unwrap();
        assert_eq!(decoded, val);

        let mut specials = IndexMap::new();
        specials.insert("nan".to_string(), Data::Float(f64::NAN));
        specials.insert("positive".to_string(), Data::Float(f64::INFINITY));
        specials.insert("negative".to_string(), Data::Float(f64::NEG_INFINITY));
        specials.insert("zero".to_string(), Data::Float(-0.0));
        specials.insert("unset".to_string(), Data::Undefined);
        let spec_val = Data::Map(specials);
        let spec_dec = decode_data(&encode_data(&spec_val)).unwrap();
        assert_eq!(spec_dec, spec_val);
    }
}
