#[tauri::command]
pub async fn list_local_font_families(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSFontManager;

        let (sender, receiver) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let families = MainThreadMarker::new()
                .map(|marker| {
                    NSFontManager::sharedFontManager(marker)
                        .availableFontFamilies()
                        .iter()
                        .map(|family| family.to_string())
                        .collect::<Vec<_>>()
                })
                .ok_or_else(|| "Font list unavailable off the main thread.".to_string());
            let _ = sender.send(families);
        })
        .map_err(|_| "Could not request the macOS font list.".to_string())?;

        let mut families = receiver
            .await
            .map_err(|_| "Could not read the macOS font list.".to_string())??;
        families.sort_by_key(|family| family.to_lowercase());
        families.dedup();
        Ok(families)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(Vec::new())
    }
}
