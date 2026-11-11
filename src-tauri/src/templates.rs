use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::files;

pub const MAX_TEMPLATES: usize = 50;
const MAX_NAME_CHARS: usize = 80;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateEntry {
    id: String,
    name: String,
    created_at: u64,
}

fn index_path(directory: &Path) -> PathBuf {
    directory.join("templates.json")
}

fn valid_template_id(id: &str) -> bool {
    let Some((millis, slug)) = id.split_once('-') else { return false; };
    !millis.is_empty() && millis.chars().all(|value| value.is_ascii_digit())
        && !slug.is_empty() && slug.len() <= 90
        && slug.chars().all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
}

fn template_path(directory: &Path, id: &str) -> Option<PathBuf> {
    if !valid_template_id(id) { return None; }
    Some(directory.join(format!("{id}.docx")))
}

pub fn validate_template_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_CHARS || trimmed.ends_with('.')
        || trimmed.chars().any(|value| value.is_control() || "<>:\"/\\|?*".contains(value)) {
        return Err("Choose A Template Name Without Slashes, Reserved Characters, Or A Trailing Dot.".into());
    }
    Ok(trimmed.to_string())
}

fn slug(name: &str) -> String {
    let slug: String = name.chars().map(|value| if value.is_ascii_alphanumeric() { value.to_ascii_lowercase() } else { '-' }).collect();
    let slug = slug.trim_matches('-').chars().take(60).collect::<String>().trim_end_matches('-').to_string();
    if slug.is_empty() { "template".into() } else { slug }
}

fn read_index(directory: &Path) -> Vec<TemplateEntry> {
    let bytes = match fs::read(index_path(directory)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(_) => return Vec::new(),
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn write_index(directory: &Path, entries: &[TemplateEntry]) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("Could Not Create The Templates Folder. {error}"))?;
    let bytes = serde_json::to_vec(entries).map_err(|error| error.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|error| format!("Could Not Update Templates. {error}"))?;
    file.write_all(&bytes).and_then(|()| file.as_file().sync_all()).map_err(|error| error.to_string())?;
    file.persist(index_path(directory)).map_err(|error| format!("Could Not Save Templates. {error}"))?;
    Ok(())
}

fn prune(directory: &Path, entries: &mut Vec<TemplateEntry>) {
    entries.retain(|entry| template_path(directory, &entry.id).is_some_and(|path| path.is_file()));
    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    for entry in entries.iter().skip(MAX_TEMPLATES) {
        if let Some(path) = template_path(directory, &entry.id) { let _ = fs::remove_file(path); }
    }
    entries.truncate(MAX_TEMPLATES);
}

pub fn save(directory: &Path, name: &str, bytes: &[u8]) -> Result<TemplateInfo, String> {
    let name = validate_template_name(name)?;
    files::validate_docx(bytes)?;
    fs::create_dir_all(directory).map_err(|error| format!("Could Not Create The Templates Folder. {error}"))?;
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_millis() as u64;
    let mut sequence = 0u32;
    let (id, path) = loop {
        let candidate_id = format!("{millis}-{}-{}", slug(&name), sequence);
        let candidate = directory.join(format!("{candidate_id}.docx"));
        if !candidate.exists() && !read_index(directory).iter().any(|entry| entry.id == candidate_id) { break (candidate_id, candidate); }
        sequence += 1;
        if sequence > 100 { return Err("Could Not Create A Unique Template Name.".into()); }
    };
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(|error| format!("Could Not Create The Template. {error}"))?;
    temporary.write_all(bytes).and_then(|()| temporary.as_file().sync_all()).map_err(|error| format!("Could Not Write The Template. {error}"))?;
    temporary.persist(&path).map_err(|error| format!("Could Not Save The Template. {error}"))?;
    let mut entries = read_index(directory);
    entries.retain(|entry| entry.id != id);
    entries.insert(0, TemplateEntry { id: id.clone(), name: name.clone(), created_at: millis });
    prune(directory, &mut entries);
    write_index(directory, &entries)?;
    Ok(TemplateInfo { id, name, created_at: millis })
}

pub fn list(directory: &Path) -> Result<Vec<TemplateInfo>, String> {
    let mut entries = read_index(directory);
    prune(directory, &mut entries);
    write_index(directory, &entries)?;
    Ok(entries.into_iter().map(|entry| TemplateInfo { id: entry.id, name: entry.name, created_at: entry.created_at }).collect())
}

pub fn read(directory: &Path, id: &str) -> Result<Vec<u8>, String> {
    let Some(path) = template_path(directory, id) else { return Err("The Template Is Unavailable.".into()); };
    let bytes = fs::read(&path).map_err(|error| format!("Could Not Read The Template. {error}"))?;
    files::validate_docx(&bytes)?;
    Ok(bytes)
}

pub fn delete(directory: &Path, id: &str) -> Result<(), String> {
    if let Some(path) = template_path(directory, id) {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could Not Delete The Template. {error}")),
        }
    }
    let mut entries = read_index(directory);
    entries.retain(|entry| entry.id != id);
    write_index(directory, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_lists_reads_and_deletes_templates() {
        let directory = tempfile::tempdir().unwrap();
        let first = save(directory.path(), "Meeting Notes", b"PK\x03\x04One").unwrap();
        let second = save(directory.path(), "Resume", b"PK\x03\x04Two").unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(read(directory.path(), &first.id).unwrap(), b"PK\x03\x04One");
        let names: Vec<String> = list(directory.path()).unwrap().into_iter().map(|entry| entry.name).collect();
        assert_eq!(names, vec![second.name, first.name]);
        delete(directory.path(), &first.id).unwrap();
        assert!(read(directory.path(), &first.id).is_err());
        assert!(list(directory.path()).unwrap().len() == 1);
    }

    #[test]
    fn validates_names_and_rejects_bad_ids() {
        assert!(validate_template_name("  Report  ").is_ok());
        for name in ["", "  ", "Bad/Name", "CON.", &"x".repeat(100)] {
            assert!(validate_template_name(name).is_err());
        }
        assert!(template_path(Path::new("."), "../evil").is_none());
        assert!(template_path(Path::new("."), "123-slug-0").is_some());
    }
}
