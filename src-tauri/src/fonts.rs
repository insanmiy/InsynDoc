use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

pub const MAX_FONT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledFont {
    pub family: String,
    pub weight: u16,
    pub style: String,
    pub path: String,
}

const REGISTRY_KEYS: &[&str] = &[
    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
    r"HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
];

const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc"];

fn font_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        directories.push(Path::new(&windir).join("Fonts"));
    }
    if let (Some(local), Some(appdata)) = (std::env::var_os("LOCALAPPDATA"), std::env::var_os("APPDATA")) {
        let _ = appdata;
        directories.push(Path::new(&local).join("Microsoft").join("Windows").join("Fonts"));
    }
    directories
}

fn resolve_font_path(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim().trim_matches('"');
    if raw.is_empty() { return None; }
    let path = Path::new(raw);
    if path.is_absolute() { return Some(path.to_path_buf()); }
    for directory in font_directories() {
        let candidate = directory.join(raw);
        if candidate.is_file() { return Some(candidate); }
    }
    font_directories().first().map(|directory| directory.join(raw))
}

fn style_token(token: &str) -> Option<(Option<u16>, bool)> {
    match token {
        "bold" => Some((Some(700), false)),
        "italic" | "oblique" => Some((None, true)),
        "regular" | "book" => Some((None, false)),
        "medium" => Some((Some(500), false)),
        "semibold" | "demibold" => Some((Some(600), false)),
        "light" | "thin" => Some((Some(300), false)),
        "extralight" | "ultralight" => Some((Some(200), false)),
        "black" | "heavy" => Some((Some(800), false)),
        _ => None,
    }
}

fn parse_font_entry(raw_name: &str, _raw_path: &str) -> Option<(String, u16, bool)> {
    let name = raw_name.split('(').next()?.trim();
    if name.is_empty() { return None; }
    let tokens: Vec<&str> = name.split_whitespace().collect();
    let mut italic = false;
    let mut weight = 400u16;
    let mut family_end = tokens.len();
    for index in (0..tokens.len()).rev() {
        let Some((value, is_italic)) = style_token(&tokens[index].to_lowercase()) else { break; };
        italic |= is_italic;
        if let Some(value) = value { weight = value; }
        family_end = index;
    }
    if family_end == 0 { return None; }
    let family = tokens[..family_end].join(" ");
    if family.is_empty() { return None; }
    Some((family, weight.clamp(100, 900), italic))
}

fn query_registry_fonts() -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for key in REGISTRY_KEYS {
        let output = Command::new("reg")
            .args(["query", key])
            .output();
        let Ok(output) = output else { continue; };
        if !output.status.success() { continue; }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let Some((name, value)) = line.split_once("REG_SZ") else { continue; };
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() || value.is_empty() { continue; }
            entries.push((name.to_string(), value.to_string()));
        }
    }
    entries
}

pub fn installed_fonts() -> Result<Vec<InstalledFont>, String> {
    let mut seen = HashSet::new();
    let mut fonts = Vec::new();
    for (name, value) in query_registry_fonts() {
        let Some((family, weight, italic)) = parse_font_entry(&name, &value) else { continue; };
        let Some(path) = resolve_font_path(&value) else { continue; };
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("").to_lowercase();
        if !FONT_EXTENSIONS.contains(&extension.as_str()) { continue; }
        let display = path.to_string_lossy().into_owned();
        if !seen.insert((family.to_lowercase(), weight, italic, display.clone())) { continue; }
        fonts.push(InstalledFont { family, weight, style: if italic { "italic".into() } else { "normal".into() }, path: display });
    }
    fonts.sort_by(|left, right| left.family.to_lowercase().cmp(&right.family.to_lowercase()).then(left.weight.cmp(&right.weight)).then(left.style.cmp(&right.style)));
    Ok(fonts)
}

fn allowed_font_root(path: &Path) -> bool {
    font_directories().iter().any(|directory| {
        let root = directory.canonicalize().unwrap_or_else(|_| directory.clone());
        path.starts_with(&root)
    })
}

pub fn read_font_file(path: &Path) -> Result<Vec<u8>, String> {
    let canonical = path.canonicalize().map_err(|_| "The Font File Is Unavailable.".to_string())?;
    if !canonical.is_file() || !allowed_font_root(&canonical) {
        return Err("The Font File Is Outside The Allowed Font Folders.".into());
    }
    let extension = canonical.extension().and_then(|value| value.to_str()).unwrap_or("").to_lowercase();
    if !FONT_EXTENSIONS.contains(&extension.as_str()) {
        return Err("This File Type Cannot Be Used As A Font.".into());
    }
    let metadata = fs::metadata(&canonical).map_err(|error| format!("Could Not Read The Font File. {error}"))?;
    if metadata.len() > MAX_FONT_BYTES {
        return Err("The Font File Exceeds The Size Limit.".into());
    }
    fs::read(&canonical).map_err(|error| format!("Could Not Read The Font File. {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_family_weight_and_style_from_registry_names() {
        assert_eq!(parse_font_entry("Arial (TrueType)", "arial.ttf"), Some(("Arial".into(), 400, false)));
        assert_eq!(parse_font_entry("Arial Bold (TrueType)", "arialbd.ttf"), Some(("Arial".into(), 700, false)));
        assert_eq!(parse_font_entry("Arial Italic (TrueType)", "ariali.ttf"), Some(("Arial".into(), 400, true)));
        assert_eq!(parse_font_entry("Arial Bold Italic (TrueType)", "arialbi.ttf"), Some(("Arial".into(), 700, true)));
        assert_eq!(parse_font_entry("Segoe UI Semibold (TrueType)", "seguisb.ttf"), Some(("Segoe UI".into(), 600, false)));
        assert_eq!(parse_font_entry("Courier New Bold Oblique (TrueType)", "courbi.ttf"), Some(("Courier New".into(), 700, true)));
        assert_eq!(parse_font_entry("Arial Narrow (TrueType)", "arialn.ttf"), Some(("Arial Narrow".into(), 400, false)));
        assert_eq!(parse_font_entry("Microsoft Sans Serif (TrueType)", "micross.ttf"), Some(("Microsoft Sans Serif".into(), 400, false)));
        assert!(parse_font_entry("(TrueType)", "x.ttf").is_none());
    }

    #[test]
    fn rejects_font_files_outside_allowed_roots() {
        let outside = std::env::temp_dir().join("not-a-font.ttf");
        std::fs::write(&outside, b"not a font").unwrap();
        assert!(read_font_file(&outside).is_err());
        let _ = std::fs::remove_file(&outside);
    }
}
