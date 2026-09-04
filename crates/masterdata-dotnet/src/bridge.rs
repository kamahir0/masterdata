use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use masterdata_core::{ErrorKind, MasterdataError, Result};
use serde::{Deserialize, Serialize};

use super::protocol::{
    BUILD_PROTOCOL_VERSION, MASTERMEMORY_VERSION, MESSAGEPACK_VERSION, MasterMemoryBuildReport,
    MasterMemoryBuildRequest,
};

#[derive(Debug, Clone)]
pub struct DotnetBridge {
    executable: OsString,
    repository_root: PathBuf,
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
        Self {
            executable,
            repository_root: repository_root_from_manifest(),
        }
    }

    pub fn with_executable(executable: impl Into<OsString>) -> Self {
        Self {
            executable: executable.into(),
            repository_root: repository_root_from_manifest(),
        }
    }

    pub fn with_repository_root(mut self, repository_root: impl Into<PathBuf>) -> Self {
        self.repository_root = repository_root.into();
        self
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

    /// Compile the staged schema-specific project and run the repository-owned
    /// MasterMemory builder. The request contains only Rust-validated,
    /// normalized values; the .NET process never reparses source YAML.
    pub fn build_mastermemory(
        &self,
        request: &MasterMemoryBuildRequest,
        generated_source_dir: &Path,
        workspace: &Path,
    ) -> Result<MasterMemoryBuildReport> {
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
        if !workspace.is_dir() || !generated_source_dir.is_dir() {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Io,
                "builder staging workspace or generated source directory is missing",
            )
            .with_source(workspace.to_path_buf()));
        }
        if !request.output_path.is_absolute() {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Config,
                "builder output path must be absolute",
            )
            .with_source(request.output_path.clone()));
        }

        let builder_project = Self::builder_project_path(&self.repository_root);
        let Some(builder_directory) = builder_project.parent() else {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-MISSING",
                ErrorKind::ExternalTool,
                "builder project has no parent directory",
            ));
        };
        let template_program = builder_directory.join("Program.cs");
        if !builder_project.is_file() || !template_program.is_file() {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-MISSING",
                ErrorKind::ExternalTool,
                "repository builder project or program is missing",
            )
            .with_source(builder_project));
        }

        let project_path = workspace.join("masterdata-builder.csproj");
        let program_path = workspace.join("Program.cs");
        let request_path = workspace.join("request.json");
        let report_path = workspace.join("report.json");
        // Keep the Source Generator option assembly outside the canonical C#
        // artifact directory. It is a builder input, not a schema-generated
        // artifact that should be published with the canonical set.
        let options_path = workspace.join("BuilderOptions.g.cs");

        fs::copy(&template_program, &program_path).map_err(|error| {
            MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Io,
                format!("could not stage the .NET builder program: {error}"),
            )
            .with_source(program_path.clone())
        })?;
        fs::write(&project_path, staged_project_file()).map_err(|error| {
            MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Io,
                format!("could not write the staged .NET project: {error}"),
            )
            .with_source(project_path.clone())
        })?;
        fs::write(&options_path, generator_options_source(&request.namespace)).map_err(
            |error| {
                MasterdataError::new(
                    "E-DOTNET-BUILDER-PREPARE",
                    ErrorKind::Io,
                    format!("could not write MasterMemory generator options: {error}"),
                )
                .with_source(options_path.clone())
            },
        )?;
        let request_json = serde_json::to_vec_pretty(request).map_err(|error| {
            MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Validation,
                format!("could not serialize the builder request: {error}"),
            )
            .with_source(request_path.clone())
        })?;
        fs::write(&request_path, request_json).map_err(|error| {
            MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Io,
                format!("could not write the builder request: {error}"),
            )
            .with_source(request_path.clone())
        })?;

        let compile = Command::new(&self.executable)
            .arg("build")
            .arg(&project_path)
            .arg("--nologo")
            .arg("--configuration")
            .arg("Release")
            .current_dir(workspace)
            .output()
            .map_err(|error| {
                MasterdataError::new(
                    "E-DOTNET-BUILDER-COMPILE",
                    ErrorKind::ExternalTool,
                    format!("could not invoke the .NET builder compiler: {error}"),
                )
                .with_source(project_path.clone())
            })?;
        if !compile.status.success() {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-COMPILE",
                ErrorKind::ExternalTool,
                format!(
                    "schema-specific .NET builder compilation failed: {}",
                    command_detail(&compile)
                ),
            )
            .with_source(project_path));
        }

        let run = Command::new(&self.executable)
            .arg("run")
            .arg("--project")
            .arg(&project_path)
            .arg("--no-build")
            .arg("--no-restore")
            .arg("--configuration")
            .arg("Release")
            .arg("--")
            .arg("--request")
            .arg(&request_path)
            .arg("--output")
            .arg(&request.output_path)
            .arg("--report")
            .arg(&report_path)
            .current_dir(workspace)
            .output()
            .map_err(|error| {
                MasterdataError::new(
                    "E-DOTNET-BUILDER-RUN",
                    ErrorKind::ExternalTool,
                    format!("could not invoke the schema-specific .NET builder: {error}"),
                )
                .with_source(project_path.clone())
            })?;
        if !run.status.success() {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-RUN",
                ErrorKind::ExternalTool,
                format!(
                    "schema-specific .NET builder failed: {}",
                    command_detail(&run)
                ),
            )
            .with_source(report_path));
        }

        let report_content = fs::read_to_string(&report_path).map_err(|error| {
            MasterdataError::new(
                "E-DOTNET-BUILDER-REPORT",
                ErrorKind::ExternalTool,
                format!("could not read the .NET builder report: {error}"),
            )
            .with_source(report_path.clone())
        })?;
        let report: MasterMemoryBuildReport =
            serde_json::from_str(&report_content).map_err(|error| {
                MasterdataError::new(
                    "E-DOTNET-BUILDER-REPORT",
                    ErrorKind::ExternalTool,
                    format!("could not parse the .NET builder report: {error}"),
                )
                .with_source(report_path.clone())
            })?;
        validate_build_report(&report, request)?;
        Ok(report)
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

