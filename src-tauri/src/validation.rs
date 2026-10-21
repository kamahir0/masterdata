use crate::model::{
    fields_of, tables, type_kind, type_source, type_sources, Data, Field, PathKey, Problem,
    Snapshot, Source, PRIMITIVES,
};
use crate::schema::{in_profile, primitive_of};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub fn int_range(primitive: &str) -> Option<(i128, i128)> {
    match primitive {
        "int" => Some((-2147483648, 2147483647)),
        "uint" => Some((0, 4294967295)),
        "long" => Some((-9223372036854775808, 9223372036854775807)),
        "ulong" => Some((0, 18446744073709551615)),
        _ => None,
    }
}

fn fingerprint_json(value: &Data) -> Value {
    match value {
        Data::Null | Data::Undefined => Value::Null,
        Data::Bool(b) => Value::Bool(*b),
        Data::Int(n) => json!({ "$integer": n.to_string() }),
        Data::Float(f) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() <= 9007199254740991.0 {
                json!({ "$integer": (*f as i128).to_string() })
            } else {
                serde_json::Number::from_f64(*f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
        }
        Data::Str(s) => Value::String(s.clone()),
        Data::Array(arr) => Value::Array(arr.iter().map(fingerprint_json).collect()),
        Data::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), fingerprint_json(v));
            }
            Value::Object(obj)
        }
    }
}

pub fn fingerprint(value: &Data) -> String {
    serde_json::to_string(&fingerprint_json(value)).unwrap_or_default()
}

pub fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn public_name(name: &str) -> String {
    let mut out = String::new();
    let mut cap_next = true;
    for c in name.chars() {
        if matches!(c, '-' | '_' | ' ') {
            cap_next = true;
        } else if cap_next && c.is_ascii_alphabetic() {
            out.push(c.to_ascii_uppercase());
            cap_next = false;
        } else {
            out.push(c);
            cap_next = false;
        }
    }
    out
}

