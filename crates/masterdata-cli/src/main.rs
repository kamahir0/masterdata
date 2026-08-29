use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use masterdata_codegen_csharp::CSharpGenerator;
use masterdata_core::{ErrorKind, InitOptions, MasterdataError, ProjectService, Result};
use masterdata_dotnet::DotnetBridge;

#[derive(Debug, Parser)]
#[command(
    name = "masterdata",
    version,
    about = "YAML-first MasterMemory development tooling"
)]
struct Cli {
    /// Explicit project directory or path to masterdata.toml.
    #[arg(long, global = true, value_name = "PATH")]
    project: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a masterdata.toml project marker and default source root.
    Init(InitArgs),
    /// Show local toolchain and GUI dependency diagnostics.
    Doctor,
    /// Display the project selected by discovery.
    ProjectInfo(OutputArgs),
    /// Parse and run basic semantic validation on project sources.
    Validate(OutputArgs),
    /// Prepare the build boundary. The MasterMemory binary stage is explicit and currently unavailable.
    Build(BuildArgs),
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Directory to initialize. Defaults to the current directory.
    #[arg(default_value = ".", value_name = "PATH")]
    path: PathBuf,
    /// Stable project identity. Defaults to `<directory>.masterdata`.
    #[arg(long)]
    id: Option<String>,
    /// Human-readable project name. Defaults to the directory name.
    #[arg(long)]
    name: Option<String>,
    /// Initial project version.
    #[arg(long, default_value = "0.1.0")]
    version: String,
}

#[derive(Debug, Args)]
struct BuildArgs {
    /// Print validation, schema hash, and C# generation plan without writing files.
    #[arg(long)]
    dry_run: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {}", error.diagnostic());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let current_dir = std::env::current_dir().map_err(|error| {
        MasterdataError::new(
            "CLI-001",
            ErrorKind::Io,
            format!("could not determine current directory: {error}"),
        )
    })?;
    let service = ProjectService::new();

    match cli.command {
        Command::Init(args) => {
            let target = if args.path.is_absolute() {
                args.path
            } else {
                current_dir.join(args.path)
            };
            let directory_name = target
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or("masterdata");
            let options = InitOptions {
                project_id: args
                    .id
                    .unwrap_or_else(|| format!("{directory_name}.masterdata")),
                name: args.name.unwrap_or_else(|| directory_name.to_owned()),
                version: args.version,
            };
            let info = service.init(&target, &options)?;
            println!(
                "initialized {} at {}",
                info.name,
                info.project_root.display()
            );
            println!("config: {}", info.config_path.display());
        }
        Command::Doctor => {
            let report = masterdata_core::diagnostics::collect_toolchain_report();
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("diagnostics serialize")
            );
        }
        Command::ProjectInfo(args) => {
            let info = service.project_info(cli.project.as_deref(), &current_dir)?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&info).expect("project info serialize")
                );
            } else {
                println!("project: {} ({})", info.name, info.project_id);
                println!("root: {}", info.project_root.display());
                println!("config: {}", info.config_path.display());
                println!("version: {}", info.version);
                println!("source roots:");
                for root in info.source_roots {
                    println!("  - {}", root.display());
                }
                println!("build output: {}", info.build_output.display());
            }
        }
        Command::Validate(args) => {
            let report = service.validate(cli.project.as_deref(), &current_dir)?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("validation serialize")
                );
            } else {
                println!(
                    "validation: {} ({} file(s), {} schema(s), {} data document(s))",
                    if report.valid { "ok" } else { "failed" },
                    report.files_scanned,
                    report.schema_documents,
                    report.data_documents
                );
                for diagnostic in &report.diagnostics {
                    println!("  - {diagnostic}");
                }
            }
            if !report.valid {
                return Err(MasterdataError::new(
                    "CLI-VALIDATE-001",
                    ErrorKind::Validation,
                    "project validation failed",
                ));
            }
        }
        Command::Build(args) => {
            let plan = service.prepare_build(cli.project.as_deref(), &current_dir)?;
            let generation = CSharpGenerator::default().plan(&plan)?;
            println!("schema hash: {}", plan.schema_hash);
            println!("C# files planned: {}", generation.files.len());
            for file in &generation.files {
                println!("  - {}", file.relative_path.display());
            }
            for note in &generation.notes {
                println!("note: {}", note.message);
            }
            if args.dry_run {
                println!("dry-run: no files written and no .NET builder invoked");
            } else {
                let output_dir = &plan.generated_output;
                let written = CSharpGenerator::default().write_to(&generation, output_dir)?;
                println!(
                    "wrote {} C# scaffold file(s) to {}",
                    written.len(),
                    output_dir.display()
                );
                DotnetBridge::default().build_mastermemory(&plan)?;
            }
        }
    }
    Ok(())
}
