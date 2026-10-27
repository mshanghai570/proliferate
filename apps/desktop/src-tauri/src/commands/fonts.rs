use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemFontFamily {
    /// The CSS family name the webview resolves with `font-family`. On macOS
    /// this is the PostScript-derived family name CoreText reports.
    pub family_name: String,
    /// Human-readable label for font pickers. Mirrors `family_name` today;
    /// kept separate so localized display names can land without a wire change.
    pub display_name: String,
}

#[tauri::command]
pub fn list_system_fonts() -> Result<Vec<SystemFontFamily>, String> {
    #[cfg(target_os = "macos")]
    {
        return macos_available_font_families();
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("list_system_fonts is only supported on macOS".to_string())
    }
}

#[cfg(target_os = "macos")]
fn macos_available_font_families() -> Result<Vec<SystemFontFamily>, String> {
    use objc2_core_text::CTFontManagerCopyAvailableFontFamilyNames;

    let names = unsafe { CTFontManagerCopyAvailableFontFamilyNames() };
    let Some(array) = names else {
        return Err("CoreText returned no font family list".to_string());
    };

    let mut families: Vec<SystemFontFamily> = Vec::new();
    for index in 0..array.len() {
        let Some(name) = array.get(index) else {
            continue;
        };
        let family_name = name.to_string();
        if family_name.is_empty() {
            continue;
        }
        families.push(SystemFontFamily {
            display_name: family_name.clone(),
            family_name,
        });
    }

    families.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
    });
    Ok(families)
}
