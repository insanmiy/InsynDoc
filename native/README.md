# InsynDoc Native Rust Rewrite

The Native Application Uses Rust, Winit, Wgpu, And Egui For The Window, Application Controls, And Document Surface. It Does Not Load A Browser, WebView2, JavaScript, Or A Web Frontend Bundle.

## Build And Run

Install Rust With The GNU Windows Toolchain:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
```

The GNU Toolchain Also Requires MinGW-w64 Build Tools On `PATH`. The Launcher Automatically Adds `.tools/mingw64/bin` When Its `gcc.exe` Is Present. Otherwise, Install MinGW-w64 And Add Its `bin` Directory To `PATH` Before Building. A Missing Compiler Or Linker Stops The Launcher With Setup Guidance.

Alternatively, Install The Microsoft C++ Build Tools With The Desktop C++ Workload And Use The MSVC Rust Toolchain Directly:

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
cargo +stable-x86_64-pc-windows-msvc run --manifest-path native/Cargo.toml
```

`Start.bat` Uses The GNU Toolchain; The MSVC Alternative Uses The Cargo Command Above.

From The Repository Root, Run `Start.bat`. The Launcher Builds The Release Executable When Native Sources, Fonts, Branding Assets, Or Cargo Files Change. Use `Start.bat build` To Force A Build.

For Development:

```powershell
cargo +stable-x86_64-pc-windows-gnu run --manifest-path native/Cargo.toml
```

For Tests:

```powershell
cargo +stable-x86_64-pc-windows-gnu test --manifest-path native/Cargo.toml
```

Node.js, npm, Tauri, And WebView2 Are Not Required By This Native Build. The Existing `src`, `src-tauri`, And npm Scripts Remain As The Original Application And A Reference During Migration. They Are Separate From The Native Launcher.

## Migration Status

This Rewrite Is In Progress. The Native Document Model And Editor Replace The Original Web Document Engine. Existing Menus And Controls Are Retained As A Visual Reference, But Complete Feature Parity And Identical Document Layout Have Not Been Established.

The Native Toolbar Includes Formatting, Lists, Indentation, Line Spacing, Highlighting, Formatting Painter, Menu Search, And Focus Controls. Native Dialogs Support Page Setup, Paragraph Settings, Date And Time, Special Characters, And Mathematical Text. Bookmarks Currently Last Only For The Open Session. Mathematical Text Is Not An OOXML Equation Object.

The Visual Comparison Harness Renders The Actual Egui Widgets Through Wgpu Without Opening A Browser. Run It With:

```powershell
cargo +stable-x86_64-pc-windows-gnu test --manifest-path native/Cargo.toml render_comparison_images -- --ignored --nocapture
```

Images Are Written To `artifacts/ui/`. Set `INSYNDOC_VISUAL_SUFFIX` To Label A Comparison Run. The Settings Fixture Uses Centimeters And A Word Goal Of 10 To Match The Original Settings Reference.

Complex DOCX Features Such As Tables, Images, Headers, Footers, Fields, And Exact Pagination Need Further Implementation And Verification. Unsupported Features Must Not Be Assumed To Render Or Edit Correctly. Keep Original Documents And Review Exported Copies While Testing The Rewrite.

The Original Tauri Installer And Release Workflow Have Not Been Migrated To Package The Native Executable.
