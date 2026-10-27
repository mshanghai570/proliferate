use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
    use objc2_core_foundation::{CFArray, CFRetained, CFString};
    use objc2_core_text::CTFontManagerCopyAvailableFontFamilyNames;

    // CoreText documents this array's entries as CFStrings.
    let array = unsafe {
        CFRetained::cast_unchecked::<CFArray<CFString>>(CTFontManagerCopyAvailableFontFamilyNames())
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

#[cfg(test)]
mod tests {
    use super::SystemFontFamily;

    #[test]
    fn system_font_wire_fields_use_camel_case() {
        let serialized = serde_json::to_value(SystemFontFamily {
            family_name: "SF Mono".to_string(),
            display_name: "SF Mono".to_string(),
        })
        .expect("font family should serialize");

        assert_eq!(serialized["familyName"], "SF Mono");
        assert_eq!(serialized["displayName"], "SF Mono");
        assert!(serialized.get("family_name").is_none());
        assert!(serialized.get("display_name").is_none());
    }
}
