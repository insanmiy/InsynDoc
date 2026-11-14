use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const RECENT_LIMIT: usize = 10;

pub fn list(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let path = directory.join("recent-documents.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read recent documents. {error}")),
    };
    let paths: Vec<PathBuf> = serde_json::from_slice(&bytes).unwrap_or_default();
    Ok(paths.into_iter().filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))).take(RECENT_LIMIT).collect())
}

fn write(directory: &Path, paths: &[PathBuf]) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("Could not create the preferences folder. {error}"))?;
    let bytes = serde_json::to_vec(paths).map_err(|error| format!("Could not prepare recent documents. {error}"))?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|error| format!("Could not update recent documents. {error}"))?;
    file.write_all(&bytes).and_then(|()| file.as_file().sync_all()).map_err(|error| format!("Could not write recent documents. {error}"))?;
    file.persist(directory.join("recent-documents.json")).map_err(|error| format!("Could not save recent documents. {error}"))?;
    Ok(())
}

pub fn remember(directory: &Path, path: &Path) -> Result<(), String> {
    let mut paths = list(directory)?;
    paths.retain(|previous| previous != path);
    paths.insert(0, path.to_path_buf());
    paths.truncate(RECENT_LIMIT);
    write(directory, &paths)
}

pub fn clear(directory: &Path) -> Result<(), String> {
    write(directory, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_ten_unique_documents_in_recent_order() {
        let directory = tempfile::tempdir().unwrap();
        for index in 0..12 {
            remember(directory.path(), &PathBuf::from(format!("Document {index}.docx"))).unwrap();
        }
        remember(directory.path(), Path::new("Document 7.docx")).unwrap();
        let paths = list(directory.path()).unwrap();
        assert_eq!(paths.len(), 10);
        assert_eq!(paths[0], PathBuf::from("Document 7.docx"));
        assert_eq!(paths.iter().filter(|path| path.as_path() == Path::new("Document 7.docx")).count(), 1);
    }

    #[test]
    fn handles_missing_and_damaged_history_and_can_clear_it() {
        let directory = tempfile::tempdir().unwrap();
        assert!(list(directory.path()).unwrap().is_empty());
        fs::write(directory.path().join("recent-documents.json"), "Invalid JSON").unwrap();
        assert!(list(directory.path()).unwrap().is_empty());
        remember(directory.path(), Path::new("My Document.docx")).unwrap();
        clear(directory.path()).unwrap();
        assert!(list(directory.path()).unwrap().is_empty());
    }
}
