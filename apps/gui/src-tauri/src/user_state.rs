use crate::ApiError;
use masterdata_core::{ErrorKind, MasterdataError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const RECENT_PROJECTS_LIMIT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreferenceDto {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjectDto {
    pub root: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUserState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_preference: Option<ThemePreferenceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_projects: Option<Vec<RecentProjectDto>>,
}

#[derive(Debug, Clone)]
pub struct UserStateStore {
    state_file: PathBuf,
}

impl UserStateStore {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            state_file: base_dir.as_ref().join("user-state.json"),
        }
    }

    #[cfg(test)]
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    pub fn load(&self) -> ApplicationUserState {
        if !self.state_file.exists() {
            return ApplicationUserState::default();
        }
        let content = match fs::read_to_string(&self.state_file) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "Warning: Failed to read user state file {:?}: {err}. Falling back to default.",
                    self.state_file
                );
                return ApplicationUserState::default();
            }
        };
        match serde_json::from_str::<ApplicationUserState>(&content) {
            Ok(mut state) => {
                if let Some(projects) = state.recent_projects.take() {
                    state.recent_projects = Some(sanitize_recent_projects(projects));
                }
                state
            }
            Err(err) => {
                eprintln!(
                    "Warning: Failed to parse user state file {:?}: {err}. Falling back to default.",
                    self.state_file
                );
                ApplicationUserState::default()
            }
        }
    }

    pub fn save(&self, state: &ApplicationUserState) -> Result<(), std::io::Error> {
        let parent = self.state_file.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No parent directory for user state file",
            )
        })?;
        fs::create_dir_all(parent)?;

        let serialized = serde_json::to_string_pretty(state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Atomic write: write to temp file in the same directory, then rename
        let temp_file = parent.join(format!(
            ".user-state.json.tmp.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        fs::write(&temp_file, serialized)?;
        if let Err(err) = fs::rename(&temp_file, &self.state_file) {
            let _ = fs::remove_file(&temp_file);
            return Err(err);
        }
        Ok(())
    }

    pub fn update_theme_preference(
        &self,
        preference: ThemePreferenceDto,
    ) -> Result<ApplicationUserState, std::io::Error> {
        let mut state = self.load();
        state.theme_preference = Some(preference);
        self.save(&state)?;
        Ok(state)
    }

    pub fn update_recent_projects(
        &self,
        projects: Vec<RecentProjectDto>,
    ) -> Result<ApplicationUserState, std::io::Error> {
        let mut state = self.load();
        state.recent_projects = Some(sanitize_recent_projects(projects));
        self.save(&state)?;
        Ok(state)
    }
}

pub fn sanitize_recent_projects(projects: Vec<RecentProjectDto>) -> Vec<RecentProjectDto> {
    let mut result = Vec::with_capacity(projects.len().min(RECENT_PROJECTS_LIMIT));
    let mut seen_roots = std::collections::HashSet::new();

    for project in projects {
        if seen_roots.insert(project.root.clone()) {
            result.push(project);
            if result.len() >= RECENT_PROJECTS_LIMIT {
                break;
            }
        }
    }
    result
}

pub fn resolve_user_state_dir(app: &tauri::AppHandle) -> Result<PathBuf, ApiError> {
    use tauri::Manager;
    app.path()
        .app_config_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|error| {
            ApiError::from(MasterdataError::new(
                "E-GUI-USER-STATE-PATH",
                ErrorKind::Io,
                format!("could not determine application user state directory: {error}"),
            ))
        })
}

