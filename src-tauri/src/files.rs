use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const MAX_DOCUMENT_BYTES: usize = 25 * 1024 * 1024;

pub fn validate_docx(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("The document exceeds the 25 MB limit.".into());
    }

    if !bytes.starts_with(b"PK\x03\x04") {
        return Err("This file is not a DOCX package. Choose a valid DOCX file.".into());
    }

    Ok(())
}

pub fn read_document(path: &Path) -> Result<Vec<u8>, String> {
    let extension = path.extension().and_then(|value| value.to_str());
    if !extension.is_some_and(|value| value.eq_ignore_ascii_case("docx")) {
        return Err("Choose a file with the DOCX extension.".into());
    }

    let file = File::open(path)
        .map_err(|error| format!("Could not open the document. Check file access. {error}"))?;
    let mut bytes = Vec::new();
    file.take((MAX_DOCUMENT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read the document. {error}"))?;
    validate_docx(&bytes)?;
    Ok(bytes)
}

pub const IMPORT_EXTENSIONS: &[&str] = &["md", "markdown", "html", "htm", "txt", "odt"];

pub fn import_extension(path: &Path) -> Option<String> {
    path.extension().and_then(|value| value.to_str())
        .filter(|value| IMPORT_EXTENSIONS.iter().any(|extension| value.eq_ignore_ascii_case(extension)))
        .map(|value| value.to_ascii_lowercase())
}

pub fn read_import(path: &Path) -> Result<(Vec<u8>, String), String> {
    let Some(extension) = import_extension(path) else {
        return Err("Choose a markdown, HTML, text, or ODT file.".into());
    };
    let file = File::open(path)
        .map_err(|error| format!("Could not open the document. Check file access. {error}"))?;
    let mut bytes = Vec::new();
    file.take((MAX_DOCUMENT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read the document. {error}"))?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("The document exceeds the 25 MB limit.".into());
    }
    Ok((bytes, extension))
}

pub fn write_document(path: &Path, bytes: &[u8]) -> Result<(), String> {
    validate_docx(bytes)?;
    write_bytes(path, bytes)
}

pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    write_bytes(path, text.as_bytes())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Could not create a temporary file. Check folder access. {error}"))?;
    temporary.write_all(bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| format!("Could not write the document. The original is unchanged. {error}"))?;
    temporary.persist(path)
        .map_err(|error| format!("Could not replace the document. Close other apps using it and retry. {error}"))?;
    Ok(())
}

pub fn document_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled.docx".into())
}

fn validate_document_name(name: &str) -> Result<(), String> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    if stem.is_empty() || name.encode_utf16().count() > 180 || name.trim() != name || name.ends_with('.') || reserved
        || name.chars().any(|value| value.is_control() || "<>:\"/\\|?*".contains(value))
        || !name.to_ascii_lowercase().ends_with(".docx") {
        return Err("Choose a valid DOCX filename without reserved characters.".into());
    }
    Ok(())
}

