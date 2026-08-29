use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: String,
    pub command: String,
    pub available: bool,
    pub version: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolchainReport {
    pub operating_system: String,
    pub architecture: String,
    pub tools: Vec<ToolStatus>,
    pub gui_dependencies: Vec<ToolStatus>,
}

pub fn collect_toolchain_report() -> ToolchainReport {
    let tools = [
        ("rustc", "rustc", &["--version"][..]),
        ("cargo", "cargo", &["--version"][..]),
        ("dotnet", "dotnet", &["--version"][..]),
        ("node", "node", &["--version"][..]),
        ("npm", npm_command(), &["--version"][..]),
    ]
    .into_iter()
    .map(|(name, command, args)| probe_tool(name, command, args))
    .collect();

    let gui_dependencies = if cfg!(target_os = "macos") {
        vec![probe_tool(
            "Xcode Command Line Tools",
            "xcode-select",
            &["--version"],
        )]
    } else if cfg!(target_os = "windows") {
        vec![probe_tool("Visual C++ compiler", "cl", &["/Bv"])]
    } else {
        vec![probe_tool("pkg-config", "pkg-config", &["--version"])]
    };

    ToolchainReport {
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        tools,
        gui_dependencies,
    }
}

fn probe_tool(name: &str, command: &str, args: &[&str]) -> ToolStatus {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            ToolStatus {
                name: name.to_owned(),
                command: command.to_owned(),
                available: true,
                version: (!version.is_empty()).then_some(version.clone()),
                detail: version,
            }
        }
        Ok(output) => ToolStatus {
            name: name.to_owned(),
            command: command.to_owned(),
            available: false,
            version: None,
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(error) => ToolStatus {
            name: name.to_owned(),
            command: command.to_owned(),
            available: false,
            version: None,
            detail: error.to_string(),
        },
    }
}

#[cfg(windows)]
fn npm_command() -> &'static str {
    "npm.cmd"
}

#[cfg(not(windows))]
fn npm_command() -> &'static str {
    "npm"
}
