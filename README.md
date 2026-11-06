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

Download The Latest Windows Installer From The Project's Releases Page And Run The Setup File. During Installation You Can Choose To Install For Just Your User, Which Does Not Require Administrator Access, Or For All Users, Which Installs To `Program Files` And Requires Administrator Approval. The Windows WebView2 Runtime Is Required; The Installer Can Download Its Bootstrapper When Needed.

## Run From This Repository

InsynDoc Targets Windows And Uses Node.js, npm, Rust, And The Tauri 2 Toolchain. Install The Windows Build Prerequisites For Tauri, Then Run:

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

The Installer Is Written Under `src-tauri/target/release/bundle/nsis/`. You Can Also Run `Start.bat` On Windows To Build When Needed And Launch The Desktop App.

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

## License

InsynDoc Is Licensed Under The MIT License. See [LICENSE](LICENSE). Third-Party Notices And Dependency Licenses Are Included In The Repository And Installer.
