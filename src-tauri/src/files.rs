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
    if name.is_empty() || Path::new(name).is_absolute() || name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    name.split(['/', '\\']).all(|p| {
        !p.is_empty()
            && p != "."
            && p != ".."
            && !p.chars().any(|c| (c as u32) <= 0x1f)
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

    let (root, config_text, config) = open_inner().map_err(|err| {
        format!(
            "プロジェクトを開けません: {}\n{}",
            input.display(),
            err
        )
    })?;

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
            fs::write(transaction.join(format!("{i}.after")), after)
                .map_err(|e| e.to_string())?;
        }
    }

    let mut applied: Vec<FileChange> = Vec::new();
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
                    let temp = PathBuf::from(format!(
                        "{}.{}.tmp",
                        change.file.display(),
                        uid()
                    ));
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
        Ok(())
    })();

    if let Err(err) = apply_res {
        let mut recovery_failed = false;
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

    let mut old_folders = workspace.folders.clone();
    old_folders.sort_by(|a, b| b.len().cmp(&a.len()));
    for name in old_folders {
        if !request.folders.contains(&name) {
            if let Ok(dir) = safe_path(&workspace.root, &format!("sources/{name}")) {
                let _ = fs::remove_dir(dir);
            }
        }
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
