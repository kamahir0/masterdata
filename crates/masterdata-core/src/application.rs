use std::path::Path;

use crate::Result;
use crate::pipeline::{BuildPlan, prepare_build, prepare_build_with_selection};
use crate::project::{InitOptions, Project, ProjectInfo, initialize_project};
use crate::table::BuildSelection;
use crate::validation::ValidationReport;

/// Application service shared by CLI and Tauri.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProjectService;

impl ProjectService {
    pub fn new() -> Self {
        Self
    }

    pub fn project_info(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ProjectInfo> {
        Project::discover(explicit_project, current_dir).map(|project| project.info())
    }

    pub fn validate(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ValidationReport> {
        Project::discover(explicit_project, current_dir)?.validate()
    }

    pub fn prepare_build(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<BuildPlan> {
        let project = Project::discover(explicit_project, current_dir)?;
        prepare_build(&project)
    }

    pub fn prepare_build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
    ) -> Result<BuildPlan> {
        let project = Project::discover(explicit_project, current_dir)?;
        prepare_build_with_selection(&project, selection)
    }

    pub fn init(&self, root: &Path, options: &InitOptions) -> Result<ProjectInfo> {
        initialize_project(root, options)
    }
}
