use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../src-tauri/icons/insyndoc/icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let icon = std::fs::canonicalize("../src-tauri/icons/insyndoc/icon.ico")
        .expect("Could Not Find The InsynDoc Icon.");
    let icon = icon.to_string_lossy().replace('\\', "\\\\");
    let major = std::env::var("CARGO_PKG_VERSION_MAJOR").expect("Missing Major Version");
    let minor = std::env::var("CARGO_PKG_VERSION_MINOR").expect("Missing Minor Version");
    let patch = std::env::var("CARGO_PKG_VERSION_PATCH").expect("Missing Patch Version");
    let version = format!("{major}.{minor}.{patch}");
    let resource = format!(r#"#include <windows.h>
1 ICON "{icon}"
1 VERSIONINFO
 FILEVERSION {major},{minor},{patch},0
 PRODUCTVERSION {major},{minor},{patch},0
 FILETYPE VFT_APP
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "FileDescription", "InsynDoc\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "ProductName", "InsynDoc\0"
   VALUE "ProductVersion", "{version}\0"
   VALUE "OriginalFilename", "insyndoc-native.exe\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
"#);
    let path = PathBuf::from(std::env::var_os("OUT_DIR").expect("Missing Build Directory"))
        .join("insyndoc.rc");
    std::fs::write(&path, resource).expect("Could Not Write Windows Resource File.");
    embed_resource::compile(path, embed_resource::NONE)
        .manifest_optional()
        .expect("Could Not Compile The Windows Application Resources.");
}
