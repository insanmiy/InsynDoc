use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::files;

pub const MAX_SNAPSHOTS_PER_DOCUMENT: usize = 20;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: String,
    pub created_at: u64,
}

fn key_hash(key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn snapshot_key(sessions: &crate::sessions::FileSessions, token: Option<&str>, name: &str) -> Result<String, String> {
    let key = match token.filter(|value| !value.is_empty()) {
        Some(token) => sessions.resolve(token)?.to_string_lossy().into_owned(),
        None => format!("untitled:{name}"),
    };
    Ok(key_hash(&key))
}

fn snapshot_directory(root: &Path, key: &str) -> PathBuf {
    root.join("Snapshots").join(key)
}

fn snapshot_id(path: &Path) -> Option<String> {
    path.file_stem()?.to_str().map(|stem| stem.to_string())
}

fn snapshot_created(id: &str) -> u64 {
    id.split('-').next().and_then(|value| value.parse().ok()).unwrap_or(0)
}

fn valid_snapshot_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|value| value.is_ascii_digit() || value == '-')
}

fn snapshot_path(directory: &Path, id: &str) -> Option<PathBuf> {
    if !valid_snapshot_id(id) { return None; }
    Some(directory.join(format!("{id}.docx")))
}

pub fn write(root: &Path, key: &str, bytes: &[u8]) -> Result<SnapshotInfo, String> {
    files::validate_docx(bytes)?;
    let directory = snapshot_directory(root, key);
    fs::create_dir_all(&directory).map_err(|error| format!("Could not create the snapshot folder. {error}"))?;
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_millis() as u64;
    let mut sequence = 0u32;
    let path = loop {
        let id = format!("{millis}-{sequence}");
        let candidate = directory.join(format!("{id}.docx"));
        if !candidate.exists() { break candidate; }
        sequence += 1;
        if sequence > 1000 { return Err("Could not create a unique snapshot name.".into()); }
    };
    let id = snapshot_id(&path).ok_or("Could not name the snapshot.".to_string())?;
    let created_at = snapshot_created(&id);
    let mut temporary = tempfile::NamedTempFile::new_in(&directory).map_err(|error| format!("Could not create the snapshot. {error}"))?;
    temporary.write_all(bytes).and_then(|()| temporary.as_file().sync_all()).map_err(|error| format!("Could not write the snapshot. {error}"))?;
    temporary.persist(&path).map_err(|error| format!("Could not save the snapshot. {error}"))?;
    prune(&directory);
    Ok(SnapshotInfo { id, created_at })
}

fn prune(directory: &Path) {
    let mut entries: Vec<(u64, PathBuf)> = fs::read_dir(directory).map(|entries| entries.filter_map(|entry| entry.ok()).filter_map(|entry| {
        let path = entry.path();
        if path.extension().is_none_or(|extension| !extension.eq_ignore_ascii_case("docx")) { return None; }
        let id = snapshot_id(&path)?;
        Some((snapshot_created(&id), path))
    }).collect()).unwrap_or_default();
    entries.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, path) in entries.into_iter().skip(MAX_SNAPSHOTS_PER_DOCUMENT) {
        let _ = fs::remove_file(path);
    }
}

pub fn list(root: &Path, key: &str) -> Result<Vec<SnapshotInfo>, String> {
    let directory = snapshot_directory(root, key);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read snapshots. {error}")),
    };
    let mut snapshots: Vec<SnapshotInfo> = entries.filter_map(|entry| entry.ok()).filter_map(|entry| {
        let path = entry.path();
        if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("docx")) { return None; }
        let id = snapshot_id(&path)?;
        Some(SnapshotInfo { created_at: snapshot_created(&id), id })
    }).collect();
    snapshots.sort_by(|left, right| right.id.cmp(&left.id));
    snapshots.truncate(MAX_SNAPSHOTS_PER_DOCUMENT);
    Ok(snapshots)
}

pub fn latest(root: &Path, key: &str) -> Result<Option<SnapshotInfo>, String> {
    Ok(list(root, key)?.into_iter().next())
}

pub fn read(root: &Path, key: &str, id: &str) -> Result<Vec<u8>, String> {
    let Some(path) = snapshot_path(&snapshot_directory(root, key), id) else {
        return Err("The snapshot is unavailable.".into());
    };
    let bytes = fs::read(&path).map_err(|error| format!("Could not read the snapshot. {error}"))?;
    files::validate_docx(&bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_lists_reads_and_prunes_snapshots() {
        let root = tempfile::tempdir().unwrap();
        let first = write(root.path(), "key", b"PK\x03\x04One").unwrap();
        let second = write(root.path(), "key", b"PK\x03\x04Two").unwrap();
        assert!(second.created_at >= first.created_at);
        assert_eq!(read(root.path(), "key", &first.id).unwrap(), b"PK\x03\x04One");
        assert_eq!(latest(root.path(), "key").unwrap().unwrap().id, second.id);
        for index in 0..25 {
            write(root.path(), "key", format!("PK\x03\x04{index}").as_bytes()).unwrap();
        }
        let snapshots = list(root.path(), "key").unwrap();
        assert_eq!(snapshots.len(), MAX_SNAPSHOTS_PER_DOCUMENT);
        assert!(read(root.path(), "key", &snapshots[0].id).is_ok());
        assert!(read(root.path(), "key", "../../etc/passwd").is_err());
        assert!(read(root.path(), "key", "missing").is_err());
        assert!(list(root.path(), "absent").unwrap().is_empty());
    }

    #[test]
    fn snapshot_keys_separate_documents() {
        let sessions = crate::sessions::FileSessions::default();
        let token = sessions.register(PathBuf::from("A.docx")).unwrap();
        let file_key = snapshot_key(&sessions, Some(&token), "A.docx").unwrap();
        let untitled = snapshot_key(&sessions, None, "Untitled.docx").unwrap();
        assert_ne!(file_key, untitled);
        assert_eq!(snapshot_key(&sessions, Some(""), "Untitled.docx").unwrap(), untitled);
        assert!(snapshot_key(&sessions, Some("expired"), "A.docx").is_err());
    }
}
