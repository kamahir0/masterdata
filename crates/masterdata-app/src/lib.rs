//! Shared application workflows for adapters such as the CLI and Tauri GUI.
//!
//! This crate orchestrates project discovery, validation, C# generation, and
//! the .NET boundary. Domain semantics remain in `masterdata-core`, C# syntax
//! remains in `masterdata-codegen-csharp`, and process invocation remains in
//! `masterdata-dotnet`.

use std::fs;
use std::path::{Path, PathBuf};

use masterdata_codegen_csharp::{CSharpGenerationPlan, CSharpGenerator};
use masterdata_core::{
    BuildPlan, BuildSelection, ErrorKind, InitOptions, MasterdataError, ProjectInfo,
    ProjectService, Result, ValidationReport,
};
use masterdata_dotnet::{
    BridgeSmokeReport, DotnetBridge, MasterMemoryBuildReport, MasterMemoryBuildRequest,
    MasterMemorySpikeReport,
};
use tempfile::TempDir;

#[derive(Debug, Clone)]
pub struct ApplicationService {
    project: ProjectService,
    generator: CSharpGenerator,
    dotnet: DotnetBridge,
}

impl Default for ApplicationService {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationService {
    pub fn new() -> Self {
        Self {
            project: ProjectService::new(),
            generator: CSharpGenerator::default(),
            dotnet: DotnetBridge::default(),
        }
    }

    /// Construct the shared workflow with an explicit bridge. This is useful
    /// for failure-path tests and keeps process invocation out of CLI/GUI
    /// adapters.
    pub fn with_dotnet(dotnet: DotnetBridge) -> Self {
        Self {
            project: ProjectService::new(),
            generator: CSharpGenerator::default(),
            dotnet,
        }
    }

    pub fn project_info(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ProjectInfo> {
        self.project.project_info(explicit_project, current_dir)
    }

    pub fn validate(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<ValidationReport> {
        self.project.validate(explicit_project, current_dir)
    }

    pub fn init(&self, root: &Path, options: &InitOptions) -> Result<ProjectInfo> {
        self.project.init(root, options)
    }

    pub fn prepare_build(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
    ) -> Result<BuildPlan> {
        self.project.prepare_build(explicit_project, current_dir)
    }

    pub fn prepare_build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
    ) -> Result<BuildPlan> {
        self.project
            .prepare_build_with_selection(explicit_project, current_dir, selection)
    }

    pub fn plan_csharp(&self, plan: &BuildPlan) -> Result<CSharpGenerationPlan> {
        self.generator.plan(plan)
    }

    /// Run the shared build workflow. `dry_run` stops after planning so that
    /// adapters can inspect the same result without writing or invoking .NET.
    pub fn build(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        dry_run: bool,
    ) -> Result<BuildExecution> {
        self.build_with_selection(
            explicit_project,
            current_dir,
            &BuildSelection::unfiltered(),
            dry_run,
        )
    }

    pub fn build_with_selection(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        selection: &BuildSelection,
        dry_run: bool,
    ) -> Result<BuildExecution> {
        let plan = self.prepare_build_with_selection(explicit_project, current_dir, selection)?;
        let generation = self.generator.plan(&plan)?;
        let (written_files, binary) = if dry_run {
            (Vec::new(), None)
        } else {
            let final_binary = plan.binary_output.clone().ok_or_else(|| {
                MasterdataError::new(
                    "E-BUILD-BINARY-OUTPUT-NOT-CONFIGURED",
                    ErrorKind::Config,
                    "non-dry-run build requires explicit build.binary_output",
                )
                .with_source(plan.project.config_path.clone())
            })?;
            let parent = final_binary.parent().unwrap_or_else(|| Path::new("."));
            fs::create_dir_all(parent).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-BINARY-PARENT-CREATE",
                    ErrorKind::Io,
                    format!("could not create binary output directory: {error}"),
                )
                .with_source(parent.to_path_buf())
            })?;

