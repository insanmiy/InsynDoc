use serde::Deserialize;
use std::io::Write;
use std::path::Path;

pub const MAX_DOWNLOAD_BYTES: usize = 100 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadFormat {
    Docx, Pdf, Odt, Txt, Rtf, Html, Epub, Md,
}

impl DownloadFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Docx => "docx", Self::Pdf => "pdf", Self::Odt => "odt", Self::Txt => "txt",
            Self::Rtf => "rtf", Self::Html => "zip", Self::Epub => "epub", Self::Md => "md",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Docx => "Microsoft Word", Self::Pdf => "PDF document", Self::Odt => "OpenDocument Text",
            Self::Txt => "Plain text", Self::Rtf => "Rich text", Self::Html => "Zipped web page",
            Self::Epub => "EPUB publication", Self::Md => "Markdown",
        }
    }

    pub fn validate_destination(self, path: &Path) -> Result<(), String> {
        if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(self.extension())) {
            return Err(format!("Use the {} extension for this download.", self.extension().to_ascii_uppercase()));
        }
        Ok(())
    }

    fn validate_bytes(self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > MAX_DOWNLOAD_BYTES { return Err("The download exceeds the 100 MB limit.".into()); }
        let valid = match self {
            Self::Docx | Self::Odt | Self::Html | Self::Epub => bytes.starts_with(b"PK\x03\x04"),
            Self::Pdf => bytes.starts_with(b"%PDF-"),
            Self::Rtf => bytes.starts_with(br"{\rtf"),
            Self::Txt | Self::Md => std::str::from_utf8(bytes).is_ok(),
        };
        if !valid { return Err("The download content does not match its file format.".into()); }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadMetadata {
    pub format: DownloadFormat,
    pub name: String,
}

impl DownloadMetadata {
    pub fn suggested_name(&self) -> Result<String, String> {
        let name = &self.name;
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if stem.is_empty() || name.encode_utf16().count() > 180 || name.trim() != name || name.ends_with('.') || reserved
            || name.chars().any(|value| value.is_control() || "<>:\"/\\|?*".contains(value)) {
            return Err("Choose a download name without reserved characters.".into());
        }
        let stem = Path::new(name).file_stem().and_then(|value| value.to_str()).unwrap_or("Untitled");
        Ok(format!("{stem}.{}", self.format.extension()))
    }
}

pub fn read_request(packet: &[u8]) -> Result<(DownloadMetadata, &[u8]), String> {
    let prefix: [u8; 4] = packet.get(..4).ok_or("Invalid download transfer.")?.try_into().map_err(|_| "Invalid download transfer.")?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_METADATA_BYTES || length > packet.len() - 4 { return Err("Invalid download metadata.".into()); }
    let metadata: DownloadMetadata = serde_json::from_slice(&packet[4..4 + length]).map_err(|error| format!("Invalid download metadata. {error}"))?;
    metadata.suggested_name()?;
    let bytes = &packet[4 + length..];
    metadata.format.validate_bytes(bytes)?;
    Ok((metadata, bytes))
}

pub fn write_download(path: &Path, format: DownloadFormat, bytes: &[u8]) -> Result<(), String> {
    format.validate_destination(path)?;
    format.validate_bytes(bytes)?;
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Could not create a temporary download. Check folder access. {error}"))?;
    temporary.write_all(bytes).and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| format!("Could not write the download. The original is unchanged. {error}"))?;
    temporary.persist(path).map_err(|error| format!("Could not replace the download. Close other apps using it and retry. {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn packet(metadata: &[u8], bytes: &[u8]) -> Vec<u8> {
        let mut packet = (metadata.len() as u32).to_le_bytes().to_vec();
        packet.extend_from_slice(metadata);
        packet.extend_from_slice(bytes);
        packet
    }

    #[test]
    fn reads_all_download_formats_and_uses_their_real_extensions() {
        for (format, extension, bytes) in [
            ("docx", "docx", b"PK\x03\x04Document".as_slice()),
            ("pdf", "pdf", b"%PDF-1.7\nDocument".as_slice()),
            ("odt", "odt", b"PK\x03\x04Document".as_slice()),
            ("txt", "txt", "Hello R\u{e9}sum\u{e9}".as_bytes()),
            ("rtf", "rtf", br"{\rtf1 Document}".as_slice()),
            ("html", "zip", b"PK\x03\x04Document".as_slice()),
            ("epub", "epub", b"PK\x03\x04Document".as_slice()),
            ("md", "md", b"# Document".as_slice())
        ] {
            let metadata = serde_json::to_vec(&serde_json::json!({ "format": format, "name": "My Report.docx" })).unwrap();
            let transfer = packet(&metadata, bytes);
            let (download, content) = read_request(&transfer).unwrap();
            assert_eq!(download.suggested_name().unwrap(), format!("My Report.{extension}"));
            assert_eq!(content, bytes);
            assert!(download.format.validate_destination(Path::new(&format!("Report.{extension}"))).is_ok());
            assert!(download.format.validate_destination(Path::new("Report.exe")).is_err());
        }
    }

    #[test]
    fn rejects_malformed_packets_unknown_formats_and_unsafe_names() {
        for transfer in [vec![], vec![0; 3], u32::MAX.to_le_bytes().to_vec(), packet(b"{", b"Text"),
            packet(br#"{"format":"exe","name":"Report.docx"}"#, b"Text"),
            packet(br#"{"format":"txt","name":"../Report.docx"}"#, b"Text"),
            packet(br#"{"format":"txt","name":"CON.docx"}"#, b"Text"),
            packet(br#"{"format":"txt","name":"Report.docx","token":"7"}"#, b"Text"),
            packet(br#"{"format":"txt","name":"Report.docx"}"#, &[255]),
            packet(br#"{"format":"pdf","name":"Report.docx"}"#, b"Not A PDF")
        ] {
            assert!(read_request(&transfer).is_err());
        }
        let metadata = br#"{"format":"txt","name":"Report.docx"}"#;
        assert!(read_request(&packet(metadata, &vec![0; MAX_DOWNLOAD_BYTES + 1])).is_err());
        let mut oversized_metadata = metadata.to_vec();
        oversized_metadata.resize(MAX_METADATA_BYTES + 1, b' ');
        assert!(read_request(&packet(&oversized_metadata, b"Text")).is_err());
    }

    #[test]
    fn writes_copies_atomically_without_damaging_the_original_on_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Report.txt");
        fs::write(&path, "Original").unwrap();
        write_download(&path, DownloadFormat::Txt, b"Replacement").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"Replacement");
        assert!(write_download(&path, DownloadFormat::Pdf, b"Not A PDF").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"Replacement");
        let blocked = directory.path().join("Blocked.txt");
        fs::create_dir(&blocked).unwrap();
        assert!(write_download(&blocked, DownloadFormat::Txt, b"Text").is_err());
        assert!(blocked.is_dir());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