fn repository_root_from_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("masterdata-dotnet must live two directories below the repository root")
        .to_path_buf()
}

fn staged_project_file() -> &'static str {
    r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <LangVersion>12.0</LangVersion>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>
    <AssemblyName>masterdata-generated-builder</AssemblyName>
    <RootNamespace>Masterdata.Builder</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="MasterMemory" Version="3.0.4" />
    <PackageReference Include="MessagePack" Version="3.1.3" />
    <Compile Include="Generated/**/*.g.cs" />
    <Compile Include="BuilderOptions.g.cs" />
    <Compile Include="Program.cs" />
  </ItemGroup>
</Project>
"#
}

fn generator_options_source(namespace: &str) -> String {
    format!(
        "using MasterMemory;\n[assembly: MasterMemoryGeneratorOptions(Namespace = \"{}\")]\n",
        escape_csharp_string(namespace)
    )
}

fn escape_csharp_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn command_detail(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let detail = match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("process exited with status {}", output.status),
        (false, true) => format!("stdout: {stdout}"),
        (true, false) => format!("stderr: {stderr}"),
        (false, false) => format!("stdout: {stdout}\nstderr: {stderr}"),
    };
    const MAX_DETAIL_CHARS: usize = 8 * 1024;
    if detail.chars().count() <= MAX_DETAIL_CHARS {
        detail
    } else {
        let truncated: String = detail.chars().take(MAX_DETAIL_CHARS).collect();
        format!("{truncated} [output truncated]")
    }
}

