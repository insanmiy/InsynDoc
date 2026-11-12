use crate::{downloads, files, fonts, links, pdf, recent, sessions::FileSessions, snapshots, startup::StartupDocument, templates};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{Manager, State};
use std::sync::Mutex;
use crate::statistics::{count_text, TextStatistics};

#[tauri::command]
async fn document_statistics(text: String) -> Result<TextStatistics, String> {
    tauri::async_runtime::spawn_blocking(move || count_text(&text))
        .await.map_err(|error| format!("Could Not Count Document Text. {error}"))
}

#[tauri::command]
async fn open_external_link(url: String, browser_path: Option<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || links::open(&url, browser_path.as_deref()))
        .await.map_err(|error| format!("Could Not Open The Browser. {error}"))?
}

#[tauri::command]
async fn choose_link_browser() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dialog = rfd::FileDialog::new().set_title("Choose A Browser Application");
        #[cfg(target_os = "windows")]
        let dialog = dialog.add_filter("Browser Application", &["exe"]);
        let Some(path) = dialog.pick_file() else { return Ok(None) };
        links::validate_browser(&path)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }).await.map_err(|error| format!("Could Not Choose A Browser. {error}"))?
}

#[derive(Default)]
struct RecentAccess(Mutex<()>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentDocument {
    token: String,
    name: String,
    path: String,
    available: bool,
}

#[tauri::command]
async fn recent_documents(app: tauri::AppHandle) -> Result<Vec<RecentDocument>, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let access = app.state::<RecentAccess>();
        let _guard = access.0.lock().map_err(|_| "Recent Documents Are Unavailable.".to_string())?;
        let paths = recent::list(&directory)?;
        let tokens = app.state::<FileSessions>().sync_recent(&paths)?;
        paths.into_iter().zip(tokens).map(|(path, token)| {
            let name = files::document_name(&path);
            let available = path.is_file();
            let display_path = path.to_string_lossy().into_owned();
            Ok(RecentDocument { token, name, path: display_path, available })
        }).collect()
    }).await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn remember_document(app: tauri::AppHandle, token: String, sessions: State<'_, FileSessions>) -> Result<(), String> {
    let path = sessions.resolve(&token)?;
    sessions.activate(&token)?;
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let access = app.state::<RecentAccess>();
        let _guard = access.0.lock().map_err(|_| "Recent Documents Are Unavailable.".to_string())?;
        recent::remember(&directory, &path)?;
        app.state::<FileSessions>().sync_recent(&recent::list(&directory)?)?;
        Ok(())
    }).await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn clear_recent_documents(app: tauri::AppHandle) -> Result<(), String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let access = app.state::<RecentAccess>();
        let _guard = access.0.lock().map_err(|_| "Recent Documents Are Unavailable.".to_string())?;
        recent::clear(&directory)?;
        app.state::<FileSessions>().sync_recent(&[])?;
        Ok(())
    }).await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn open_recent_document(token: String, sessions: State<'_, FileSessions>) -> Result<Response, String> {
    let path = sessions.resolve(&token)?;
    let name = files::document_name(&path);
    let read_path = path.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || files::read_document(&read_path)).await.map_err(|error| error.to_string())??;
    let token = sessions.register(path)?;
    document_response(Some(SavedFile { token, name, import_format: None }), &bytes)
}

#[tauri::command]
async fn export_plain_text(text: String, token: Option<String>, sessions: State<'_, FileSessions>) -> Result<Option<String>, String> {
    let suggested = token.as_deref().map(|token| sessions.resolve(token)).transpose()?
        .map(|path| path.with_extension("txt")).as_deref().map(files::document_name)
        .unwrap_or_else(|| "Untitled.txt".into());
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new().set_title("Export Plain Text").add_filter("Plain Text", &["txt"]).set_file_name(suggested).save_file();
        let Some(path) = path else { return Ok(None) };
        if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("txt")) {
            return Err("Use The TXT Extension When Exporting Plain Text.".into());
        }
        files::write_text(&path, &text)?;
        Ok(Some(files::document_name(&path)))
    }).await.map_err(|error| format!("Could Not Export Plain Text. {error}"))?
}

#[tauri::command]
async fn download_document(request: Request<'_>) -> Result<Option<String>, String> {
    let InvokeBody::Raw(packet) = request.body() else { return Err("Use A Binary Download Transfer.".into()) };
    let (metadata, bytes) = downloads::read_request(packet)?;
    let bytes = bytes.to_vec();
    let suggested = metadata.suggested_name()?;
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new().set_title("Download Document")
            .add_filter(metadata.format.label(), &[metadata.format.extension()]).set_file_name(suggested).save_file();
        let Some(path) = path else { return Ok(None) };
        downloads::write_download(&path, metadata.format, &bytes)?;
        Ok(Some(files::document_name(&path)))
    }).await.map_err(|error| format!("Could Not Download The Document. {error}"))?
}

