use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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

const REGISTRY_FONT_PATH: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";

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

const EXCLUDED_FAMILIES: &[&str] = &[
    "bookshelf symbol 7",
    "hololens mdl2 assets",
    "marlett",
    "ms outlook",
    "ms reference specialty",
    "mt extra",
    "segoe fluent icons",
    "segoe mdl2 assets",
    "symbol",
    "webdings",
    "wingdings",
    "wingdings 2",
    "wingdings 3",
];

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
    if family.is_empty() || EXCLUDED_FAMILIES.contains(&family.to_lowercase().as_str()) { return None; }
    Some((family, weight.clamp(100, 900), italic))
}

#[cfg(target_os = "windows")]
fn query_registry_fonts() -> Vec<(String, String)> {
    use windows_registry::{CURRENT_USER, LOCAL_MACHINE, Type};

    let mut entries = Vec::new();
    for hive in [LOCAL_MACHINE, CURRENT_USER] {
        let Ok(key) = hive.open(REGISTRY_FONT_PATH) else { continue; };
        let Ok(values) = key.values() else { continue; };
        for (name, value) in values {
            if !matches!(value.ty(), Type::String) { continue; }
            let Ok(text) = String::try_from(value) else { continue; };
            let name = name.trim();
            let text = text.trim();
            if name.is_empty() || text.is_empty() { continue; }
            entries.push((name.to_string(), text.to_string()));
        }
    }
    entries
}

#[cfg(not(target_os = "windows"))]
fn query_registry_fonts() -> Vec<(String, String)> {
    Vec::new()
}

static FONT_CACHE: OnceLock<Mutex<Option<Vec<InstalledFont>>>> = OnceLock::new();

fn font_cache() -> &'static Mutex<Option<Vec<InstalledFont>>> {
    FONT_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn installed_fonts() -> Result<Vec<InstalledFont>, String> {
    if let Ok(cache) = font_cache().lock() {
        if let Some(fonts) = cache.as_ref() {
            return Ok(fonts.clone());
        }
    }
    let fonts = collect_installed_fonts()?;
    if let Ok(mut cache) = font_cache().lock() {
        *cache = Some(fonts.clone());
    }
    Ok(fonts)
}

fn collect_installed_fonts() -> Result<Vec<InstalledFont>, String> {
    let mut seen = HashSet::new();
    let mut fonts = Vec::new();
    for (name, value) in query_registry_fonts() {
        let Some(path) = resolve_font_path(&value) else { continue; };
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("").to_lowercase();
        if !FONT_EXTENSIONS.contains(&extension.as_str()) { continue; }
        let display = path.to_string_lossy().into_owned();
        let base_name = name.split('(').next().unwrap_or(&name);
        for part in base_name.split('&') {
            let Some((family, weight, italic)) = parse_font_entry(part, &value) else { continue; };
            if !seen.insert((family.to_lowercase(), weight, italic, display.clone())) { continue; }
            fonts.push(InstalledFont { family, weight, style: if italic { "italic".into() } else { "normal".into() }, path: display.clone() });
        }
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
