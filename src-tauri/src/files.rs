use crate::config::{read_config, write_config, PROJECT_FILE_NAME};
use crate::model::{uid, LoadedProject, SaveRequest, SaveResult, SavedFile, Snapshot, Source};
use crate::yaml::parse_source;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub token: String,
    pub baseline: Vec<SavedFile>,
    pub config_text: String,
    pub folders: Vec<String>,
}

pub fn digest(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn safe_name(name: &str) -> bool {
    if name.is_empty()
        || Path::new(name).is_absolute()
        || name.starts_with('/')
        || name.starts_with('\\')
    {
        return false;
    }
    name.split('/').all(|p| {
        !p.is_empty()
            && p != "."
            && p != ".."
            && !p
                .chars()
                .any(|c| c == ':' || c == '\\' || (c as u32) <= 0x1f)
    })
}

pub fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !safe_name(relative) {
        return Err("Project外のパスは使用できません。".to_string());
    }
    let mut current = root.to_path_buf();
    for part in relative.split('/') {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err("シンボリックリンクは変更しません。".to_string());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(current)
}

pub fn optional_read(file: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(file) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn optional_read_bytes(file: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(file) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn load_workspace(input: &Path) -> Result<(Workspace, LoadedProject), String> {
    let open_inner = || -> Result<(PathBuf, String, crate::model::Config), String> {
        let is_valid_file = input
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == PROJECT_FILE_NAME)
            .unwrap_or(false)
            && fs::symlink_metadata(input)
                .map(|m| m.is_file() && !m.file_type().is_symlink())
                .unwrap_or(false);
        if !is_valid_file {
            return Err(format!("{PROJECT_FILE_NAME} ファイルを選択してください。"));
        }
        let parent = input
            .parent()
            .ok_or_else(|| format!("{PROJECT_FILE_NAME} ファイルを選択してください。"))?;
        let root = fs::canonicalize(parent).map_err(|e| e.to_string())?;
        let config_path = safe_path(&root, PROJECT_FILE_NAME)?;
        let config_text = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        let config = read_config(&config_text)?;
        Ok((root, config_text, config))
    };

    let (root, config_text, config) = open_inner()
        .map_err(|err| format!("プロジェクトを開けません: {}\n{}", input.display(), err))?;

    let mut sources: Vec<Source> = Vec::new();
    let mut saved: Vec<SavedFile> = Vec::new();
    let mut folders: Vec<String> = Vec::new();

    fn walk_sources(
        root: &Path,
        relative: &str,
        sources: &mut Vec<Source>,
        saved: &mut Vec<SavedFile>,
        folders: &mut Vec<String>,
    ) -> Result<(), String> {
        let dir = if relative.is_empty() {
            root.join("sources")
        } else {
            root.join("sources").join(relative)
        };
        let entries = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        for entry_res in entries {
            let entry = entry_res.map_err(|e| e.to_string())?;
            let ft = entry.file_type().map_err(|e| e.to_string())?;
            let entry_name = entry.file_name().to_string_lossy().to_string();
            if ft.is_symlink() || entry_name.starts_with('.') {
                continue;
            }
            let rel_name = if relative.is_empty() {
                entry_name.clone()
            } else {
                format!("{relative}/{entry_name}")
            };
            if ft.is_dir() {
                folders.push(rel_name.clone());
                walk_sources(root, &rel_name, sources, saved, folders)?;
            } else if ft.is_file() {
                let lower = entry_name.to_ascii_lowercase();
                if lower.ends_with(".yaml") || lower.ends_with(".yml") {
                    let text = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let source = parse_source(&rel_name, &text, None);
                    saved.push(SavedFile {
                        id: source.id.clone(),
                        path: rel_name,
                        text,
                    });
                    sources.push(source);
                }
            }
        }
        Ok(())
    }

    walk_sources(&root, "", &mut sources, &mut saved, &mut folders)?;
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    saved.sort_by(|a, b| a.path.cmp(&b.path));
    folders.sort();

    let token = uid();
    let root_str = root.to_string_lossy().to_string();
    let workspace = Workspace {
        root,
        token: token.clone(),
        baseline: saved.clone(),
        config_text: config_text.clone(),
        folders: folders.clone(),
    };
    let project = LoadedProject {
        root: root_str,
        token,
        snapshot: Snapshot {
            config,
            sources,
            folders,
        },
        saved,
        config_text,
    };
    Ok((workspace, project))
}

#[derive(Debug, Clone)]
struct FileChange {
    file: PathBuf,
    before: Option<String>,
    after: Option<String>,
}

pub fn save_workspace(
    workspace: &mut Workspace,
    request: &SaveRequest,
) -> Result<SaveResult, String> {
    for name in request
        .files
        .iter()
        .map(|f| &f.path)
        .chain(request.folders.iter())
    {
        if name.split('/').any(|part| part.starts_with('.')) {
            return Err("dotで始まる名前は読込対象外のため使用できません。".to_string());
        }
    }
    let mut desired: HashMap<String, &SavedFile> = HashMap::new();
    let mut ids: HashSet<&str> = HashSet::new();
    for f in &request.files {
        desired.insert(f.path.clone(), f);
        ids.insert(&f.id);
    }
    if desired.len() != request.files.len() || ids.len() != request.files.len() {
        return Err("保存先のパスが重複しています。".to_string());
    }

    let old: HashMap<String, &SavedFile> = workspace
        .baseline
        .iter()
        .map(|f| (f.path.clone(), f))
        .collect();

    let mut names: BTreeSet<String> = BTreeSet::new();
    for k in old.keys() {
        names.insert(k.clone());
    }
    for k in desired.keys() {
        names.insert(k.clone());
    }

    let mut changes: Vec<FileChange> = Vec::new();
    for name in names {
        let file = safe_path(&workspace.root, &format!("sources/{name}"))?;
        let before = old.get(&name).map(|f| f.text.clone());
        let after = desired.get(&name).map(|f| f.text.clone());
        if before == after {
            continue;
        }
        let current = optional_read(&file)?;
        if current != before {
            return Err(format!(
                "外部で変更されています: {name}。保存を中止しました。現在の入力は保持しています。"
            ));
        }
        changes.push(FileChange {
            file,
            before,
            after,
        });
    }

    let config_text = write_config(&request.config, Some(&workspace.config_text))?;
    let config_file = safe_path(&workspace.root, PROJECT_FILE_NAME)?;
    if config_text != workspace.config_text {
        if optional_read(&config_file)?.as_deref() != Some(workspace.config_text.as_str()) {
            return Err("プロジェクト設定が外部で変更されています。".to_string());
        }
        changes.push(FileChange {
            file: config_file,
            before: Some(workspace.config_text.clone()),
            after: Some(config_text.clone()),
        });
    }

    let mut directories = Vec::new();
    for f in &request.folders {
        directories.push(safe_path(&workspace.root, &format!("sources/{f}"))?);
    }

    let mut removed_folders = Vec::new();
    for name in &workspace.folders {
        let prefix = format!("{name}/");
        if request
            .folders
            .iter()
            .any(|p| p == name || p.starts_with(&prefix))
            || request.files.iter().any(|f| f.path.starts_with(&prefix))
        {
            continue;
        }
        removed_folders.push(safe_path(&workspace.root, &format!("sources/{name}"))?);
    }
    removed_folders.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    let removals: HashSet<_> = changes
        .iter()
        .filter(|c| c.after.is_none())
        .map(|c| c.file.clone())
        .chain(removed_folders.iter().cloned())
        .collect();
    // Refuse unknown children before changing any file. Nonrecursive deletion
    // below checks again, so a concurrent writer cannot turn this into data loss.
    for dir in &removed_folders {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if !removals.contains(&entry.path())
                || entry.file_type().map_err(|e| e.to_string())?.is_symlink()
            {
                return Err(format!(
                    "保存対象外の項目があるためfolderを削除できません: {}",
                    entry.path().display()
                ));
            }
        }
    }

    let transaction = safe_path(
        &workspace.root,
        &format!(".masterdata/transactions/{}", uid()),
    )?;
    fs::create_dir_all(&transaction).map_err(|e| e.to_string())?;

    let manifest_entries: Vec<serde_json::Value> = changes
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let rel = c
                .file
                .strip_prefix(&workspace.root)
                .unwrap_or(&c.file)
                .to_string_lossy()
                .to_string();
            json!({
                "path": rel,
                "before": c.before.as_ref().map(|_| format!("{i}.before")),
                "after": c.after.as_ref().map(|_| format!("{i}.after")),
            })
        })
        .collect();
    fs::write(
        transaction.join("manifest.json"),
        serde_json::to_string_pretty(&manifest_entries).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;

    for (i, change) in changes.iter().enumerate() {
        if let Some(before) = &change.before {
            fs::write(transaction.join(format!("{i}.before")), before)
                .map_err(|e| e.to_string())?;
        }
        if let Some(after) = &change.after {
            fs::write(transaction.join(format!("{i}.after")), after).map_err(|e| e.to_string())?;
        }
    }
    fs::write(
        transaction.join("removed-folders.json"),
        serde_json::to_vec(&removed_folders).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let mut applied: Vec<FileChange> = Vec::new();
    let mut removed = Vec::new();
    let apply_res = (|| -> Result<(), String> {
        for dir in &directories {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        for change in &changes {
            if optional_read(&change.file)? != change.before {
                return Err(format!(
                    "保存中に外部変更を検出しました: {}",
                    change.file.display()
                ));
            }
            if let Some(parent) = change.file.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            match &change.after {
                None => {
                    fs::remove_file(&change.file).map_err(|e| e.to_string())?;
                }
                Some(after_text) => {
                    let temp = PathBuf::from(format!("{}.{}.tmp", change.file.display(), uid()));
                    let mut f = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&temp)
                        .map_err(|e| e.to_string())?;
                    f.write_all(after_text.as_bytes())
                        .map_err(|e| e.to_string())?;
                    drop(f);
                    fs::rename(&temp, &change.file).map_err(|e| e.to_string())?;
                }
            }
            applied.push(change.clone());
        }
        for dir in &removed_folders {
            match fs::remove_dir(dir) {
                Ok(()) => removed.push(dir.clone()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("folderを削除できません: {}: {e}", dir.display())),
            }
        }
        Ok(())
    })();

    if let Err(err) = apply_res {
        let mut recovery_failed = false;
        for dir in removed.iter().rev() {
            if fs::create_dir_all(dir).is_err() {
                recovery_failed = true;
            }
        }
        for change in applied.into_iter().rev() {
            match optional_read(&change.file) {
                Ok(cur) if cur == change.after => {
                    let res = match &change.before {
                        None => fs::remove_file(&change.file),
                        Some(b) => fs::write(&change.file, b),
                    };
                    if res.is_err() {
                        recovery_failed = true;
                    }
                }
                _ => {
                    recovery_failed = true;
                }
            }
        }
        return Err(format!(
            "{err}{}",
            if recovery_failed {
                format!("\n復旧情報を保持しました: {}", transaction.display())
            } else {
                "\n今回の書込みは元に戻しました。".to_string()
            }
        ));
    }

    let _ = fs::write(transaction.join("complete"), "ok");
    workspace.baseline = request.files.clone();
    workspace.config_text = config_text.clone();
    workspace.folders = request.folders.clone();

    Ok(SaveResult {
        files: request.files.clone(),
        config_text,
    })
}

pub fn compute_workspace_digest(project: &LoadedProject) -> String {
    let mut sorted = project.saved.clone();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut parts = Vec::with_capacity(sorted.len() + 1);
    parts.push(format!("config:{}", digest(&project.config_text)));
    for f in sorted {
        parts.push(format!("source:{}:{}", f.path, digest(&f.text)));
    }
    digest(&parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn workspace() -> (Temp, Workspace, LoadedProject) {
        let root = std::env::temp_dir().join(format!("masterdata-save-{}", uid()));
        fs::create_dir_all(root.join("sources/types/child")).unwrap();
        fs::write(root.join(PROJECT_FILE_NAME), "name = \"Safety\"\n").unwrap();
        fs::write(
            root.join("sources/types/child/type.yaml"),
            "kind: type\nname: Stats\ncustom: {fields: []}\n",
        )
        .unwrap();
        let (workspace, project) = load_workspace(&root.join(PROJECT_FILE_NAME)).unwrap();
        (Temp(root), workspace, project)
    }
    #[test]
    fn every_path_segment_rejects_windows_prefixes_and_separators() {
        let (root, _, _) = workspace();
        for name in [
            "sources/C:/escape",
            "sources/C:escape",
            "sources/a/C:/escape",
            "sources/a\\..\\escape",
            "C:/escape",
            "//server/share",
            "sources/../escape",
        ] {
            assert!(!safe_name(name), "{name}");
            assert!(safe_path(&root.0, name).is_err(), "{name}");
        }
        assert_eq!(
            safe_path(&root.0, "sources/types/new.yaml").unwrap(),
            root.0.join("sources/types/new.yaml")
        );
    }
    #[test]
    fn hidden_source_destinations_fail_before_any_writes() {
        let (root, mut workspace, project) = workspace();
        for name in [".hidden.yaml", "types/.hidden/type.yaml"] {
            let mut files = project.saved.clone();
            files[0].path = name.to_string();
            let request = SaveRequest {
                token: project.token.clone(),
                files,
                folders: project.snapshot.folders.clone(),
                config: project.snapshot.config.clone(),
            };
            assert!(save_workspace(&mut workspace, &request)
                .unwrap_err()
                .contains("dot"));
            assert_eq!(workspace.baseline, project.saved);
            assert!(root.0.join("sources/types/child/type.yaml").exists());
        }
        let request = SaveRequest {
            token: project.token.clone(),
            files: project.saved.clone(),
            folders: vec![".hidden".to_string()],
            config: project.snapshot.config.clone(),
        };
        assert!(save_workspace(&mut workspace, &request).is_err());
        assert!(!root.0.join(".masterdata").exists());
    }
    #[test]
    fn folder_delete_preflights_unknown_children_then_can_retry() {
        let (root, mut workspace, project) = workspace();
        let original = project.saved.clone();
        let request = SaveRequest {
            token: project.token.clone(),
            files: vec![],
            folders: vec![],
            config: project.snapshot.config.clone(),
        };
        for unknown in [
            "types/notes.txt",
            "types/.hidden.yaml",
            "types/child/foreign.yaml",
        ] {
            let path = root.0.join("sources").join(unknown);
            fs::write(&path, "do not change").unwrap();
            assert!(save_workspace(&mut workspace, &request)
                .unwrap_err()
                .contains("保存対象外"));
            assert_eq!(workspace.baseline, original);
            assert_eq!(fs::read_to_string(&path).unwrap(), "do not change");
            assert_eq!(
                fs::read_to_string(root.0.join("sources/types/child/type.yaml")).unwrap(),
                original[0].text
            );
            fs::remove_file(path).unwrap();
        }
        save_workspace(&mut workspace, &request).unwrap();
        assert!(!root.0.join("sources/types").exists());
        assert!(workspace.baseline.is_empty());
        assert!(load_workspace(&root.0.join(PROJECT_FILE_NAME))
            .unwrap()
            .1
            .snapshot
            .folders
            .is_empty());
    }
}

#[cfg(all(test, unix))]
mod unix_save_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn removal_failure_restores_deleted_children_and_baseline() {
        let root = std::env::temp_dir().join(format!("masterdata-rollback-{}", uid()));
        fs::create_dir_all(root.join("sources/types/child")).unwrap();
        fs::write(root.join(PROJECT_FILE_NAME), "name = \"Rollback\"\n").unwrap();
        let file = root.join("sources/types/child/type.yaml");
        fs::write(&file, "kind: type\nname: Stats\ncustom: {fields: []}\n").unwrap();
        let (mut workspace, project) = load_workspace(&root.join(PROJECT_FILE_NAME)).unwrap();
        fs::set_permissions(root.join("sources"), fs::Permissions::from_mode(0o555)).unwrap();
        let request = SaveRequest {
            token: project.token.clone(),
            files: vec![],
            folders: vec![],
            config: project.snapshot.config.clone(),
        };
        let result = save_workspace(&mut workspace, &request);
        fs::set_permissions(root.join("sources"), fs::Permissions::from_mode(0o755)).unwrap();
        // A privileged test runner bypasses POSIX permissions.
        if result.is_err() {
            assert!(result.unwrap_err().contains("元に戻しました"));
            assert_eq!(fs::read_to_string(file).unwrap(), project.saved[0].text);
            assert_eq!(workspace.baseline, project.saved);
            assert_eq!(workspace.folders, project.snapshot.folders);
            save_workspace(&mut workspace, &request).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}
