use std::env::consts::OS;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use clap::{Args, Subcommand};
use masterdata_core::{ErrorKind, MasterdataError, Result};

use super::{cargo_command, gui, npm_command, repository_root, run_program};

#[derive(Debug, Args)]
pub struct AppArgs {
    #[command(subcommand)]
    command: AppCommand,
}

#[derive(Debug, Subcommand)]
enum AppCommand {
    Dev,
    Verify,
    Package,
    Install,
    Smoke,
    Reinstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Macos,
    Windows,
}

pub fn run(args: AppArgs) -> Result<()> {
    match args.command {
        AppCommand::Dev => gui(),
        AppCommand::Verify => verify(),
        AppCommand::Package => package(),
        AppCommand::Install => install(),
        AppCommand::Smoke => smoke(),
        AppCommand::Reinstall => reinstall(),
    }
}

fn platform() -> Result<Platform> {
    match OS {
        "macos" => Ok(Platform::Macos),
        "windows" => Ok(Platform::Windows),
        other => Err(app_error(
            "E-XTASK-APP-UNSUPPORTED-PLATFORM",
            format!("local GUI workflow does not support `{other}`"),
            None,
        )),
    }
}

fn local_dist_root() -> PathBuf {
    repository_root().join("target/local-dist")
}
fn platform_dist_root(platform: Platform) -> PathBuf {
    local_dist_root().join(match platform {
        Platform::Macos => "macos",
        Platform::Windows => "windows",
    })
}

fn verify() -> Result<()> {
    let root = repository_root();
    let gui_dir = root.join("apps/gui");
    phase("frontend lint", || {
        run_program(
            npm_command(),
            ["run", "lint"].map(OsString::from),
            &gui_dir,
            &[],
        )
    })?;
    phase("frontend test", || {
        run_program(
            npm_command(),
            ["run", "test"].map(OsString::from),
            &gui_dir,
            &[],
        )
    })?;
    phase("frontend production build", || {
        run_program(
            npm_command(),
            ["run", "build"].map(OsString::from),
            &gui_dir,
            &[],
        )
    })?;
    phase("GUI Rust tests", || {
        run_program(
            cargo_command(),
            ["test", "--package", "masterdata-gui"].map(OsString::from),
            &root,
            &[],
        )
    })?;
    phase("Tauri compile validation", || {
        run_program(
            cargo_command(),
            ["check", "--manifest-path", "apps/gui/src-tauri/Cargo.toml"].map(OsString::from),
            &root,
            &[],
        )
    })
}

fn package() -> Result<()> {
    let target = platform()?;
    let root = repository_root();
    let dist = platform_dist_root(target);
    if local_dist_root().exists() {
        fs::remove_dir_all(local_dist_root()).map_err(|e| {
            app_error(
                "E-XTASK-APP-PACKAGE-CLEAR",
                format!("could not clear local-dist: {e}"),
                Some(local_dist_root()),
            )
        })?;
    }
    fs::create_dir_all(&dist).map_err(|e| {
        app_error(
            "E-XTASK-APP-PACKAGE-CREATE",
            format!("could not create local-dist: {e}"),
            Some(dist.clone()),
        )
    })?;
    let bundle_args = match target {
        Platform::Macos => vec!["run", "tauri", "--", "build", "--bundles", "app"],
        // Keep the installer target available, but use the release executable
        // as the portable per-user install source below.
        Platform::Windows => vec!["run", "tauri", "--", "build", "--bundles", "nsis"],
    };
    phase("Tauri production package", || {
        run_program(
            npm_command(),
            bundle_args.into_iter().map(OsString::from),
            &root.join("apps/gui"),
            &[],
        )
    })?;
    let source = discover_package(&root, target)?;
    let destination = dist.join(source.file_name().expect("artifact has a name"));
    copy_path(&source, &destination).map_err(|e| {
        app_error(
            "E-XTASK-APP-ARTIFACT-COPY",
            format!("could not collect artifact: {e}"),
            Some(destination.clone()),
        )
    })?;
    println!("local package: {}", destination.display());
    Ok(())
}

fn install() -> Result<()> {
    let target = platform()?;
    let source = installed_source(target)?;
    let destination = install_destination(target);
    if target == Platform::Macos && destination.exists() {
        fs::remove_dir_all(&destination).map_err(|e| {
            app_error(
                "E-XTASK-APP-INSTALL",
                format!("could not replace installed app: {e}"),
                Some(destination.clone()),
            )
        })?;
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            app_error(
                "E-XTASK-APP-INSTALL",
                format!("could not create install directory: {e}"),
                Some(parent.to_path_buf()),
            )
        })?;
    }
    copy_path(&source, &destination).map_err(|e| {
        app_error(
            "E-XTASK-APP-INSTALL",
            format!("could not install local app: {e}"),
            Some(destination.clone()),
        )
    })?;
    println!(
        "installed build: {}\ninstall location: {}\nexecutable: {}",
        source.display(),
        destination.display(),
        executable_path(target, &destination).display()
    );
    Ok(())
}

