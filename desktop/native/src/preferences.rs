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
    #[serde(default, rename = "recentProjects")]
    pub recent_projects: Vec<RecentProject>,
    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecentProject {
    pub root: String,
    pub name: String,
}
impl Preferences {
    pub fn opened(&mut self, root: String, name: String) {
        self.recent_projects.retain(|p| p.root != root);
        self.recent_projects.insert(0, RecentProject { root, name });
        self.recent_projects.truncate(10);
    }
    pub fn remove_recent(&mut self, root: &str) {
        self.recent_projects.retain(|p| p.root != root);
    }
}
pub fn read(path: &Path) -> Preferences {
    let mut preferences: Preferences = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let mut seen = std::collections::BTreeSet::new();
    preferences
        .recent_projects
        .retain(|p| seen.insert(p.root.clone()));
    preferences.recent_projects.truncate(10);
    preferences
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
    #[test]
    fn recent_roots_are_local_bounded_ordered_and_removal_never_touches_project_files() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("user/preferences.json");
        let mut p = Preferences::default();
        for index in 0..12 {
            let root = t.path().join(format!("project-{index}"));
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("notes"), b"user bytes").unwrap();
            p.opened(root.to_string_lossy().into(), format!("Project {index}"));
        }
        assert_eq!(p.recent_projects.len(), 10);
        let old = p.recent_projects[9].root.clone();
        p.opened(old.clone(), "Renamed display name".into());
        assert_eq!(p.recent_projects[0].root, old);
        assert_eq!(p.recent_projects.len(), 10);
        write(&path, &p).unwrap();
        let mut loaded = read(&path);
        assert_eq!(loaded.recent_projects, p.recent_projects);
        loaded.remove_recent(&old);
        write(&path, &loaded).unwrap();
        assert_eq!(read(&path).recent_projects.len(), 9);
        assert_eq!(
            std::fs::read(Path::new(&old).join("notes")).unwrap(),
            b"user bytes"
        );
        assert!(!Path::new(&old).join("masterdata.toml").exists());
        loaded.remove_recent("already missing");
        assert_eq!(loaded.recent_projects.len(), 9);
    }
}
