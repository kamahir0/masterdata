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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MasterMemorySpikeReport {
    pub status: String,
    #[serde(rename = "masterMemoryVersion")]
    pub master_memory_version: String,
    #[serde(rename = "messagePackVersion")]
    pub message_pack_version: String,
    #[serde(rename = "binaryPath")]
    pub binary_path: PathBuf,
    #[serde(rename = "binarySize")]
    pub binary_size: usize,
    #[serde(rename = "reloadedItemId")]
    pub reloaded_item_id: i32,
    #[serde(rename = "reloadedItemName")]
    pub reloaded_item_name: String,
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
                "E-DOTNET-SDK-UNAVAILABLE",
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
                "E-DOTNET-BUILDER-MISSING",
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
                    "E-DOTNET-BRIDGE-INVOKE",
                    ErrorKind::ExternalTool,
                    format!("could not invoke .NET builder: {error}"),
                )
                .with_source(builder_project.clone())
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MasterdataError::new(
                "E-DOTNET-BRIDGE-SMOKE-FAILED",
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
            "E-DOTNET-MASTERMEMORY-NOT-IMPLEMENTED",
            ErrorKind::NotImplemented,
            "MasterMemory v3 Source Generator and binary build are not implemented yet",
        ))
    }

    /// Run the isolated, hand-written MasterMemory v3 technical spike. This
    /// is evidence that the .NET dependency and bridge work; it is not the
    /// production schema-driven generator.
    pub fn mastermemory_spike(&self, repository_root: &Path) -> Result<MasterMemorySpikeReport> {
        let probe = self.probe();
        if !probe.available {
            return Err(MasterdataError::new(
                "E-DOTNET-SDK-UNAVAILABLE",
                ErrorKind::ExternalTool,
                format!(
                    ".NET SDK is unavailable through `{}`: {}",
                    probe.executable, probe.detail
                ),
            ));
        }
        let spike_project = Self::spike_project_path(repository_root);
        if !spike_project.is_file() {
            return Err(MasterdataError::new(
                "E-DOTNET-SPIKE-MISSING",
                ErrorKind::ExternalTool,
                "MasterMemory technical spike project is missing",
            )
            .with_source(spike_project));
        }
        let binary_path = repository_root
            .join("target")
            .join("mastermemory-spike")
            .join("masterdata.bytes");
        let output = Command::new(&self.executable)
            .arg("run")
            .arg("--project")
            .arg(&spike_project)
            .arg("--no-launch-profile")
            .arg("--")
            .arg("--output")
            .arg(&binary_path)
            .current_dir(repository_root)
            .output()
            .map_err(|error| {
                MasterdataError::new(
                    "E-DOTNET-SPIKE-INVOKE",
                    ErrorKind::ExternalTool,
                    format!("could not invoke MasterMemory technical spike: {error}"),
                )
                .with_source(spike_project.clone())
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MasterdataError::new(
                "E-DOTNET-SPIKE-FAILED",
                ErrorKind::ExternalTool,
                format!("MasterMemory technical spike failed: {detail}"),
            )
            .with_source(spike_project));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json = stdout
            .lines()
            .rev()
            .find(|line| line.trim_start().starts_with('{'))
            .ok_or_else(|| {
                MasterdataError::new(
                    "E-DOTNET-SPIKE-REPORT",
                    ErrorKind::ExternalTool,
                    "MasterMemory technical spike did not emit a JSON report",
                )
            })?;
        let report: MasterMemorySpikeReport = serde_json::from_str(json).map_err(|error| {
            MasterdataError::new(
                "E-DOTNET-SPIKE-REPORT",
                ErrorKind::ExternalTool,
                format!("could not parse MasterMemory technical spike report: {error}"),
            )
            .with_source(spike_project)
        })?;
        if report.status != "ok" || report.binary_size == 0 || !report.binary_path.is_file() {
            return Err(MasterdataError::new(
                "E-DOTNET-SPIKE-REPORT",
                ErrorKind::ExternalTool,
                "MasterMemory technical spike reported an invalid binary result",
            )
            .with_source(report.binary_path));
        }
        Ok(report)
    }

    pub fn spike_project_path(repository_root: &Path) -> PathBuf {
        repository_root
            .join("dotnet")
            .join("spike")
            .join("masterdata-mastermemory-spike.csproj")
    }
}
