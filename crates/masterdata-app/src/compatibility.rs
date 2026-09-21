//! Application adapter for the read-only Released Compatibility operation.

use std::path::{Path, PathBuf};

use masterdata_core::{
    CompatibilityReport, CompatibilitySnapshot, Project, Result, compare_compatibility,
};
use serde::{Deserialize, Serialize};

use crate::NativeApplicationService;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityRequest {
    pub baseline_project: PathBuf,
    pub current_project: PathBuf,
}

impl NativeApplicationService {
    /// Compare two explicit project snapshots without mutating either project.
    pub fn analyze_compatibility(
        &self,
        baseline_project: &Path,
        current_project: &Path,
        current_dir: &Path,
    ) -> Result<CompatibilityReport> {
        let baseline = materialize_snapshot(baseline_project, current_dir)?;
        let current = materialize_snapshot(current_project, current_dir)?;
        compare_compatibility(&baseline, &current)
    }
}

fn materialize_snapshot(path: &Path, current_dir: &Path) -> Result<CompatibilitySnapshot> {
    let project = Project::discover(Some(path), current_dir)?;
    let metadata = project.config().project.clone();
    let documents = project.load_documents()?;
    Ok(CompatibilitySnapshot::new(metadata, documents))
}
