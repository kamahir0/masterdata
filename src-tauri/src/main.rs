#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod bridge;
use bridge::Bridge;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
struct Host {
    bridge: Arc<Bridge>,
    dirty: AtomicBool,
    busy: AtomicUsize,
    closing: AtomicBool,
    project: Option<String>,
    sample: PathBuf,
    data: PathBuf,
    smoke: bool,
}
fn app_navigation(url: &tauri::Url) -> bool {
    matches!(
        (url.scheme(), url.host_str(), url.port()),
        ("tauri", Some("localhost"), None) | ("http" | "https", Some("tauri.localhost"), None)
    ) || (cfg!(debug_assertions)
        && url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port() == Some(5173))
}

fn wire(args: Value) -> String {
    json!([args, []]).to_string()
}
fn emit(window: &tauri::WebviewWindow, command: &str) {
    let _ = window.emit("command", command);
}
fn trusted(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        Err("許可されていない呼出しです。".into())
    } else {
        Ok(())
    }
}
#[tauri::command]
fn set_dirty(
    window: tauri::WebviewWindow,
    host: tauri::State<Host>,
    value: bool,
) -> Result<(), String> {
    trusted(&window)?;
    host.dirty.store(value, Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    {
        let native = window.clone();
        let _ = window.run_on_main_thread(move || {
            if let Ok(pointer) = native.ns_window() {
                unsafe {
                    let window = &*pointer.cast::<objc2_app_kit::NSWindow>();
                    window.setDocumentEdited(value);
                    // AppKit resets traffic lights on setDocumentEdited. Reapply the
                    // public NSView geometry, matching the Tauri inset after dirty updates.
                    use objc2_app_kit::NSWindowButton;
                    if let (Some(close), Some(minimize), Some(zoom)) = (
                        window.standardWindowButton(NSWindowButton::CloseButton),
                        window.standardWindowButton(NSWindowButton::MiniaturizeButton),
                        window.standardWindowButton(NSWindowButton::ZoomButton),
                    ) {
                        let spacing = minimize.frame().origin.x - close.frame().origin.x;
                        if let Some(container) = close.superview().and_then(|view| view.superview())
                        {
                            let mut frame = container.frame();
                            frame.size.height = close.frame().size.height + 12.0;
                            frame.origin.y = window.frame().size.height - frame.size.height;
                            container.setFrame(frame);
                        }
                        for (index, button) in [close, minimize, zoom].into_iter().enumerate() {
                            let mut origin = button.frame().origin;
                            origin.x = 18.0 + index as f64 * spacing;
                            button.setFrameOrigin(origin);
                        }
                    }
                }
            }
        });
    }

    Ok(())
}
#[tauri::command]
fn close_project(window: tauri::WebviewWindow, host: tauri::State<Host>) -> Result<(), String> {
    trusted(&window)?;
    if host.busy.load(Ordering::SeqCst) != 0 {
        return Err("保存・生成処理が完了するまでお待ちください。".into());
    }
    host.closing.store(true, Ordering::SeqCst);
    window.close().map_err(|e| e.to_string())
}
#[tauri::command]
async fn project_request(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    host: tauri::State<'_, Host>,
    method: String,
    payload: String,
) -> Result<String, String> {
    trusted(&window)?;
    // Native-only path selection. Never forward arbitrary method names/open paths from the renderer.
    let parsed: Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
    let args = &parsed[0];
    let selected = match method.as_str() {
        "initial" => wire(json!([host.project])),
        "open" | "create" => {
            if host.busy.load(Ordering::SeqCst) != 0 {
                return Err("保存・生成処理が完了するまでお待ちください。".into());
            }
            let title = if method == "create" {
                "プロジェクトを作成する保存先フォルダ"
            } else {
                "MasterDataプロジェクトを開く"
            };
            let dialog = app.dialog().file().set_title(title).set_parent(&window);
            let folder =
                tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_folder())
                    .await
                    .map_err(|e| e.to_string())?;
            let Some(folder) = folder else {
                return Ok(wire(Value::Null));
            };
            let path = folder.into_path().map_err(|e| e.to_string())?;
            if method == "create" {
                wire(json!([args[0], path]))
            } else {
                wire(json!([path]))
            }
        }
        "sample" => wire(json!([host.sample, host.data])),
        "save" | "build" | "publish" | "reveal" => payload,
        _ => return Err("許可されていない操作です。".into()),
    };
    let mutation = matches!(
        method.as_str(),
        "save" | "build" | "publish" | "open" | "create" | "sample"
    );
    if mutation {
        host.busy.fetch_add(1, Ordering::SeqCst);
    }
    let bridge = host.bridge.clone();
    let operation = method.clone();
    let result = tauri::async_runtime::spawn_blocking(move || bridge.request(&operation, selected))
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
    if mutation {
        host.busy.fetch_sub(1, Ordering::SeqCst);
    }
    let result = result?;
    if method == "reveal" {
        let value: Value = serde_json::from_str(&result).map_err(|e| e.to_string())?;
        let path = value[0].as_str().ok_or("出力先が見つかりません。")?;
        app.opener()
            .open_path(path, None::<&str>)
            .map_err(|e| e.to_string())?;
        return Ok(wire(Value::Null));
    }
    if method == "initial" && host.smoke {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            app.exit(0);
        });
    }
    Ok(result)
}
fn menu(app: &tauri::App) -> tauri::Result<()> {
    let item = |id: &str, text: &str, shortcut: Option<&str>| {
        MenuItem::with_id(app, id, text, true, shortcut)
    };
    let file = Submenu::with_items(
        app,
        "ファイル",
        true,
        &[
            &item("new", "新しいプロジェクト", Some("CmdOrCtrl+Shift+N"))?,
            &item("open", "開く…", Some("CmdOrCtrl+O"))?,
            &item("save", "保存", Some("CmdOrCtrl+S"))?,
            &item("save-all", "すべて保存", Some("CmdOrCtrl+Shift+S"))?,
            &PredefinedMenuItem::separator(app)?,
            &item("build", "Build", Some("CmdOrCtrl+B"))?,
            &item("close-file", "ファイルを閉じる", Some("CmdOrCtrl+W"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "編集",
        true,
        &[
            &item("undo", "元に戻す", Some("CmdOrCtrl+Z"))?,
            &item("redo", "やり直す", Some("CmdOrCtrl+Shift+Z"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "表示",
        true,
        &[
            &item(
                "editor-settings",
                "エディター設定…",
                Some("CmdOrCtrl+Comma"),
            )?,
            &item(
                "explorer",
                "エクスプローラーの表示切替",
                Some("CmdOrCtrl+Shift+E"),
            )?,
            &item("find", "検索", Some("CmdOrCtrl+F"))?,
            &item("problems", "問題", Some("CmdOrCtrl+Shift+M"))?,
            &item("build-results", "生成結果", None)?,
            &PredefinedMenuItem::separator(app)?,
            &item("zoom-reset", "実際のサイズ", Some("CmdOrCtrl+0"))?,
            &item("zoom-in", "拡大", Some("CmdOrCtrl+Plus"))?,
            &item("zoom-out", "縮小", Some("CmdOrCtrl+-"))?,
        ],
    )?;
    let menu = Menu::new(app)?;
    #[cfg(target_os = "macos")]
    {
        let name = &app
            .config()
            .product_name
            .clone()
            .unwrap_or("MasterData".into());
        menu.append(&Submenu::with_items(
            app,
            name,
            true,
            &[
                &PredefinedMenuItem::about(app, None, None)?,
                &PredefinedMenuItem::separator(app)?,
                &item("close", "終了", Some("CmdOrCtrl+Q"))?,
            ],
        )?)?;
    }
    menu.append(&file)?;
    menu.append(&edit)?;
    menu.append(&view)?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if let Some(window) = app.get_webview_window("main") {
            let id = event.id().as_ref();
            if id == "close" {
                let _ = window.close();
            } else {
                emit(&window, id);
            }
        }
    });
    Ok(())
}
fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke-test");
    let mut builder = tauri::Builder::default();
    if !smoke {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            project_request,
            set_dirty,
            close_project
        ])
        .setup(move |app| {
            let executable = std::env::current_exe()?;
            let name = if cfg!(windows) {
                "masterdata-host.exe"
            } else {
                "masterdata-host"
            };
            let host_exe = executable.parent().unwrap().join(name);
            let resource = app.path().resource_dir()?;
            let args: Vec<String> = std::env::args().collect();
            let project = args
                .iter()
                .position(|a| a == "--project")
                .and_then(|i| args.get(i + 1))
                .cloned();
            app.manage(Host {
                bridge: Arc::new(Bridge::start(&host_exe)?),
                dirty: AtomicBool::new(false),
                busy: AtomicUsize::new(0),
                closing: AtomicBool::new(false),
                project,
                sample: resource.join("sample"),
                data: app.path().app_data_dir()?,
                smoke,
            });
            let config = &app.config().app.windows[0];
            let window = tauri::WebviewWindowBuilder::from_config(app, config)?
                .on_navigation(app_navigation)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
            // Set the webview attribute explicitly: the config-only window inset is
            // overwritten by the webview's default inset in the pinned Tauri runtime.
            #[cfg(target_os = "macos")]
            let window = window.traffic_light_position(tauri::LogicalPosition::new(18.0, 30.0));
            window.build()?;
            menu(app)?;
            if smoke {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(15));
                    handle.exit(1);
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let host = window.state::<Host>();
                if !host.closing.load(Ordering::SeqCst) {
                    let busy = host.busy.load(Ordering::SeqCst) != 0;
                    if busy || host.dirty.load(Ordering::SeqCst) {
                        api.prevent_close();
                        let _ = window.emit("command", if busy { "busy-close" } else { "close" });
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauriの起動に失敗しました")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => app.state::<Host>().bridge.stop(),
            tauri::RunEvent::ExitRequested { api, .. } => {
                let host = app.state::<Host>();
                let busy = host.busy.load(Ordering::SeqCst) != 0;
                if !host.closing.load(Ordering::SeqCst)
                    && (busy || host.dirty.load(Ordering::SeqCst))
                {
                    api.prevent_exit();
                    if let Some(window) = app.get_webview_window("main") {
                        emit(&window, if busy { "busy-close" } else { "close" });
                    }
                }
            }
            _ => {}
        });
}

#[cfg(test)]
mod navigation_tests {
    use super::*;
    #[test]
    fn native_bridge_stays_on_the_app_origin() {
        for url in [
            "tauri://localhost/index.html",
            "http://tauri.localhost/",
            "https://tauri.localhost/",
        ] {
            assert!(app_navigation(&tauri::Url::parse(url).unwrap()));
        }
        for url in [
            "https://example.org/",
            "http://tauri.localhost.attacker.test/",
            "file:///tmp/index.html",
            "http://127.0.0.1:8888/",
        ] {
            assert!(!app_navigation(&tauri::Url::parse(url).unwrap()));
        }
        assert_eq!(
            app_navigation(&tauri::Url::parse("http://127.0.0.1:5173/").unwrap()),
            cfg!(debug_assertions)
        );
    }
}
