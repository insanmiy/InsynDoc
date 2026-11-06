#[cfg(target_os = "windows")]
pub async fn print_to_file(window: tauri::WebviewWindow, destination: std::path::PathBuf) -> Result<(), String> {
    use std::io::Read;
    use std::sync::mpsc;
    use std::time::Duration;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment6, ICoreWebView2_7};
    use webview2_com::PrintToPdfCompletedHandler;
    use windows_core::{HSTRING, Interface};

    let parent = destination.parent().ok_or("Choose A Download Folder.")?;
    let temporary_directory = tempfile::tempdir_in(parent).map_err(|error| format!("Could Not Prepare The PDF Download. {error}"))?;
    let temporary_pdf = temporary_directory.path().join("document.pdf");
    let temporary_path = temporary_pdf.clone();
    let (completion, result) = mpsc::sync_channel::<Result<(), String>>(1);
    window.with_webview(move |webview| {
        let started = (|| -> windows_core::Result<()> {
            // WebView2 Interfaces And The Completion Handler Stay On The Window Thread.
            unsafe {
                let environment: ICoreWebView2Environment6 = webview.environment().cast()?;
                let settings = environment.CreatePrintSettings()?;
                settings.SetShouldPrintBackgrounds(true)?;
                settings.SetShouldPrintHeaderAndFooter(false)?;
                settings.SetShouldPrintSelectionOnly(false)?;
                settings.SetPageWidth(8.2677)?;
                settings.SetPageHeight(11.6929)?;
                settings.SetMarginTop(0.5)?;
                settings.SetMarginBottom(0.5)?;
                settings.SetMarginLeft(0.5)?;
                settings.SetMarginRight(0.5)?;
                let browser: ICoreWebView2_7 = webview.controller().CoreWebView2()?.cast()?;
                let completed = completion.clone();
                let handler = PrintToPdfCompletedHandler::create(Box::new(move |status, success| {
                    let outcome = status.map_err(|error| format!("Could Not Create The PDF. {error}"))
                        .and_then(|()| if success { Ok(()) } else { Err("The PDF Could Not Be Created. Try Again After The Document Finishes Loading.".into()) });
                    let _ = completed.try_send(outcome);
                    Ok(())
                }));
                browser.PrintToPdf(&HSTRING::from(temporary_path.as_os_str()), &settings, &handler)?;
            }
            Ok(())
        })();
        if let Err(error) = started { let _ = completion.try_send(Err(format!("PDF Export Is Unavailable. Update WebView2 And Try Again. {error}"))); }
    }).map_err(|error| format!("Could Not Access The PDF Renderer. {error}"))?;

    tauri::async_runtime::spawn_blocking(move || {
        result.recv_timeout(Duration::from_secs(120)).map_err(|_| "The PDF Download Timed Out. Try A Smaller Document.".to_string())??;
        let file = std::fs::File::open(&temporary_pdf).map_err(|error| format!("Could Not Read The Generated PDF. {error}"))?;
        let mut bytes = Vec::new();
        file.take((crate::downloads::MAX_DOWNLOAD_BYTES + 1) as u64).read_to_end(&mut bytes)
            .map_err(|error| format!("Could Not Read The Generated PDF. {error}"))?;
        crate::downloads::write_download(&destination, crate::downloads::DownloadFormat::Pdf, &bytes)?;
        drop(temporary_directory);
        Ok(())
    }).await.map_err(|error| format!("Could Not Complete The PDF Download. {error}"))?
}

#[cfg(not(target_os = "windows"))]
pub async fn print_to_file(_window: tauri::WebviewWindow, _destination: std::path::PathBuf) -> Result<(), String> {
    Err("PDF Download Requires The Windows Desktop App.".into())
}
