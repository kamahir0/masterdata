//! Application boundary for the typed Project Settings editor.
//!
//! The core owns TOML target location and lossless candidate construction.
//! This module owns project binding, exact-byte conflict checks, and the
//! one-file safe save protocol used by the GUI.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use masterdata_core::{
    BuildProfileInfo, Diagnostic, ErrorKind, MasterdataError, Project, ProjectConfigEditOperation,
    ProjectConfigEditPreview, ProjectInfo, PublishTargetInfo, PublishTargetKind, Result,
    preview_project_config_edit, source_content_identity,
};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::NativeApplicationService;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigSnapshot {
    pub project_root: PathBuf,
    pub config_path: PathBuf,
    pub base_source: String,
    pub base_content_identity: String,
    pub config_valid: bool,
    pub project: Option<ProjectInfo>,
    pub profiles: Vec<BuildProfileInfo>,
    pub publish_targets: Vec<PublishTargetInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigEditPreviewView {
    pub base_content_identity: String,
    pub candidate_content_identity: String,
    pub candidate_source: String,
    pub changed: bool,
    pub config_valid: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectConfigEditRequest {
    AddProfile {
        name: String,
        #[serde(default)]
        include_tags: Vec<String>,
        #[serde(default)]
        exclude_tags: Vec<String>,
    },
    UpdateProfile {
        name: String,
        #[serde(default)]
        include_tags: Vec<String>,
        #[serde(default)]
        exclude_tags: Vec<String>,
    },
    AddPublishTarget {
        kind: PublishTargetKind,
        path: String,
    },
    UpdatePublishTargetPath {
        index: usize,
        path: String,
    },
}

impl ProjectConfigEditRequest {
    fn operation(&self) -> ProjectConfigEditOperation {
        match self {
            Self::AddProfile {
                name,
                include_tags,
                exclude_tags,
            } => ProjectConfigEditOperation::AddProfile {
                name: name.clone(),
                include_tags: include_tags.clone(),
                exclude_tags: exclude_tags.clone(),
            },
            Self::UpdateProfile {
                name,
                include_tags,
                exclude_tags,
            } => ProjectConfigEditOperation::UpdateProfile {
                name: name.clone(),
                include_tags: include_tags.clone(),
                exclude_tags: exclude_tags.clone(),
            },
            Self::AddPublishTarget { kind, path } => ProjectConfigEditOperation::AddPublishTarget {
                kind: *kind,
                path: path.clone(),
            },
            Self::UpdatePublishTargetPath { index, path } => {
                ProjectConfigEditOperation::UpdatePublishTargetPath {
                    index: *index,
                    path: path.clone(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSaveStatus {
    Success,
    Conflict,
    Failure,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSaveReport {
    pub status: ConfigSaveStatus,
    pub snapshot: Option<ProjectConfigSnapshot>,
    pub current: Option<ProjectConfigSnapshot>,
    pub diagnostic: Option<Diagnostic>,
}

impl NativeApplicationService {
    /// Read the exact config bytes and typed settings.  A domain-invalid
    /// config remains openable so the editor can repair it; syntax-invalid
    /// input is returned with diagnostics and no fabricated settings.
    pub fn open_project_config(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ProjectConfigSnapshot> {
        let path = resolve_config_path(explicit_project, current_dir)?;
        let source = read_config_source(&path)?;
        Ok(config_snapshot(path, source))
    }

    pub fn preview_project_config_edit(
        &self,
        base_source: &str,
        base_content_identity: &str,
        request: &ProjectConfigEditRequest,
    ) -> Result<ProjectConfigEditPreviewView> {
        if source_content_identity(base_source) != base_content_identity {
            return Err(config_error(
                "E-CONFIG-EDIT-BASE-IDENTITY",
                "provided config source does not match its content identity",
                None,
            ));
        }
        let preview = preview_project_config_edit(base_source, &request.operation())?;
        Ok(config_preview_view(preview))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_project_config_edit(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        base_source: &str,
        base_content_identity: &str,
        request: &ProjectConfigEditRequest,
        overwrite_expected_identity: Option<&str>,
    ) -> Result<ConfigSaveReport> {
        let path = resolve_config_path(explicit_project, current_dir)?;
        if source_content_identity(base_source) != base_content_identity {
            return Err(config_error(
                "E-CONFIG-EDIT-BASE-IDENTITY",
                "provided config source does not match its content identity",
                Some(path),
            ));
        }
        let preview = preview_project_config_edit(base_source, &request.operation())?;
        let current_source = read_config_source(&path)?;
        let current_identity = source_content_identity(&current_source);
        let expected = overwrite_expected_identity.unwrap_or(base_content_identity);
        if current_identity != expected {
            return Ok(ConfigSaveReport {
                status: ConfigSaveStatus::Conflict,
                snapshot: None,
                current: Some(config_snapshot(path.clone(), current_source)),
                diagnostic: Some(
                    config_error(
                        "E-CONFIG-EDIT-CONFLICT",
                        "masterdata.toml changed after the settings base snapshot",
                        Some(path),
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }

        if current_source == preview.candidate_source {
            return Ok(ConfigSaveReport {
                status: ConfigSaveStatus::Success,
                snapshot: Some(config_snapshot(path, current_source)),
                current: None,
                diagnostic: None,
            });
        }

        if let Err(error) = install_config_candidate(
            &path,
            current_source.as_bytes(),
            preview.candidate_source.as_bytes(),
        ) {
            let after = read_config_source(&path)
                .ok()
                .map(|source| config_snapshot(path.clone(), source));
            if error.diagnostic().code == "E-CONFIG-EDIT-CONFLICT" {
                return Ok(ConfigSaveReport {
                    status: ConfigSaveStatus::Conflict,
                    snapshot: None,
                    current: after,
                    diagnostic: Some(error.diagnostic().clone()),
                });
            }
            let status = if after
                .as_ref()
                .is_some_and(|snapshot| snapshot.base_content_identity == current_identity)
            {
                ConfigSaveStatus::Failure
            } else {
                ConfigSaveStatus::OutcomeUnknown
            };
            return Ok(ConfigSaveReport {
                status,
                snapshot: None,
                current: after,
                diagnostic: Some(error.diagnostic().clone()),
            });
        }

        let after_source = match read_config_source(&path) {
            Ok(source) => source,
            Err(error) => {
                return Ok(ConfigSaveReport {
                    status: ConfigSaveStatus::OutcomeUnknown,
                    snapshot: None,
                    current: None,
                    diagnostic: Some(error.diagnostic().clone()),
                });
            }
        };
        if source_content_identity(&after_source) != preview.candidate_content_identity {
            return Ok(ConfigSaveReport {
                status: ConfigSaveStatus::OutcomeUnknown,
                snapshot: None,
                current: Some(config_snapshot(path.clone(), after_source)),
                diagnostic: Some(
                    config_error(
                        "E-CONFIG-EDIT-WRITE-VERIFY",
                        "saved config could not be verified as the complete candidate content",
                        Some(path),
                    )
                    .diagnostic()
                    .clone(),
                ),
            });
        }
        Ok(ConfigSaveReport {
            status: ConfigSaveStatus::Success,
            snapshot: Some(config_snapshot(path, after_source)),
            current: None,
            diagnostic: None,
        })
    }
}

fn config_preview_view(preview: ProjectConfigEditPreview) -> ProjectConfigEditPreviewView {
    let diagnostics = config_diagnostics(&preview.candidate_source, None);
    ProjectConfigEditPreviewView {
        base_content_identity: preview.base_content_identity,
        candidate_content_identity: preview.candidate_content_identity,
        candidate_source: preview.candidate_source,
        changed: preview.changed,
        config_valid: diagnostics.is_empty(),
        diagnostics,
    }
}

fn config_snapshot(path: PathBuf, source: String) -> ProjectConfigSnapshot {
    let project_root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let diagnostics = config_diagnostics(&source, Some(&path));
    let project = Project::from_config_path(path.clone()).ok();
    let project_info = project.as_ref().map(Project::info);
    let (profiles, publish_targets) = project
        .as_ref()
        .map(|project| {
            let info = project.info();
            (info.profiles, info.publish_targets)
        })
        .unwrap_or_else(|| raw_settings(&source, &project_root));
    ProjectConfigSnapshot {
        project_root,
        config_path: path,
        base_content_identity: source_content_identity(&source),
        base_source: source,
        config_valid: diagnostics.is_empty(),
        project: project_info,
        profiles,
        publish_targets,
        diagnostics,
    }
}

fn raw_settings(source: &str, root: &Path) -> (Vec<BuildProfileInfo>, Vec<PublishTargetInfo>) {
    let Ok(value) = toml::from_str::<toml::Value>(source) else {
        return (Vec::new(), Vec::new());
    };
    let profiles = value
        .get("build")
        .and_then(toml::Value::as_table)
        .and_then(|build| build.get("profiles"))
        .and_then(toml::Value::as_table)
        .map(|profiles| {
            profiles
                .iter()
                .map(|(name, profile)| BuildProfileInfo {
                    name: name.clone(),
                    include_tags: string_array(profile, "include_tags"),
                    exclude_tags: string_array(profile, "exclude_tags"),
                })
                .collect()
        })
        .unwrap_or_default();
    let publish_targets = value
        .get("publish")
        .and_then(toml::Value::as_table)
        .and_then(|publish| publish.get("targets"))
        .and_then(toml::Value::as_array)
        .map(|targets| {
            targets
                .iter()
                .filter_map(|target| {
                    let table = target.as_table()?;
                    let kind = match table.get("kind").and_then(toml::Value::as_str) {
                        Some("csharp") => PublishTargetKind::CSharp,
                        Some("binary") => PublishTargetKind::Binary,
                        _ => return None,
                    };
                    let path = table.get("path")?.as_str()?.to_owned();
                    Some(PublishTargetInfo {
                        kind,
                        resolved_path: root.join(&path),
                        path,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (profiles, publish_targets)
}

fn string_array(value: &toml::Value, key: &str) -> Vec<String> {
    value
        .as_table()
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn config_diagnostics(source: &str, path: Option<&Path>) -> Vec<Diagnostic> {
    let raw = match toml::from_str::<toml::Value>(source) {
        Ok(raw) => raw,
        Err(error) => {
            return vec![
                config_error(
                    "E-CONFIG-EDIT-TOML-PARSE",
                    format!("configuration is not valid TOML: {error}"),
                    path.map(Path::to_path_buf),
                )
                .diagnostic()
                .clone(),
            ];
        }
    };
    let config = match raw.try_into::<masterdata_core::ProjectConfig>() {
        Ok(config) => config,
        Err(error) => {
            return vec![
                config_error(
                    "E-CONFIG-EDIT-DOMAIN-INVALID",
                    format!("configuration shape is not supported: {error}"),
                    path.map(Path::to_path_buf),
                )
                .diagnostic()
                .clone(),
            ];
        }
    };
    match config.validate() {
        Ok(()) => Vec::new(),
        Err(error) => {
            let mut diagnostic = error.diagnostic().clone();
            diagnostic.source = path.map(Path::to_path_buf);
            vec![diagnostic]
        }
    }
}

fn resolve_config_path(explicit: Option<&Path>, current_dir: &Path) -> Result<PathBuf> {
    let start = if current_dir.is_absolute() {
        current_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| config_error("E-CONFIG-EDIT-ROOT", error.to_string(), None))?
            .join(current_dir)
    };
    let path = if let Some(explicit) = explicit {
        let explicit = if explicit.is_absolute() {
            explicit.to_path_buf()
        } else {
            start.join(explicit)
        };
        if explicit.file_name().and_then(|name| name.to_str()) == Some("masterdata.toml")
            || explicit.is_file()
        {
            explicit
        } else {
            explicit.join("masterdata.toml")
        }
    } else {
        let mut cursor = start;
        loop {
            let candidate = cursor.join("masterdata.toml");
            if candidate.is_file() {
                break candidate;
            }
            if !cursor.pop() {
                return Err(config_error(
                    "E-PROJECT-NOT-FOUND",
                    "could not find masterdata.toml from the current directory",
                    None,
                ));
            }
        }
    };
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-READ",
            format!("could not inspect {}: {error}", path.display()),
            Some(path.clone()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(config_error(
            "E-CONFIG-EDIT-PATH-UNSAFE",
            "settings target must be a regular non-symlink file",
            Some(path),
        ));
    }
    Ok(path)
}

fn read_config_source(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-READ",
            format!("could not read {}: {error}", path.display()),
            Some(path.to_path_buf()),
        )
    })
}

fn install_config_candidate(path: &Path, expected_current: &[u8], candidate: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        config_error(
            "E-CONFIG-EDIT-PATH-UNSAFE",
            "settings target has no parent directory",
            Some(path.to_path_buf()),
        )
    })?;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-READ",
            format!("could not inspect settings target: {error}"),
            Some(path.to_path_buf()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(config_error(
            "E-CONFIG-EDIT-PATH-UNSAFE",
            "settings target is no longer a regular non-symlink file",
            Some(path.to_path_buf()),
        ));
    }
    let transaction = TempDir::new_in(parent).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-STAGE",
            format!("could not create settings staging directory: {error}"),
            Some(path.to_path_buf()),
        )
    })?;
    let staged = transaction.path().join("candidate");
    let backup = transaction.path().join("base");
    {
        let mut file = fs::File::create(&staged).map_err(|error| {
            config_error(
                "E-CONFIG-EDIT-STAGE",
                format!("could not stage settings candidate: {error}"),
                Some(path.to_path_buf()),
            )
        })?;
        file.write_all(candidate).map_err(|error| {
            config_error(
                "E-CONFIG-EDIT-STAGE",
                format!("could not write settings candidate: {error}"),
                Some(path.to_path_buf()),
            )
        })?;
        file.sync_all().map_err(|error| {
            config_error(
                "E-CONFIG-EDIT-STAGE",
                format!("could not sync settings candidate: {error}"),
                Some(path.to_path_buf()),
            )
        })?;
    }
    fs::set_permissions(&staged, metadata.permissions()).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-STAGE",
            format!("could not preserve settings permissions: {error}"),
            Some(path.to_path_buf()),
        )
    })?;
    let current = fs::read(path).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-READ",
            format!("could not recheck settings before save: {error}"),
            Some(path.to_path_buf()),
        )
    })?;
    if current != expected_current {
        return Err(config_error(
            "E-CONFIG-EDIT-CONFLICT",
            "settings changed during save preflight",
            Some(path.to_path_buf()),
        ));
    }
    fs::rename(path, &backup).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-STAGE",
            format!("could not stage existing settings for replacement: {error}"),
            Some(path.to_path_buf()),
        )
    })?;
    let actual_base = match fs::read(&backup) {
        Ok(bytes) => bytes,
        Err(error) => {
            rollback_config_backup(
                path,
                &backup,
                &format!("could not verify staged settings: {error}"),
            )?;
            return Err(config_error(
                "E-CONFIG-EDIT-OUTCOME-UNKNOWN",
                format!("could not verify staged settings: {error}"),
                Some(path.to_path_buf()),
            ));
        }
    };
    if actual_base != expected_current {
        rollback_config_backup(path, &backup, "settings changed during replacement")?;
        return Err(config_error(
            "E-CONFIG-EDIT-CONFLICT",
            "settings changed during the save replacement window",
            Some(path.to_path_buf()),
        ));
    }
    if let Err(error) = fs::rename(&staged, path) {
        rollback_config_backup(
            path,
            &backup,
            &format!("could not install settings candidate: {error}"),
        )?;
        return Err(config_error(
            "E-CONFIG-EDIT-STAGE",
            format!("could not install settings candidate: {error}"),
            Some(path.to_path_buf()),
        ));
    }
    Ok(())
}

fn rollback_config_backup(path: &Path, backup: &Path, context: &str) -> Result<()> {
    if path.exists() {
        return Err(config_error(
            "E-CONFIG-EDIT-OUTCOME-UNKNOWN",
            format!("{context}; settings path was recreated before rollback"),
            Some(path.to_path_buf()),
        ));
    }
    fs::rename(backup, path).map_err(|error| {
        config_error(
            "E-CONFIG-EDIT-OUTCOME-UNKNOWN",
            format!("{context}; rollback also failed ({error})"),
            Some(path.to_path_buf()),
        )
    })
}

fn config_error(code: &str, message: impl Into<String>, path: Option<PathBuf>) -> MasterdataError {
    let mut error = MasterdataError::new(code, ErrorKind::Config, message)
        .with_related_requirement("CONFIG-EDIT-004");
    error.diagnostic.source = path;
    error
}

#[cfg(test)]
mod tests {
    use super::{ConfigSaveStatus, NativeApplicationService, ProjectConfigEditRequest};
    use std::fs;
    use std::io::Write;

    fn project() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("temp project");
        fs::write(
            temp.path().join("masterdata.toml"),
            "[project]\nid = \"config.test\"\nname = \"Config\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
        )
        .expect("config");
        fs::create_dir(temp.path().join("sources")).expect("sources");
        temp
    }

    #[test]
    fn config_save_conflict_keeps_external_bytes() {
        let temp = project();
        let service = NativeApplicationService::new();
        let snapshot = service
            .open_project_config(Some(temp.path()), temp.path())
            .expect("snapshot");
        fs::OpenOptions::new()
            .append(true)
            .open(temp.path().join("masterdata.toml"))
            .expect("open")
            .write_all(b"\n# external\n")
            .expect("external edit");
        let report = service
            .save_project_config_edit(
                Some(temp.path()),
                temp.path(),
                &snapshot.base_source,
                &snapshot.base_content_identity,
                &ProjectConfigEditRequest::AddProfile {
                    name: "prod".into(),
                    include_tags: vec!["release".into()],
                    exclude_tags: Vec::new(),
                },
                None,
            )
            .expect("conflict report");
        assert_eq!(report.status, ConfigSaveStatus::Conflict);
        assert!(
            fs::read_to_string(temp.path().join("masterdata.toml"))
                .expect("read")
                .contains("# external")
        );
    }

    #[test]
    fn domain_invalid_profile_is_saved_and_reported() {
        let temp = project();
        let service = NativeApplicationService::new();
        let snapshot = service
            .open_project_config(Some(temp.path()), temp.path())
            .expect("snapshot");
        let report = service
            .save_project_config_edit(
                Some(temp.path()),
                temp.path(),
                &snapshot.base_source,
                &snapshot.base_content_identity,
                &ProjectConfigEditRequest::AddProfile {
                    name: "prod".into(),
                    include_tags: vec!["Not-valid".into()],
                    exclude_tags: Vec::new(),
                },
                None,
            )
            .expect("domain-invalid config is a saved state");
        assert_eq!(report.status, ConfigSaveStatus::Success);
        assert_eq!(
            report
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.config_valid),
            Some(false)
        );
        assert!(
            report
                .snapshot
                .as_ref()
                .expect("snapshot")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-BUILD-PROFILE-INVALID-TAG")
        );
    }
}
