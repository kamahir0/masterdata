use masterdata_lib::cli::{run_cli, run_stdio_host, CliOptions};
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--host-stdio") {
        run_stdio_host();
        return;
    }
    let result = run_cli(
        &args,
        CliOptions {
            cwd: None,
            stdin: None,
        },
    );
    if !result.stdout.is_empty() {
        let _ = io::stdout().write_all(result.stdout.as_bytes());
    }
    if !result.stderr.is_empty() {
        let _ = io::stderr().write_all(result.stderr.as_bytes());
    }
    std::process::exit(result.exit_code);
}
