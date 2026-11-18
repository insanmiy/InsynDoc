# Changelog

## [Unreleased]

## [1.0.3] - 2026-11-18

### Added

- **Interface layouts.** Choose Classic, Rail, Canvas, or Studio in Settings.
- **Save as.** Save to a new file and continue editing that file. Export still saves a separate copy.
- **Selection statistics.** Word count can report statistics for selected text and supports Unicode characters. It shows character counts with and without spaces.
- **Windows uninstaller.** Open the InsynDoc uninstaller from Settings. InsynDoc checks for unsaved changes first.

### Changed

- **Settings sidebar.** Jump to any section from the sidebar. The highlight follows the section you scroll to.
- **Startup.** The app window can appear while the DOCX editor loads. InsynDoc reads startup documents once and defers settings, font loading, and spelling checks.
- **Page counts.** Counts update after edits, large pastes, undo, and redo, with fewer layout measurements while you type.
- **Grammar review.** Start a review from the Tools menu or press `Ctrl+Alt+X`. Spelling checks run automatically. Editing cancels a review in progress, and InsynDoc frees review memory when the check ends.
- **Spelling suggestions.** Hover over a spelling mark to see suggestions without moving the caret. Clicking still moves the caret, and keyboard review remains available.
- **Suggestion ranking.** Spelling suggestions consider nearby keyboard keys and give common words a small boost. Difficult misspellings can get additional suggestions when dictionary matches are weak.
- **Fonts.** InsynDoc bundles its interface font for consistent offline rendering and caches the list of installed fonts it reads from the Windows registry.
- **Text style.** App labels, messages, document templates, and the README use sentence case.

### Fixed

- **Version History.** InsynDoc prompts about unsaved changes before restoring a version. Cancelling leaves Version History open.
- **Dictionary.** Only **Add to dictionary** saves a custom word. Accepting a spelling suggestion now leaves the dictionary intact.
- **Spelling popup.** Suggestions support touch and stay open as the pointer moves to the popup. The popup leaves Escape available to other menus.
- **Spelling underlines.** Underlines stay readable on white document pages in dark mode.
- **Keyboard navigation.** Download menus support opening with the arrow keys and navigating with Tab. They preserve keyboard focus after export. Closing Templates or Version History returns focus to the control that opened it.
- **System theme.** The system theme follows changes to Windows appearance while InsynDoc is open.
- **Caps Lock.** The indicator reflects the Windows keyboard state when InsynDoc opens or regains focus.

### Development

- **Builds.** Asset preparation reuses cached output when inputs match. Frontend bundles are split, and type checking runs alongside frontend compilation.
- **Editor benchmarks.** Measure paste performance, typing after paste, and page counts.
- **Regression tests.** Cover Save as, selection statistics, page counts, spelling suggestions, dictionary approval, and spelling hover interactions.
- **Release notes.** GitHub releases and in-app updates use this changelog. Publishing requires a nonempty section matching the version tag. Build and release workflows run frontend tests and check release note extraction.

## [1.0.2] - 2026-11-10

### Added

- **File import.** Open Markdown, HTML, plain text, and OpenDocument text files as editable documents.
- **Templates.** Start with a resume, business letter, APA paper, or meeting notes template. You can also save and delete custom templates.
- **Version History.** InsynDoc saves local recovery snapshots of pending document changes every two minutes. Use Version History to restore a snapshot.
- **Smart punctuation.** Straight quotes become curly quotes, and double dashes become em dashes as you type.
- **Snippets.** Type a custom trigger to insert its saved replacement text.
- **Export privacy.** A preference removes hidden author, company, timestamp, and revision metadata from exported copies.
- **Local fonts.** Documents can use installed fonts while offline.

### Changed

- **Editor menus.** Menus show existing keyboard shortcuts and add icons for more actions.

### Fixed

- **DOCX export.** Resized images keep their dimensions in the exported file.
- **Update settings.** Corrected the layout of the update controls.

### Development

- **CI.** The local Windows GitHub Actions runner handles build and release jobs. CI prepares app assets before Rust tests and retries intermittent end-to-end test failures.
- **Repository maintenance.** Restored README visibility, expanded its setup, usage, and development instructions, and removed unused test and font helper scripts.
