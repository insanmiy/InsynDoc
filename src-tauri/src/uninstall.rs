use std::env;
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::null;

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        window: *mut c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}

fn uninstaller_path(executable: &Path) -> Result<PathBuf, String> {
    // The NSIS installer writes uninstall.exe beside the installed app.
    let path = executable.with_file_name("uninstall.exe");
    if !path.is_file() {
        return Err("The InsynDoc uninstaller was not found. If InsynDoc is installed, try uninstalling it from Windows Settings > Apps > Installed apps.".into());
    }
    Ok(path)
}

pub fn open() -> Result<(), String> {
    let executable = env::current_exe()
        .map_err(|error| format!("Could not find the InsynDoc installation. {error}"))?;
    let path = uninstaller_path(&executable)?;
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // ShellExecute handles the uninstaller's Windows elevation prompt.
    let result = unsafe { ShellExecuteW(std::ptr::null_mut(), operation.as_ptr(), file.as_ptr(), null(), null(), 1) };
    if result <= 32 {
        return Err("Could not open the InsynDoc uninstaller. Try uninstalling InsynDoc from Windows Settings > Apps > Installed apps.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_only_a_file_beside_the_running_executable() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("insyndoc.exe");
        let uninstaller = directory.path().join("uninstall.exe");
        assert!(uninstaller_path(&executable).is_err());

        std::fs::create_dir(&uninstaller).unwrap();
        assert!(uninstaller_path(&executable).is_err());
        std::fs::remove_dir(&uninstaller).unwrap();

        std::fs::write(&uninstaller, []).unwrap();
        assert_eq!(uninstaller_path(&executable).unwrap(), uninstaller);
    }
}
