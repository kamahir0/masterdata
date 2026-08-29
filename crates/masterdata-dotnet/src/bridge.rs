use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use masterdata_core::{BuildPlan, ErrorKind, MasterdataError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DotnetBridge {
    executable: OsString,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DotnetProbe {
    pub available: bool,
    pub executable: String,
    pub version: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BridgeSmokeStatus {
    Passed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeSmokeReport {
    pub status: BridgeSmokeStatus,
    pub builder_project: PathBuf,
    pub detail: String,
}

impl Default for DotnetBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl DotnetBridge {
    pub fn new() -> Self {
        let executable =
            std::env::var_os("MASTERDATA_DOTNET").unwrap_or_else(|| OsString::from("dotnet"));
        Self { executable }
    }

    pub fn with_executable(executable: impl Into<OsString>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn executable(&self) -> &OsString {
        &self.executable
    }

    pub fn probe(&self) -> DotnetProbe {
        let executable = self.executable.to_string_lossy().to_string();
        match Command::new(&self.executable).arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                DotnetProbe {
                    available: true,
                    executable,
                    version: (!version.is_empty()).then_some(version.clone()),
                    detail: version,
                }
            }
            Ok(output) => DotnetProbe {
                available: false,
                executable,
                version: None,
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            },
            Err(error) => DotnetProbe {
                available: false,
                executable,
                version: None,
                detail: error.to_string(),
            },
        }
    }

    pub fn builder_project_path(repository_root: &Path) -> PathBuf {
        repository_root
            .join("dotnet")
            .join("builder")
            .join("masterdata-builder.csproj")
    }

    /// Run the repository's real .NET builder contract test.
    ///
    /// The builder receives `--self-test` and verifies that the Rust bridge can
    /// invoke a .NET project. It does not create a MasterMemory artifact.
    pub fn smoke_test(&self, repository_root: &Path) -> Result<BridgeSmokeReport> {
        let probe = self.probe();
        if !probe.available {
            return Err(MasterdataError::new(
                "DOTNET-TOOL-001",
                ErrorKind::ExternalTool,
                format!(
                    ".NET SDK is unavailable through `{}`: {}",
                    probe.executable, probe.detail
                ),
            ));
        }
        let builder_project = Self::builder_project_path(repository_root);
        if !builder_project.is_file() {
            return Err(MasterdataError::new(
                "DOTNET-BUILDER-001",
                ErrorKind::ExternalTool,
                "builder project is missing",
            )
            .with_source(builder_project));
        }

        let output = Command::new(&self.executable)
            .arg("run")
            .arg("--project")
            .arg(&builder_project)
            .arg("--no-launch-profile")
            .arg("--")
            .arg("--self-test")
            .current_dir(repository_root)
            .output()
            .map_err(|error| {
                MasterdataError::new(
                    "DOTNET-BRIDGE-001",
                    ErrorKind::ExternalTool,
                    format!("could not invoke .NET builder: {error}"),
                )
                .with_source(builder_project.clone())
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MasterdataError::new(
                "DOTNET-BRIDGE-002",
                ErrorKind::ExternalTool,
                format!(".NET builder smoke test failed: {detail}"),
            )
            .with_source(builder_project));
        }
        let detail = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(BridgeSmokeReport {
            status: BridgeSmokeStatus::Passed,
            builder_project,
            detail,
        })
    }

    /// The real MasterMemory build is intentionally an explicit boundary and
    /// not a successful placeholder.
    pub fn build_mastermemory(&self, _plan: &BuildPlan) -> Result<()> {
        Err(MasterdataError::new(
            "DOTNET-MASTERMEMORY-001",
            ErrorKind::NotImplemented,
            "MasterMemory v3 Source Generator and binary build are not implemented yet",
        ))
    }
}