pub fn save_new_document(directory: &Path, name: &str, bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    validate_document_name(name)?;
    validate_docx(bytes)?;
    std::fs::create_dir_all(directory).map_err(|error| format!("Could not create the documents folder. {error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(|error| format!("Could not create the document. {error}"))?;
    temporary.write_all(bytes).and_then(|()| temporary.as_file().sync_all()).map_err(|error| format!("Could not save the document. {error}"))?;
    let stem = Path::new(name).file_stem().and_then(|value| value.to_str()).unwrap_or("Untitled");
    for index in 0..10000 {
        let filename = if index == 0 { name.to_string() } else { format!("{stem} ({index}).docx") };
        let path = directory.join(filename);
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => temporary = error.file,
            Err(error) => return Err(format!("Could not save the document. {error}")),
        }
    }
    Err("Too many documents share this name. Rename the document and try again.".into())
}

pub fn rename_document(path: &Path, name: &str) -> Result<std::path::PathBuf, String> {
    validate_document_name(name)?;
    let destination = path.with_file_name(name);
    if destination == path { return Ok(destination); }
    let same_name_case = cfg!(windows) && path.file_name().and_then(|value| value.to_str()).is_some_and(|value| value.eq_ignore_ascii_case(name));
    if destination.try_exists().map_err(|error| format!("Could not check the filename. {error}"))? && !same_name_case {
        return Err("A document with that name already exists. Choose another name.".into());
    }
    std::fs::rename(path, &destination).map_err(|error| format!("Could not rename the document. Close other apps using it and retry. {error}"))?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const PACKAGE: &[u8] = b"PK\x03\x04Test Package";

    #[test]
    fn saves_new_documents_without_overwriting_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        let first = save_new_document(directory.path(), "Notes.docx", PACKAGE).unwrap();
        let second = save_new_document(directory.path(), "Notes.docx", b"PK\x03\x04Second").unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(first).unwrap(), PACKAGE);
        assert_eq!(fs::read(second).unwrap(), b"PK\x03\x04Second");
        assert!(save_new_document(directory.path(), "../Bad.docx", PACKAGE).is_err());
    }

    #[test]
    fn renames_without_overwriting_and_rejects_invalid_names() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("Original.docx");
        fs::write(&original, PACKAGE).unwrap();
        for name in ["../Other.docx", "CON.docx", "Bad?.docx", "Other.txt", ""] {
            assert!(rename_document(&original, name).is_err());
        }
        let existing = directory.path().join("Existing.docx");
        fs::write(&existing, b"Keep This").unwrap();
        assert!(rename_document(&original, "Existing.docx").is_err());
        assert_eq!(fs::read(existing).unwrap(), b"Keep This");
        let renamed = rename_document(&original, "Renamed.docx").unwrap();
        assert!(!original.exists());
        assert_eq!(fs::read(renamed).unwrap(), PACKAGE);
    }

    #[test]
    fn validates_filename_boundaries() {
        assert!(validate_document_name("Report.final.docx").is_ok());
        assert!(validate_document_name(".docx").is_err());
        assert!(validate_document_name("CON.extra.docx").is_err());
        assert!(validate_document_name(&format!("{}a.docx", "\u{1f600}".repeat(87))).is_ok());
        assert!(validate_document_name(&format!("{}.docx", "\u{1f600}".repeat(88))).is_err());
    }

    #[test]
    fn changes_filename_capitalization() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("Report.docx");
        fs::write(&original, PACKAGE).unwrap();
        let renamed = rename_document(&original, "REPORT.docx").unwrap();
        assert_eq!(document_name(&renamed), "REPORT.docx");
        assert_eq!(fs::read(renamed).unwrap(), PACKAGE);
    }

    #[test]
    fn replaces_existing_file_and_cleans_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Document.docx");
        fs::write(&path, b"Original").unwrap();
        write_document(&path, PACKAGE).unwrap();
        assert_eq!(read_document(&path).unwrap(), PACKAGE);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn invalid_save_preserves_original() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Document.docx");
        fs::write(&path, PACKAGE).unwrap();
        assert!(write_document(&path, b"Not A Package").is_err());
        assert_eq!(fs::read(path).unwrap(), PACKAGE);
    }

    #[test]
    fn failed_replace_preserves_existing_directory() {
        let directory = tempfile::tempdir().unwrap();
        assert!(write_document(directory.path(), PACKAGE).is_err());
        assert!(directory.path().is_dir());
    }

    #[test]
    fn rejects_oversized_missing_and_non_docx_files() {
        assert!(validate_docx(&vec![0; MAX_DOCUMENT_BYTES + 1]).is_err());
        assert!(read_document(Path::new("Missing.docx")).is_err());
        assert!(read_document(Path::new("Document.txt")).is_err());
    }

    #[test]
    fn preserves_the_original_filename_case() {
        assert_eq!(document_name(Path::new("Folder/Mixed CASE.docx")), "Mixed CASE.docx");
    }

    #[test]
    fn exports_unicode_plain_text_without_changing_line_breaks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Export.txt");
        let text = "Café And 你好\nSecond Line";
        write_text(&path, text).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}