pub fn validate(input: &Snapshot, profile: &str) -> Vec<Problem> {
    let snapshot = if !profile.is_empty() {
        in_profile(input, profile)
    } else {
        input.clone()
    };
    let mut problems: Vec<Problem> = Vec::new();
    let mut add = |source: &Source, path: Vec<PathKey>, message: String, code: &str| {
        if problems.len() < 2000 {
            problems.push(Problem {
                source: source.id.clone(),
                path,
                message,
                code: code.to_string(),
            });
        }
    };

    let all_tables = tables(&snapshot);
    let all_types = type_sources(&snapshot);
    let mut names: HashMap<String, Source> = HashMap::new();

    for source in &snapshot.sources {
        if let Some(err) = &source.syntax_error {
            add(source, vec![], err.clone(), "YAML");
            continue;
        }
        if !source.data.is_object() {
            add(
                source,
                vec![],
                "定義は項目と値を持つ形式にしてください。".to_string(),
                "SHAPE",
            );
            continue;
        }
        let kind = source.data.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(kind, "schema" | "data" | "type") {
            add(
                source,
                vec![PathKey::from("kind")],
                "種類はschema / data / typeを指定してください。".to_string(),
                "KIND",
            );
        }
        if kind == "data" {
            let tbl = source.data.get("table").map(|v| v.text()).unwrap_or_default();
            if !all_tables.iter().any(|t| t.name == tbl) {
                add(
                    source,
                    vec![PathKey::from("table")],
                    format!("表「{tbl}」が見つかりません。"),
                    "TABLE",
                );
            }
        }
        let decl_name_opt = if kind == "schema" {
            Some(source.data.get("table"))
        } else if kind == "type" {
            Some(source.data.get("name"))
        } else {
            None
        };
        if let Some(decl_val) = decl_name_opt {
            let name_str = decl_val.and_then(|v| v.as_str()).unwrap_or("");
            if name_str.is_empty() {
                add(source, vec![], "名前を入力してください。".to_string(), "NAME");
            } else if names.contains_key(&format!("{kind}:{name_str}")) {
                add(
                    source,
                    vec![],
                    format!("名前「{name_str}」が重複しています。"),
                    "NAME",
                );
            }
            let key_str = decl_val.map(|v| v.text()).unwrap_or_default();
            names.insert(format!("{kind}:{key_str}"), source.clone());

            let generated_name = if kind == "schema" {
                source
                    .data
                    .get("csharpName")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| public_name(&key_str))
            } else {
                name_str.to_string()
            };
            if !valid_identifier(&generated_name) {
                add(
                    source,
                    vec![],
                    "生成する型名は英字・数字・_で指定してください。".to_string(),
                    "NAME",
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_field_value(
        snapshot: &Snapshot,
        source: &Source,
        path: Vec<PathKey>,
        field: &Field,
        value: Option<&Data>,
        seen: &HashSet<String>,
        add: &mut dyn FnMut(&Source, Vec<PathKey>, String, &str),
    ) {
        let Some(val) = value else {
            add(
                source,
                path,
                "値がありません。入力するか初期値を作成してください。".to_string(),
                "VALUE",
            );
            return;
        };
        if matches!(val, Data::Undefined) {
            add(
                source,
                path,
                "値がありません。入力するか初期値を作成してください。".to_string(),
                "VALUE",
            );
            return;
        }
        if matches!(val, Data::Null) {
            if !field.nullable {
                add(
                    source,
                    path,
                    "値が必要です。「値を作成」から初期値を用意できます。".to_string(),
                    "VALUE",
                );
            }
            return;
        }
        if field.array {
            let Some(arr) = val.as_array() else {
                add(
                    source,
                    path,
                    "配列の値が必要です。配列を開いて修復してください。".to_string(),
                    "VALUE",
                );
                return;
            };
            let mut elem_field = field.clone();
            elem_field.array = false;
            elem_field.nullable = false;
            for (i, item) in arr.iter().enumerate() {
                let mut sub_path = path.clone();
                sub_path.push(PathKey::from(i));
                check_field_value(snapshot, source, sub_path, &elem_field, Some(item), seen, add);
            }
            return;
        }

        let t_src = type_source(snapshot, &field.r#type);
        let kind = type_kind(t_src.as_ref());
        let primitive = primitive_of(snapshot, field);

        if primitive == "string" {
            if val.as_str().is_none() {
                add(
                    source,
                    path,
                    "文字列を入力してください。".to_string(),
                    "VALUE",
                );
            }
            return;
        }
        if primitive == "bool" {
            if val.as_bool().is_none() {
                add(
                    source,
                    path,
                    "真偽値を選択してください。".to_string(),
                    "VALUE",
                );
            }
            return;
        }
        if let Some((min, max)) = int_range(&primitive) {
            let ok = val
                .as_i128()
                .map(|n| n >= min && n <= max)
                .unwrap_or(false);
            if !ok {
                add(
                    source,
                    path,
                    format!("{primitive}の整数（{min} ～ {max}）を入力してください。"),
                    "VALUE",
                );
            }
            return;
        }
        if primitive == "float" || primitive == "double" {
            let ok = val
                .as_f64()
                .map(|f| {
                    f.is_finite()
                        && (primitive != "float" || f.abs() <= 3.4028234663852886e38)
                })
                .unwrap_or(false);
            if !ok {
                add(
                    source,
                    path,
                    "範囲内の有限な数値を入力してください。".to_string(),
                    "VALUE",
                );
            }
            return;
        }
        if kind == Some("enum") {
            let t = t_src.as_ref().unwrap();
            let members = t
                .data
                .get("enum")
                .and_then(|e| e.get("members"))
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let ok = val
                .as_str()
                .map(|s| {
                    members
                        .iter()
                        .any(|m| m.get("name").and_then(|v| v.as_str()) == Some(s))
                })
                .unwrap_or(false);
            if !ok {
                add(
                    source,
                    path,
                    "定義されたEnumの候補を選択してください。".to_string(),
                    "VALUE",
                );
            }
            return;
        }
        if kind == Some("flags") {
            let t = t_src.as_ref().unwrap();
            let member_names: Vec<String> = t
                .data
                .get("flags")
                .and_then(|e| e.get("members"))
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("name").and_then(|v| v.as_str()).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let ok = if let Some(arr) = val.as_array() {
                let strs: Vec<&str> = arr.iter().filter_map(|x| x.as_str()).collect();
                let unique: HashSet<&str> = strs.iter().copied().collect();
                !arr.is_empty()
                    && strs.len() == arr.len()
                    && strs.iter().all(|s| member_names.iter().any(|m| m == s))
                    && unique.len() == strs.len()
                    && !(strs.len() > 1 && strs.contains(&"None"))
            } else {
                false
            };
            if !ok {
                add(
                    source,
                    path,
                    "Flagsの候補を選択してください。「なし」は単独で選択します。".to_string(),
                    "VALUE",
                );
            }
            return;
        }
        if kind == Some("custom") {
            if !val.is_object() {
                add(
                    source,
                    path,
                    "構造体の値が必要です。開いて修復してください。".to_string(),
                    "VALUE",
                );
                return;
            }
            if seen.contains(&field.r#type) {
                add(
                    source,
                    path,
                    "型の循環があります。".to_string(),
                    "VALUE",
                );
                return;
            }
            let mut next_seen = seen.clone();
            next_seen.insert(field.r#type.clone());
            for f in fields_of(t_src.as_ref().unwrap()) {
                let mut sub_path = path.clone();
                sub_path.push(PathKey::from(f.name.clone()));
                check_field_value(
                    snapshot,
                    source,
                    sub_path,
                    &f,
                    val.get(&f.name),
                    &next_seen,
                    add,
                );
            }
            return;
        }
        let t_label = if field.r#type.is_empty() {
            "未選択"
        } else {
            &field.r#type
        };
        add(
            source,
            path,
            format!("型「{t_label}」が見つかりません。"),
            "TYPE",
        );
    }

    let mut schema_and_types: Vec<Source> = all_tables.iter().map(|t| t.source.clone()).collect();
    schema_and_types.extend(all_types.clone());

    for source in &schema_and_types {
        let fields = fields_of(source);
        let mut field_names = HashSet::new();
        let mut field_keys = HashSet::new();
        let container: Vec<PathKey> = if source.data.get("custom").is_some() {
            vec![PathKey::from("custom"), PathKey::from("fields")]
        } else {
            vec![PathKey::from("fields")]
        };

        for (i, field) in fields.iter().enumerate() {
            let mut name_path = container.clone();
            name_path.push(PathKey::from(i));
            name_path.push(PathKey::from("name"));
            if field.name.is_empty() || field_names.contains(&field.name) {
                add(
                    source,
                    name_path.clone(),
                    "項目名が空か重複しています。".to_string(),
                    "FIELD",
                );
            }
            if !valid_identifier(&field.name) {
                add(
                    source,
                    name_path,
                    "項目名は英字・数字・_で指定してください。".to_string(),
                    "FIELD",
                );
            }
            field_names.insert(field.name.clone());

            let key = field.key.unwrap_or(i as i64);
            let mut key_path = container.clone();
            key_path.push(PathKey::from(i));
            key_path.push(PathKey::from("key"));
            if key < 0 || field_keys.contains(&key) {
                add(
                    source,
                    key_path,
                    "シリアライズ番号は重複しない0以上の整数にしてください。".to_string(),
                    "KEY",
                );
            }
            field_keys.insert(key);

            if !PRIMITIVES.contains(&field.r#type.as_str())
                && type_source(&snapshot, &field.r#type).is_none()
            {
                let mut type_path = container.clone();
                type_path.push(PathKey::from(i));
                type_path.push(PathKey::from("type"));
                add(
                    source,
                    type_path,
                    format!("型「{}」が見つかりません。", field.r#type),
                    "TYPE",
                );
            }
        }

        let kind = type_kind(Some(source));
        let src_kind = source.data.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if src_kind == "type" {
            let defined_count = ["enum", "flags", "custom", "valueObject"]
                .iter()
                .filter(|k| {
                    source
                        .data
                        .get(**k)
                        .map(|v| !v.is_null_or_undefined() && v != &Data::Bool(false))
                        .unwrap_or(false)
                })
                .count();
            if defined_count != 1 {
                add(
                    source,
                    vec![],
                    "型の種類を一つ定義してください。ソースから修正できます。".to_string(),
                    "TYPE",
                );
            }
        }
        if (src_kind == "schema"
            && source.data.get("fields").and_then(|v| v.as_array()).is_none())
            || (kind == Some("custom")
                && source
                    .data
                    .get("custom")
                    .and_then(|c| c.get("fields"))
                    .and_then(|v| v.as_array())
                    .is_none())
        {
            add(
                source,
                vec![],
                "フィールドの一覧は配列にしてください。".to_string(),
                "FIELD",
            );
        }

        for k in ["secondaryKeys", "references"] {
            if let Some(val) = source.data.get(k) {
                if !matches!(val, Data::Undefined) && val.as_array().is_none() {
                    add(
                        source,
                        vec![PathKey::from(k)],
                        "キー・参照の一覧は配列にしてください。ソースから修正できます。".to_string(),
                        "SHAPE",
                    );
                }
            }
        }

        if matches!(kind, Some("enum") | Some("flags")) {
            let k_str = kind.unwrap();
            let def = source.data.get(k_str).cloned().unwrap_or(Data::Null);
            let members = def
                .get("members")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut m_names = HashSet::new();
            let mut m_values = HashSet::new();
            if members.is_empty() {
                add(
                    source,
                    vec![PathKey::from(k_str), PathKey::from("members")],
                    "候補を追加してください。".to_string(),
                    "ENUM",
                );
            }
            let underlying = def
                .get("underlying")
                .and_then(|v| v.as_str())
                .unwrap_or("int");
            let range = int_range(underlying);
            for (i, m) in members.iter().enumerate() {
                let m_name = m.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if !valid_identifier(m_name) || m_names.contains(m_name) {
                    add(
                        source,
                        vec![
                            PathKey::from(k_str),
                            PathKey::from("members"),
                            PathKey::from(i),
                        ],
                        "候補名が空・不正・重複しています。".to_string(),
                        "ENUM",
                    );
                }
                m_names.insert(m_name.to_string());
                let val_str = m.get("value").map(|v| v.text()).unwrap_or_default();
                if m_values.contains(&val_str) {
                    add(
                        source,
                        vec![
                            PathKey::from(k_str),
                            PathKey::from("members"),
                            PathKey::from(i),
                        ],
                        "候補の数値が重複しています。".to_string(),
                        "ENUM",
                    );
                }
                m_values.insert(val_str);

                let parsed_n = m.get("value").and_then(|v| v.parse_bigint());
                if let (Some(n), Some((min, max))) = (parsed_n, range) {
                    if n < min || n > max {
                        add(
                            source,
                            vec![
                                PathKey::from(k_str),
                                PathKey::from("members"),
                                PathKey::from(i),
                            ],
                            "整数の値と基になる型を確認してください。".to_string(),
                            "ENUM",
                        );
                    } else if k_str == "flags" {
                        let width: u32 = if underlying == "long" || underlying == "ulong" {
                            64
                        } else {
                            32
                        };
                        let bits: u128 = if n < 0 {
                            ((1i128 << width) + n) as u128
                        } else {
                            n as u128
                        };
                        let bad_flag = if m_name == "None" {
                            n != 0
                        } else {
                            bits == 0 || (bits & (bits - 1)) != 0
                        };
                        if bad_flag {
                            add(
                                source,
                                vec![
                                    PathKey::from(k_str),
                                    PathKey::from("members"),
                                    PathKey::from(i),
                                ],
                                "FlagsはNone=0、その他は重複しない単一bitにしてください。"
                                    .to_string(),
                                "FLAGS",
                            );
                        }
                    }
                } else {
                    add(
                        source,
                        vec![
                            PathKey::from(k_str),
                            PathKey::from("members"),
                            PathKey::from(i),
                        ],
                        "整数の値と基になる型を確認してください。".to_string(),
                        "ENUM",
                    );
                }
            }
            if k_str == "flags" && !m_names.contains("None") {
                add(
                    source,
                    vec![PathKey::from(k_str), PathKey::from("members")],
                    "None=0を追加してください。".to_string(),
                    "FLAGS",
                );
            }
        }

        if kind == Some("valueObject") {
            let u = source
                .data
                .get("valueObject")
                .and_then(|v| v.get("underlying"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !matches!(u, "int" | "uint" | "long" | "ulong" | "string") {
                add(
                    source,
                    vec![PathKey::from("valueObject")],
                    "基になる型は整数または文字列を選んでください。".to_string(),
                    "TYPE",
                );
            }
        }
    }

    fn visit_cycle(
        snapshot: &Snapshot,
        name: &str,
        stack: &HashSet<String>,
        origin: &Source,
        add: &mut dyn FnMut(&Source, Vec<PathKey>, String, &str),
    ) {
        if stack.contains(name) {
            add(
                origin,
                vec![],
                format!("型「{name}」の循環があります。"),
                "CYCLE",
            );
            return;
        }
        let Some(src) = type_source(snapshot, name) else {
            return;
        };
        if src.data.get("custom").is_none() {
            return;
        }
        let mut next = stack.clone();
        next.insert(name.to_string());
        for f in fields_of(&src) {
            visit_cycle(snapshot, &f.r#type, &next, origin, add);
        }
    }

    for s in &all_types {
        let t_name = s.data.get("name").map(|v| v.text()).unwrap_or_default();
        visit_cycle(&snapshot, &t_name, &HashSet::new(), s, &mut add);
    }

    for table in &all_tables {
        let mut rows: Vec<(&Source, &Data, usize)> = Vec::new();
        for source in &table.sources {
            if let Some(rec_val) = source.data.get("records") {
                if !matches!(rec_val, Data::Undefined) && rec_val.as_array().is_none() {
                    add(
                        source,
                        vec![PathKey::from("records")],
                        "レコード一覧は配列にしてください。".to_string(),
                        "RECORDS",
                    );
                    continue;
                }
            }
            let Some(rec_arr) = source.data.get("records").and_then(|v| v.as_array()) else {
                continue;
            };
            for (index, row) in rec_arr.iter().enumerate() {
                rows.push((source, row, index));
                if !row.is_object() {
                    add(
                        source,
                        vec![PathKey::from("records"), PathKey::from(index)],
                        "レコードを項目と値の形式にしてください。".to_string(),
                        "VALUE",
                    );
                    continue;
                }
                for f in &table.fields {
                    check_field_value(
                        &snapshot,
                        source,
                        vec![
                            PathKey::from("records"),
                            PathKey::from(index),
                            PathKey::from(f.name.clone()),
                        ],
                        f,
                        row.get(&f.name),
                        &HashSet::new(),
                        &mut add,
                    );
                }
            }
        }

        let pk_val = table.source.data.get("primaryKey");
        let pk_fields_len = pk_val
            .and_then(|v| v.get("fields"))
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if pk_fields_len == 0 {
            add(
                &table.source,
                vec![PathKey::from("primaryKey")],
                "主キーの項目を選択してください。".to_string(),
                "KEY",
            );
        }

        let mut all_keys: Vec<&Data> = Vec::new();
        if let Some(pk) = pk_val.filter(|v| !v.is_null_or_undefined()) {
            all_keys.push(pk);
        }
        if let Some(sk_arr) = table
            .source
            .data
            .get("secondaryKeys")
            .and_then(|v| v.as_array())
        {
            for sk in sk_arr {
                if !sk.is_null_or_undefined() {
                    all_keys.push(sk);
                }
            }
        }

        for (key_index, key) in all_keys.iter().enumerate() {
            let Some(k_fields_arr) = key.get("fields").and_then(|v| v.as_array()) else {
                add(
                    &table.source,
                    vec![PathKey::from("primaryKey")],
                    "キーの項目を選択してください。".to_string(),
                    "KEY",
                );
                continue;
            };
            if k_fields_arr.is_empty() {
                add(
                    &table.source,
                    vec![PathKey::from("primaryKey")],
                    "キーの項目を選択してください。".to_string(),
                    "KEY",
                );
                continue;
            }
            let k_fields: Vec<String> = k_fields_arr
                .iter()
                .map(|x| x.text())
                .collect();
            for name in &k_fields {
                let field = table.fields.iter().find(|f| &f.name == name);
                let valid_key_field = if let Some(f) = field {
                    let base = primitive_of(&snapshot, f);
                    !f.array
                        && !f.nullable
                        && (matches!(base.as_str(), "int" | "uint" | "long" | "ulong" | "string")
                            || type_kind(type_source(&snapshot, &f.r#type).as_ref())
                                == Some("enum"))
                } else {
                    false
                };
                if !valid_key_field {
                    add(
                        &table.source,
                        vec![PathKey::from("primaryKey")],
                        format!("キー「{name}」は必須の整数・文字列・ID・Enum項目にしてください。"),
                        "KEY",
                    );
                }
            }

            let non_unique = key
                .get("nonUnique")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if non_unique && key_index > 0 {
                continue;
            }
            let mut used = HashSet::new();
            for (source, row, index) in &rows {
                let tuple = Data::Array(
                    k_fields
                        .iter()
                        .map(|n| row.get(n).cloned().unwrap_or(Data::Undefined))
                        .collect(),
                );
                let sig = fingerprint(&tuple);
                if used.contains(&sig) {
                    add(
                        source,
                        vec![PathKey::from("records"), PathKey::from(*index)],
                        format!("キー（{}）が重複しています。", k_fields.join(", ")),
                        "DUPLICATE",
                    );
                }
                used.insert(sig);
            }
        }

        if let Some(refs) = table
            .source
            .data
            .get("references")
            .and_then(|v| v.as_array())
        {
            for reference in refs {
                let ref_name = reference
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let csharp_name = reference
                    .get("csharpName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !valid_identifier(ref_name)
                    || (!csharp_name.is_empty() && !valid_identifier(csharp_name))
                {
                    add(
                        &table.source,
                        vec![PathKey::from("references")],
                        "参照の名前は英字・数字・_で指定してください。".to_string(),
                        "NAME",
                    );
                }
                let target_table_name = reference
                    .get("target")
                    .and_then(|t| t.get("table"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let target = all_tables.iter().find(|t| t.name == target_table_name);
                let own_fields: Option<Vec<String>> = reference
                    .get("fields")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().map(|x| x.text()).collect());
                let target_fields: Option<Vec<String>> = reference
                    .get("target")
                    .and_then(|t| t.get("fields"))
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().map(|x| x.text()).collect());

                let (Some(target_tbl), Some(own_f), Some(target_f)) =
                    (target, own_fields, target_fields)
                else {
                    add(
                        &table.source,
                        vec![PathKey::from("references")],
                        format!("参照「{ref_name}」の表と項目を設定してください。"),
                        "REFERENCE",
                    );
                    continue;
                };
                if own_f.is_empty()
                    || own_f.len() != target_f.len()
                    || own_f
                        .iter()
                        .any(|n| !table.fields.iter().any(|f| &f.name == n))
                    || target_f
                        .iter()
                        .any(|n| !target_tbl.fields.iter().any(|f| &f.name == n))
                {
                    add(
                        &table.source,
                        vec![PathKey::from("references")],
                        format!("参照「{ref_name}」の表と項目を設定してください。"),
                        "REFERENCE",
                    );
                    continue;
                }

                let type_mismatch = own_f.iter().enumerate().any(|(i, n)| {
                    let own_t = table.fields.iter().find(|f| &f.name == n).map(|f| &f.r#type);
                    let tgt_t = target_tbl
                        .fields
                        .iter()
                        .find(|f| f.name == target_f[i])
                        .map(|f| &f.r#type);
                    own_t != tgt_t
                });
                if type_mismatch {
                    add(
                        &table.source,
                        vec![PathKey::from("references")],
                        "参照元と参照先の項目の型を揃えてください。".to_string(),
                        "REFERENCE",
                    );
                }

                let mut target_keys: Vec<Vec<String>> = Vec::new();
                if let Some(pk_arr) = target_tbl
                    .source
                    .data
                    .get("primaryKey")
                    .and_then(|k| k.get("fields"))
                    .and_then(|v| v.as_array())
                {
                    target_keys.push(pk_arr.iter().map(|x| x.text()).collect());
                }
                if let Some(sks) = target_tbl
                    .source
                    .data
                    .get("secondaryKeys")
                    .and_then(|v| v.as_array())
                {
                    for sk in sks {
                        if let Some(f_arr) = sk.get("fields").and_then(|v| v.as_array()) {
                            target_keys.push(f_arr.iter().map(|x| x.text()).collect());
                        }
                    }
                }
                if !target_keys.iter().any(|k| k == &target_f) {
                    add(
                        &table.source,
                        vec![PathKey::from("references")],
                        "参照先に対応するキーがありません。".to_string(),
                        "REFERENCE",
                    );
                }

                let available: HashSet<String> = target_tbl
                    .sources
                    .iter()
                    .filter_map(|s| s.data.get("records").and_then(|v| v.as_array()))
                    .flatten()
                    .map(|r| {
                        let tuple = Data::Array(
                            target_f
                                .iter()
                                .map(|n| r.get(n).cloned().unwrap_or(Data::Undefined))
                                .collect(),
                        );
                        fingerprint(&tuple)
                    })
                    .collect();

                for (source, row, index) in &rows {
                    let vals: Vec<Data> = own_f
                        .iter()
                        .map(|n| row.get(n).cloned().unwrap_or(Data::Undefined))
                        .collect();
                    if vals.iter().all(|v| matches!(v, Data::Null)) {
                        continue;
                    }
                    let sig = fingerprint(&Data::Array(vals));
                    if !available.contains(&sig) {
                        add(
                            source,
                            vec![
                                PathKey::from("records"),
                                PathKey::from(*index),
                                PathKey::from(own_f[0].clone()),
                            ],
                            format!("参照先「{}」に対応するデータがありません。", target_tbl.name),
                            "REFERENCE",
                        );
                    }
                }
            }
        }
    }

    problems
}
