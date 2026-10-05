use crate::actor::{Intent, Session, UiError};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

struct Host {
    session: Session,
    clipboard: crate::clipboard::Clipboard,
    exit_allowed: Arc<AtomicBool>,
    initial_project: Option<String>,
    evidence_output: Option<PathBuf>,
    evidence_kind: String,
    preferences_path: PathBuf,
    preferences: Arc<Mutex<crate::preferences::Preferences>>,
}
#[tauri::command]
async fn application_preferences(
    state: tauri::State<'_, Host>,
    theme: Option<crate::preferences::Theme>,
) -> Result<crate::preferences::Preferences, String> {
    let path = state.preferences_path.clone();
    let preferences = state.preferences.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut current = preferences.lock().map_err(|e| e.to_string())?;
        if let Some(theme) = theme {
            let mut next = current.clone();
            next.theme = theme;
            crate::preferences::write(&path, &next)?;
            *current = next;
        }
        Ok(current.clone())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn clipboard_text(
    state: tauri::State<'_, Host>,
    text: Option<String>,
) -> Result<String, String> {
    state.clipboard.text(text).await
}
#[tauri::command]
async fn workspace(
    intent: Intent,
    epoch: u64,
    state: tauri::State<'_, Host>,
) -> Result<tauri::ipc::Response, UiError> {
    state
        .session
        .request_at(intent, epoch)
        .await
        .map(tauri::ipc::Response::new)
}
#[tauri::command]
async fn pick_project(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path.map(|p| p.to_string()));
    });
    rx.await.map_err(|e| e.to_string())
}
#[tauri::command]
fn boot(state: tauri::State<'_, Host>) -> Value {
    json!({"platform":std::env::consts::OS,"initialProject":state.initial_project,"preferences":*state.preferences.lock().unwrap(),"evidence":cfg!(feature="desktop-evidence")&&state.evidence_output.is_some(),"evidenceKind":state.evidence_kind})
}
#[tauri::command]
fn finish_exit(
    app: tauri::AppHandle,
    state: tauri::State<'_, Host>,
    discard: bool,
) -> Result<(), String> {
    if state.session.protected.load(Ordering::Acquire) && !discard {
        return Err("unsaved state remains".into());
    }
    state.exit_allowed.store(true, Ordering::Release);
    app.exit(0);
    Ok(())
}
#[tauri::command]
fn evidence_write(
    app: tauri::AppHandle,
    state: tauri::State<'_, Host>,
    mut report: Value,
) -> Result<(), String> {
    #[cfg(feature = "desktop-evidence")]
    {
        if let Some(window) = app.get_webview_window("main") {
            report["nativeWindow"] = json!({"visible": window.is_visible().ok(),
                "minimized": window.is_minimized().ok(), "focused": window.is_focused().ok()});
        }
        let path = state
            .evidence_output
            .as_ref()
            .ok_or("evidence output not configured")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "desktop-evidence"))]
    {
        let _ = (app, state, &mut report);
        Err("evidence adapter is disabled in production".into())
    }
}
pub fn run() {
    let args = std::env::args().collect::<Vec<_>>();
    let argument = |name: &str| {
        args.iter()
            .position(|s| s == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let initial = argument("--project");
    let output = if cfg!(feature = "desktop-evidence") {
        argument("--evidence-output").map(PathBuf::from)
    } else {
        None
    };
    let evidence_kind = match (output.is_some(), argument("--evidence-kind").as_deref()) {
        (true, Some("authoring")) => "authoring",
        (true, Some("external")) => "external",
        (true, Some("creation")) => "creation",
        (true, Some("path")) => "path",
        (true, Some("migration")) => "migration",
        _ => "navigation",
    }
    .to_string();
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let preferences_path = app.path().app_config_dir()?.join("preferences.json");
            let preferences = Arc::new(Mutex::new(crate::preferences::read(&preferences_path)));
            let handle = app.handle().clone();
            let session = Session::new(Arc::new(move |status| {
                let _ = handle.emit("workspace-status", status);
            }));
            app.manage(Host {
                session,
                clipboard: crate::clipboard::Clipboard::new(),
                exit_allowed: Arc::new(AtomicBool::new(false)),
                initial_project: initial,
                evidence_output: output,
                evidence_kind,
                preferences_path,
                preferences,
            });
            app.set_menu(tauri::menu::Menu::default(app.handle())?)?;
            #[cfg(target_os = "macos")]
            crate::macos_exit::install(
                app.handle().clone(),
                app.state::<Host>().exit_allowed.clone(),
            )?;
            #[cfg(feature = "desktop-evidence")]
            if let Some(window) = app.get_webview_window("main") {
                window.set_focus()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            workspace,
            clipboard_text,
            pick_project,
            boot,
            application_preferences,
            finish_exit,
            evidence_write
        ])
        .build(tauri::generate_context!())
        .expect("MasterData Desktop host");
    application.run(|app, event| match event {
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            let state = app.state::<Host>();
            if !state.exit_allowed.load(Ordering::Acquire) {
                api.prevent_close();
                let _ = app.emit("exit-requested", "close");
            }
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            let state = app.state::<Host>();
            if !state.exit_allowed.load(Ordering::Acquire) {
                api.prevent_exit();
                let _ = app.emit("exit-requested", "quit");
            }
        }
        tauri::RunEvent::Exit => app.state::<Host>().session.stop(),
        _ => {}
    });
}
