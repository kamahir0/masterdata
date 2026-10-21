use crate::build::{build_workspace, publish_build};
use crate::config::{ write_config, PROJECT_FILE_NAME};
use crate::files::{load_workspace, save_workspace, Workspace};
use crate::model::{default_config, uid, BuildResult, Data};
use crate::wire::{
    build_result_to_data, decode_data, encode_data, loaded_project_to_data,
    publish_config_from_data, save_request_from_data, save_result_to_data, snapshot_from_data,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

struct ServiceState {
    workspace: Option<Workspace>,
    build: Option<BuildResult>,
}

struct FlagGuard<'a>(&'a AtomicBool);
impl<'a> Drop for FlagGuard<'a> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub struct ProjectService {
    state: Mutex<ServiceState>,
    writing: AtomicBool,
    building: AtomicBool,
}

impl Default for ProjectService {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectService {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ServiceState {
                workspace: None,
                build: None,
            }),
            writing: AtomicBool::new(false),
            building: AtomicBool::new(false),
        }
    }

    fn idle(&self) -> Result<(), String> {
        if self.writing.load(Ordering::SeqCst) || self.building.load(Ordering::SeqCst) {
            return Err("保存・生成処理が完了するまでお待ちください。".to_string());
        }
        Ok(())
    }

    fn current_workspace(&self, token: &str) -> Result<Workspace, String> {
        let guard = self.state.lock().unwrap();
        let Some(ws) = &guard.workspace else {
            return Err("Projectが切り替わっています。".to_string());
        };
        if ws.token != token {
            return Err("Projectが切り替わっています。".to_string());
        }
        Ok(ws.clone())
    }

    fn open_path(&self, root: &Path) -> Result<Data, String> {
        self.idle()?;
        let (workspace, project) = load_workspace(root)?;
        let mut guard = self.state.lock().unwrap();
        guard.workspace = Some(workspace);
        guard.build = None;
        Ok(loaded_project_to_data(&project))
    }

    pub fn request_wire(&self, method: &str, payload: &str) -> Result<String, String> {
        let decoded = decode_data(payload)?;
        let args = decoded.as_array().cloned().unwrap_or_default();
        let response = self.request(method, &args)?;
        Ok(encode_data(&response))
    }

    pub fn request(&self, method: &str, args: &[Data]) -> Result<Data, String> {
        match method {
            "initial" => {
                if let Some(path_str) = args.first().and_then(|v| v.as_str()) {
                    if !path_str.is_empty() {
                        return self.open_path(Path::new(path_str));
                    }
                }
                Ok(Data::Null)
            }
            "open" => {
                let path_str = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "プロジェクトパスが指定されていません。".to_string())?;
                self.open_path(Path::new(path_str))
            }
            "sample" => {
                self.idle()?;
                let source = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "サンプル元パスが不正です。".to_string())?;
                let user_data = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "保存先パスが不正です。".to_string())?;
                let short_id = &uid()[..8];
                let destination = PathBuf::from(user_data)
                    .join("workspaces")
                    .join(format!("Adventure-{short_id}"));
                copy_dir_all(Path::new(source), &destination)?;
                self.open_path(&destination.join(PROJECT_FILE_NAME))
            }
            "create" => {
                self.idle()?;
                let name = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let parent = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "保存先フォルダが指定されていません。".to_string())?;
                let folder_name = name.trim();
                if folder_name.is_empty()
                    || folder_name == "."
                    || folder_name == ".."
                    || folder_name.chars().any(|c| {
                        matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
                            || (c as u32) <= 0x1f
                    })
                {
                    return Err(
                        "プロジェクト名にはフォルダ名に使える文字を入力してください。".to_string(),
                    );
                }
                let destination = PathBuf::from(parent).join(folder_name);
                fs::create_dir(&destination).map_err(|e| e.to_string())?;
                fs::create_dir(destination.join("sources")).map_err(|e| e.to_string())?;
                let cfg_text = write_config(&default_config(folder_name), None)?;
                let toml_path = destination.join(PROJECT_FILE_NAME);
                fs::write(&toml_path, cfg_text).map_err(|e| e.to_string())?;
                self.open_path(&toml_path)
            }
            "save" => {
                if self.writing.swap(true, Ordering::SeqCst) {
                    return Err("保存処理中です。".to_string());
                }
                let _guard = FlagGuard(&self.writing);
                let req_data = args
                    .first()
                    .ok_or_else(|| "保存リクエストが不正です。".to_string())?;
                let req = save_request_from_data(req_data)?;
                let mut ws = self.current_workspace(&req.token)?;
                let result = save_workspace(&mut ws, &req)?;
                let mut state = self.state.lock().unwrap();
                if state.workspace.as_ref().map(|w| &w.token) == Some(&ws.token) {
                    state.workspace = Some(ws);
                }
                Ok(save_result_to_data(&result))
            }
            "build" => {
                if self.building.swap(true, Ordering::SeqCst) {
                    return Err("Build処理中です。".to_string());
                }
                let _guard = FlagGuard(&self.building);
                let token = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let snap_data = args
                    .get(1)
                    .ok_or_else(|| "Snapshotが指定されていません。".to_string())?;
                let snapshot = snapshot_from_data(snap_data)?;
                let profile = args.get(2).and_then(|v| v.as_str()).unwrap_or("");
                let revision = args
                    .get(3)
                    .and_then(|v| v.as_i128())
                    .unwrap_or(0) as i64;
                let owner = self.current_workspace(token)?;
                let result = build_workspace(&owner, &snapshot, profile, revision)?;
                let mut state = self.state.lock().unwrap();
                if state.workspace.as_ref().map(|w| &w.token) == Some(&owner.token) {
                    state.build = Some(result.clone());
                }
                Ok(build_result_to_data(&result))
            }
            "publish" => {
                let token = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let targets = args
                    .get(1)
                    .map(publish_config_from_data)
                    .unwrap_or_else(|| crate::model::PublishConfig {
                        csharp: String::new(),
                        binary: String::new(),
                    });
                let ws = self.current_workspace(token)?;
                let build = {
                    let state = self.state.lock().unwrap();
                    state
                        .build
                        .clone()
                        .ok_or_else(|| "先にBuildを実行してください。".to_string())?
                };
                self.idle()?;
                if self.writing.swap(true, Ordering::SeqCst) {
                    return Err("保存処理中です。".to_string());
                }
                let _guard = FlagGuard(&self.writing);
                let message = publish_build(&build, &ws.root, &targets)?;
                Ok(Data::Str(message))
            }
            "reveal" => {
                let token = args.first().and_then(|v| v.as_str()).unwrap_or("");
                let want_build = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
                let ws = self.current_workspace(token)?;
                let state = self.state.lock().unwrap();
                if want_build {
                    if let Some(build) = &state.build {
                        return Ok(Data::Str(build.directory.clone()));
                    }
                }
                Ok(Data::Str(ws.root.to_string_lossy().to_string()))
            }
            _ => Err("許可されていない操作です。".to_string()),
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