#[tauri::command]
async fn download_pdf(window: tauri::WebviewWindow, name: String) -> Result<Option<String>, String> {
    let format = downloads::DownloadFormat::Pdf;
    let suggested = downloads::DownloadMetadata { format, name }.suggested_name()?;
    let selected = tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new().set_title("Download PDF").add_filter(format.label(), &[format.extension()]).set_file_name(suggested).save_file()
    }).await.map_err(|error| format!("Could Not Choose A PDF Download Location. {error}"))?;
    let Some(path) = selected else { return Ok(None) };
    format.validate_destination(&path)?;
    pdf::print_to_file(window, path.clone()).await?;
    Ok(Some(files::document_name(&path)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedFile {
    token: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    import_format: Option<String>,
}

const MAX_METADATA_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveMetadata {
    token: Option<String>,
    save_as: bool,
    name: Option<String>,
}

fn document_response(metadata: Option<SavedFile>, bytes: &[u8]) -> Result<Response, String> {
    let header = serde_json::to_vec(&metadata).map_err(|error| error.to_string())?;
    if header.len() > MAX_METADATA_BYTES || bytes.len() > files::MAX_DOCUMENT_BYTES {
        return Err("The Document Transfer Is Too Large.".into());
    }
    let mut packet = Vec::with_capacity(4 + header.len() + bytes.len());
    packet.extend_from_slice(&(header.len() as u32).to_le_bytes());
    packet.extend_from_slice(&header);
    packet.extend_from_slice(bytes);
    Ok(Response::new(packet))
}

fn save_request(body: &InvokeBody) -> Result<(SaveMetadata, Vec<u8>), String> {
    let InvokeBody::Raw(packet) = body else { return Err("Use A Binary Document Transfer.".into()) };
    let prefix: [u8; 4] = packet.get(..4).ok_or("Invalid Document Transfer.")?.try_into().map_err(|_| "Invalid Document Transfer.")?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_METADATA_BYTES || length > packet.len() - 4 {
        return Err("Invalid Document Metadata.".into());
    }
    let bytes = &packet[4 + length..];
    if bytes.len() > files::MAX_DOCUMENT_BYTES { return Err("The Document Exceeds The 25 MB Limit.".into()); }
    let metadata = serde_json::from_slice(&packet[4..4 + length]).map_err(|error| format!("Invalid Document Metadata. {error}"))?;
    Ok((metadata, bytes.to_vec()))
}

#[tauri::command]
async fn open_startup_document(startup: State<'_, StartupDocument>, sessions: State<'_, FileSessions>) -> Result<Response, String> {
    let Some(path) = startup.take()? else { return document_response(None, &[]) };
    let read_path = path.clone();
    let check_path = path.clone();
    let (bytes, name, import_format, registered) = tauri::async_runtime::spawn_blocking(move || {
        if check_path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("docx")) {
            files::read_document(&read_path).map(|bytes| (bytes, files::document_name(&check_path), None, true))
        } else {
            files::read_import(&read_path).map(|(bytes, format)| (bytes, files::document_name(&check_path), Some(format), false))
        }
    })
        .await.map_err(|error| format!("Could Not Open The Startup Document. {error}"))??;
    let token = if registered { sessions.register(path)? } else { String::new() };
    document_response(Some(SavedFile { token, name, import_format }), &bytes)
}

#[tauri::command]
async fn open_document(sessions: State<'_, FileSessions>) -> Result<Response, String> {
    let selection = tauri::async_runtime::spawn_blocking(|| {
        let path = rfd::FileDialog::new().set_title("Open Document")
            .add_filter("Word Document", &["docx"])
            .add_filter("Markdown", &["md", "markdown"])
            .add_filter("HTML Document", &["html", "htm"])
            .add_filter("Plain Text", &["txt"])
            .add_filter("OpenDocument Text", &["odt"])
            .pick_file();
        path.map(|path| {
            if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("docx")) {
                files::read_document(&path).map(|bytes| (path, bytes, None))
            } else {
                files::read_import(&path).map(|(bytes, format)| (path, bytes, Some(format)))
            }
        }).transpose()
    }).await.map_err(|error| format!("Could Not Open The File Dialog. {error}"))??;

    let Some((path, bytes, import_format)) = selection else { return document_response(None, &[]) };
    let name = files::document_name(&path);
    let token = if import_format.is_some() { String::new() } else { sessions.register(path)? };
    document_response(Some(SavedFile { token, name, import_format }), &bytes)
}

