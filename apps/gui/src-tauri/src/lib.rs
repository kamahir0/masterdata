use std::path::Path;

use masterdata_core::{ProjectInfo, ProjectService};

#[tauri::command(rename_all = "camelCase")]
fn project_info(project_path: Option<String>) -> std::result::Result<ProjectInfo, String> {
    let current_dir = std::env::current_dir().map_err(|error| error.to_string())?;
    let configured_path = project_path.or_else(|| std::env::var("MASTERDATA_PROJECT_PATH").ok());
    let explicit_path = configured_path.as_deref().map(Path::new);
    ProjectService::new()
        .project_info(explicit_path, &current_dir)
        .map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![project_info])
        .run(tauri::generate_context!())
        .expect("error while running masterdata GUI");
}
