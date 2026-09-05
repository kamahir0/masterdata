use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use masterdata_app::NativeApplicationService;
use masterdata_core::{ErrorKind, MasterdataError, Project, Result};

mod rationale_check;
mod spec_check;

#[derive(Debug, Parser)]
#[command(
    name = "cargo xtask",
    version,
    about = "masterdata repository development tasks"
)]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    /// Inspect OS, architecture, compilers, SDKs, and GUI prerequisites.
    Doctor,
    /// Copy minimal fixture into target/dev-project and run the CLI smoke flow.
    Cli,
    /// Copy minimal fixture and start the Tauri development application.
    Gui,
    /// Run fixture discovery, validation, production binary build, and .NET bridge smoke test.
    TestIntegration,
    /// Check specification, RFC, proposal, ADR, and relative-link integrity.
    CheckSpecs,
    /// Check high-confidence references from implementation rationale comments.
    CheckRationale,
    /// Run the isolated real MasterMemory v3 .NET technical spike.
    MastermemorySpike,
    /// Run the repository's main checks.
    CheckAll,
    /// Recreate target/dev-project from fixtures/minimal.
    DevReset,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask error: {}", error.diagnostic());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CommandKind::Doctor => doctor(),
        CommandKind::Cli => cli_smoke(),
        CommandKind::Gui => gui(),
        CommandKind::TestIntegration => test_integration(),
        CommandKind::CheckSpecs => check_specs(),
        CommandKind::CheckRationale => check_rationale(),
        CommandKind::MastermemorySpike => mastermemory_spike(),
        CommandKind::CheckAll => check_all(),
        CommandKind::DevReset => {
            let destination = reset_dev_project()?;
            println!("development project recreated at {}", destination.display());
            Ok(())
        }
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask must live two directories below repository root")
        .to_path_buf()
}

fn fixture_root() -> PathBuf {
    repository_root().join("fixtures").join("minimal")
}

fn development_project_root() -> PathBuf {
    repository_root().join("target").join("dev-project")
}

fn reset_dev_project() -> Result<PathBuf> {
    let source = fixture_root();
    let destination = development_project_root();
    if !source.is_dir() {
        return Err(MasterdataError::new(
            "E-XTASK-FIXTURE-MISSING",
            ErrorKind::Io,
            "minimal fixture directory is missing",
        )
        .with_source(source));
    }
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|error| {
            MasterdataError::new(
                "E-XTASK-FIXTURE-CLEAR",
                ErrorKind::Io,
                format!("could not clear development project: {error}"),
            )
            .with_source(destination.clone())
        })?;
    }
    copy_directory(&source, &destination)?;
    Ok(destination)
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|error| {
        MasterdataError::new(
            "E-XTASK-FIXTURE-CREATE",
            ErrorKind::Io,
            format!("could not create development project: {error}"),
        )
        .with_source(destination.to_path_buf())
    })?;
    let entries = fs::read_dir(source).map_err(|error| {
        MasterdataError::new(
            "E-XTASK-FIXTURE-READ",
            ErrorKind::Io,
            format!("could not read fixture: {error}"),
        )
        .with_source(source.to_path_buf())
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            MasterdataError::new(
                "E-XTASK-FIXTURE-ENUMERATE",
                ErrorKind::Io,
                format!("could not enumerate fixture: {error}"),
            )
            .with_source(source.to_path_buf())
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| {
            MasterdataError::new(
                "E-XTASK-FIXTURE-INSPECT",
                ErrorKind::Io,
                format!("could not inspect fixture entry: {error}"),
            )
            .with_source(source_path.clone())
        })?;
        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                MasterdataError::new(
                    "E-XTASK-FIXTURE-COPY",
                    ErrorKind::Io,
                    format!("could not copy fixture file: {error}"),
                )
                .with_source(source_path.clone())
            })?;
        }
    }
    Ok(())
}