#[tauri::command]
async fn save_document(app: tauri::AppHandle, request: Request<'_>, sessions: State<'_, FileSessions>) -> Result<Option<SavedFile>, String> {
    let (SaveMetadata { token, save_as, name }, bytes) = save_request(request.body())?;
    files::validate_docx(&bytes)?;
    let current_path = token.as_deref().map(|value| sessions.resolve(value)).transpose()?;
    let documents_directory = app.path().app_data_dir().map_err(|error| error.to_string())?.join("Documents");
    let saved_path = tauri::async_runtime::spawn_blocking(move || {
        if !save_as && current_path.is_none() {
            return files::save_new_document(&documents_directory, name.as_deref().unwrap_or("Untitled.docx"), &bytes).map(Some);
        }
        let path = if save_as {
            let suggested_name = name.unwrap_or_else(|| current_path.as_deref().map(files::document_name).unwrap_or_else(|| "Untitled.docx".into()));
            let mut dialog = rfd::FileDialog::new().set_title("Export DOCX Copy").add_filter("Word Document", &["docx"]).set_file_name(suggested_name);
            if let Some(parent) = current_path.as_deref().and_then(|path| path.parent()) {
                dialog = dialog.set_directory(parent);
            }
            dialog.save_file()
        } else { current_path };

        let Some(path) = path else { return Ok(None) };
        if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("docx")) {
            return Err("Use The DOCX Extension When Saving A Document.".into());
        }
        files::write_document(&path, &bytes)?;
        Ok::<_, String>(Some(path))
    }).await.map_err(|error| format!("Could Not Save The Document. {error}"))??;

    let Some(path) = saved_path else { return Ok(None) };
    let name = files::document_name(&path);
    let token = if save_as { sessions.register_export(path)? } else {
        let token = sessions.register(path)?;
        sessions.activate(&token)?;
        token
    };
    Ok(Some(SavedFile { token, name, import_format: None }))
}

#[tauri::command]
async fn rename_document(app: tauri::AppHandle, token: String, name: String) -> Result<SavedFile, String> {
    let sessions = app.state::<FileSessions>();
    let path = sessions.resolve(&token)?;
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let access = app.state::<RecentAccess>();
        let _guard = access.0.lock().map_err(|_| "Recent Documents Are Unavailable.".to_string())?;
        let renamed = files::rename_document(&path, &name)?;
        app.state::<FileSessions>().rename(&token, renamed.clone())?;
        replace_recent_path(&directory, &path, &renamed)?;
        app.state::<FileSessions>().sync_recent(&recent::list(&directory)?)?;
        Ok(SavedFile { token, name: files::document_name(&renamed), import_format: None })
    }).await.map_err(|error| error.to_string())?
}

