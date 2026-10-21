use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub const PRIMITIVES: &[&str] = &[
    "bool", "int", "uint", "long", "ulong", "float", "double", "string",
];

#[derive(Debug, Clone)]
pub enum Data {
    Null,
    Undefined,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    Array(Vec<Data>),
    Map(IndexMap<String, Data>),
}

impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Data::Null, Data::Null) => true,
            (Data::Undefined, Data::Undefined) => true,
            (Data::Bool(a), Data::Bool(b)) => a == b,
            (Data::Int(a), Data::Int(b)) => a == b,
            (Data::Float(a), Data::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b && a.is_sign_positive() == b.is_sign_positive()
                }
            }
            (Data::Int(a), Data::Float(b)) | (Data::Float(b), Data::Int(a)) => {
                b.is_finite()
                    && b.fract() == 0.0
                    && *b >= (i64::MIN as f64)
                    && *b <= (u64::MAX as f64)
                    && (*b as i128) == *a
            }
            (Data::Str(a), Data::Str(b)) => a == b,
            (Data::Array(a), Data::Array(b)) => a == b,
            (Data::Map(a), Data::Map(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k) == Some(v))
            }
            _ => false,
        }
    }
}

impl Eq for Data {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathKey {
    Key(String),
    Index(usize),
}

impl From<&str> for PathKey {
    fn from(s: &str) -> Self {
        PathKey::Key(s.to_string())
    }
}

impl From<String> for PathKey {
    fn from(s: String) -> Self {
        PathKey::Key(s)
    }
}

impl From<usize> for PathKey {
    fn from(i: usize) -> Self {
        PathKey::Index(i)
    }
}

impl Data {
    pub fn is_object(&self) -> bool {
        matches!(self, Data::Map(_))
    }

    pub fn is_null_or_undefined(&self) -> bool {
        matches!(self, Data::Null | Data::Undefined)
    }

