fn main() {
    #[cfg(feature = "host")]
    tauri_build::build();
}
