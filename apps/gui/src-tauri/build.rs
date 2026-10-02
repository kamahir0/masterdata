fn main() {
    #[cfg(feature = "navigation-evidence")]
    {
        tauri_build::try_build(tauri_build::Attributes::new().plugin(
            "navigation-evidence",
            tauri_build::InlinedPlugin::new().commands(&["report"]),
        ))
        .unwrap();
    }
    #[cfg(not(feature = "navigation-evidence"))]
    tauri_build::build();
}
