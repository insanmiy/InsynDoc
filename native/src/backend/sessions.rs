use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct FileSessions {
    state: Mutex<SessionPaths>,
    next_id: AtomicU64,
}

#[derive(Default)]
struct SessionPaths {
    paths: HashMap<String, PathBuf>,
    active: Option<String>,
    pending: Option<String>,
    exported: Option<String>,
    recent: HashSet<String>,
}

impl SessionPaths {
    fn prune(&mut self) {
        self.paths.retain(|token, _| self.active.as_ref() == Some(token)
            || self.pending.as_ref() == Some(token) || self.exported.as_ref() == Some(token)
            || self.recent.contains(token));
    }
}

impl FileSessions {
    fn token(&self, state: &mut SessionPaths, path: PathBuf) -> String {
        if let Some((token, _)) = state.paths.iter().find(|(_, existing)| *existing == &path) {
            return token.clone();
        }
        let token = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        state.paths.insert(token.clone(), path);
        token
    }

    pub fn register(&self, path: PathBuf) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|_| "File Session Is Unavailable. Restart The App.".to_string())?;
        let token = self.token(&mut state, path);
        state.pending = Some(token.clone());
        state.prune();
        Ok(token)
    }

    pub fn register_export(&self, path: PathBuf) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|_| "File Session Is Unavailable. Restart The App.".to_string())?;
        let token = self.token(&mut state, path);
        state.exported = Some(token.clone());
        state.prune();
        Ok(token)
    }

    pub fn activate(&self, token: &str) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "File Session Is Unavailable. Restart The App.".to_string())?;
        if !state.paths.contains_key(token) { return Err("The File Session Has Expired. Reopen The Document.".into()); }
        state.active = Some(token.to_string());
        state.pending = None;
        state.prune();
        Ok(())
    }

    pub fn sync_recent(&self, paths: &[PathBuf]) -> Result<Vec<String>, String> {
        let mut state = self.state.lock().map_err(|_| "File Session Is Unavailable. Restart The App.".to_string())?;
        let tokens: Vec<String> = paths.iter().take(10).map(|path| self.token(&mut state, path.clone())).collect();
        state.recent = tokens.iter().cloned().collect();
        state.prune();
        Ok(tokens)
    }

    pub fn rename(&self, token: &str, path: PathBuf) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "File Session Is Unavailable. Restart The App.".to_string())?;
        let previous = state.paths.get_mut(token).ok_or_else(|| "The File Session Has Expired. Reopen The Document.".to_string())?;
        *previous = path;
        Ok(())
    }

    pub fn resolve(&self, token: &str) -> Result<PathBuf, String> {
        self.state.lock().map_err(|_| "File Session Is Unavailable. Restart The App.".to_string())?
            .paths.get(token).cloned().ok_or_else(|| "The File Session Has Expired. Use Save As.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_registered_file_tokens_resolve() {
        let sessions = FileSessions::default();
        assert!(sessions.resolve("C:/Unexpected.docx").is_err());
        let token = sessions.register(PathBuf::from("Document.docx")).unwrap();
        assert_eq!(sessions.resolve(&token).unwrap(), PathBuf::from("Document.docx"));
    }

    #[test]
    fn abandoned_opens_do_not_retain_unbounded_tokens() {
        let sessions = FileSessions::default();
        let first = sessions.register(PathBuf::from("First.docx")).unwrap();
        for index in 0..100 {
            sessions.register(PathBuf::from(format!("Document {index}.docx"))).unwrap();
        }
        assert!(sessions.resolve(&first).is_err());
    }

    #[test]
    fn recent_rotation_and_exports_preserve_current_access_with_bounded_storage() {
        let sessions = FileSessions::default();
        let current_path = PathBuf::from("Current.docx");
        let current = sessions.register(current_path.clone()).unwrap();
        sessions.activate(&current).unwrap();
        let mut previous_recent: Vec<String> = Vec::new();
        for index in 0..30 {
            let paths: Vec<_> = (index..index + 10).map(|number| PathBuf::from(format!("Recent {number}.docx"))).collect();
            let tokens = sessions.sync_recent(&paths).unwrap();
            assert_eq!(sessions.sync_recent(&paths).unwrap(), tokens);
            for (token, path) in tokens.iter().zip(&paths) {
                assert_eq!(sessions.resolve(token).unwrap(), *path);
            }
            if let Some(token) = previous_recent.first() { assert!(sessions.resolve(token).is_err()); }
            previous_recent = tokens;
            let exported = sessions.register_export(PathBuf::from(format!("Export {index}.docx"))).unwrap();
            assert!(sessions.resolve(&exported).is_ok());
            sessions.register(PathBuf::from(format!("Rejected Open {index}.docx"))).unwrap();
            assert_eq!(sessions.resolve(&current).unwrap(), current_path);
            assert!(sessions.state.lock().unwrap().paths.len() <= 13);
        }
        sessions.sync_recent(&[]).unwrap();
        assert!(previous_recent.iter().all(|token| sessions.resolve(token).is_err()));
        assert_eq!(sessions.resolve(&current).unwrap(), current_path);
        assert!(sessions.state.lock().unwrap().paths.len() <= 3);
    }

    #[test]
    fn repeated_renames_keep_one_token_and_release_previous_active_files() {
        let sessions = FileSessions::default();
        let token = sessions.register(PathBuf::from("Original.docx")).unwrap();
        sessions.activate(&token).unwrap();
        for index in 0..100 {
            let path = PathBuf::from(format!("Renamed {index}.docx"));
            sessions.rename(&token, path.clone()).unwrap();
            assert_eq!(sessions.resolve(&token).unwrap(), path);
            assert_eq!(sessions.register(path).unwrap(), token);
            assert_eq!(sessions.state.lock().unwrap().paths.len(), 1);
        }
        let replacement = sessions.register(PathBuf::from("Replacement.docx")).unwrap();
        assert!(sessions.resolve(&token).is_ok());
        sessions.activate(&replacement).unwrap();
        assert!(sessions.resolve(&token).is_err());
    }
}
