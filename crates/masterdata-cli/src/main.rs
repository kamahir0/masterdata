use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use masterdata_app::{
    BuildAndPublishFailure, BuildExecution, NativeApplicationService, PublishExecutionReport,
    PublishTargetStatus,
};
use masterdata_core::{ErrorKind, InitOptions, MasterdataError, PublishTargetKind, Result};

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
    /// Validate and build the project's canonical artifacts.
    Build(BuildArgs),
    /// Publish the existing receipt-valid canonical artifact set.
    Publish,
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
    #[arg(long, conflicts_with = "publish")]
    dry_run: bool,
    /// Publish the newly built canonical artifact set after a successful full build.
    #[arg(long, conflicts_with = "dry_run")]
    publish: bool,
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
            "E-CLI-CURRENT-DIRECTORY",
            ErrorKind::Io,
            format!("could not determine current directory: {error}"),
        )
    })?;
    let service = NativeApplicationService::new();

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
                println!("canonical artifact root: {}", info.artifact_root.display());
                println!("canonical C# output: {}", info.csharp_output.display());
                println!("canonical binary output: {}", info.binary_output.display());
                println!("build cache: {}", info.cache.display());
                if !info.publish_targets.is_empty() {
                    println!("configured publish targets:");
                    for target in info.publish_targets {
                        println!("  - {:?}: {}", target.kind, target.resolved_path.display());
                    }
                }
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
                    "validation: {} ({} file(s), {} schema(s), {} type(s), {} data document(s))",
                    if report.valid { "ok" } else { "failed" },
                    report.files_scanned,
                    report.schema_documents,
                    report.type_documents,
                    report.data_documents
                );
                for diagnostic in &report.diagnostics {
                    println!("  - {diagnostic}");
                }
            }
            if !report.valid {
                return Err(MasterdataError::new(
                    "E-CLI-VALIDATION-FAILED",
                    ErrorKind::Validation,
                    "project validation failed",
                ));
            }
        }
        Command::Build(args) => {
            if args.publish {
                match service.build_and_publish(cli.project.as_deref(), &current_dir) {
                    Ok(execution) => {
                        render_build_execution(&execution.build, false);
                        println!("{}", render_publish_report(&execution.publish, "succeeded"));
                    }
                    Err(failure) => {
                        render_build_publish_failure(&failure);
                        return Err(failure.into_error());
                    }
                }
            } else {
                let execution =
                    service.build(cli.project.as_deref(), &current_dir, args.dry_run)?;
                render_build_execution(&execution, args.dry_run);
            }
        }
        Command::Publish => match service.publish(cli.project.as_deref(), &current_dir) {
            Ok(report) => println!("{}", render_publish_report(&report, "succeeded")),
            Err(failure) => {
                eprintln!("{}", render_publish_report(&failure.report, "failed"));
                return Err(failure.error);
            }
        },
    }
    Ok(())
}

fn render_build_execution(execution: &BuildExecution, dry_run: bool) {
    let plan = &execution.plan;
    let generation = &execution.generation;
    println!(
        "schema source content hash: {}",
        plan.schema_source_content_hash
    );
    println!("C# files planned: {}", generation.files.len());
    for file in &generation.files {
        println!("  - {}", file.relative_path.display());
    }
    for note in &generation.notes {
        println!("note: {}", note.message);
    }
    if dry_run {
        println!("dry-run: no files written and no .NET builder invoked");
    } else {
        println!(
            "wrote {} canonical C# file(s) to {}",
            execution.written_files.len(),
            plan.csharp_output.display()
        );
        if let Some(binary) = &execution.binary {
            println!(
                "built canonical MasterMemory binary: {} ({} bytes)",
                binary.binary_path.display(),
                binary.binary_size
            );
        }
    }
}

fn render_build_publish_failure(failure: &BuildAndPublishFailure) {
    if let BuildAndPublishFailure::Publish { build, failure } = failure {
        render_build_execution(build, false);
        eprintln!("{}", render_publish_report(&failure.report, "failed"));
    }
}

fn render_publish_report(report: &PublishExecutionReport, outcome: &str) -> String {
    let mut rendered = format!("publish: {outcome}");
    if report.targets.is_empty() {
        rendered.push_str("\n  (no targets)");
        return rendered;
    }

    for target in &report.targets {
        let status = match target.status {
            PublishTargetStatus::NotAttempted => "not attempted",
            PublishTargetStatus::Succeeded => "succeeded",
            PublishTargetStatus::Failed => "failed",
        };
        rendered.push_str(&format!(
            "\n  [{}] {} {} {status}",
            target.index,
            publish_target_kind_name(target.kind),
            target.configured_path
        ));
        if let Some(failure) = &target.failure {
            rendered.push_str(&format!(": {failure}"));
        }
    }
    rendered
}

fn publish_target_kind_name(kind: PublishTargetKind) -> &'static str {
    match kind {
        PublishTargetKind::CSharp => "csharp",
        PublishTargetKind::Binary => "binary",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use masterdata_app::{PublishTargetResult, PublishTargetStatus};
    use masterdata_core::{Diagnostic, ErrorKind, PublishTargetKind};

    use super::{PublishExecutionReport, render_publish_report};

    #[test]
    fn publish_report_preserves_per_target_status() {
        let report = PublishExecutionReport {
            targets: vec![
                PublishTargetResult {
                    index: 0,
                    kind: PublishTargetKind::CSharp,
                    configured_path: "first".to_owned(),
                    destination: PathBuf::from("first"),
                    status: PublishTargetStatus::Succeeded,
                    failure: None,
                },
                PublishTargetResult {
                    index: 1,
                    kind: PublishTargetKind::Binary,
                    configured_path: "second.bytes".to_owned(),
                    destination: PathBuf::from("second.bytes"),
                    status: PublishTargetStatus::Failed,
                    failure: Some(Diagnostic::new(
                        "E-PUBLISH-TEST",
                        ErrorKind::Validation,
                        "target failed",
                    )),
                },
                PublishTargetResult {
                    index: 2,
                    kind: PublishTargetKind::CSharp,
                    configured_path: "third".to_owned(),
                    destination: PathBuf::from("third"),
                    status: PublishTargetStatus::Succeeded,
                    failure: None,
                },
            ],
        };

        let rendered = render_publish_report(&report, "failed");
        let first = rendered.find("[0] csharp first succeeded").unwrap();
        let second = rendered.find("[1] binary second.bytes failed").unwrap();
        let third = rendered.find("[2] csharp third succeeded").unwrap();

        assert!(first < second && second < third);
        assert!(rendered.contains("[E-PUBLISH-TEST] target failed"));
    }
}
