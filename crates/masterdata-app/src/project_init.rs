//! Shared application boundary for the Desktop Create Project workflow.

use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{
    Diagnostic, ErrorKind, InitOptions, ProjectInfo, Result, initialize_gui_project,
};
use serde::{Deserialize, Serialize};

use crate::NativeApplicationService;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectInitRequest {
    pub destination: PathBuf,
    pub project_id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectInitStatus {
    Success,
    Conflict,
    Failure,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInitReport {
    pub status: ProjectInitStatus,
    pub destination: PathBuf,
    pub project: Option<ProjectInfo>,
    pub created_entries: Vec<PathBuf>,
    pub diagnostic: Option<Diagnostic>,
}

impl NativeApplicationService {
    pub fn create_project(
        &self,
        current_dir: &Path,
        request: &ProjectInitRequest,
    ) -> Result<ProjectInitReport> {
        let destination = absolute_path(&request.destination, current_dir)?;
        let options = InitOptions {
            project_id: request.project_id.clone(),
            name: request.name.clone(),
            version: request.version.clone(),
        };
        match initialize_gui_project(&destination, &options) {
            Ok(project) => Ok(ProjectInitReport {
                status: ProjectInitStatus::Success,
                destination: destination.clone(),
                project: Some(project),
                created_entries: expected_entries(&destination),
                diagnostic: None,
            }),
            Err(error) => {
                let status = match error.diagnostic().code.as_str() {
                    "E-PROJECT-INIT-CONFLICT"
                    | "E-PROJECT-INIT-NONEMPTY"
                    | "E-PROJECT-INIT-SYMLINK"
                    | "E-PROJECT-INIT-NOT-DIRECTORY" => ProjectInitStatus::Conflict,
                    _ => ProjectInitStatus::Failure,
                };
                Ok(ProjectInitReport {
                    status,
                    destination: destination.clone(),
                    project: None,
                    created_entries: existing_entries(&destination),
                    diagnostic: Some(error.diagnostic().clone()),
                })
            }
        }
    }
}

fn absolute_path(path: &Path, current_dir: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    if current_dir.is_absolute() {
        Ok(current_dir.join(path))
    } else {
        Err(masterdata_core::MasterdataError::new(
            "E-PROJECT-INIT-PATH",
            ErrorKind::Config,
            "Create Project requires an absolute or current-directory-relative target",
        ))
    }
}

fn expected_entries(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("masterdata.toml"),
        root.join("sources"),
        root.join("sources/schemas"),
        root.join("sources/types"),
        root.join("sources/data"),
        root.join(".gitignore"),
    ]
}

fn existing_entries(root: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if root.exists() {
        entries.push(root.to_path_buf());
        for path in expected_entries(root) {
            if fs::symlink_metadata(&path).is_ok() {
                entries.push(path);
            }
        }
    }
    entries
}
