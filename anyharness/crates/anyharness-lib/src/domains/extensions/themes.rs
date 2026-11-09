use std::collections::BTreeMap;

use serde_json::Value;

use super::manifest::resource_path;
use crate::adapters::extensions::management::PackageDirectory;

pub const MAX_COLOR_THEMES: usize = 32;
const COLOR_KEYS: [&str; 6] = [
    "editor.background",
    "editor.foreground",
    "sideBar.background",
    "sideBar.foreground",
    "button.background",
    "button.foreground",
];

pub struct PackageColorTheme {
    pub label: String,
    pub color_mode: Option<&'static str>,
    pub colors: BTreeMap<String, String>,
    pub available: bool,
}

/// Only declared color resources are read. Includes and executable contributions
/// are never followed; callers receive bounded data for the app color importer.
pub fn read_color_themes(
    packages: &PackageDirectory,
    name: &str,
    manifest: &Value,
) -> (Vec<PackageColorTheme>, bool) {
    let entries = manifest
        .pointer("/contributes/themes")
        .and_then(Value::as_array);
    let mut themes = Vec::new();
    for entry in entries.into_iter().flatten().take(MAX_COLOR_THEMES) {
        let label = entry
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Unnamed theme")
            .chars()
            .take(80)
            .collect();
        let source = entry
            .get("path")
            .and_then(Value::as_str)
            .and_then(|path| resource_path(path).ok())
            .and_then(|path| packages.read_theme(name, &path).ok());
        let color_mode = match entry.get("uiTheme").and_then(Value::as_str) {
            Some("vs" | "hc-light") => Some("light"),
            Some("vs-dark" | "hc-black") => Some("dark"),
            _ => match source
                .as_ref()
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str)
            {
                Some("light") => Some("light"),
                Some("dark") => Some("dark"),
                _ => None,
            },
        };
        let colors: BTreeMap<String, String> = COLOR_KEYS
            .into_iter()
            .filter_map(|key| {
                let value = source.as_ref()?.get("colors")?.get(key)?.as_str()?;
                (value.len() <= 32).then(|| (key.to_owned(), value.to_owned()))
            })
            .collect();
        let available = color_mode.is_some()
            && source
                .as_ref()
                .is_some_and(|value| value.get("include").is_none())
            && COLOR_KEYS.into_iter().all(|key| {
                source
                    .as_ref()
                    .and_then(|value| value.get("colors"))
                    .and_then(|value| value.get(key))
                    .is_none_or(|value| value.as_str().is_some_and(|value| value.len() <= 32))
            })
            && colors.contains_key("editor.background")
            && colors.contains_key("editor.foreground");
        themes.push(PackageColorTheme {
            label,
            color_mode,
            colors,
            available,
        });
    }
    (
        themes,
        entries.is_some_and(|entries| entries.len() > MAX_COLOR_THEMES),
    )
}