fn smoke() -> Result<()> {
    let target = platform()?;
    let source = installed_source(target)?;
    let installed = install_destination(target);
    if !source.exists() {
        return Err(app_error(
            "E-XTASK-APP-ARTIFACT-MISSING",
            "installed source is missing".into(),
            Some(source),
        ));
    }
    let executable = executable_path(target, &installed);
    if !executable.is_file() {
        return Err(app_error(
            "E-XTASK-APP-EXECUTABLE-MISSING",
            "installed executable is missing".into(),
            Some(executable),
        ));
    }
    let mut child = Command::new(&executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            app_error(
                "E-XTASK-APP-LAUNCH",
                format!("could not launch installed app: {e}"),
                Some(executable.clone()),
            )
        })?;
    thread::sleep(Duration::from_secs(2));
    if let Some(status) = child.try_wait().map_err(|e| {
        app_error(
            "E-XTASK-APP-SMOKE",
            format!("could not inspect app process: {e}"),
            None,
        )
    })? {
        return Err(app_error(
            "E-XTASK-APP-SMOKE",
            format!("installed app exited during smoke test: {status}"),
            Some(executable),
        ));
    }
    let _ = child.kill();
    println!("smoke passed: production app remained alive for 2 seconds");
    Ok(())
}

fn reinstall() -> Result<()> {
    for (number, (name, action)) in [
        ("verify", verify as fn() -> Result<()>),
        ("package", package),
        ("install", install),
        ("smoke", smoke),
    ]
    .into_iter()
    .enumerate()
    {
        println!("[{}/5] {name}", number + 1);
        action().map_err(|e| {
            app_error(
                "E-XTASK-APP-PHASE",
                format!("phase `{name}` failed: {}", e.diagnostic()),
                None,
            )
        })?;
    }
    println!("[5/5] launch");
    let target = platform()?;
    let executable = executable_path(target, &install_destination(target));
    Command::new(&executable).spawn().map_err(|e| {
        app_error(
            "E-XTASK-APP-LAUNCH",
            format!("could not launch installed app: {e}"),
            Some(executable.clone()),
        )
    })?;
    println!(
        "launched: {}\n✓ local production build verified\n✓ package created\n✓ installed\n✓ smoke passed\n✓ launched",
        executable.display()
    );
    Ok(())
}

fn installed_source(target: Platform) -> Result<PathBuf> {
    let dir = platform_dist_root(target);
    let entries = fs::read_dir(&dir).map_err(|e| {
        app_error(
            "E-XTASK-APP-ARTIFACT-MISSING",
            format!("local package directory unavailable: {e}"),
            Some(dir.clone()),
        )
    })?;
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|path| match target {
            Platform::Macos => path.extension().is_some_and(|x| x == "app"),
            Platform::Windows => path.extension().is_some_and(|x| x == "exe"),
        })
        .ok_or_else(|| {
            app_error(
                "E-XTASK-APP-ARTIFACT-MISSING",
                "no installable local package found".into(),
                Some(dir),
            )
        })
}

fn discover_package(root: &Path, target: Platform) -> Result<PathBuf> {
    // Tauri is a workspace member, so Cargo places release output in the
    // repository target directory rather than beside the Tauri manifest.
    let bundle = root.join("target/release/bundle");
    let dirs = match target {
        Platform::Macos => vec![bundle.join("macos")],
        Platform::Windows => vec![
            root.join("target/release"),
            bundle.join("nsis"),
            bundle.join("msi"),
        ],
    };
    for directory in dirs {
        if let Ok(entries) = fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                let match_kind = match target {
                    Platform::Macos => path.extension().is_some_and(|x| x == "app"),
                    Platform::Windows => {
                        path.extension().is_some_and(|x| x == "exe") && path.is_file()
                    }
                };
                if match_kind {
                    return Ok(path);
                }
            }
        }
    }
    Err(app_error(
        "E-XTASK-APP-ARTIFACT-DISCOVERY",
        "Tauri produced no supported local package artifact".into(),
        Some(bundle),
    ))
}

fn install_destination(target: Platform) -> PathBuf {
    match target {
        Platform::Macos => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Applications/masterdata-local.app"),
        Platform::Windows => std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Programs/masterdata-local/masterdata-gui.exe"),
    }
}
fn executable_path(target: Platform, installed: &Path) -> PathBuf {
    match target {
        Platform::Macos => installed.join("Contents/MacOS/masterdata-gui"),
        Platform::Windows => installed.to_path_buf(),
    }
}
fn copy_path(source: &Path, destination: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path(&entry.path(), &destination.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(source, destination).map(|_| ())
    }
}
fn phase(name: &str, action: impl FnOnce() -> Result<()>) -> Result<()> {
    action().map_err(|e| {
        app_error(
            "E-XTASK-APP-PHASE",
            format!("phase `{name}` failed: {}", e.diagnostic()),
            None,
        )
    })
}
fn app_error(code: &str, message: String, source: Option<PathBuf>) -> MasterdataError {
    let error = MasterdataError::new(code, ErrorKind::ExternalTool, message);
    source.map_or(error.clone(), |path| error.with_source(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_dist_is_below_target() {
        assert!(local_dist_root().ends_with(Path::new("target/local-dist")));
    }
    #[test]
    fn platform_dist_names_are_stable() {
        assert!(
            platform_dist_root(Platform::Macos).ends_with(Path::new("target/local-dist/macos"))
        );
        assert!(
            platform_dist_root(Platform::Windows).ends_with(Path::new("target/local-dist/windows"))
        );
    }
    #[test]
    fn install_destinations_are_per_user() {
        assert!(
            install_destination(Platform::Macos).ends_with("Applications/masterdata-local.app")
        );
        assert!(
            install_destination(Platform::Windows)
                .ends_with("Programs/masterdata-local/masterdata-gui.exe")
        );
    }
}