fn validate_build_report(
    report: &MasterMemoryBuildReport,
    request: &MasterMemoryBuildRequest,
) -> Result<()> {
    if report.status != "ok" {
        return Err(MasterdataError::new(
            "E-DOTNET-BUILDER-REPORT",
            ErrorKind::ExternalTool,
            format!(".NET builder reported status `{}`", report.status),
        ));
    }
    if report.protocol_version != BUILD_PROTOCOL_VERSION {
        return Err(MasterdataError::new(
            "E-DOTNET-BUILDER-REPORT",
            ErrorKind::ExternalTool,
            format!(
                "unsupported builder protocol version {} (expected {})",
                report.protocol_version, BUILD_PROTOCOL_VERSION
            ),
        ));
    }
    if report.master_memory_version != MASTERMEMORY_VERSION
        || report.message_pack_version != MESSAGEPACK_VERSION
    {
        return Err(MasterdataError::new(
            "E-DOTNET-BUILDER-REPORT",
            ErrorKind::ExternalTool,
            format!(
                "builder package versions are MasterMemory {} / MessagePack {}, expected {} / {}",
                report.master_memory_version,
                report.message_pack_version,
                MASTERMEMORY_VERSION,
                MESSAGEPACK_VERSION
            ),
        ));
    }
    if !same_filesystem_path(&report.binary_path, &request.output_path) {
        return Err(MasterdataError::new(
            "E-DOTNET-BUILDER-REPORT",
            ErrorKind::ExternalTool,
            format!(
                "builder report path `{}` does not match requested output `{}`",
                report.binary_path.display(),
                request.output_path.display()
            ),
        ));
    }
    let metadata = fs::metadata(&request.output_path).map_err(|error| {
        MasterdataError::new(
            "E-DOTNET-BINARY-INVALID",
            ErrorKind::ExternalTool,
            format!("builder binary is missing or unreadable: {error}"),
        )
        .with_source(request.output_path.clone())
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() != report.binary_size {
        return Err(MasterdataError::new(
            "E-DOTNET-BINARY-INVALID",
            ErrorKind::ExternalTool,
            format!(
                "builder binary size is invalid (report {}, filesystem {})",
                report.binary_size,
                metadata.len()
            ),
        )
        .with_source(request.output_path.clone()));
    }
    let expected_records = request.record_count();
    if report.table_count != request.tables.len()
        || report.record_count != expected_records
        || report.tables.len() != request.tables.len()
    {
        return Err(MasterdataError::new(
            "E-DOTNET-BUILDER-REPORT",
            ErrorKind::ExternalTool,
            "builder report table or record counts do not match the request",
        ));
    }
    for (expected, actual) in request.tables.iter().zip(&report.tables) {
        if expected.identity != actual.identity || expected.records.len() != actual.record_count {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-REPORT",
                ErrorKind::ExternalTool,
                format!(
                    "builder report count for table `{}` does not match the request",
                    expected.identity
                ),
            ));
        }
    }
    Ok(())
}