    pub fn as_map(&self) -> Option<&IndexMap<String, Data>> {
        match self {
            Data::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_map_mut(&mut self) -> Option<&mut IndexMap<String, Data>> {
        match self {
            Data::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Data>> {
        match self {
            Data::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Data::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Data::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Data::Int(n) => Some(*n),
            Data::Float(f)
                if f.is_finite()
                    && f.fract() == 0.0
                    && f.abs() <= 9007199254740991.0 =>
            {
                Some(*f as i128)
            }
            _ => None,
        }
    }

    pub fn parse_bigint(&self) -> Option<i128> {
        match self {
            Data::Int(n) => Some(*n),
            Data::Float(f)
                if f.is_finite()
                    && f.fract() == 0.0
                    && f.abs() <= 9007199254740991.0 =>
            {
                Some(*f as i128)
            }
            Data::Str(s) => s.trim().parse::<i128>().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Data::Float(f) => Some(*f),
            Data::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Data> {
        match self {
            Data::Map(m) => m.get(key),
            _ => None,
        }
    }

    pub fn get_index(&self, idx: usize) -> Option<&Data> {
        match self {
            Data::Array(a) => a.get(idx),
            _ => None,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Data::Null => "null".to_string(),
            Data::Undefined => String::new(),
            Data::Bool(b) => b.to_string(),
            Data::Int(n) => n.to_string(),
            Data::Float(f) => format_float(*f),
            Data::Str(s) => s.clone(),
            Data::Array(a) => format!("[{}]", a.len()),
            Data::Map(_) => "{ … }".to_string(),
        }
    }

    pub fn set_in(&self, path: &[PathKey], next: Data) -> Data {
        if path.is_empty() {
            return next;
        }
        match &path[0] {
            PathKey::Index(idx) => {
                let mut arr = match self {
                    Data::Array(a) => a.clone(),
                    _ => Vec::new(),
                };
                if *idx >= arr.len() {
                    arr.resize(*idx + 1, Data::Undefined);
                }
                let child = arr.get(*idx).cloned().unwrap_or(Data::Undefined);
                arr[*idx] = child.set_in(&path[1..], next);
                Data::Array(arr)
            }
            PathKey::Key(k) => {
                let mut map = match self {
                    Data::Map(m) => m.clone(),
                    _ => IndexMap::new(),
                };
                let child = map.get(k).cloned().unwrap_or(Data::Undefined);
                let updated = child.set_in(&path[1..], next);
                if matches!(updated, Data::Undefined) {
                    map.shift_remove(k);
                } else {
                    map.insert(k.clone(), updated);
                }
                Data::Map(map)
            }
        }
    }

    pub fn to_json_value(&self) -> Value {
        match self {
            Data::Null | Data::Undefined => Value::Null,
            Data::Bool(b) => Value::Bool(*b),
            Data::Int(n) => {
                if *n >= i64::MIN as i128 && *n <= i64::MAX as i128 {
                    json!(*n as i64)
                } else if *n >= 0 && *n <= u64::MAX as i128 {
                    json!(*n as u64)
                } else {
                    Value::String(n.to_string())
                }
            }
            Data::Float(f) => serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Data::Str(s) => Value::String(s.clone()),
            Data::Array(arr) => Value::Array(arr.iter().map(|v| v.to_json_value()).collect()),
            Data::Map(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    if !matches!(v, Data::Undefined) {
                        obj.insert(k.clone(), v.to_json_value());
                    }
                }
                Value::Object(obj)
            }
        }
    }

    pub fn to_dotnet_json_value(&self) -> Value {
        match self {
            Data::Null | Data::Undefined => Value::Null,
            Data::Bool(b) => Value::Bool(*b),
            Data::Int(n) => Value::String(n.to_string()),
            Data::Float(f) => serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Data::Str(s) => Value::String(s.clone()),
            Data::Array(arr) => {
                Value::Array(arr.iter().map(|v| v.to_dotnet_json_value()).collect())
            }
            Data::Map(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    if !matches!(v, Data::Undefined) {
                        obj.insert(k.clone(), v.to_dotnet_json_value());
                    }
                }
                Value::Object(obj)
            }
        }
    }

    pub fn from_json_value(v: &Value, int_as_bigint: bool) -> Data {
        match v {
            Value::Null => Data::Null,
            Value::Bool(b) => Data::Bool(*b),
            Value::Number(n) => {
                if int_as_bigint {
                    if let Some(i) = n.as_i64() {
                        return Data::Int(i as i128);
                    }
                    if let Some(u) = n.as_u64() {
                        return Data::Int(u as i128);
                    }
                }
                if let Some(f) = n.as_f64() {
                    Data::Float(f)
                } else {
                    Data::Null
                }
            }
            Value::String(s) => Data::Str(s.clone()),
            Value::Array(arr) => Data::Array(
                arr.iter()
                    .map(|item| Data::from_json_value(item, int_as_bigint))
                    .collect(),
            ),
            Value::Object(obj) => {
                let mut map = IndexMap::new();
                for (k, val) in obj {
                    map.insert(k.clone(), Data::from_json_value(val, int_as_bigint));
                }
                Data::Map(map)
            }
        }
    }
}

pub fn format_float(f: f64) -> String {
    if f.is_nan() {
        "NaN".to_string()
    } else if f == f64::INFINITY {
        "Infinity".to_string()
    } else if f == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if f == 0.0 {
        "0".to_string()
    } else {
        f.to_string()
    }
}

pub fn uid() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<i64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub array: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nullable: bool,
}

impl Field {
    pub fn to_data(&self) -> Data {
        let mut map = IndexMap::new();
        if let Some(k) = self.key {
            map.insert("key".to_string(), Data::Int(k as i128));
        }
        map.insert("name".to_string(), Data::Str(self.name.clone()));
        map.insert("type".to_string(), Data::Str(self.r#type.clone()));
        if self.array {
            map.insert("array".to_string(), Data::Bool(true));
        }
        if self.nullable {
            map.insert("nullable".to_string(), Data::Bool(true));
        }
        Data::Map(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: String,
    pub path: String,
    pub data: Data,
    pub raw: Option<String>,
    pub format_text: Option<String>,
    pub syntax_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishConfig {
    pub csharp: String,
    pub binary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub profiles: IndexMap<String, Profile>,
    pub publish: PublishConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub sources: Vec<Source>,
    pub folders: Vec<String>,
    pub config: Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedFile {
    pub id: String,
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub root: String,
    pub token: String,
    pub snapshot: Snapshot,
    pub saved: Vec<SavedFile>,
    pub config_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRequest {
    pub token: String,
    pub files: Vec<SavedFile>,
    pub folders: Vec<String>,
    pub config: Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveResult {
    pub files: Vec<SavedFile>,
    #[serde(rename = "configText")]
    pub config_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub source: String,
    pub path: Vec<PathKey>,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub source: Source,
    pub name: String,
    pub fields: Vec<Field>,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildResult {
    pub directory: String,
    pub revision: i64,
    pub profile: String,
    pub tables: usize,
    pub records: usize,
    pub log: String,
    pub files: Vec<String>,
    pub hashes: IndexMap<String, String>,
}

pub fn default_config(name: &str) -> Config {
    let mut profiles = IndexMap::new();
    profiles.insert(
        "production".to_string(),
        Profile {
            include: Vec::new(),
            exclude: vec!["debug".to_string()],
        },
    );
    Config {
        name: if name.is_empty() {
            "新しいプロジェクト".to_string()
        } else {
            name.to_string()
        },
        profiles,
        publish: PublishConfig {
            csharp: String::new(),
            binary: String::new(),
        },
    }
}

pub fn fields_of(source: &Source) -> Vec<Field> {
    let raw_list = if let Some(custom) = source.data.get("custom") {
        custom.get("fields").and_then(|v| v.as_array())
    } else {
        source.data.get("fields").and_then(|v| v.as_array())
    };
    let Some(items) = raw_list else {
        return Vec::new();
    };
    items
        .iter()
        .map(|f| {
            let name = f
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let r#type = f
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let key = f.get("key").and_then(|v| v.as_i128()).map(|k| k as i64);
            let array = f.get("array").and_then(|v| v.as_bool()).unwrap_or(false);
            let nullable = f.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);
            Field {
                name,
                r#type,
                key,
                array,
                nullable,
            }
        })
        .collect()
}

pub fn type_kind(source: Option<&Source>) -> Option<&'static str> {
    let s = source?;
    for kind in ["enum", "flags", "custom", "valueObject"] {
        if let Some(val) = s.data.get(kind) {
            if !val.is_null_or_undefined() && val != &Data::Bool(false) {
                return Some(kind);
            }
        }
    }
    None
}

pub fn tables(snapshot: &Snapshot) -> Vec<TableInfo> {
    let mut data_by_table: IndexMap<String, Vec<Source>> = IndexMap::new();
    for source in &snapshot.sources {
        let kind = source.data.get("kind").and_then(|v| v.as_str());
        if matches!(kind, Some("schema") | Some("data")) {
            let name = source
                .data
                .get("table")
                .map(|v| v.text())
                .unwrap_or_default();
            data_by_table.entry(name).or_default().push(source.clone());
        }
    }
    snapshot
        .sources
        .iter()
        .filter(|s| s.data.get("kind").and_then(|v| v.as_str()) == Some("schema"))
        .map(|source| {
            let name = source
                .data
                .get("table")
                .map(|v| v.text())
                .unwrap_or_default();
            let sources = data_by_table.get(&name).cloned().unwrap_or_default();
            TableInfo {
                source: source.clone(),
                name,
                fields: fields_of(source),
                sources,
            }
        })
        .collect()
}

pub fn type_sources(snapshot: &Snapshot) -> Vec<Source> {
    snapshot
        .sources
        .iter()
        .filter(|s| s.data.get("kind").and_then(|v| v.as_str()) == Some("type"))
        .cloned()
        .collect()
}

pub fn type_source(snapshot: &Snapshot, name: &str) -> Option<Source> {
    snapshot
        .sources
        .iter()
        .find(|s| {
            s.data.get("kind").and_then(|v| v.as_str()) == Some("type")
                && s.data.get("name").map(|v| v.text()).as_deref() == Some(name)
        })
        .cloned()
}

pub fn selected(record: &Data, profile: Option<&Profile>) -> bool {
    let Some(profile) = profile else {
        return true;
    };
    let tags: Vec<&str> = record
        .get("$tags")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let include_ok = profile.include.is_empty()
        || profile
            .include
            .iter()
            .any(|t| tags.contains(&t.as_str()));
    let exclude_ok = !profile
        .exclude
        .iter()
        .any(|t| tags.contains(&t.as_str()));
    include_ok && exclude_ok
}