            // WHY: Keep every intermediate file beside the final binary until
            // the real builder and reload validation have succeeded. This
            // preserves the previous artifact on any prepare/compile/run
            // failure and gives the final rename same-filesystem semantics.
            // EVIDENCE: docs/specs/build-pipeline.md; Regression: unavailable_dotnet_preserves_existing_binary_and_generated_output.
            let workspace = TempDir::new_in(parent).map_err(|error| {
                MasterdataError::new(
                    "E-BUILD-STAGING-CREATE",
                    ErrorKind::Io,
                    format!("could not create build staging workspace: {error}"),
                )
                .with_source(parent.to_path_buf())
            })?;
            let staged_generated = workspace.path().join("Generated");
            self.generator.write_to(&generation, &staged_generated)?;
            let staged_binary = workspace.path().join("masterdata.bytes");
            let request =
                MasterMemoryBuildRequest::from_plan(&plan, &generation, staged_binary.clone())?;
            let mut report =
                self.dotnet
                    .build_mastermemory(&request, &staged_generated, workspace.path())?;

            // WHY: The .NET builder writes only a staging file. Replacing the
            // final destination in one filesystem operation prevents readers
            // from observing a partially written binary and avoids the
            // non-atomic remove-then-rename pattern.
            // EVIDENCE: docs/specs/build-pipeline.md; Regression: atomic_replace_publishes_when_final_is_absent_or_existing.
            atomic_replace(&staged_binary, &final_binary)?;
            report.binary_path = final_binary;
            let written_files = self
                .generator
                .write_to(&generation, &plan.generated_output)?;
            (written_files, Some(report))
        };
        Ok(BuildExecution {
            plan,
            generation,
            written_files,
            binary,
        })
    }

    pub fn bridge_smoke_test(&self, repository_root: &Path) -> Result<BridgeSmokeReport> {
        self.dotnet.smoke_test(repository_root)
    }

    pub fn mastermemory_spike(&self, repository_root: &Path) -> Result<MasterMemorySpikeReport> {
        self.dotnet.mastermemory_spike(repository_root)
    }
}

#[derive(Debug, Clone)]
pub struct BuildExecution {
    pub plan: BuildPlan,
    pub generation: CSharpGenerationPlan,
    pub written_files: Vec<PathBuf>,
    pub binary: Option<MasterMemoryBuildReport>,
}

fn atomic_replace(staged: &Path, final_path: &Path) -> Result<()> {
    if !staged.is_file() {
        return Err(MasterdataError::new(
            "E-BUILD-BINARY-PUBLISH",
            ErrorKind::Io,
            "staged binary is missing",
        )
        .with_source(staged.to_path_buf()));
    }

    #[cfg(windows)]
    {
        atomic_replace_windows(staged, final_path)
    }
    #[cfg(not(windows))]
    {
        fs::rename(staged, final_path).map_err(|error| {
            MasterdataError::new(
                "E-BUILD-BINARY-PUBLISH",
                ErrorKind::Io,
                format!("could not atomically publish binary: {error}"),
            )
            .with_source(final_path.to_path_buf())
        })
    }
}