fn doctor() -> Result<()> {
    let report = masterdata_core::diagnostics::collect_toolchain_report();
    println!(
        "host: {} / {}",
        report.operating_system, report.architecture
    );
    println!("tools:");
    for tool in report.tools.iter().chain(report.gui_dependencies.iter()) {
        let state = if tool.available { "ok" } else { "missing" };
        let detail = if tool.detail.is_empty() {
            "no version output"
        } else {
            &tool.detail
        };
        println!("  {state:7} {:24} {detail}", tool.name);
    }
    println!(
        "doctor does not modify the machine; install missing optional GUI dependencies manually."
    );
    Ok(())
}

fn cli_smoke() -> Result<()> {
    let destination = reset_dev_project()?;
    println!("development project: {}", destination.display());
    run_program(
        cargo_command(),
        [
            OsString::from("run"),
            OsString::from("--quiet"),
            OsString::from("--package"),
            OsString::from("masterdata-cli"),
            OsString::from("--"),
            OsString::from("--project"),
            destination.as_os_str().to_owned(),
            OsString::from("project-info"),
        ],
        &repository_root(),
        &[],
    )?;
    run_program(
        cargo_command(),
        [
            OsString::from("run"),
            OsString::from("--quiet"),
            OsString::from("--package"),
            OsString::from("masterdata-cli"),
            OsString::from("--"),
            OsString::from("--project"),
            destination.as_os_str().to_owned(),
            OsString::from("build"),
            OsString::from("--dry-run"),
        ],
        &repository_root(),
        &[],
    )?;
    run_program(
        cargo_command(),
        [
            OsString::from("run"),
            OsString::from("--quiet"),
            OsString::from("--package"),
            OsString::from("masterdata-cli"),
            OsString::from("--"),
            OsString::from("--project"),
            destination.as_os_str().to_owned(),
            OsString::from("validate"),
        ],
        &repository_root(),
        &[],
    )
}

fn gui() -> Result<()> {
    let destination = reset_dev_project()?;
    let gui_directory = repository_root().join("apps").join("gui");
    println!("development project: {}", destination.display());
    println!("starting Tauri GUI; close the window or press Ctrl-C to stop");
    run_program(
        npm_command(),
        [
            OsString::from("run"),
            OsString::from("tauri"),
            OsString::from("--"),
            OsString::from("dev"),
        ],
        &gui_directory,
        &[(
            OsString::from("MASTERDATA_PROJECT_PATH"),
            destination.into_os_string(),
        )],
    )
}

fn test_integration() -> Result<()> {
    let destination = reset_dev_project()?;
    let project = Project::discover(Some(&destination), &repository_root())?;
    let documents = project.load_documents()?;
    let report = masterdata_core::validate_documents(&documents);
    if !report.valid {
        return Err(MasterdataError::new(
            "E-XTASK-INTEGRATION-VALIDATION",
            ErrorKind::Validation,
            "minimal fixture validation failed",
        ));
    }
    let service = NativeApplicationService::new();
    let plan = service.prepare_build(Some(&destination), &repository_root())?;
    let generation = service.plan_csharp(&plan)?;
    println!("fixture: {}", destination.display());
    println!(
        "validated {} file(s), schema source content hash {}",
        report.files_scanned, plan.schema_source_content_hash
    );
    println!("planned {} C# scaffold file(s)", generation.files.len());

    let execution = service.build(Some(&destination), &repository_root(), false)?;
    let binary = execution.binary.ok_or_else(|| {
        MasterdataError::new(
            "E-XTASK-INTEGRATION-BINARY",
            ErrorKind::ExternalTool,
            "production integration build did not return a binary report",
        )
    })?;
    if !binary.binary_path.is_file() || binary.binary_size == 0 {
        return Err(MasterdataError::new(
            "E-XTASK-INTEGRATION-BINARY",
            ErrorKind::ExternalTool,
            "production integration build did not publish a valid binary",
        ));
    }
    println!(
        "production binary: {} ({} bytes, {} record(s))",
        binary.binary_path.display(),
        binary.binary_size,
        binary.record_count
    );

    let smoke = service.bridge_smoke_test(&repository_root())?;
    println!(
        ".NET bridge smoke test: {:?} ({})",
        smoke.status, smoke.detail
    );
    Ok(())
}

