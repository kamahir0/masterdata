//! Automation adapter; Desktop uses the same Rust operations with its own
//! long-lived workspace and authoring lifecycle.
use masterdata_engine::{
    Error, Result,
    delivery::{self, BuildOptions},
    migration,
    native::{self, Outcome, publish},
    project::{self, Metadata, Project},
    source::content_identity,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, Write},
    path::{Path, PathBuf},
};

const HELP: &str = "MasterData — YAML master-data authoring\n\
\nCommands: init, doctor, validate, build, publish, migrate\n\
\nCommon: --project <directory|masterdata.toml> --json\n\
init: --id <id> --name <name> --project-version <version>\n\
validate/build: --profile <name>\n\
build: --dry-run --publish\n\
publish: --dry-run\n\
migrate: --command <json file> [--apply] [--allow-destructive] [--dry-run]\n\
migrate recovery: --recheck <id> | --restore <id> --authorize-restore\n\
\nMigration defaults to Plan only. Numeric command values are lossless strings.\n\
Publish distributes the last successful receipt and never builds implicitly.";

#[derive(Default)]
struct Arguments {
    command: String,
    values: BTreeMap<String, OsString>,
    flags: Vec<String>,
}
impl Arguments {
    fn parse(args: Vec<OsString>) -> Result<Self> {
        let mut parsed = Self::default();
        let mut input = args.into_iter();
        while let Some(value) = input.next() {
            let name = value
                .to_str()
                .ok_or_else(|| Error::new("E-CLI-ARGUMENT", "option or command must be UTF-8"))?;
            if matches!(name, "--help" | "-h" | "--version") {
                parsed.flags.push(name.into());
                continue;
            }
            if name.starts_with('-') {
                match name {
                    "--json"
                    | "--dry-run"
                    | "--publish"
                    | "--apply"
                    | "--allow-destructive"
                    | "--authorize-restore" => {
                        if parsed.flags.iter().any(|flag| flag == name) {
                            return Err(argument(format!("duplicate {name}")));
                        }
                        parsed.flags.push(name.into());
                    }
                    "--project" | "--profile" | "--id" | "--name" | "--project-version"
                    | "--command" | "--recheck" | "--restore" => {
                        let next = input
                            .next()
                            .ok_or_else(|| argument(format!("{name} requires a value")))?;
                        if next.to_str().is_some_and(|s| s.starts_with("--"))
                            || parsed.values.insert(name.into(), next).is_some()
                        {
                            return Err(argument(format!("missing or duplicate value for {name}")));
                        }
                    }
                    _ => return Err(argument(format!("unknown option {name}"))),
                }
            } else if parsed.command.is_empty() {
                parsed.command = name.into();
            } else {
                return Err(argument(format!("unexpected argument {name}")));
            }
        }
        if parsed.flag("--help") || parsed.flag("-h") || parsed.flag("--version") {
            return Ok(parsed);
        }
        if !matches!(
            parsed.command.as_str(),
            "init" | "doctor" | "validate" | "build" | "publish" | "migrate"
        ) {
            return Err(argument(
                "expected init, doctor, validate, build, publish, or migrate",
            ));
        }
        let (options, flags): (&[&str], &[&str]) = match parsed.command.as_str() {
            "init" => (&["--id", "--name", "--project-version"], &[]),
            "validate" => (&["--profile"], &[]),
            "build" => (&["--profile"], &["--dry-run", "--publish"]),
            "publish" => (&[], &["--dry-run"]),
            "migrate" => (
                &["--command", "--recheck", "--restore"],
                &[
                    "--apply",
                    "--dry-run",
                    "--allow-destructive",
                    "--authorize-restore",
                ],
            ),
            _ => (&[], &[]),
        };
        for name in parsed.values.keys() {
            if name != "--project" && !options.contains(&name.as_str()) {
                return Err(argument(format!(
                    "{} does not accept {name}",
                    parsed.command
                )));
            }
        }
        for name in &parsed.flags {
            if name != "--json" && !flags.contains(&name.as_str()) {
                return Err(argument(format!(
                    "{} does not accept {name}",
                    parsed.command
                )));
            }
        }
        if parsed.command == "build" && parsed.flag("--publish") && parsed.flag("--dry-run") {
            return Err(argument(
                "--dry-run cannot publish an uncommitted candidate",
            ));
        }
        Ok(parsed)
    }
    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|flag| flag == name)
    }
    fn path(&self, name: &str) -> Option<PathBuf> {
        self.values.get(name).map(PathBuf::from)
    }
    fn text(&self, name: &str) -> Result<Option<&str>> {
        self.values
            .get(name)
            .map(|value| {
                value
                    .to_str()
                    .ok_or_else(|| argument(format!("{name} must be UTF-8")))
            })
            .transpose()
    }
}
fn argument(message: impl Into<String>) -> Error {
    Error::new("E-CLI-ARGUMENT", message)
}
fn emit(value: &Value) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)
        .map_err(|e| Error::new("E-CLI-OUTPUT", e.to_string()))?;
    writeln!(stdout).map_err(project::io_error)
}
fn success(value: &Value) -> bool {
    matches!(
        value.get("outcome").and_then(Value::as_str),
        Some("Success" | "NotAttempted")
    )
}
fn plan_preview(plan: &migration::Plan) -> Value {
    json!({
        "command":plan.command, "destructive":plan.destructive,
        "affectedRecords":plan.affected_records, "postcondition":"confirmed",
        "sources":plan.candidates.iter().map(|(source, candidate)|json!({
            "source":source,
            "beforeIdentity":content_identity(candidate.before.bytes.as_bytes()),
            "afterIdentity":content_identity(candidate.after.bytes.as_bytes()),
            "before":candidate.before.bytes, "after":candidate.after.bytes
        })).collect::<Vec<_>>()
    })
}
fn migration_operation(root: &Path, args: &Arguments) -> Result<Value> {
    let modes = ["--command", "--recheck", "--restore"]
        .iter()
        .filter(|name| args.values.contains_key(**name))
        .count();
    if modes != 1 {
        return Err(argument(
            "migrate requires exactly one of --command, --recheck, --restore",
        ));
    }
    if let Some(id) = args.text("--recheck")? {
        if args.flags.iter().any(|flag| flag != "--json") {
            return Err(argument("recheck does not accept mutation flags"));
        }
        return Ok(json!({"outcome":"Success","recovery":native::recheck_recovery(root,id)?}));
    }
    if let Some(id) = args.text("--restore")? {
        if args
            .flags
            .iter()
            .any(|flag| !matches!(flag.as_str(), "--json" | "--authorize-restore"))
        {
            return Err(argument("restore accepts --authorize-restore only"));
        }
        return Ok(
            json!({"outcome":"Success","recovery":native::restore_recovery(root,id,args.flag("--authorize-restore"))?}),
        );
    }
    if args.flag("--authorize-restore") {
        return Err(argument("--authorize-restore requires --restore"));
    }
    let bytes = std::fs::read(args.path("--command").unwrap()).map_err(project::io_error)?;
    let command: migration::Command = serde_json::from_slice(&bytes)
        .map_err(|e| argument(format!("invalid Migration command: {e}")))?;
    let project = Project::open(root)?;
    let plan = migration::derive(&project, command)?;
    let preview = plan_preview(&plan);
    let prepared = native::SourceSetPlan::prepare(&project, plan)?;
    if !args.flag("--apply") || args.flag("--dry-run") {
        return Ok(json!({"outcome":"NotAttempted","plan":preview}));
    }
    // The operation remains source-set authorized and freshly checked in Rust.
    // This command never imports an old serialized Plan or skips derivation.
    Ok(
        match prepared.commit(args.flag("--allow-destructive"), native::SetFault::None) {
            Ok(committed) => json!({"outcome":committed.outcome,"plan":preview,"result":committed}),
            Err(e) => {
                json!({"outcome":"Failure","plan":preview,"diagnostic":{"code":e.code,"kind":"error","message":e.message}})
            }
        },
    )
}
fn run(args: &Arguments) -> Result<Value> {
    let cwd = std::env::current_dir().map_err(project::io_error)?;
    let explicit = args.path("--project");
    if args.command == "init" {
        let mut root = explicit.unwrap_or(cwd);
        if root
            .file_name()
            .is_some_and(|name| name == "masterdata.toml")
        {
            root = root.parent().unwrap_or(Path::new(".")).to_path_buf();
        }
        let fallback = root
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.trim().is_empty() && *s != ".")
            .unwrap_or("masterdata");
        return Ok(serde_json::to_value(native::initialize_project(
            &root,
            Metadata {
                id: args.text("--id")?.unwrap_or(fallback).into(),
                name: args.text("--name")?.unwrap_or(fallback).into(),
                version: args.text("--project-version")?.unwrap_or("0.1.0").into(),
            },
        )?)
        .unwrap());
    }
    let root = project::discover(explicit.as_deref(), &cwd)?;
    match args.command.as_str() {
        "validate" | "doctor" => {
            let project = Project::open(&root)?;
            let (mut diagnostics, rows) = project.validate(args.text("--profile")?)?;
            let environment = if args.command == "doctor" {
                if native::has_pending_recovery(&project.root)? {
                    diagnostics.push(project::Diagnostic::error(
                        ".masterdata",
                        &Error::new("E-RECOVERY-REQUIRED", "source-set recovery is pending"),
                        project.generation,
                    ));
                }
                match native::dotnet::environment() {
                    Ok(sdks) => Some(json!({"dotnetSdk":sdks})),
                    Err(e) => {
                        diagnostics.push(project::Diagnostic::error("", &e, project.generation));
                        Some(json!({"dotnetSdk":null}))
                    }
                }
            } else {
                None
            };
            Ok(json!({
                "outcome":if diagnostics.iter().any(|d|d.kind=="error") {"Failure"} else {"Success"},
                "projectId":project.config.project.id,
                "diagnostics":diagnostics, "environment":environment,
                "rowCounts":rows.iter().map(|(table,rows)|(table,rows.len())).collect::<BTreeMap<_,_>>()
            }))
        }
        "build" => {
            let built = delivery::build(
                &root,
                &BuildOptions {
                    profile: args.text("--profile")?.map(str::to_owned),
                    dry_run: args.flag("--dry-run"),
                    ..Default::default()
                },
            )?;
            if args.flag("--publish") && built.outcome == Outcome::Success {
                // Failed Publish cannot roll back this completed canonical Build.
                // A preflight error is also part of the combined result boundary.
                let published = publish::PublishPlan::prepare(&root)
                    .and_then(|plan| plan.execute(publish::PublishFault::None));
                return Ok(match published {
                    Ok(published) => {
                        json!({"outcome":published.outcome,"build":built,"publish":published})
                    }
                    Err(e) => {
                        json!({"outcome":"Failure","build":built,"publish":{"outcome":"NotAttempted","diagnostic":{"code":e.code,"kind":"error","message":e.message}}})
                    }
                });
            }
            Ok(serde_json::to_value(built).unwrap())
        }
        "publish" => {
            let plan = publish::PublishPlan::prepare(&root)?;
            if args.flag("--dry-run") {
                return Ok(json!({"outcome":"NotAttempted","preview":plan.preview()}));
            }
            Ok(serde_json::to_value(plan.execute(publish::PublishFault::None)?).unwrap())
        }
        "migrate" => migration_operation(&root, args),
        _ => unreachable!(),
    }
}
fn main() {
    let input = std::env::args_os().skip(1).collect::<Vec<_>>();
    let result = Arguments::parse(input).and_then(|args| {
        if args.flag("--version") {
            println!("MasterData {}", env!("CARGO_PKG_VERSION"));
            return Ok(json!({"outcome":"Success","printed":true}));
        }
        if args.flag("--help") || args.flag("-h") {
            println!("{HELP}");
            return Ok(json!({"outcome":"Success","printed":true}));
        }
        run(&args)
    });
    let result = result.unwrap_or_else(|e| {
        json!({
            "outcome":if e.code=="E-INIT-UNKNOWN" {"OutcomeUnknown"} else {"Failure"},
            "diagnostic":{"code":e.code,"kind":"error","message":e.message}
        })
    });
    let passed = success(&result);
    if result.get("printed").is_none()
        && let Err(e) = emit(&result)
    {
        eprintln!("{e}");
        std::process::exit(2);
    }
    if !passed {
        std::process::exit(1);
    }
}