// WHY: The .NET builder reports the path after Windows has resolved it, while
// the Rust request can retain an 8.3 short-name spelling. Comparing filesystem
// identities accepts equivalent representations without accepting another
// staged artifact.
// IF REMOVED: Windows short/long path pairs are rejected by report validation.
// EVIDENCE: docs/specs/build-pipeline.md; Regression: build_report_accepts_windows_short_path_for_same_file.
fn same_filesystem_path(left: &Path, right: &Path) -> bool {
    same_file::is_same_file(left, right).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    #[cfg(windows)]
    use std::path::Path;

    use tempfile::tempdir;

    use super::{same_filesystem_path, validate_build_report};
    use crate::protocol::{
        BUILD_PROTOCOL_VERSION, MASTERMEMORY_VERSION, MESSAGEPACK_VERSION, MasterMemoryBuildReport,
        MasterMemoryBuildRequest,
    };

    fn request(output_path: PathBuf) -> MasterMemoryBuildRequest {
        MasterMemoryBuildRequest {
            protocol_version: BUILD_PROTOCOL_VERSION,
            namespace: "Masterdata.Generated".to_owned(),
            output_path,
            tables: Vec::new(),
        }
    }

    fn report(binary_path: PathBuf, binary_size: u64) -> MasterMemoryBuildReport {
        MasterMemoryBuildReport {
            status: "ok".to_owned(),
            protocol_version: BUILD_PROTOCOL_VERSION,
            master_memory_version: MASTERMEMORY_VERSION.to_owned(),
            message_pack_version: MESSAGEPACK_VERSION.to_owned(),
            binary_path,
            binary_size,
            table_count: 0,
            record_count: 0,
            tables: Vec::new(),
        }
    }

    #[test]
    fn build_report_accepts_requested_path_with_spaces() {
        let directory = tempdir().expect("temporary directory");
        let output_directory = directory.path().join("output directory with spaces");
        fs::create_dir(&output_directory).expect("output directory");
        let output_path = output_directory.join("masterdata.bytes");
        fs::write(&output_path, b"binary").expect("binary");

        let result = validate_build_report(&report(output_path.clone(), 6), &request(output_path));

        assert!(result.is_ok());
    }

    #[test]
    fn build_report_accepts_equivalent_existing_path() {
        let directory = tempdir().expect("temporary directory");
        let output_directory = directory.path().join("output");
        fs::create_dir(&output_directory).expect("output directory");
        let output_path = output_directory.join("masterdata.bytes");
        fs::write(&output_path, b"binary").expect("binary");
        let equivalent_path = output_directory.join(".").join("masterdata.bytes");

        assert!(same_filesystem_path(&output_path, &equivalent_path));
        let result = validate_build_report(&report(equivalent_path, 6), &request(output_path));

        assert!(result.is_ok());
    }

    #[test]
    fn build_report_rejects_different_existing_file() {
        let directory = tempdir().expect("temporary directory");
        let requested_path = directory.path().join("requested.bytes");
        let reported_path = directory.path().join("sibling.bytes");
        fs::write(&requested_path, b"binary").expect("requested binary");
        fs::write(&reported_path, b"binary").expect("reported binary");

        assert!(!same_filesystem_path(&requested_path, &reported_path));
        let error = validate_build_report(&report(reported_path, 6), &request(requested_path))
            .expect_err("different files must be rejected");

        assert_eq!(error.diagnostic().code, "E-DOTNET-BUILDER-REPORT");
    }

    #[cfg(windows)]
    #[test]
    fn build_report_accepts_windows_short_path_for_same_file() {
        let directory = tempdir().expect("temporary directory");
        let output_directory = directory.path().join("output directory with spaces");
        fs::create_dir(&output_directory).expect("output directory");
        let output_path = output_directory.join("masterdata.bytes");
        fs::write(&output_path, b"binary").expect("binary");

        let Some(short_path) = windows_short_path(&output_path) else {
            eprintln!("Windows 8.3 short paths are unavailable; skipping representation check");
            return;
        };
        if short_path == output_path {
            eprintln!("Windows 8.3 short paths are disabled; skipping representation check");
            return;
        }

        let result = validate_build_report(&report(short_path, 6), &request(output_path));

        assert!(result.is_ok());
    }

    #[cfg(windows)]
    fn windows_short_path(path: &Path) -> Option<PathBuf> {
        use std::os::windows::ffi::OsStrExt;

        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut buffer = vec![0u16; 32_768];
        let length = unsafe {
            GetShortPathNameW(wide_path.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
        };
        if length == 0 || length as usize >= buffer.len() {
            return None;
        }
        Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..length as usize],
        )))
    }

    #[cfg(windows)]
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetShortPathNameW(
            long_path: *const u16,
            short_path: *mut u16,
            buffer_length: u32,
        ) -> u32;
    }
}
