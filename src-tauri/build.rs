fn main() {
    tauri_build::build();
    #[cfg(target_os = "windows")]
    let _ = embed_resource::compile_for_tests("test-manifest.rc", embed_resource::NONE);
}
