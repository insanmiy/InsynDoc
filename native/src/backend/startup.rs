use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct StartupDocument {
    pending: Mutex<Option<Result<PathBuf, String>>>,
}

impl StartupDocument {
    pub fn from_args(arguments: impl IntoIterator<Item = OsString>) -> Self {
        let mut arguments = arguments.into_iter().skip(1);
        let pending = arguments.next().map(|argument| {
            if arguments.next().is_some() {
                return Err("Open One DOCX File Per Window.".into());
            }
            let path = PathBuf::from(argument);
            if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("docx")) {
                return Err("Choose A File With The DOCX Extension.".into());
            }
            Ok(path)
        });
        Self { pending: Mutex::new(pending) }
    }

    pub fn take(&self) -> Result<Option<PathBuf>, String> {
        self.pending.lock().map_err(|_| "The Startup Document Is Unavailable. Reopen The App.".to_string())?
            .take().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn opens_a_unicode_path_with_spaces_once_without_splitting_it() {
        let path = PathBuf::from("C:/Users/Writer/Downloads/Résumé Final.DOCX");
        let startup = StartupDocument::from_args([OsString::from("insyndoc.exe"), path.clone().into_os_string()]);
        assert_eq!(startup.take().unwrap(), Some(path));
        assert_eq!(startup.take().unwrap(), None);
    }

    #[test]
    fn starts_blank_without_a_document_argument() {
        let startup = StartupDocument::from_args([OsString::from("insyndoc.exe")]);
        assert_eq!(startup.take().unwrap(), None);
    }

    #[test]
    fn rejects_non_docx_and_multiple_document_arguments_once() {
        for arguments in [
            vec!["insyndoc.exe", "Document.txt"],
            vec!["insyndoc.exe", "One.docx", "Two.docx"],
            vec!["insyndoc.exe", "--unexpected"]
        ] {
            let startup = StartupDocument::from_args(arguments.into_iter().map(OsString::from));
            assert!(startup.take().is_err());
            assert_eq!(startup.take().unwrap(), None);
        }
    }

    #[test]
    fn validates_downloaded_documents_and_preserves_the_original_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Downloaded Report.docx");
        let package = b"PK\x03\x04Example Package";
        std::fs::write(&path, package).unwrap();
        let startup = StartupDocument::from_args([OsString::from("insyndoc.exe"), path.clone().into_os_string()]);
        let selected = startup.take().unwrap().unwrap();
        assert_eq!(crate::backend::files::read_document(&selected).unwrap(), package);
        assert_eq!(std::fs::read(&path).unwrap(), package);
        std::fs::write(&path, b"Not A DOCX").unwrap();
        assert!(crate::backend::files::read_document(&path).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(crate::backend::files::read_document(&path).is_err());
    }
}
