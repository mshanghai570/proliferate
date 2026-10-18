use super::manifest::resource_path;
use super::themes::{read_color_themes, PackageColorTheme};
use crate::adapters::extensions::management::PackageDirectory;
use serde_json::Value;

pub const MAX_SYNTAX_RULES: usize = 256;

pub struct SyntaxColorRule {
    pub scopes: Vec<String>,
    pub foreground: String,
}
pub struct PackageSyntaxTheme {
    pub theme: PackageColorTheme,
    pub rules: Vec<SyntaxColorRule>,
}

pub fn read_syntax_themes(
    packages: &PackageDirectory,
    name: &str,
    manifest: &Value,
) -> (Vec<PackageSyntaxTheme>, bool) {
    let (themes, truncated) = read_color_themes(packages, name, manifest);
    let entries = manifest
        .pointer("/contributes/themes")
        .and_then(Value::as_array);
    let themes = themes
        .into_iter()
        .enumerate()
        .map(|(index, mut theme)| {
            let rules = entries
                .and_then(|entries| entries.get(index))
                .and_then(|entry| entry.get("path"))
                .and_then(Value::as_str)
                .and_then(|path| resource_path(path).ok())
                .and_then(|path| packages.read_theme(name, &path).ok())
                .and_then(|source| parse_rules(&source));
            theme.available &= rules.as_ref().is_some_and(|rules| !rules.is_empty());
            PackageSyntaxTheme {
                theme,
                rules: rules.unwrap_or_default(),
            }
        })
        .collect();
    (themes, truncated)
}

fn parse_rules(source: &Value) -> Option<Vec<SyntaxColorRule>> {
    if source.get("include").is_some() {
        return None;
    }
    let entries = source.get("tokenColors")?.as_array()?;
    if entries.len() > MAX_SYNTAX_RULES {
        return None;
    }
    let mut rules = Vec::new();
    for entry in entries {
        let settings = entry.get("settings")?.as_object()?;
        let Some(foreground) = settings.get("foreground") else {
            continue;
        };
        let foreground = foreground.as_str()?;
        if foreground.len() != 7
            || !foreground.starts_with('#')
            || !foreground[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return None;
        }
        let scopes: Vec<String> = match entry.get("scope") {
            Some(Value::String(scope)) if scope.len() <= 512 => scope
                .split(',')
                .map(|scope| scope.trim().to_owned())
                .collect(),
            Some(Value::Array(scopes)) if scopes.len() <= 16 => scopes
                .iter()
                .map(|scope| scope.as_str().map(str::to_owned))
                .collect::<Option<_>>()?,
            None => continue,
            _ => return None,
        };
        if scopes.is_empty()
            || scopes.len() > 16
            || scopes.iter().any(|scope| {
                scope.is_empty() || scope.len() > 256 || scope.chars().any(char::is_control)
            })
        {
            return None;
        }
        rules.push(SyntaxColorRule {
            scopes,
            foreground: foreground.to_owned(),
        });
    }
    let chars: usize = rules
        .iter()
        .map(|rule| rule.foreground.len() + rule.scopes.iter().map(String::len).sum::<usize>())
        .sum();
    (chars <= 32768).then_some(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn syntax_theme_rules_are_bounded_data_only() {
        let valid = json!({"tokenColors":[{"scope":"comment, string", "settings":{"foreground":"#aabbcc", "fontStyle":"italic"}}]});
        let parsed = parse_rules(&valid).unwrap();
        assert_eq!(parsed[0].scopes, vec!["comment", "string"]);
        assert_eq!(parsed[0].foreground, "#aabbcc");
        for value in [
            json!({"include":"outside.json", "tokenColors":[]}),
            json!({"tokenColors":"outside.json"}),
            json!({"tokenColors":[{"scope":"comment", "settings":{"foreground":"url(secret)"}}]}),
            json!({"tokenColors":[{"scope":["a".repeat(257)], "settings":{"foreground":"#ffffff"}}]}),
            json!({"tokenColors":vec![valid["tokenColors"][0].clone();257]}),
        ] {
            assert!(parse_rules(&value).is_none());
        }
    }
}
