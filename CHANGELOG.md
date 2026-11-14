# Changelog

User-visible changes are recorded here. Entries under Unreleased are not yet part of a published release.

## [Unreleased]

### Added

- Save as creates a new active document, while export continues to save a separate copy.
- Word count can show statistics for selected text, including Unicode characters and character counts with and without spaces.
- A maintained changelog supplies GitHub release notes and in-app update notes.

### Changed

- Startup shows an immediate loading screen and opens startup documents without parsing them twice. Settings, font loading, and spelling checks are deferred to keep the editor responsive.
- The interface font is bundled locally for consistent offline rendering.
- Page counts update after edits, bulk paste, undo, and redo without repeatedly reading layout during typing.
- Installed Windows fonts are discovered through the registry and cached.
- Application labels, messages, document templates, and the README now use sentence case.
- Hovering over a spelling mark shows suggestions without moving the caret. Clicking in the document continues to position the caret normally, and keyboard spelling review remains available.
- Spelling suggestions consider nearby keyboard keys and give common words a small ranking preference.
- Difficult misspellings can use additional suggestions even when the dictionary returns weak matches.

### Fixed

- Version History Now Confirms Unsaved Changes Before Restoring And Keeps The History Dialog Open When Cancelled.
- Spelling Suggestions Support Touch, Stay Open While Crossing To The Popup, And Respect Escape In Other Menus.
- Download Menus Preserve Keyboard Focus After Export And Support Directional Opening And Tab Navigation.
- Closing Templates And Version History Returns Focus To The Invoking Control.
- Spelling Underlines Remain Readable On White Document Pages In Dark Mode.
- The Caps Lock indicator reads the Windows keyboard state when the app opens or regains focus.
- Accepting a spelling correction no longer silently adds its replacement to the dictionary. Only the explicit Add to dictionary action approves a custom word.

### Development

- Asset preparation is cached when its inputs have not changed, frontend bundles are split, and type checking and frontend compilation run in parallel.
- Editor benchmarks measure paste performance, continued typing, and page counts.
- Added regression coverage for save as, selection statistics, page counts, spelling suggestions, dictionary approval, and spelling hover interactions.
- Build and release workflows run frontend tests and release-note extraction tests. Releases require a nonempty changelog section matching the version tag.

## [1.0.2] - 2026-11-10

### Added

- Import Markdown, HTML, plain text, and OpenDocument text files into editable documents.
- A template gallery includes resume, business letter, APA paper, and meeting notes templates, with support for saving and deleting custom templates.
- Automatic local recovery snapshots save pending document changes every two minutes, with version history and snapshot restoration.
- Smart punctuation converts straight quotes to curly quotes and double dashes to em dashes while typing.
- Custom snippet triggers expand into saved replacement text.
- An export privacy preference removes hidden author, company, timestamp, and revision metadata from exported copies.
- Local font discovery allows documents to use fonts installed on the device while offline.

### Changed

- Editor menus display the existing keyboard shortcuts and include missing action icons.
- README setup, usage, and development instructions were expanded.

### Fixed

- Resized images retain their dimensions when exported to DOCX.
- Corrected the layout of update settings.
- Restored README visibility in the repository.

### Development

- Configured the local Windows GitHub Actions runner for build and release jobs.
- App assets are prepared before Rust tests in CI, and CI retries intermittent end-to-end test failures.
- Removed unused test and font helper scripts.
