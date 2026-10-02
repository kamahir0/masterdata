//! Test-only Desktop adapter. The portable oracle does not depend on this host.
#[tauri::command]
fn report(report: serde_json::Value, done: bool) -> Result<(), String> {
    let output = std::env::var("MASTERDATA_NAVIGATION_EVIDENCE_OUTPUT")
        .map_err(|error| error.to_string())?;
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap())
        .map_err(|error| error.to_string())?;
    // This isolated measurement process owns only a disposable fixture. Avoid
    // production quit/dirty guards: no user workspace is loaded by the harness.
    if done {
        std::process::exit(if report.get("error").is_some() { 1 } else { 0 });
    }
    Ok(())
}
pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("navigation-evidence")
        .invoke_handler(tauri::generate_handler![report])
        .on_page_load(|webview, payload| {
            eprintln!("navigation evidence page: {:?}", payload.event());
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                let mode = std::env::var("MASTERDATA_NAVIGATION_EVIDENCE_MODE").unwrap_or_default();
                webview
                    .eval(format!(
                        "window.__navigationEvidenceMode = {}; setTimeout(function() {{ {} }}, 0);",
                        serde_json::to_string(&mode).unwrap(),
                        include_str!("../../tests/navigation-inprocess.js")
                    ))
                    .unwrap();
            }
        })
        .build()
}
