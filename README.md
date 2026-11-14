<div align="center">

<img src="src-tauri/icons/insyndoc/128x128.png" alt="InsynDoc icon" width="96" height="96">

# InsynDoc

### A private, offline word processing workspace for Windows.

Create, edit, and export DOCX documents. Your documents and preferences stay on your device.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform: Windows](https://img.shields.io/badge/platform-Windows-0078D4)
![Offline first](https://img.shields.io/badge/offline-first-2E8B57)
![Built with Tauri](https://img.shields.io/badge/built_with-Tauri_2-FFC131)

</div>

---

## Features

- Create a new document or open an existing DOCX file.
- Edit and format text, lists, tables, headers, footers, and page layout.
- Find text, replace text, navigate by page or heading, and view word statistics.
- Check English spelling and sentence context locally, with a personal dictionary.
- Export documents as DOCX or PDF.
- Choose light, dark, or system theme, ruler units, word goal, and editor preferences.
- Use focus mode, keyboard shortcuts, and automatic formatting.

InsynDoc is designed to work offline. Opening links in a browser and downloading document fonts may require an internet connection.

## Install

Download the [Windows installer](https://github.com/insanmiy/InsynDoc/releases/latest) and run the setup file. Choose an installation for your user or for all users. Installing for all users requires administrator approval.

## Getting started

1. Open InsynDoc and start a new document, or select **Import** to open a DOCX file.
2. Write and format your document using the toolbar and menus.
3. Save your changes with **Ctrl+S**, or use the Download menu to export a copy.

| Shortcut | Action |
| --- | --- |
| `Ctrl+N` | New document |
| `Ctrl+O` | Open a document |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Export a DOCX copy |
| `Ctrl+F` | Find text |
| `Ctrl+G` | Go to a page |
| `Ctrl+/` | Show keyboard shortcuts |

## Run from this repository

Install Node.js, npm, Rust 1.85 or newer, and the [Tauri Windows Prerequisites](https://v2.tauri.app/start/prerequisites/#windows), including Microsoft C++ Build Tools and WebView2. From the repository directory, run:

```powershell
npm ci
npm run dev
```

To build the web frontend:

```powershell
npm run build
```

To build the Windows installer:

```powershell
npm run dist
```

The installer is written under `src-tauri/target/release/bundle/nsis/`. You can also run `Start.bat` on Windows to build when needed and launch the desktop app.

## Development commands

| Command | Description |
| --- | --- |
| `npm run dev` | Start the Tauri desktop development app. |
| `npm run dev:web` | Start the frontend development server. |
| `npm run build` | Typecheck and build the frontend. |
| `npm run typecheck` | Run the TypeScript type checker. |
| `npm run test:rust` | Run the Rust tests. |
| `npm run dist` | Build the Windows NSIS installer. |

## Release notes

Keep user-visible changes in [CHANGELOG.md](CHANGELOG.md) under `Unreleased`, grouped into Added, Changed, and Fixed as needed.

Before publishing a release, move its entries into a section such as `## [1.0.3] - 2026-11-14`, leaving an empty `Unreleased` section for future work. Use the actual release version and date. The version must match the application version and the release tag, such as `v1.0.3`.

The release workflow reads that version's changelog section for both GitHub release notes and in-app update notes. It stops if the section is missing, empty, or duplicated. To check the notes locally before tagging, run:

```powershell
node scripts/release-notes.mjs v1.0.3
```

Use your release's version in the command. The extracted notes are written to `artifacts/release-notes.md` for review.

## Privacy and storage

Documents are opened and saved locally. InsynDoc does not require an account or a hosted service. Preferences, including personal spelling words and learned corrections, are stored on this device.

Opening external links, downloading fonts, and checking for updates require a network connection.

## License

InsynDoc is licensed under the MIT license. See [LICENSE](LICENSE) and [Third party notices](THIRD_PARTY_NOTICES.md) for dependency and font licensing information.
