use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{ErrorKind, MasterdataError, Result};
use serde_json::Value;

const PACKAGE_RELATIVE_ROOT: &str = "unity/Packages/com.kamahir0.masterdata";

pub fn check_repository(root: &Path) -> Result<()> {
    let package_root = root.join(PACKAGE_RELATIVE_ROOT);
    let package_json = package_root.join("package.json");
    let runtime_asmdef = package_root.join("Runtime/MasterData.Unity.Runtime.asmdef");
    let editor_asmdef = package_root.join("Editor/MasterData.Unity.Editor.asmdef");
    let runtime_source = package_root.join("Runtime/MasterDataRuntime.cs");
    let editor_source = package_root.join("Editor/MasterDataUnityDeliveryStatus.cs");
    let editor_window = package_root.join("Editor/MasterDataUnityDeliveryStatusWindow.cs");

    for path in [
        &package_json,
        &runtime_asmdef,
        &editor_asmdef,
        &runtime_source,
        &editor_source,
        &editor_window,
    ] {
        require_file(path)?;
    }

    let package = read_json(&package_json)?;
    require_string(&package, "name", "com.kamahir0.masterdata", &package_json)?;
    require_string(&package, "version", "0.1.0", &package_json)?;
    let dependencies = package
        .get("dependencies")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(&package_json, "package dependencies are missing"))?;
    if dependencies.get("com.unity.modules.unitywebrequest")
        != Some(&Value::String("1.0.0".to_owned()))
    {
        return Err(invalid(
            &package_json,
            "UnityWebRequest must use the built-in 1.0.0 dependency",
        ));
    }

    let runtime = read_json(&runtime_asmdef)?;
    require_string(
        &runtime,
        "name",
        "MasterData.Unity.Runtime",
        &runtime_asmdef,
    )?;
    require_empty_array(&runtime, "includePlatforms", &runtime_asmdef)?;
    let editor = read_json(&editor_asmdef)?;
    require_string(&editor, "name", "MasterData.Unity.Editor", &editor_asmdef)?;
    require_array_contains(&editor, "includePlatforms", "Editor", &editor_asmdef)?;
    require_array_contains(
        &editor,
        "references",
        "MasterData.Unity.Runtime",
        &editor_asmdef,
    )?;

    let runtime_text = read_text(&runtime_source)?;
    if runtime_text.contains("UnityEditor")
        || runtime_text.contains("AssetDatabase")
        || runtime_text.contains("CompilationPipeline")
    {
        return Err(invalid(
            &runtime_source,
            "runtime source must not reference UnityEditor APIs",
        ));
    }
    for required in [
        "LoadDatabase",
        "LoadStreamingAssetsAsync",
        "NormalizeRelativePath",
        "MASTERDATA-UNITY-RUNTIME-",
    ] {
        if !runtime_text.contains(required) {
            return Err(invalid(
                &runtime_source,
                format!("runtime source is missing `{required}`"),
            ));
        }
    }

    let editor_text = format!(
        "{}\n{}",
        read_text(&editor_source)?,
        read_text(&editor_window)?
    );
    for required in [
        "AssetDatabase",
        "CompilationPipeline",
        "MASTERDATA-UNITY-",
        "OpenFolderPanel",
        "OpenFilePanel",
    ] {
        if !editor_text.contains(required) {
            return Err(invalid(
                &editor_source,
                format!("editor source is missing `{required}`"),
            ));
        }
    }
    for forbidden in [
        "AssetDatabase.Refresh",
        "AssetDatabase.DeleteAsset",
        "GUID.Generate",
        "new MemoryDatabase",
    ] {
        if runtime_text.contains(forbidden) || editor_text.contains(forbidden) {
            return Err(invalid(
                &package_root,
                format!("Unity package must not contain `{forbidden}`"),
            ));
        }
    }

    println!(
        "Unity package checks passed: {}",
        package_root
            .strip_prefix(root)
            .unwrap_or(&package_root)
            .display()
    );
    Ok(())
}

fn require_file(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(invalid(path, "required Unity package file is missing"))
    }
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| {
        MasterdataError::new(
            "E-XTASK-UNITY-PACKAGE-READ",
            ErrorKind::Io,
            format!("could not read Unity package file: {error}"),
        )
        .with_source(path.to_path_buf())
    })
}

fn read_json(path: &Path) -> Result<Value> {
    let text = read_text(path)?;
    serde_json::from_str(&text).map_err(|error| invalid(path, format!("invalid JSON: {error}")))
}

fn require_string(path_value: &Value, key: &str, expected: &str, path: &Path) -> Result<()> {
    if path_value.get(key).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(invalid(path, format!("`{key}` must be `{expected}`")))
    }
}

fn require_empty_array(value: &Value, key: &str, path: &Path) -> Result<()> {
    if value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        Ok(())
    } else {
        Err(invalid(path, format!("`{key}` must be an empty array")))
    }
}

fn require_array_contains(value: &Value, key: &str, expected: &str, path: &Path) -> Result<()> {
    if value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
    {
        Ok(())
    } else {
        Err(invalid(path, format!("`{key}` must contain `{expected}`")))
    }
}

fn invalid(path: &Path, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new(
        "E-XTASK-UNITY-PACKAGE-INVALID",
        ErrorKind::Validation,
        message,
    )
    .with_source(PathBuf::from(path))
}
