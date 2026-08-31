//! Shared application workflows for adapters such as the CLI and Tauri GUI.
//!
//! This crate orchestrates project discovery, validation, C# generation, and
//! the .NET boundary. Domain semantics remain in `masterdata-core`, C# syntax
//! remains in `masterdata-codegen-csharp`, and process invocation remains in
//! `masterdata-dotnet`.

use std::path::{Path, PathBuf};

use masterdata_codegen_csharp::{CSharpGenerationPlan, CSharpGenerator};
use masterdata_core::{
    BuildPlan, BuildSelection, InitOptions, ProjectInfo, ProjectService, Result, ValidationReport,
};
use masterdata_dotnet::{BridgeSmokeReport, DotnetBridge, MasterMemorySpikeReport};

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
        let written_files = if dry_run {
            Vec::new()
        } else {
            // Keep final generated output untouched until the build boundary
            // succeeds; writing first would leave a misleading partial artifact
            // after a failed build. Regression:
            // failed_mastermemory_build_does_not_write_final_generated_output.
            self.dotnet.build_mastermemory(&plan)?;
            self.generator
                .write_to(&generation, &plan.generated_output)?
        };
        Ok(BuildExecution {
            plan,
            generation,
            written_files,
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
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::ApplicationService;
    use masterdata_core::PROJECT_CONFIG_FILENAME;

    #[test]
    fn failed_mastermemory_build_does_not_write_final_generated_output() {
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
            .expect_err("production MasterMemory build is not implemented");

        assert_eq!(
            error.diagnostic().code,
            "E-DOTNET-MASTERMEMORY-NOT-IMPLEMENTED"
        );
        assert!(!directory.path().join(".masterdata/generated").exists());
    }
}
