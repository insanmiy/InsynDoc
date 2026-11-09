fn main() {
    tauri_build::build();
    #[cfg(target_os = "windows")]
    if std::path::Path::new("tests/bridge.rs").is_file() {
        let _ = embed_resource::compile_for_tests("test-manifest.rc", embed_resource::NONE);
    }
}
