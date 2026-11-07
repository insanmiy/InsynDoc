use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::backend::{downloads, files, recent};
use crate::editor::Editor;
use crate::state::{AppState, Modal, UnsavedChoice};

#[derive(Clone, Debug, PartialEq)]
enum PendingAction {
    New,
    Open(PathBuf),
    Quit,
}

#[derive(Default)]
pub struct Workspace {
    pub editor: Editor,
    path: Option<PathBuf>,
    pending: Option<PendingAction>,
}

impl Workspace {
    pub fn startup(&mut self, state: &mut AppState) {
        match crate::backend::startup::StartupDocument::from_args(std::env::args_os()).take() {
            Ok(Some(path)) => self.open(&path, state),
            Ok(None) => self.editor.sync_state(state),
            Err(error) => state.surface.error = Some(error),
        }
    }

    pub fn request_close(&mut self, state: &mut AppState) {
        self.request(PendingAction::Quit, state);
    }

    fn request(&mut self, action: PendingAction, state: &mut AppState) {
        if !state.surface.dirty {
            self.perform(action, state);
            return;
        }
        self.pending = Some(action);
        state.confirm_message = Some(format!(
            "Your Changes To {} Have Not Been Saved.",
            state.surface.name
        ));
        state.modal = Modal::ConfirmClose;
    }

    pub fn confirm(&mut self, choice: UnsavedChoice, state: &mut AppState) {
        if choice == UnsavedChoice::Cancel {
            self.pending = None;
            state.modal = Modal::None;
            return;
        }
        if choice == UnsavedChoice::Save && !self.save(false, state) {
            return;
        }
        state.modal = Modal::None;
        if let Some(action) = self.pending.take() {
            self.perform(action, state);
        }
    }

    fn perform(&mut self, action: PendingAction, state: &mut AppState) {
        match action {
            PendingAction::Quit => state.quitting = true,
            PendingAction::Open(path) => self.open(&path, state),
            PendingAction::New => {
                self.editor = Editor::default();
                self.path = None;
                state.surface.name = "Untitled.docx".into();
                state.surface.token = None;
                state.surface.dirty = false;
                state.surface.error = None;
                self.editor.sync_state(state);
            }
        }
    }

    fn open(&mut self, path: &Path, state: &mut AppState) {
        match crate::document::Document::open(path) {
            Ok(document) => {
                self.editor.document = document;
                self.editor.reset_history();
                self.path = Some(path.to_path_buf());
                state.surface.name = files::document_name(path);
                state.surface.token = Some(path.to_string_lossy().into_owned());
                state.surface.dirty = false;
                state.surface.error = None;
                self.editor.sync_state(state);
                self.remember(path, state);
            }
            Err(error) => state.surface.error = Some(error.to_string()),
        }
    }

    fn remember(&self, path: &Path, state: &mut AppState) {
        if let Err(error) = recent::remember(&crate::prefs::app_data_dir(), path) {
            state.surface.notice = Some(error);
        }
    }

