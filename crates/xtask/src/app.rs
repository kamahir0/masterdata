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
        Platform::Macos => vec![
            "run",
            "tauri",
            "--",
            "build",
            "--config",
            r#"{"bundle":{"active":true,"targets":["app"]}}"#,
        ],
        // The release executable is the portable local install source.
        Platform::Windows => vec![
            "run",
            "tauri",
            "--",
            "build",
            "--config",
            r#"{"bundle":{"active":true,"targets":["nsis"]}}"#,
        ],
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
    let destination = install_destination(target)?;
    replace_path(&source, &destination)?;
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
    let installed = install_destination(target)?;
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
    let executable = executable_path(target, &install_destination(target)?);
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
    let path = match target {
        Platform::Macos => platform_dist_root(target).join("masterdata.app"),
        Platform::Windows => platform_dist_root(target).join("masterdata-gui.exe"),
    };
    if path.is_file() || (target == Platform::Macos && path.is_dir()) {
        Ok(path)
    } else {
        Err(app_error(
            "E-XTASK-APP-ARTIFACT-MISSING",
            "expected installable local package is missing".into(),
            Some(path),
        ))
    }
}

fn discover_package(root: &Path, target: Platform) -> Result<PathBuf> {
    // These paths are derived from the GUI crate/binary name. Directory
    // enumeration is intentionally avoided so another executable cannot be
    // selected by accident.
    let path = match target {
        Platform::Macos => root.join("target/release/bundle/macos/masterdata.app"),
        Platform::Windows => root.join("target/release/masterdata-gui.exe"),
    };
    if path.is_file() || (target == Platform::Macos && path.is_dir()) {
        Ok(path)
    } else {
        Err(app_error(
            "E-XTASK-APP-ARTIFACT-DISCOVERY",
            "expected GUI package artifact is missing".into(),
            Some(path),
        ))
    }
}

fn install_destination(target: Platform) -> Result<PathBuf> {
    let variable = match target {
        Platform::Macos => "HOME",
        Platform::Windows => "LOCALAPPDATA",
    };
    install_destination_from_env(target, variable, std::env::var_os(variable).as_deref())
}

fn install_destination_from_env(
    target: Platform,
    variable: &str,
    base: Option<&std::ffi::OsStr>,
) -> Result<PathBuf> {
    let base = base.ok_or_else(|| {
        app_error(
            "E-XTASK-APP-INSTALL-ENV",
            format!("required environment variable `{variable}` is not set"),
            None,
        )
    })?;
    Ok(install_destination_from_base(target, Path::new(base)))
}

fn install_destination_from_base(target: Platform, base: &Path) -> PathBuf {
    match target {
        Platform::Macos => base.join("Applications/masterdata-local.app"),
        Platform::Windows => base.join("Programs/masterdata-local/masterdata-gui.exe"),
    }
}
fn executable_path(target: Platform, installed: &Path) -> PathBuf {
    match target {
        Platform::Macos => installed.join("Contents/MacOS/masterdata-gui"),
        Platform::Windows => installed.to_path_buf(),
    }
}

fn replace_path(source: &Path, destination: &Path) -> Result<()> {
    if source == destination {
        return Err(app_error(
            "E-XTASK-APP-INSTALL-SAME-PATH",
            "package source and install destination must differ".into(),
            Some(destination.to_path_buf()),
        ));
    }
    let parent = destination.parent().ok_or_else(|| {
        app_error(
            "E-XTASK-APP-INSTALL",
            "install destination has no parent directory".into(),
            Some(destination.to_path_buf()),
        )
    })?;
    fs::create_dir_all(parent).map_err(|e| {
        app_error(
            "E-XTASK-APP-INSTALL",
            format!("could not create install directory: {e}"),
            Some(parent.to_path_buf()),
        )
    })?;
    let staging = destination.with_extension("installing");
    let backup = destination.with_extension("previous");
    let _ = remove_path(&staging);
    copy_path(source, &staging).map_err(|e| {
        let _ = remove_path(&staging);
        app_error(
            "E-XTASK-APP-INSTALL-STAGE",
            format!("could not stage local app: {e}"),
            Some(staging.clone()),
        )
    })?;
    if destination.exists() {
        let _ = remove_path(&backup);
        if let Err(error) = fs::rename(destination, &backup) {
            let _ = remove_path(&staging);
            return Err(app_error(
                "E-XTASK-APP-INSTALL-RUNNING",
                format!(
                    "could not replace installed app; close the workflow-installed GUI and retry: {error}"
                ),
                Some(destination.to_path_buf()),
            ));
        }
    }
    if let Err(error) = fs::rename(&staging, destination) {
        let _ = remove_path(&staging);
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(app_error(
            "E-XTASK-APP-INSTALL-REPLACE",
            format!("could not activate staged app: {error}"),
            Some(destination.to_path_buf()),
        ));
    }
    let _ = remove_path(&backup);
    Ok(())
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else if path.exists() {
        fs::remove_file(path)
    } else {
        Ok(())
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
            install_destination_from_base(Platform::Macos, Path::new("/users/test"))
                .ends_with("Applications/masterdata-local.app")
        );
        assert!(
            install_destination_from_base(
                Platform::Windows,
                Path::new("C:/Users/test/AppData/Local")
            )
            .ends_with("Programs/masterdata-local/masterdata-gui.exe")
        );
    }

    #[test]
    fn missing_install_environment_is_structured() {
        let error =
            install_destination_from_env(Platform::Windows, "LOCALAPPDATA", None).unwrap_err();
        assert_eq!(error.diagnostic().code, "E-XTASK-APP-INSTALL-ENV");
    }

    #[test]
    fn replacement_uses_staging_and_leaves_destination_complete() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("masterdata-gui.exe");
        let destination = temp.path().join("install/masterdata-gui.exe");
        fs::write(&source, b"new").unwrap();
        replace_path(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert!(!destination.with_extension("installing").exists());
    }

    #[test]
    fn exact_gui_artifact_is_selected_when_other_executables_exist() {
        let temp = tempfile::tempdir().unwrap();
        let release = temp.path().join("target/release");
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join("masterdata-cli.exe"), b"cli").unwrap();
        fs::write(release.join("masterdata-gui.exe"), b"gui").unwrap();
        assert_eq!(
            discover_package(temp.path(), Platform::Windows).unwrap(),
            release.join("masterdata-gui.exe")
        );
    }

    #[test]
    fn missing_exact_artifact_has_discovery_code() {
        let temp = tempfile::tempdir().unwrap();
        let error = discover_package(temp.path(), Platform::Windows).unwrap_err();
        assert_eq!(error.diagnostic().code, "E-XTASK-APP-ARTIFACT-DISCOVERY");
    }
}
