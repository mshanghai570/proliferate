use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionPackageIdentity {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("extension identity is invalid")]
    InvalidIdentity,
    #[error("manifest publisher, name, or version does not match the requested package")]
    IdentityMismatch,
    #[error("Node and browser extension entrypoints are not supported")]
    ExecutableUnsupported,
    #[error("extension dependencies and extension packs are not supported by this installer")]
    DependenciesUnsupported,
    #[error("unsupported contribution: {0}")]
    UnsupportedContribution(String),
    #[error("manifest contributions are missing or malformed")]
    InvalidContributions,
    #[error("contribution resource paths must stay inside the extension directory")]
    InvalidResourcePath,
}

impl ExtensionPackageIdentity {
    pub fn directory_name(&self) -> Result<String, ManifestError> {
        for value in [&self.namespace, &self.name] {
            if value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !value.as_bytes()[0].is_ascii_alphanumeric()
            {
                return Err(ManifestError::InvalidIdentity);
            }
        }
        if !valid_version(&self.version) {
            return Err(ManifestError::InvalidIdentity);
        }
        Ok(format!("{}.{}-{}", self.namespace, self.name, self.version))
    }
}

fn valid_version(version: &str) -> bool {
    if version.len() > 128 {
        return false;
    }
    let (base, build) = version
        .split_once('+')
        .map_or((version, None), |(base, build)| (base, Some(build)));
    let (core, pre) = base
        .split_once('-')
        .map_or((base, None), |(core, pre)| (core, Some(pre)));
    let numbers: Vec<_> = core.split('.').collect();
    if numbers.len() != 3
        || numbers.iter().any(|n| {
            n.is_empty()
                || !n.bytes().all(|b| b.is_ascii_digit())
                || (n.len() > 1 && n.starts_with('0'))
        })
    {
        return false;
    }
    for (value, is_pre) in [(build, false), (pre, true)] {
        if let Some(value) = value {
            for part in value.split('.') {
                if part.is_empty()
                    || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    || (is_pre
                        && part.len() > 1
                        && part.starts_with('0')
                        && part.bytes().all(|b| b.is_ascii_digit()))
                {
                    return false;
                }
            }
        }
    }
    true
}

pub fn inspect_install_manifest(
    raw: &Value,
    expected: &ExtensionPackageIdentity,
) -> Result<Vec<PathBuf>, ManifestError> {
    expected.directory_name()?;
    if raw.get("publisher").and_then(Value::as_str) != Some(expected.namespace.as_str())
        || raw.get("name").and_then(Value::as_str) != Some(expected.name.as_str())
        || raw.get("version").and_then(Value::as_str) != Some(expected.version.as_str())
    {
        return Err(ManifestError::IdentityMismatch);
    }
    if raw.get("main").is_some() || raw.get("browser").is_some() {
        return Err(ManifestError::ExecutableUnsupported);
    }
    for key in ["extensionDependencies", "extensionPack"] {
        if let Some(value) = raw.get(key) {
            if !value.as_array().is_some_and(Vec::is_empty) {
                return Err(ManifestError::DependenciesUnsupported);
            }
        }
    }
    let contributions = raw
        .get("contributes")
        .and_then(Value::as_object)
        .filter(|c| !c.is_empty())
        .ok_or(ManifestError::InvalidContributions)?;
    let mut resources = Vec::new();
    for (key, value) in contributions {
        let optional_path = key == "languages";
        if !matches!(
            key.as_str(),
            "themes" | "iconThemes" | "productIconThemes" | "grammars" | "snippets" | "languages"
        ) {
            return Err(ManifestError::UnsupportedContribution(key.clone()));
        }
        let entries = value
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or(ManifestError::InvalidContributions)?;
        for entry in entries {
            if !entry.is_object() {
                return Err(ManifestError::InvalidContributions);
            }
            if optional_path
                && !entry
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.is_empty())
            {
                return Err(ManifestError::InvalidContributions);
            }
            let path_key = if optional_path {
                "configuration"
            } else {
                "path"
            };
            match entry.get(path_key) {
                None if optional_path => {}
                Some(Value::String(path)) => resources.push(resource_path(path)?),
                _ => return Err(ManifestError::InvalidResourcePath),
            }
        }
    }
    Ok(resources)
}

pub(super) fn resource_path(raw: &str) -> Result<PathBuf, ManifestError> {
    if raw.is_empty() || raw.contains(['\\', ':']) || raw.chars().any(char::is_control) {
        return Err(ManifestError::InvalidResourcePath);
    }
    let mut path = PathBuf::new();
    for part in Path::new(raw).components() {
        match part {
            Component::CurDir => {}
            Component::Normal(value) if !value.to_string_lossy().ends_with(['.', ' ']) => {
                path.push(value)
            }
            _ => return Err(ManifestError::InvalidResourcePath),
        }
    }
    if path.as_os_str().is_empty() {
        return Err(ManifestError::InvalidResourcePath);
    }
    Ok(path)
}