fn replace_recent_path(directory: &Path, previous: &Path, renamed: &Path) -> Result<(), String> {
    let mut paths = recent::list(directory)?;
    if !paths.iter().any(|path| path == previous) { return Ok(()) }
    paths.retain(|path| path != renamed);
    for path in &mut paths {
        if path == previous { *path = renamed.to_path_buf(); }
    }
    let bytes = serde_json::to_vec(&paths).map_err(|error| error.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    file.write_all(&bytes).and_then(|()| file.as_file().sync_all()).map_err(|error| error.to_string())?;
    file.persist(directory.join("recent-documents.json")).map_err(|error| error.to_string())?;
    Ok(())
}

fn raw_document_request(body: &InvokeBody) -> Result<(serde_json::Value, Vec<u8>), String> {
    let InvokeBody::Raw(packet) = body else { return Err("Use A Binary Document Transfer.".into()) };
    let prefix: [u8; 4] = packet.get(..4).ok_or("Invalid Document Transfer.")?.try_into().map_err(|_| "Invalid Document Transfer.")?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_METADATA_BYTES || length > packet.len() - 4 {
        return Err("Invalid Document Metadata.".into());
    }
    let bytes = &packet[4 + length..];
    if bytes.len() > files::MAX_DOCUMENT_BYTES { return Err("The Document Exceeds The 25 MB Limit.".into()); }
    let metadata: serde_json::Value = serde_json::from_slice(&packet[4..4 + length]).map_err(|error| format!("Invalid Document Metadata. {error}"))?;
    Ok((metadata, bytes.to_vec()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotMetadata {
    token: Option<String>,
    name: String,
}

#[tauri::command]
async fn write_document_snapshot(app: tauri::AppHandle, request: Request<'_>, sessions: State<'_, FileSessions>) -> Result<(), String> {
    let (metadata, bytes) = raw_document_request(request.body())?;
    let metadata: SnapshotMetadata = serde_json::from_value(metadata).map_err(|error| format!("Invalid Document Metadata. {error}"))?;
    let token = metadata.token.filter(|value| !value.is_empty());
    let key = snapshots::snapshot_key(sessions.inner(), token.as_deref(), &metadata.name)?;
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        snapshots::write(&directory, &key, &bytes)?;
        Ok(())
    }).await.map_err(|error| format!("Could Not Save The Snapshot. {error}"))?
}

#[tauri::command]
async fn list_document_snapshots(app: tauri::AppHandle, token: Option<String>, name: String, sessions: State<'_, FileSessions>) -> Result<Vec<snapshots::SnapshotInfo>, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    let token = token.filter(|value| !value.is_empty());
    let key = snapshots::snapshot_key(sessions.inner(), token.as_deref(), &name)?;
    tauri::async_runtime::spawn_blocking(move || snapshots::list(&directory, &key))
        .await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn read_document_snapshot(app: tauri::AppHandle, token: Option<String>, name: String, id: String, sessions: State<'_, FileSessions>) -> Result<Response, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    let token = token.filter(|value| !value.is_empty());
    let key = snapshots::snapshot_key(sessions.inner(), token.as_deref(), &name)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || snapshots::read(&directory, &key, &id))
        .await.map_err(|error| error.to_string())??;
    Ok(Response::new(bytes))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotStatus {
    latest_id: Option<String>,
    latest_created_at: Option<u64>,
    file_modified_at: Option<u64>,
}

#[tauri::command]
async fn document_snapshot_status(app: tauri::AppHandle, token: Option<String>, name: String, sessions: State<'_, FileSessions>) -> Result<SnapshotStatus, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    let token = token.filter(|value| !value.is_empty());
    let file_path = token.as_deref().and_then(|value| sessions.resolve(value).ok());
    let key = snapshots::snapshot_key(sessions.inner(), token.as_deref(), &name)?;
    tauri::async_runtime::spawn_blocking(move || {
        let latest = snapshots::latest(&directory, &key)?;
        let file_modified_at = file_path
            .and_then(|path| std::fs::metadata(path).and_then(|metadata| metadata.modified()).ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64);
        Ok(SnapshotStatus {
            latest_id: latest.as_ref().map(|entry| entry.id.clone()),
            latest_created_at: latest.map(|entry| entry.created_at),
            file_modified_at,
        })
    }).await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn save_document_template(app: tauri::AppHandle, request: Request<'_>) -> Result<templates::TemplateInfo, String> {
    let (metadata, bytes) = raw_document_request(request.body())?;
    let name = metadata.get("name").and_then(|value| value.as_str()).ok_or("Template Metadata Requires A Name.")?.to_string();
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?.join("Templates");
    tauri::async_runtime::spawn_blocking(move || templates::save(&directory, &name, &bytes))
        .await.map_err(|error| format!("Could Not Save The Template. {error}"))?
}

#[tauri::command]
async fn list_document_templates(app: tauri::AppHandle) -> Result<Vec<templates::TemplateInfo>, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?.join("Templates");
    tauri::async_runtime::spawn_blocking(move || templates::list(&directory))
        .await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn read_document_template(app: tauri::AppHandle, id: String) -> Result<Response, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?.join("Templates");
    let bytes = tauri::async_runtime::spawn_blocking(move || templates::read(&directory, &id))
        .await.map_err(|error| error.to_string())??;
    Ok(Response::new(bytes))
}

#[tauri::command]
async fn delete_document_template(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?.join("Templates");
    tauri::async_runtime::spawn_blocking(move || templates::delete(&directory, &id))
        .await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn installed_fonts() -> Result<Vec<fonts::InstalledFont>, String> {
    tauri::async_runtime::spawn_blocking(fonts::installed_fonts)
        .await.map_err(|error| format!("Could Not List Installed Fonts. {error}"))?
}

#[tauri::command]
async fn read_font_file(path: String) -> Result<Response, String> {
    let bytes = tauri::async_runtime::spawn_blocking(move || fonts::read_font_file(std::path::Path::new(&path)))
        .await.map_err(|error| format!("Could Not Read The Font File. {error}"))??;
    Ok(Response::new(bytes))
}

#[tauri::command]
fn caps_lock_on() -> bool {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetKeyState(virtual_key: i32) -> i16;
        }
        unsafe { (GetKeyState(0x14) & 1) != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(FileSessions::default())
        .manage(StartupDocument::from_args(std::env::args_os()))
        .manage(RecentAccess::default())
        .setup(|_app| {
            let _ = tauri::async_runtime::spawn_blocking(fonts::installed_fonts);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_external_link, choose_link_browser, open_startup_document, open_document, save_document, rename_document, document_statistics, recent_documents, remember_document, clear_recent_documents, open_recent_document, export_plain_text, download_document, download_pdf, write_document_snapshot, list_document_snapshots, read_document_snapshot, document_snapshot_status, save_document_template, list_document_templates, read_document_template, delete_document_template, installed_fonts, read_font_file, caps_lock_on])
        .run(tauri::generate_context!())
        .expect("Could Not Start InsynDoc");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::ipc::{InvokeResponseBody, IpcResponse};

    fn packet(metadata: &[u8], bytes: &[u8]) -> InvokeBody {
        let mut packet = (metadata.len() as u32).to_le_bytes().to_vec();
        packet.extend_from_slice(metadata);
        packet.extend_from_slice(bytes);
        InvokeBody::Raw(packet)
    }

    #[test]
    fn binary_requests_preserve_large_documents_and_unicode_metadata() {
        let metadata = serde_json::to_vec(&serde_json::json!({ "token": "7", "saveAs": true, "name": "Résumé.docx" })).unwrap();
        let bytes = vec![255; files::MAX_DOCUMENT_BYTES];
        let (saved, received) = save_request(&packet(&metadata, &bytes)).unwrap();
        assert_eq!(saved.token.as_deref(), Some("7"));
        assert_eq!(saved.name.as_deref(), Some("Résumé.docx"));
        assert!(saved.save_as);
        assert_eq!(received, bytes);
        assert!(save_request(&packet(&metadata, &vec![0; files::MAX_DOCUMENT_BYTES + 1])).is_err());
    }

    #[test]
    fn rejects_malformed_binary_requests_and_enforces_metadata_boundary() {
        for body in [InvokeBody::Json(serde_json::json!({})), InvokeBody::Raw(vec![]), InvokeBody::Raw(vec![0; 3]),
            InvokeBody::Raw(u32::MAX.to_le_bytes().to_vec()), packet(b"{", &[])] {
            assert!(save_request(&body).is_err());
        }
        let mut metadata = br#"{"saveAs":false}"#.to_vec();
        metadata.resize(MAX_METADATA_BYTES, b' ');
        assert!(save_request(&packet(&metadata, &[])).is_ok());
        metadata.push(b' ');
        assert!(save_request(&packet(&metadata, &[])).is_err());
    }

    #[test]
    fn document_responses_use_raw_bytes_including_cancellation() {
        let bytes = vec![255; files::MAX_DOCUMENT_BYTES];
        let response = document_response(Some(SavedFile { token: "7".into(), name: "Résumé.docx".into(), import_format: None }), &bytes).unwrap();
        let InvokeResponseBody::Raw(raw) = response.body().unwrap() else { panic!("Expected Binary Response") };
        let length = u32::from_le_bytes(raw[..4].try_into().unwrap()) as usize;
        let metadata: serde_json::Value = serde_json::from_slice(&raw[4..4 + length]).unwrap();
        assert_eq!(metadata["name"], "Résumé.docx");
        assert_eq!(&raw[4 + length..], bytes);
        let InvokeResponseBody::Raw(cancelled) = document_response(None, &[]).unwrap().body().unwrap() else { panic!("Expected Binary Response") };
        assert_eq!(cancelled, [4, 0, 0, 0, b'n', b'u', b'l', b'l']);
        assert!(document_response(None, &vec![0; files::MAX_DOCUMENT_BYTES + 1]).is_err());
    }

    #[test]
    fn repeated_renames_replace_persisted_recent_paths() {
        let directory = tempfile::tempdir().unwrap();
        let mut previous = std::path::PathBuf::from("Original.docx");
        recent::remember(directory.path(), &previous).unwrap();
        recent::remember(directory.path(), Path::new("Other.docx")).unwrap();
        for index in 0..30 {
            let renamed = std::path::PathBuf::from(format!("Renamed {index}.docx"));
            replace_recent_path(directory.path(), &previous, &renamed).unwrap();
            let paths = recent::list(directory.path()).unwrap();
            assert_eq!(paths, vec![std::path::PathBuf::from("Other.docx"), renamed.clone()]);
            previous = renamed;
        }
    }
}

