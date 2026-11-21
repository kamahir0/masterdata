use crate::model::{fields_of, tables, Data, Field, PathKey, Problem, Snapshot, Source};
use crate::validation::fingerprint;
use serde_json::json;
use std::collections::{HashMap, HashSet};

pub fn is_master(source: &Source) -> bool {
    source.data.get("kind").and_then(Data::as_str) == Some("masterValue")
}

fn report(problems: &mut Vec<Problem>, source: &Source, path: Vec<PathKey>, message: &str) {
    problems.push(Problem {
        source: source.id.clone(),
        path,
        code: "MASTER_VALUE".into(),
        message: message.into(),
    });
}

pub fn resolve(input: &Snapshot) -> (Snapshot, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut items = HashMap::new();
    let mut definitions = HashSet::new();
    let schemas: HashMap<_, _> = tables(input)
        .into_iter()
        .map(|t| (t.name, t.fields))
        .collect();
    let generated_name = |s: &Source| {
        if s.data.get("kind").and_then(Data::as_str) == Some("type") {
            s.data
                .get("name")
                .and_then(Data::as_str)
                .unwrap_or("")
                .to_string()
        } else {
            s.data
                .get("csharpName")
                .and_then(Data::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    crate::validation::public_name(
                        s.data.get("table").and_then(Data::as_str).unwrap_or(""),
                    )
                })
        }
    };
    let declarations: Vec<_> = input
        .sources
        .iter()
        .filter(|s| {
            matches!(
                s.data.get("kind").and_then(Data::as_str),
                Some("schema" | "masterValue" | "type")
            )
        })
        .collect();
    if input.sources.iter().any(is_master) {
        for source in &declarations {
            let name = generated_name(source).to_ascii_lowercase();
            if name == "mastervaluedatabase"
                || (is_master(source)
                    && (matches!(
                        name.as_str(),
                        "memorydatabase" | "databasebuilder" | "mastermemoryresolver"
                    ) || declarations.iter().any(|s| {
                        s.id != source.id && generated_name(s).eq_ignore_ascii_case(&name)
                    })))
            {
                report(
                    &mut problems,
                    source,
                    vec![],
                    "生成するC#名が他の定義またはデータベースの型名と衝突しています。",
                );
            }
        }
    }
    for source in input.sources.iter().filter(|s| is_master(s)) {
        let data = &source.data;
        let name = data.get("table").and_then(Data::as_str).unwrap_or("");
        definitions.insert(name);
        if data.get("csharpName").is_some() || data.get("fields").and_then(Data::as_array).is_none()
        {
            report(
                &mut problems,
                source,
                vec!["fields".into()],
                "マスター値には項目一覧と定義名を指定してください。",
            );
        }
        let mode = data.get("keyMode").and_then(Data::as_str);
        if !matches!(mode, Some("number" | "string")) {
            report(
                &mut problems,
                source,
                vec!["keyMode".into()],
                "キー方式を指定してください。",
            );
        }
        let records = data.get("records").and_then(Data::as_array);
        if !records.is_some_and(|r| r.len() == 1 && r[0].is_object()) {
            report(
                &mut problems,
                source,
                vec!["records".into()],
                "マスター値には1件のレコードが必要です。",
            );
        }
        for (i, field) in fields_of(source).into_iter().enumerate() {
            if field.name == generated_name(source) {
                report(
                    &mut problems,
                    source,
                    vec!["fields".into(), i.into(), "name".into()],
                    "型と同名のプロパティは使用できません。",
                );
            }
            let raw = data.get("fields").and_then(|v| v.get_index(i));
            let key = raw
                .and_then(|v| v.get("valueKey"))
                .unwrap_or(&Data::Undefined);
            let valid = match mode {
                Some("string") => key.as_str().is_some_and(|s| !s.is_empty()),
                Some("number") => matches!(key, Data::Int(n) if (0..=i32::MAX as i128).contains(n)),
                _ => false,
            };
            let value = records
                .and_then(|r| r.first())
                .and_then(|r| r.get(&field.name));
            if value.is_some_and(|v| v.get("$masterValue").is_some()) {
                report(
                    &mut problems,
                    source,
                    vec!["records".into(), 0.into(), field.name.clone().into()],
                    "マスター値同士は参照できません。",
                );
            }
            if field.key.unwrap_or(0) < 1 {
                report(
                    &mut problems,
                    source,
                    vec!["fields".into(), i.into(), "key".into()],
                    "フィールド番号には正の整数を指定してください。",
                );
            }
            let matches = items
                .entry((name, fingerprint(key)))
                .or_insert_with(Vec::new);
            matches.push((field, value));
            if !valid || matches.len() > 1 {
                report(
                    &mut problems,
                    source,
                    vec!["fields".into(), i.into(), "valueKey".into()],
                    "項目キーの形式または重複を確認してください。",
                );
            }
        }
    }
    let mut snapshot = input.clone();
    for source in &mut snapshot.sources {
        if is_master(source) {
            let map = source.data.as_map_mut().unwrap();
            map.insert("kind".into(), Data::Str("schema".into()));
            let mut fields = map
                .get("fields")
                .and_then(Data::as_array)
                .cloned()
                .unwrap_or_default();
            fields.push(Data::from_json_value(
                &json!({"key":0,"name":"MasterValueRowId","type":"int"}),
                true,
            ));
            map.insert("fields".into(), Data::Array(fields));
            map.insert(
                "primaryKey".into(),
                Data::from_json_value(&json!({"fields":["MasterValueRowId"]}), true),
            );
            let mut row = map
                .get("records")
                .and_then(|r| r.get_index(0))
                .and_then(Data::as_map)
                .cloned()
                .unwrap_or_default();
            row.insert("MasterValueRowId".into(), Data::Int(0));
            map.insert("records".into(), Data::Array(vec![Data::Map(row)]));
            continue;
        }
        let kind = source.data.get("kind").and_then(Data::as_str);
        if !matches!(kind, Some("schema" | "data")) {
            continue;
        }
        let name = source
            .data
            .get("table")
            .and_then(Data::as_str)
            .unwrap_or("");
        if kind == Some("data") && definitions.contains(name) {
            report(
                &mut problems,
                source,
                vec!["table".into()],
                "マスター値のレコードは別ファイルに追加できません。",
            );
        }
        let fields = schemas.get(name);
        let Some(Data::Array(records)) =
            source.data.as_map_mut().and_then(|m| m.get_mut("records"))
        else {
            continue;
        };
        for (i, row) in records.iter_mut().enumerate() {
            let Some(map) = row.as_map_mut() else {
                continue;
            };
            for (property, cell) in map {
                let Some(reference) = cell.get("$masterValue") else {
                    continue;
                };
                let target = reference
                    .get("definition")
                    .and_then(Data::as_str)
                    .unwrap_or("");
                let key = reference.get("key").unwrap_or(&Data::Undefined);
                let field = fields.and_then(|fs| fs.iter().find(|f| &f.name == property));
                if let (Some(field), Some([(other, Some(value))])) = (
                    field,
                    items.get(&(target, fingerprint(key))).map(Vec::as_slice),
                ) {
                    if same_shape(field, other) {
                        *cell = (*value).clone();
                        continue;
                    }
                }
                problems.push(Problem {
                    source: source.id.clone(),
                    path: vec!["records".into(), i.into(), property.clone().into()],
                    code: "MASTER_VALUE".into(),
                    message: format!(
                        "マスター値「{target} / {}」の参照先または型を確認してください。",
                        key.text()
                    ),
                });
            }
        }
    }
    (snapshot, problems)
}

fn same_shape(a: &Field, b: &Field) -> bool {
    a.r#type == b.r#type && a.nullable == b.nullable && a.array == b.array
}
