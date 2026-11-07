//! Application State Shared By Native Controls And The Rust Editor.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    Light,
    Dark,
    #[default]
    #[serde(rename = "system")]
    System,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Pages,
    Continuous,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RulerUnit {
    Inch,
    Cm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellingCorrection {
    pub source: String,
    pub replacement: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    #[serde(default = "default_theme")]
    pub theme: ThemePref,
    #[serde(default = "default_true")]
    pub show_ruler: bool,
    #[serde(default = "default_true")]
    pub show_outline: bool,
    #[serde(default = "default_true")]
    pub show_word_count: bool,
    #[serde(default)]
    pub ruler_unit: RulerUnit,
    #[serde(default)]
    pub word_goal: u32,
    #[serde(default)]
    pub browser_path: Option<String>,
    #[serde(default = "default_true")]
    pub check_spelling: bool,
    #[serde(default = "default_true")]
    pub update_notifications: bool,
    #[serde(default)]
    pub spelling_words: Vec<String>,
    #[serde(default)]
    pub spelling_corrections: Vec<SpellingCorrection>,
}

fn default_theme() -> ThemePref {
    ThemePref::System
}
fn default_true() -> bool {
    true
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemePref::System,
            show_ruler: true,
            show_outline: true,
            show_word_count: true,
            ruler_unit: RulerUnit::Inch,
            word_goal: 0,
            browser_path: None,
            check_spelling: true,
            update_notifications: true,
            spelling_words: Vec::new(),
            spelling_corrections: Vec::new(),
        }
    }
}

impl Default for RulerUnit {
    fn default() -> Self {
        RulerUnit::Inch
    }
}

/// Current Document And Workspace State.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SurfaceState {
    pub name: String,
    pub token: Option<String>,
    pub dirty: bool,
    pub ready: bool,
    pub busy: bool,
    pub message: String,
    pub notice: Option<String>,
    pub error: Option<String>,
    pub view: ViewMode,
    pub page_count: u32,
    pub statistics: TextStatistics,
    pub find_open: bool,
    pub focus_mode: bool,
    pub editor_ready: bool,
    #[serde(default = "default_zoom")]
    pub zoom: f32,
}

fn default_zoom() -> f32 {
    1.0
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextStatistics {
    pub words: u64,
    pub characters: u64,
    pub characters_without_spaces: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    None,
    Settings,
    GoToPage,
    Shortcuts,
    Statistics,
    Update,
    ConfirmClose,
    DownloadMenu,
    Rename,
}

#[derive(Clone)]
pub struct PendingUpdate {
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

pub struct AppState {
    pub surface: SurfaceState,
    pub preferences: Preferences,
    pub modal: Modal,
    pub open_menu: Option<&'static str>,
    pub link_error: Option<String>,
    pub page_number_input: String,
    pub page_error: Option<String>,
    pub renaming: bool,
    pub title_draft: String,
    pub update: Option<PendingUpdate>,
    pub update_installing: bool,
    pub update_error: Option<String>,
    pub confirm_message: Option<String>,
    pub pending_close_choice: Option<UnsavedChoice>,
    pub quitting: bool,
    /// Correlates an in-app unsaved-changes request from the web surface.
    pub confirm_request_id: Option<u64>,
    /// True when the confirm modal was triggered by the OS window close.
    pub confirm_for_window_close: bool,
    /// Editor undo/redo availability for menu disabled states.
    pub can_undo: bool,
    pub can_redo: bool,
    /// View toggle states for menu checkmarks.
    pub show_bookmarks: bool,
    pub show_paragraph_breaks: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            surface: SurfaceState {
                name: "Untitled.docx".into(),
                message: "Opening Editor".into(),
                ..Default::default()
            },
            preferences: Preferences::default(),
            modal: Modal::None,
            open_menu: None,
            link_error: None,
            page_number_input: "1".into(),
            page_error: None,
            renaming: false,
            title_draft: String::new(),
            update: None,
            update_installing: false,
            update_error: None,
            confirm_message: None,
            pending_close_choice: None,
            quitting: false,
            confirm_request_id: None,
            confirm_for_window_close: false,
            can_undo: false,
            can_redo: false,
            show_bookmarks: false,
            show_paragraph_breaks: false,
        }
    }
}

impl AppState {
    pub fn unavailable(&self) -> bool {
        !self.surface.ready || self.surface.busy
    }
}
