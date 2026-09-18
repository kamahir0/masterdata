//! Build/Publish preview DTOs and identity-bound publish confirmation.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind as IoErrorKind;
use std::path::Path;

use masterdata_core::{ErrorKind, MasterdataError, Project, PublishTargetKind, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::receipt::{ValidatedArtifactSet, validate_artifact_set};
use crate::{
    NativeApplicationService, PublishExecutionReport, PublishPreflightPlan, PublishTargetPreflight,
    preflight_publish,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishPreview {
    pub artifact_set_identity: String,
    pub config_content_identity: String,
    pub publish_plan_identity: String,
    pub targets: Vec<PublishTargetPreview>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishTargetPreview {
    pub index: usize,
    pub kind: PublishTargetKind,
    pub configured_path: String,
    pub destination: std::path::PathBuf,
    pub additions: Vec<String>,
    pub updates: Vec<String>,
    pub removals: Vec<String>,
    pub binary_replacement: bool,
    pub preflight_ok: bool,
}

impl NativeApplicationService {
    pub fn publish_preview(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<PublishPreview> {
        let project = Project::discover(explicit_project, current_dir)?;
        let info = project.info();
        let artifacts = validate_artifact_set(&info.artifact_root, &info.project_id)?;
        let artifact_identity = artifact_set_identity(&artifacts);
        let plan = preflight_publish(&info, artifacts)?;
        render_publish_preview(
            artifact_identity,
            project.config_content_identity(),
            &plan,
        )
    }

    /// Confirm a previously rendered preview.  Both the canonical artifact
    /// hashes and the exact project config bytes are checked again before any
    /// target mutation starts.
    pub fn publish_from_preview(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        expected_artifact_set_identity: &str,
        expected_config_content_identity: &str,
        expected_publish_plan_identity: &str,
    ) -> std::result::Result<PublishExecutionReport, crate::PublishExecutionFailure> {
        let project = match Project::discover(explicit_project, current_dir) {
            Ok(project) => project,
            Err(error) => {
                return Err(crate::PublishExecutionFailure {
                    report: PublishExecutionReport {
                        targets: Vec::new(),
                    },
                    error,
                });
            }
        };
        let info = project.info();
        let report = crate::publish::report_for_project(
            &info,
            crate::PublishTargetStatus::NotAttempted,
            None,
        );
        if project.config_content_identity() != expected_config_content_identity {
            return Err(crate::PublishExecutionFailure {
                report,
                error: delivery_stale_error(
                    "E-PUBLISH-PREVIEW-STALE-CONFIG",
                    "project configuration changed after the publish preview; preview again",
                    &info.config_path,
                ),
            });
        }
        let artifacts = match validate_artifact_set(&info.artifact_root, &info.project_id) {
            Ok(artifacts) => artifacts,
            Err(error) => return Err(crate::PublishExecutionFailure { report, error }),
        };
        if artifact_set_identity(&artifacts) != expected_artifact_set_identity {
            return Err(crate::PublishExecutionFailure {
                report,
                error: delivery_stale_error(
                    "E-PUBLISH-PREVIEW-STALE-ARTIFACT",
                    "canonical artifacts changed after the publish preview; preview again",
                    &info.artifact_root,
                ),
            });
        }
        let plan = match preflight_publish(&info, artifacts) {
            Ok(plan) => plan,
            Err(error) => return Err(crate::PublishExecutionFailure { report, error }),
        };
        let current_plan_identity = match publish_plan_identity(&plan) {
            Ok(identity) => identity,
            Err(error) => return Err(crate::PublishExecutionFailure { report, error }),
        };
        if current_plan_identity != expected_publish_plan_identity {
            return Err(crate::PublishExecutionFailure {
                report,
                error: delivery_stale_error(
                    "E-PUBLISH-PREVIEW-STALE-DESTINATION",
                    "publish destination ownership or content changed after the preview; preview again",
                    &info.project_root,
                ),
            });
        }
        crate::publish::execute_publish_plan(&info, plan, &[])
    }
}

fn render_publish_preview(
    artifact_identity: String,
    config_identity: String,
    plan: &PublishPreflightPlan,
) -> Result<PublishPreview> {
    let mut targets = Vec::with_capacity(plan.targets.len());
    for (index, target) in plan.targets.iter().enumerate() {
        let (additions, updates, removals, binary_replacement) = match target {
            PublishTargetPreflight {
                csharp: Some(csharp),
                binary: None,
                ..
            } => {
                let previous = csharp
                    .previous_managed_paths
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let current = csharp
                    .current_generated_paths
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                (
                    current.difference(&previous).cloned().collect(),
                    current.intersection(&previous).cloned().collect(),
                    previous.difference(&current).cloned().collect(),
                    false,
                )
            }
            PublishTargetPreflight {
                csharp: None,
                binary: Some(binary),
                ..
            } => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                binary.existing_regular_file,
            ),
            _ => (Vec::new(), Vec::new(), Vec::new(), false),
        };
        let target = &plan.targets[index];
        targets.push(PublishTargetPreview {
            index,
            kind: target.kind,
            configured_path: target.configured_path.clone(),
            destination: target.destination.clone(),
            additions,
            updates,
            removals,
            binary_replacement,
            preflight_ok: true,
        });
    }
    Ok(PublishPreview {
        artifact_set_identity: artifact_identity,
        config_content_identity: config_identity,
        publish_plan_identity: publish_plan_identity(plan)?,
        targets,
    })
}

fn hash_identity_part(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
}

fn publish_plan_identity(plan: &PublishPreflightPlan) -> Result<String> {
    let mut hasher = Sha256::new();
    hash_identity_part(&mut hasher, &(plan.targets.len() as u64).to_le_bytes());
    for target in &plan.targets {
        let kind = match target.kind {
            PublishTargetKind::CSharp => b"csharp".as_slice(),
            PublishTargetKind::Binary => b"binary".as_slice(),
        };
        hash_identity_part(&mut hasher, kind);
        hash_identity_part(&mut hasher, target.configured_path.as_bytes());
        hash_identity_part(&mut hasher, target.destination.to_string_lossy().as_bytes());
        match (&target.csharp, &target.binary) {
            (Some(csharp), None) => {
                hash_identity_part(&mut hasher, b"csharp-plan");
                hash_path_state(&mut hasher, &csharp.manifest_path)?;
                let mut paths = csharp
                    .previous_managed_paths
                    .iter()
                    .chain(csharp.current_generated_paths.iter())
                    .cloned()
                    .collect::<BTreeSet<_>>();
                for path in &paths {
                    hash_identity_part(&mut hasher, path.as_bytes());
                    hash_path_state(&mut hasher, &target.destination.join(path))?;
                }
                paths.clear();
            }
            (None, Some(binary)) => {
                hash_identity_part(&mut hasher, b"binary-plan");
                hash_path_state(&mut hasher, &binary.destination)?;
            }
            _ => hash_identity_part(&mut hasher, b"invalid-plan-shape"),
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_path_state(hasher: &mut Sha256, path: &Path) -> Result<()> {
    hash_identity_part(hasher, path.to_string_lossy().as_bytes());
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            hash_identity_part(hasher, b"symlink");
            let target = fs::read_link(path).map_err(|error| destination_identity_error(path, error))?;
            hash_identity_part(hasher, target.to_string_lossy().as_bytes());
        }
        Ok(metadata) if metadata.is_file() => {
            hash_identity_part(hasher, b"file");
            let bytes = fs::read(path).map_err(|error| destination_identity_error(path, error))?;
            hash_identity_part(hasher, &bytes);
        }
        Ok(metadata) if metadata.is_dir() => {
            hash_identity_part(hasher, b"directory");
        }
        Ok(_) => hash_identity_part(hasher, b"other"),
        Err(error) if error.kind() == IoErrorKind::NotFound => {
            hash_identity_part(hasher, b"missing");
        }
        Err(error) => return Err(destination_identity_error(path, error)),
    }
    Ok(())
}

fn destination_identity_error(path: &Path, error: std::io::Error) -> MasterdataError {
    MasterdataError::new(
        "E-PUBLISH-PREVIEW-DESTINATION-IDENTITY",
        ErrorKind::Io,
        format!("could not bind publish preview to {}: {error}", path.display()),
    )
    .with_source(path.to_path_buf())
    .with_related_requirement("PUBLISH-PREVIEW-002")
}

fn artifact_set_identity(artifacts: &ValidatedArtifactSet) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&artifacts.receipt).unwrap_or_default());
    for file in &artifacts.csharp {
        hasher.update(file.relative_path.as_bytes());
        hasher.update(file.hash.as_bytes());
    }
    hasher.update(artifacts.binary.relative_path.as_bytes());
    hasher.update(artifacts.binary.hash.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn delivery_stale_error(code: &str, message: &str, path: &Path) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement("PUBLISH-PREVIEW-002")
}