    fn save(&mut self, copy: bool, state: &mut AppState) -> bool {
        let destination = if copy || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("Word Document", &["docx"])
                .set_file_name(&state.surface.name)
                .save_file()
        } else {
            self.path.clone()
        };
        let Some(destination) = destination else {
            return false;
        };
        if let Err(error) = self.editor.document.save(&destination) {
            state.surface.error = Some(error.to_string());
            return false;
        }
        if !copy {
            self.path = Some(destination.clone());
            state.surface.name = files::document_name(&destination);
            state.surface.token = Some(destination.to_string_lossy().into_owned());
            state.surface.dirty = false;
        }
        state.surface.error = None;
        state.surface.notice = Some("Document Saved Locally".into());
        self.remember(&destination, state);
        true
    }

    pub fn recents(&self, ctx: &egui::Context, state: &mut AppState) {
        match recent::list(&crate::prefs::app_data_dir()) {
            Ok(paths) => crate::ui::store_recents(
                ctx,
                paths
                    .into_iter()
                    .map(|path| crate::ui::RecentEntry {
                        token: path.to_string_lossy().into_owned(),
                        name: files::document_name(&path),
                        available: path.is_file(),
                        path: path.to_string_lossy().into_owned(),
                    })
                    .collect(),
            ),
            Err(error) => state.surface.error = Some(error),
        }
    }

    pub fn execute(&mut self, ctx: &egui::Context, name: &str, args: &Value, state: &mut AppState) {
        match name {
            "newDocument" => self.request(PendingAction::New, state),
            "open" => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Word Document", &["docx"])
                    .pick_file()
                {
                    self.request(PendingAction::Open(path), state);
                }
            }
            "openRecent" => {
                let token = args.get("token").and_then(Value::as_str).unwrap_or("");
                let allowed = recent::list(&crate::prefs::app_data_dir()).unwrap_or_default();
                if let Some(path) = allowed
                    .into_iter()
                    .find(|path| path.to_string_lossy() == token)
                {
                    self.request(PendingAction::Open(path), state);
                }
            }
            "clearRecent" => {
                if let Err(error) = recent::clear(&crate::prefs::app_data_dir()) {
                    state.surface.error = Some(error);
                }
                self.recents(ctx, state);
            }
            "save" => {
                self.save(false, state);
            }
            "saveAs" => {
                self.save(true, state);
            }
            "download" if args.get("format").and_then(Value::as_str) == Some("docx") => {
                self.save(true, state);
            }
            "download" if args.get("format").and_then(Value::as_str) == Some("md") => {
                let markdown = match crate::backend::markdown::export(&self.editor.document) {
                    Ok(markdown) => markdown,
                    Err(error) => {
                        state.surface.error = Some(error);
                        return;
                    }
                };
                let filename = format!("{}.md", state.surface.name.trim_end_matches(".docx"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md"])
                    .set_file_name(filename)
                    .save_file()
                {
                    if let Err(error) = downloads::write_download(&path, downloads::DownloadFormat::Md, markdown.as_bytes()) {
                        state.surface.error = Some(error);
                    }
                }
            }
            "download" if args.get("format").and_then(Value::as_str) == Some("rtf") => {
                let rtf = match crate::backend::rtf::export(&self.editor.document) {
                    Ok(rtf) => rtf,
                    Err(error) => {
                        state.surface.error = Some(error);
                        return;
                    }
                };
                let filename = format!("{}.rtf", state.surface.name.trim_end_matches(".docx"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Rich Text", &["rtf"])
                    .set_file_name(filename)
                    .save_file()
                {
                    if let Err(error) = downloads::write_download(&path, downloads::DownloadFormat::Rtf, rtf.as_bytes()) {
                        state.surface.error = Some(error);
                    }
                }
            }
            "download" if args.get("format").and_then(Value::as_str) == Some("pdf") => {
                let pdf = match crate::backend::pdf::export(ctx, &self.editor.document) {
                    Ok(pdf) => pdf,
                    Err(error) => {
                        state.surface.error = Some(error);
                        return;
                    }
                };
                let filename = format!("{}.pdf", state.surface.name.trim_end_matches(".docx"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF Document", &["pdf"])
                    .set_file_name(filename)
                    .save_file()
                {
                    if let Err(error) = downloads::write_download(&path, downloads::DownloadFormat::Pdf, &pdf) {
                        state.surface.error = Some(error);
                    }
                }
            }
            "download" if args.get("format").and_then(Value::as_str) == Some("html") => {
                let archive = match crate::backend::html::export(&self.editor.document, &state.surface.name) {
                    Ok(archive) => archive,
                    Err(error) => {
                        state.surface.error = Some(error);
                        return;
                    }
                };
                let filename = format!("{}.zip", state.surface.name.trim_end_matches(".docx"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Zipped Web Page", &["zip"])
                    .set_file_name(filename)
                    .save_file()
                {
                    if let Err(error) = downloads::write_download(&path, downloads::DownloadFormat::Html, &archive) {
                        state.surface.error = Some(error);
                    }
                }
            }
            "exportPlainText" | "download" => {
                let format = args.get("format").and_then(Value::as_str).unwrap_or("txt");
                if format != "txt" {
                    state.surface.error = Some(format!(
                        "The Rust Editor Does Not Yet Support {format} Export."
                    ));
                    return;
                }
                let filename = format!("{}.txt", state.surface.name.trim_end_matches(".docx"));
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Plain Text", &["txt"])
                    .set_file_name(filename)
                    .save_file()
                {
                    if let Err(error) = files::write_text(&path, &self.editor.document.plain_text())
                    {
                        state.surface.error = Some(error);
                    }
                }
            }
            "rename" => {
                let name = args
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                if name.is_empty() {
                    return;
                }
                let name = if name.to_ascii_lowercase().ends_with(".docx") {
                    name.to_owned()
                } else {
                    format!("{name}.docx")
                };
                if let Some(path) = &self.path {
                    match files::rename_document(path, &name) {
                        Ok(path) => {
                            self.path = Some(path.clone());
                            state.surface.name = name;
                            state.surface.token = Some(path.to_string_lossy().into_owned());
                            self.remember(&path, state);
                        }
                        Err(error) => state.surface.error = Some(error),
                    }
                } else if name
                    .chars()
                    .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
                {
                    state.surface.error = Some("Choose A Valid Document Name.".into());
                } else {
                    state.surface.name = name;
                }
            }
            "clearNotice" => state.surface.notice = None,
            "clearError" => state.surface.error = None,
            _ => {
                if !self.editor.execute(name, args, state) {
                    state.surface.error = Some(format!(
                        "This Feature Is Not Yet Available In The Rust Editor: {name}"
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_document_requires_confirmation_and_cancel_keeps_it_open() {
        let mut workspace = Workspace::default();
        let mut state = AppState::default();
        state.surface.dirty = true;
        workspace.request_close(&mut state);
        assert!(!state.quitting);
        assert!(state.modal == Modal::ConfirmClose);
        workspace.confirm(UnsavedChoice::Cancel, &mut state);
        assert!(!state.quitting);
        assert!(state.surface.dirty);
        assert!(workspace.pending.is_none());
    }

    #[test]
    fn discard_performs_pending_action_and_clean_close_needs_no_prompt() {
        let mut workspace = Workspace::default();
        let mut state = AppState::default();
        state.surface.dirty = true;
        workspace.request(PendingAction::New, &mut state);
        workspace.confirm(UnsavedChoice::Discard, &mut state);
        assert!(!state.surface.dirty);
        workspace.request_close(&mut state);
        assert!(state.quitting);
    }
}
