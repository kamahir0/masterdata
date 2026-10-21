use crate::model::{default_config, Config, Profile};
use indexmap::IndexMap;

pub const PROJECT_FILE_NAME: &str = "masterdata.toml";

fn invalid<T>(key: &str) -> Result<T, String> {
    Err(format!("プロジェクト設定の {key} の形式が不正です。"))
}

pub fn read_config(text: &str) -> Result<Config, String> {
    let data: toml::Table =
        toml::from_str(text).map_err(|e| format!("TOMLの構文が不正です: {e}"))?;
    let Some(name) = data.get("name").and_then(|v| v.as_str()) else {
        return invalid("name（空でない文字列）");
    };
    if name.trim().is_empty() {
        return invalid("name（空でない文字列）");
    }
    let mut config = default_config(name);
    if let Some(profiles_val) = data.get("profiles") {
        let Some(profiles_table) = profiles_val.as_table() else {
            return invalid("profiles（名前付きの設定）");
        };
        let mut profiles = IndexMap::new();
        for (p_name, p_val) in profiles_table {
            if p_name.trim().is_empty() {
                return invalid(&format!("profiles.{p_name}"));
            }
            let Some(p_table) = p_val.as_table() else {
                return invalid(&format!("profiles.{p_name}"));
            };
            let mut include = Vec::new();
            let mut exclude = Vec::new();
            for key in ["include", "exclude"] {
                if let Some(tags_val) = p_table.get(key) {
                    let Some(arr) = tags_val.as_array() else {
                        return invalid(&format!("profiles.{p_name}.{key}（文字列の配列）"));
                    };
                    let mut parsed = Vec::with_capacity(arr.len());
                    for item in arr {
                        let Some(s) = item.as_str() else {
                            return invalid(&format!("profiles.{p_name}.{key}（文字列の配列）"));
                        };
                        parsed.push(s.to_string());
                    }
                    if key == "include" {
                        include = parsed;
                    } else {
                        exclude = parsed;
                    }
                }
            }
            profiles.insert(p_name.clone(), Profile { include, exclude });
        }
        config.profiles = profiles;
    }
    if let Some(publish_val) = data.get("publish") {
        let Some(publish_table) = publish_val.as_table() else {
            return invalid("publish");
        };
        for key in ["csharp", "binary"] {
            if let Some(target_val) = publish_table.get(key) {
                let Some(s) = target_val.as_str() else {
                    return invalid(&format!("publish.{key}（文字列）"));
                };
                if key == "csharp" {
                    config.publish.csharp = s.to_string();
                } else {
                    config.publish.binary = s.to_string();
                }
            }
        }
    }
    Ok(config)
}

pub fn write_config(config: &Config, before: Option<&str>) -> Result<String, String> {
    if let Some(before_text) = before {
        if let Ok(existing) = read_config(before_text) {
            if &existing == config {
                return Ok(before_text.to_string());
            }
        }
    }
    if config.name.trim().is_empty() {
        return invalid("name（空でない文字列）");
    }
    let previous: toml::Table = match before {
        Some(b) => toml::from_str(b).unwrap_or_default(),
        None => toml::Table::new(),
    };
    let mut next = previous.clone();
    next.insert("name".to_string(), toml::Value::String(config.name.clone()));

    let mut pub_table = previous
        .get("publish")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    pub_table.insert(
        "csharp".to_string(),
        toml::Value::String(config.publish.csharp.clone()),
    );
    pub_table.insert(
        "binary".to_string(),
        toml::Value::String(config.publish.binary.clone()),
    );
    next.insert("publish".to_string(), toml::Value::Table(pub_table));

    let prev_profiles = previous.get("profiles").and_then(|v| v.as_table());
    let mut profiles_table = toml::Table::new();
    for (p_name, prof) in &config.profiles {
        let mut entry = prev_profiles
            .and_then(|t| t.get(p_name))
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default();
        entry.insert(
            "include".to_string(),
            toml::Value::Array(
                prof.include
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        );
        entry.insert(
            "exclude".to_string(),
            toml::Value::Array(
                prof.exclude
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        );
        profiles_table.insert(p_name.clone(), toml::Value::Table(entry));
    }
    next.insert("profiles".to_string(), toml::Value::Table(profiles_table));

    let output = toml::to_string_pretty(&next).map_err(|e| e.to_string())?;
    read_config(&output)?;
    Ok(output)
}
