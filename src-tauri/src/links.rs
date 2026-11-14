use std::path::Path;
use std::process::{Command, Stdio};

pub fn validate_url(value: &str) -> Result<String, String> {
    if value.len() > 8192 || value.chars().any(char::is_control) {
        return Err("This link is invalid or too long.".into());
    }
    let url = url::Url::parse(value).map_err(|_| "This link is not a valid URL.".to_string())?;
    match url.scheme() {
        "http" | "https" if url.host_str().is_some() => Ok(url.to_string()),
        "mailto" if !url.path().is_empty() => Ok(url.to_string()),
        _ => Err("Only HTTP, HTTPS, and email links can be opened.".into()),
    }
}

pub fn validate_browser(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Choose an installed browser using its full path.".into());
    }
    #[cfg(target_os = "macos")]
    if path.is_dir() && path.extension().is_some_and(|extension| extension == "app") {
        return Ok(());
    }
    if !path.is_file() {
        return Err("The selected browser is unavailable. Choose it again in settings.".into());
    }
    #[cfg(target_os = "windows")]
    if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe")) {
        return Err("Choose a browser application with the EXE extension.".into());
    }
    Ok(())
}

fn spawn_browser(command: &mut Command) -> Result<(), String> {
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|error| format!("Could not open the browser. Check your link browser setting. {error}"))?;
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}

pub fn open(url: &str, browser_path: Option<&str>) -> Result<(), String> {
    let url = validate_url(url)?;
    if let Some(browser_path) = browser_path {
        let path = Path::new(browser_path);
        validate_browser(path)?;
        #[cfg(target_os = "macos")]
        return spawn_browser(Command::new("/usr/bin/open").arg("-a").arg(path).arg(&url));
        #[cfg(not(target_os = "macos"))]
        return spawn_browser(Command::new(path).arg(&url));
    }
    open_default(&url)
}

#[cfg(target_os = "windows")]
fn open_default(url: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(window: *mut std::ffi::c_void, operation: *const u16, file: *const u16,
            parameters: *const u16, directory: *const u16, show: i32) -> isize;
    }
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = OsStr::new(url).encode_wide().chain(Some(0)).collect();
    // The URL Is Validated, And Both Strings Remain Null Terminated For This Call.
    let result = unsafe { ShellExecuteW(std::ptr::null_mut(), operation.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), 1) };
    if result <= 32 {
        return Err("Could not open the system default browser. Check your default apps or choose a browser in settings.".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_default(url: &str) -> Result<(), String> {
    spawn_browser(Command::new("/usr/bin/open").arg(url))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_default(url: &str) -> Result<(), String> {
    spawn_browser(Command::new("xdg-open").arg(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_web_links_without_interpreting_shell_characters() {
        for url in ["https://example.com/?a=1&b=2", "http://example.com/", "mailto:hello@example.com"] {
            assert_eq!(validate_url(url).unwrap(), url);
        }
    }

    #[test]
    fn refuses_unsafe_schemes_and_malformed_links() {
        for url in ["javascript:alert(1)", "file:///C:/Windows/notepad.exe", "data:text/html,hello", "example.com", "https://", "https://example.com/\n", "mailto:"] {
            assert!(validate_url(url).is_err(), "Unexpectedly Accepted {url}");
        }
        assert!(validate_url(&format!("https://example.com/{}", "a".repeat(8192))).is_err());
    }

    #[test]
    fn refuses_missing_and_relative_browser_paths() {
        assert!(validate_browser(Path::new("browser.exe")).is_err());
        let directory = tempfile::tempdir().unwrap();
        assert!(validate_browser(&directory.path().join("missing.exe")).is_err());
        assert!(validate_browser(directory.path()).is_err());
        let executable = directory.path().join("Browser With Spaces.exe");
        std::fs::write(&executable, []).unwrap();
        assert!(validate_browser(&executable).is_ok());
    }
}