#[tauri::command(rename_all = "camelCase")]
pub fn load_application_user_state(
    app: tauri::AppHandle,
) -> Result<ApplicationUserState, ApiError> {
    let dir = resolve_user_state_dir(&app)?;
    let store = UserStateStore::new(dir);
    Ok(store.load())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_theme_preference(
    app: tauri::AppHandle,
    preference: ThemePreferenceDto,
) -> Result<ApplicationUserState, ApiError> {
    let dir = resolve_user_state_dir(&app)?;
    let store = UserStateStore::new(dir);
    store.update_theme_preference(preference).map_err(|error| {
        ApiError::from(MasterdataError::new(
            "E-GUI-USER-STATE-WRITE",
            ErrorKind::Io,
            format!("could not persist theme preference: {error}"),
        ))
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_recent_projects(
    app: tauri::AppHandle,
    projects: Vec<RecentProjectDto>,
) -> Result<ApplicationUserState, ApiError> {
    let dir = resolve_user_state_dir(&app)?;
    let store = UserStateStore::new(dir);
    store.update_recent_projects(projects).map_err(|error| {
        ApiError::from(MasterdataError::new(
            "E-GUI-USER-STATE-WRITE",
            ErrorKind::Io,
            format!("could not persist recent projects: {error}"),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());
        let state = store.load();
        assert_eq!(state.theme_preference, None);
        assert_eq!(state.recent_projects, None);
    }

    #[test]
    fn theme_preference_round_trip() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());

        for pref in [
            ThemePreferenceDto::Light,
            ThemePreferenceDto::Dark,
            ThemePreferenceDto::System,
        ] {
            let updated = store.update_theme_preference(pref).unwrap();
            assert_eq!(updated.theme_preference, Some(pref));

            let loaded = store.load();
            assert_eq!(loaded.theme_preference, Some(pref));
        }
    }

    #[test]
    fn recent_projects_round_trip_and_sanitization() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());

        let raw = vec![
            RecentProjectDto {
                root: "/p1".into(),
                name: "P1".into(),
            },
            RecentProjectDto {
                root: "/p2".into(),
                name: "P2".into(),
            },
            RecentProjectDto {
                root: "/p1".into(),
                name: "P1 Duplicate".into(),
            },
            RecentProjectDto {
                root: "/p3".into(),
                name: "P3".into(),
            },
        ];

        let updated = store.update_recent_projects(raw).unwrap();
        let projects = updated
            .recent_projects
            .expect("should have recent projects");
        assert_eq!(projects.len(), 3);
        assert_eq!(projects[0].root, "/p1");
        assert_eq!(projects[0].name, "P1");
        assert_eq!(projects[1].root, "/p2");
        assert_eq!(projects[2].root, "/p3");

        let loaded = store.load();
        assert_eq!(loaded.recent_projects.unwrap(), projects);
    }

    #[test]
    fn recent_projects_capped_at_limit() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());

        let raw: Vec<_> = (0..20)
            .map(|i| RecentProjectDto {
                root: format!("/path/project_{i}"),
                name: format!("Project {i}"),
            })
            .collect();

        let updated = store.update_recent_projects(raw).unwrap();
        let projects = updated
            .recent_projects
            .expect("should have recent projects");
        assert_eq!(projects.len(), RECENT_PROJECTS_LIMIT);
        assert_eq!(projects[0].root, "/path/project_0");
        assert_eq!(projects[9].root, "/path/project_9");
    }

    #[test]
    fn malformed_json_falls_back_to_default_without_panic() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());
        fs::write(store.state_file(), "{ invalid json content").unwrap();

        let loaded = store.load();
        assert_eq!(loaded, ApplicationUserState::default());
    }

    #[test]
    fn unknown_json_fields_are_safely_ignored() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());
        let json_with_unknown = r#"{
            "themePreference": "dark",
            "unknownFutureField": 42,
            "anotherObject": { "nested": true }
        }"#;
        fs::write(store.state_file(), json_with_unknown).unwrap();

        let loaded = store.load();
        assert_eq!(loaded.theme_preference, Some(ThemePreferenceDto::Dark));
    }

    #[test]
    fn writes_do_not_escape_into_project_or_dot_masterdata() {
        let dir = tempdir().unwrap();
        let store = UserStateStore::new(dir.path());
        store
            .update_theme_preference(ThemePreferenceDto::Light)
            .unwrap();

        let state_path = store.state_file().to_string_lossy();
        assert!(!state_path.contains(".masterdata"));
        assert!(!state_path.contains("masterdata.toml"));
    }
}
