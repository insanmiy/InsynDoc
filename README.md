<div align="center">

<img src="src-tauri/icons/insyndoc/128x128.png" alt="InsynDoc Icon" width="96" height="96">

# InsynDoc

### A Private, Offline Word Processing Workspace For Windows.

Create, Edit, And Export DOCX Documents. Your Documents And Preferences Stay On Your Device.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform: Windows](https://img.shields.io/badge/platform-Windows-0078D4)
![Offline First](https://img.shields.io/badge/offline-first-2E8B57)
![Built With Tauri](https://img.shields.io/badge/built_with-Tauri_2-FFC131)

</div>

---

## Native Rust Rewrite

`Start.bat` Now Builds And Launches The Native Rust Application In `native/`. It Uses Egui, Winit, And Wgpu Without WebView2, JavaScript, Node.js, Or A Web Bundle At Runtime. See [Native Build Instructions](native/README.md).

The Rewrite Is In Progress. Full Feature And Visual Parity Have Not Been Reached. The Features And Installer Instructions Below Describe The Original Tauri Application, Which Remains In `src` And `src-tauri` As A Migration Reference. The Existing Release Installer Still Packages That Original Application.

## Features

- Create A New Document Or Open An Existing DOCX File.
- Edit And Format Text, Lists, Tables, Headers, Footers, And Page Layout.
- Find Text, Replace Text, Navigate By Page Or Heading, And View Word Statistics.
- Check English Spelling And Sentence Context Locally, With A Personal Dictionary.
- Export Documents As DOCX Or PDF.
- Choose Light, Dark, Or System Theme, Ruler Units, Word Goal, And Editor Preferences.
- Use Focus Mode, Keyboard Shortcuts, And Automatic Formatting.

InsynDoc Is Designed To Work Offline. Opening Links In A Browser And Downloading Document Fonts May Require An Internet Connection.

## Install

Download The Windows Installer From [GitHub Releases](https://github.com/insanmiy/InsynDoc/releases/latest) And Run The Setup File. Choose An Installation For Your User Or For All Users. Installing For All Users Requires Administrator Approval.

The Windows WebView2 Runtime Is Required. The Installer Can Download It When Needed.

## Getting Started

1. Open InsynDoc And Start A New Document, Or Select **Import** To Open A DOCX File.
2. Write And Format Your Document Using The Toolbar And Menus.
3. Save Your Changes With **Ctrl+S**, Or Use The Download Menu To Export A Copy.

| Shortcut | Action |
| --- | --- |
| `Ctrl+N` | New Document |
| `Ctrl+O` | Open A Document |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Export A DOCX Copy |
| `Ctrl+F` | Find Text |
| `Ctrl+G` | Go To A Page |
| `Ctrl+/` | Show Keyboard Shortcuts |

## Run From This Repository

Install Node.js, npm, Rust 1.85 Or Newer, And The [Tauri Windows Prerequisites](https://v2.tauri.app/start/prerequisites/#windows), Including Microsoft C++ Build Tools And WebView2. From The Repository Directory, Run:

```powershell
npm ci
npm run dev
```

To Build The Web Frontend:

```powershell
npm run build
```

To Build The Windows Installer:

```powershell
npm run dist
```

The Installer Is Written Under `src-tauri/target/release/bundle/nsis/`. `Start.bat` Builds And Launches The Separate Native Rust Rewrite Described Above.

## Development Commands

| Command | Description |
| --- | --- |
| `npm run dev` | Start The Tauri Desktop Development App. |
| `npm run dev:web` | Start The Frontend Development Server. |
| `npm run build` | Typecheck And Build The Frontend. |
| `npm run typecheck` | Run The TypeScript Type Checker. |
| `npm test` | Run The Vitest Unit Tests. |
| `npm run test:e2e` | Build The Frontend And Run Playwright End-To-End Tests. |
| `npm run test:rust` | Run The Rust Tests. |
| `npm run dist` | Build The Windows NSIS Installer. |

## Privacy And Storage

Documents Are Opened And Saved Locally. InsynDoc Does Not Require An Account Or A Hosted Service. Preferences, Including Personal Spelling Words And Learned Corrections, Are Stored On This Device.

Opening External Links, Downloading Fonts, And Checking For Updates Require A Network Connection.

## License

InsynDoc Is Licensed Under The MIT License. See [LICENSE](LICENSE) And [Third Party Notices](THIRD_PARTY_NOTICES.md) For Dependency And Font Licensing Information.