#[cfg(windows)]
fn atomic_replace_windows(staged: &Path, final_path: &Path) -> Result<()> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    const REPLACEFILE_WRITE_THROUGH: u32 = 0x00000001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x00000008;

    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut c_void,
            reserved: *mut c_void,
        ) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let staged = wide(staged);
    let final_path_wide = wide(final_path);
    let success = unsafe {
        if final_path.exists() {
            ReplaceFileW(
                final_path_wide.as_ptr(),
                staged.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ) != 0
        } else {
            MoveFileExW(
                staged.as_ptr(),
                final_path_wide.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            ) != 0
        }
    };
    if success {
        Ok(())
    } else {
        Err(MasterdataError::new(
            "E-BUILD-BINARY-PUBLISH",
            ErrorKind::Io,
            format!(
                "could not atomically publish binary: {}",
                std::io::Error::last_os_error()
            ),
        )
        .with_source(final_path.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{ApplicationService, atomic_replace};
    use masterdata_core::PROJECT_CONFIG_FILENAME;
    use masterdata_dotnet::DotnetBridge;

    #[test]
    fn missing_binary_output_is_rejected_without_writing_generated_output() {
        let directory = tempdir().expect("temporary directory");
        fs::create_dir(directory.path().join("sources")).expect("sources");
        fs::write(
            directory.path().join(PROJECT_CONFIG_FILENAME),
            "[project]\nid = \"build.test\"\nname = \"Build Test\"\nversion = \"0.1.0\"\n",
        )
        .expect("config");
        fs::write(
            directory.path().join("sources/item.yaml"),
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        )
        .expect("schema");

        let error = ApplicationService::new()
            .build(Some(directory.path()), directory.path(), false)
            .expect_err("binary output configuration");

        assert_eq!(
            error.diagnostic().code,
            "E-BUILD-BINARY-OUTPUT-NOT-CONFIGURED"
        );
        assert!(!directory.path().join(".masterdata/generated").exists());
    }

    #[test]
    fn unavailable_dotnet_preserves_existing_binary_and_generated_output() {
        let directory = test_project("binary_output = \".masterdata/output/masterdata.bytes\"\n");
        let binary_path = directory.path().join(".masterdata/output/masterdata.bytes");
        let generated_path = directory.path().join(".masterdata/generated/old.g.cs");
        fs::create_dir_all(binary_path.parent().expect("binary parent")).expect("binary parent");
        fs::create_dir_all(generated_path.parent().expect("generated parent"))
            .expect("generated parent");
        fs::write(&binary_path, b"OLD-BINARY").expect("old binary");
        fs::write(&generated_path, b"OLD-GENERATED").expect("old generated output");

        let service = ApplicationService::with_dotnet(DotnetBridge::with_executable(
            "/definitely/missing/masterdata-dotnet",
        ));
        let error = service
            .build(Some(directory.path()), directory.path(), false)
            .expect_err("missing dotnet SDK");

        assert_eq!(error.diagnostic().code, "E-DOTNET-SDK-UNAVAILABLE");
        assert_eq!(fs::read(&binary_path).expect("binary"), b"OLD-BINARY");
        assert_eq!(
            fs::read(&generated_path).expect("generated"),
            b"OLD-GENERATED"
        );
    }

    #[test]
    fn missing_builder_preserves_existing_binary_and_generated_output() {
        let directory = test_project("binary_output = \".masterdata/output/masterdata.bytes\"\n");
        let binary_path = directory.path().join(".masterdata/output/masterdata.bytes");
        let generated_path = directory.path().join(".masterdata/generated/old.g.cs");
        fs::create_dir_all(binary_path.parent().expect("binary parent")).expect("binary parent");
        fs::create_dir_all(generated_path.parent().expect("generated parent"))
            .expect("generated parent");
        fs::write(&binary_path, b"OLD-BINARY").expect("old binary");
        fs::write(&generated_path, b"OLD-GENERATED").expect("old generated output");

        let service = ApplicationService::with_dotnet(
            DotnetBridge::new().with_repository_root(directory.path()),
        );
        let error = service
            .build(Some(directory.path()), directory.path(), false)
            .expect_err("missing builder project");

        assert_eq!(error.diagnostic().code, "E-DOTNET-BUILDER-MISSING");
        assert_eq!(fs::read(&binary_path).expect("binary"), b"OLD-BINARY");
        assert_eq!(
            fs::read(&generated_path).expect("generated"),
            b"OLD-GENERATED"
        );
    }

    #[test]
    fn atomic_replace_publishes_when_final_is_absent_or_existing() {
        let directory = tempdir().expect("temporary directory");
        let staged = directory.path().join("staged.bytes");
        let final_path = directory.path().join("output.bytes");
        fs::write(&staged, b"NEW").expect("staged");
        atomic_replace(&staged, &final_path).expect("first publish");
        assert_eq!(fs::read(&final_path).expect("final"), b"NEW");

        let staged = directory.path().join("staged-again.bytes");
        fs::write(&staged, b"NEWER").expect("staged again");
        atomic_replace(&staged, &final_path).expect("replacement publish");
        assert_eq!(fs::read(&final_path).expect("final"), b"NEWER");
        assert!(!staged.exists());
    }

    #[test]
    fn atomic_replace_rejects_missing_staged_file() {
        let directory = tempdir().expect("temporary directory");
        let error = atomic_replace(
            &directory.path().join("missing.bytes"),
            &directory.path().join("output.bytes"),
        )
        .expect_err("missing staged file");
        assert_eq!(error.diagnostic().code, "E-BUILD-BINARY-PUBLISH");
    }

    fn test_project(build: &str) -> tempfile::TempDir {
        let directory = tempdir().expect("temporary directory");
        fs::create_dir(directory.path().join("sources")).expect("sources");
        fs::write(
            directory.path().join(PROJECT_CONFIG_FILENAME),
            format!(
                "[project]\nid = \"build.test\"\nname = \"Build Test\"\nversion = \"0.1.0\"\n\n[build]\n{build}"
            ),
        )
        .expect("config");
        fs::write(
            directory.path().join("sources/item.yaml"),
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        )
        .expect("schema");
        directory
    }
}
