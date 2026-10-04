//! Application appearance belongs to the native user config, never the Project.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Write, path::Path};

#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Preferences {
    #[serde(default)]
    pub theme: Theme,
    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
}
pub fn read(path: &Path) -> Preferences {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
pub fn write(path: &Path, value: &Preferences) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or("application config directory missing")?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    temporary
        .write_all(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_appearance_round_trip_and_invalid_value_fallback() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        assert_eq!(read(&path).theme, Theme::System);
        let value = Preferences {
            theme: Theme::Dark,
            ..Default::default()
        };
        write(&path, &value).unwrap();
        assert_eq!(read(&path).theme, Theme::Dark);
        write(&path, &Preferences::default()).unwrap();
        assert_eq!(read(&path).theme, Theme::System);
        std::fs::write(&path, br#"{"theme":"unknown"}"#).unwrap();
        assert_eq!(read(&path).theme, Theme::System);
    }
}
