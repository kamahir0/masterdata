use std::path::Path;

use crate::Result;
use crate::pipeline::{BuildPlan, prepare_build_from_documents};
use crate::project::{InitOptions, Project, ProjectInfo, initialize_project};
use crate::table::BuildSelection;
use crate::validation::ValidationReport;

/// Native project service used by native composition roots.
//
// WHY: project discovery, filesystem reads, and native path resolution are
// host I/O responsibilities. Naming this boundary explicitly prevents a
// future Browser Host from treating native project access as pure semantics.
// IF REMOVED: native filesystem authority can leak into shared/browser
// preparation and create a second, incompatible project-loading path.
// EVIDENCE: docs/specs/runtime-hosts.md; docs/adr/0006-host-capability-composition.md
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeProjectService;

impl NativeProjectService {
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
        let documents = project.load_documents()?;
        prepare_build_from_documents(project.info(), documents, &BuildSelection::unfiltered())
    }

    pub fn prepare_build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
    ) -> Result<BuildPlan> {
        let project = Project::discover(explicit_project, current_dir)?;
        let documents = project.load_documents()?;
        prepare_build_from_documents(project.info(), documents, selection)
    }

    pub fn init(&self, root: &Path, options: &InitOptions) -> Result<ProjectInfo> {
        initialize_project(root, options)
    }
}

/// Compatibility name for existing core consumers.
pub type ProjectService = NativeProjectService;