fn mastermemory_spike() -> Result<()> {
    let report = NativeApplicationService::new().mastermemory_spike(&repository_root())?;
    println!(
        "MasterMemory {} / MessagePack {}: binary {} bytes, reloaded item {} = {}",
        report.master_memory_version,
        report.message_pack_version,
        report.binary_size,
        report.reloaded_item_id,
        report.reloaded_item_name
    );
    Ok(())
}

fn check_specs() -> Result<()> {
    let summary = spec_check::check_repository(&repository_root())?;
    println!(
        "spec checks passed: {} spec file(s), {} GUI spec file(s), {} requirement ID(s), {} ADR number(s), {} RFC number(s), {} proposal number(s), {} relative link(s)",
        summary.spec_files,
        summary.gui_spec_files,
        summary.requirement_ids,
        summary.adr_numbers,
        summary.rfc_numbers,
        summary.proposal_numbers,
        summary.relative_links
    );
    Ok(())
}

fn check_rationale() -> Result<()> {
    let summary = rationale_check::check_repository(&repository_root())?;
    println!(
        "rationale checks passed: {} source file(s), {} reference-bearing comment block(s), {} reference(s)",
        summary.source_files, summary.rationale_blocks, summary.references
    );
    Ok(())
}

fn check_all() -> Result<()> {
    let root = repository_root();
    check_specs()?;
    check_rationale()?;
    run_program(
        cargo_command(),
        [
            OsString::from("fmt"),
            OsString::from("--all"),
            OsString::from("--"),
            OsString::from("--check"),
        ],
        &root,
        &[],
    )?;
    run_program(
        cargo_command(),
        [
            OsString::from("clippy"),
            OsString::from("--workspace"),
            OsString::from("--all-targets"),
            OsString::from("--all-features"),
            OsString::from("--"),
            OsString::from("-D"),
            OsString::from("warnings"),
        ],
        &root,
        &[],
    )?;
    run_program(
        cargo_command(),
        [
            OsString::from("test"),
            OsString::from("--workspace"),
            OsString::from("--exclude"),
            OsString::from("masterdata-gui"),
        ],
        &root,
        &[],
    )?;
    run_program(
        npm_command(),
        [OsString::from("run"), OsString::from("frontend:lint")],
        &root,
        &[],
    )?;
    run_program(
        npm_command(),
        [OsString::from("run"), OsString::from("frontend:test")],
        &root,
        &[],
    )?;
    run_program(
        npm_command(),
        [OsString::from("run"), OsString::from("frontend:build")],
        &root,
        &[],
    )?;
    run_program(
        cargo_command(),
        [
            OsString::from("check"),
            OsString::from("--package"),
            OsString::from("masterdata-gui"),
        ],
        &root,
        &[],
    )?;
    run_program(
        cargo_command(),
        [
            OsString::from("test"),
            OsString::from("--package"),
            OsString::from("masterdata-gui"),
        ],
        &root,
        &[],
    )?;
    mastermemory_spike()?;
    test_integration()
}

fn run_program<I>(
    program: OsString,
    args: I,
    current_dir: &Path,
    environment: &[(OsString, OsString)],
) -> Result<()>
where
    I: IntoIterator<Item = OsString>,
{
    let args: Vec<OsString> = args.into_iter().collect();
    let display_args = args
        .iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("$ {} {}", program.to_string_lossy(), display_args);
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(current_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (key, value) in environment {
        command.env(key, value);
    }
    let status = command.status().map_err(|error| {
        MasterdataError::new(
            "E-XTASK-COMMAND-START",
            ErrorKind::ExternalTool,
            format!("could not start `{}`: {error}", program.to_string_lossy()),
        )
    })?;
    if !status.success() {
        return Err(MasterdataError::new(
            "E-XTASK-COMMAND-FAILED",
            ErrorKind::ExternalTool,
            format!("command exited unsuccessfully: {program:?} {display_args}"),
        ));
    }
    Ok(())
}

fn cargo_command() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

#[cfg(windows)]
fn npm_command() -> OsString {
    OsString::from("npm.cmd")
}

#[cfg(not(windows))]
fn npm_command() -> OsString {
    OsString::from("npm")
}
