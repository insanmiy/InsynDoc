//! Preferences persistence for the native shell.
//! Stores Preferences as JSON in the app data directory.

use std::path::{Path, PathBuf};

use crate::state::Preferences;

const MAX_WORD_GOAL: u32 = 100_000;
const PREFERENCES_FILE: &str = "preferences.json";

/// Returns the per-user app data directory (%APPDATA%/InsynDoc on Windows).
pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("InsynDoc")
}

/// Loads Preferences from `dir/preferences.json`.
/// Returns Preferences::default() on any missing file, parse error, or IO error.
pub fn load(dir: &Path) -> Preferences {
    let path = dir.join(PREFERENCES_FILE);
    let Ok(bytes) = std::fs::read(&path) else {
        return Preferences::default();
    };
    let mut preferences: Preferences = match serde_json::from_slice(&bytes) {
        Ok(preferences) => preferences,
        Err(_) => return Preferences::default(),
    };
    preferences.word_goal = preferences.word_goal.min(MAX_WORD_GOAL);
    preferences
}

/// Stores Preferences as JSON in `dir/preferences.json`.
/// Creates the directory if needed. Errors are ignored so a read-only
/// profile never blocks the editor.
pub fn store(dir: &Path, preferences: &Preferences) {
    let _ = std::fs::create_dir_all(dir);
    let Ok(bytes) = serde_json::to_vec_pretty(preferences) else {
        return;
    };
    let _ = std::fs::write(dir.join(PREFERENCES_FILE), bytes);
}
