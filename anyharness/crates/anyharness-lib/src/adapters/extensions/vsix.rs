//! Bounded VSIX inspection and extraction into an empty, caller-owned directory.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use thiserror::Error;
use zip::ZipArchive;

pub const VSIX_MANIFEST_PATH: &str = "extension/package.json";
pub const MAX_VSIX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VsixLimits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
}

pub const DEFAULT_VSIX_LIMITS: VsixLimits = VsixLimits {
    max_entries: 4_096,
    max_entry_bytes: 32 * 1024 * 1024,
    max_total_bytes: 256 * 1024 * 1024,
};

impl Default for VsixLimits {
    fn default() -> Self {
        DEFAULT_VSIX_LIMITS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsafeEntry {
    EmptyName,
    AbsolutePath,
    TraversalAttempt,
    WindowsPrefix,
    BackslashSeparator,
    InvalidName,
    NonRegularFile,
}

impl std::fmt::Display for UnsafeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyName => "entry name is empty",
            Self::AbsolutePath => "entry path is absolute",
            Self::TraversalAttempt => "entry path traverses upward",
            Self::WindowsPrefix => "entry path carries a Windows prefix",
            Self::BackslashSeparator => "entry path uses a backslash separator",
            Self::InvalidName => "entry path contains a reserved or ambiguous name",
            Self::NonRegularFile => "entry is a symlink",
        })
    }
}

#[derive(Debug, Error)]
pub enum VsixError {
    #[error("archive could not be read: {0}")]
    Io(#[from] io::Error),
    #[error("archive is not a readable zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("unsafe archive entry {path:?}: {reason}")]
    UnsafeEntry { path: String, reason: UnsafeEntry },
    #[error("archive contains duplicate paths")]
    DuplicateEntry,
    #[error("extraction requires an empty directory, without a symlink at its root")]
    InvalidDestination,
    #[error("archive declares {entries} entries, over the limit of {limit}")]
    TooManyEntries { entries: usize, limit: usize },
    #[error("archive entry {path:?} is {size} bytes, over the per-entry limit of {limit}")]
    EntryTooLarge { path: String, size: u64, limit: u64 },
    #[error("archive expands to at least {total} bytes, over the limit of {limit}")]
    ArchiveTooLarge { total: u64, limit: u64 },
    #[error("package manifest {VSIX_MANIFEST_PATH} is missing")]
    ManifestMissing,
    #[error("package manifest is invalid: {0}")]
    InvalidManifest(String),
}

pub(super) fn validate_entry_path(name: &str) -> Result<PathBuf, UnsafeEntry> {
    if name.is_empty() {
        return Err(UnsafeEntry::EmptyName);
    }
    if name.contains('\\') {
        return Err(UnsafeEntry::BackslashSeparator);
    }
    if name.as_bytes().get(1) == Some(&b':') {
        return Err(UnsafeEntry::WindowsPrefix);
    }
    if Path::new(name).is_absolute() {
        return Err(UnsafeEntry::AbsolutePath);
    }
    let mut normalized = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::ParentDir => return Err(UnsafeEntry::TraversalAttempt),
            Component::RootDir => return Err(UnsafeEntry::AbsolutePath),
            Component::Prefix(_) => return Err(UnsafeEntry::WindowsPrefix),
            Component::CurDir => {}
            Component::Normal(value) => {
                let part = value.to_string_lossy();
                let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
                let device = matches!(
                    stem.as_str(),
                    "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                ) || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|suffix| {
                        suffix.chars().count() == 1 && "0123456789¹²³".contains(suffix)
                    })
                });
                if device
                    || part.ends_with(['.', ' '])
                    || part
                        .chars()
                        .any(|c| c.is_control() || "<>:\"|?*".contains(c))
                {
                    return Err(UnsafeEntry::InvalidName);
                }
                normalized.push(value);
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(UnsafeEntry::EmptyName);
    }
    Ok(normalized)
}

fn validate_archive(archive: &mut ZipArchive<File>, limits: VsixLimits) -> Result<(), VsixError> {
    if archive.len() > limits.max_entries {
        return Err(VsixError::TooManyEntries {
            entries: archive.len(),
            limit: limits.max_entries,
        });
    }
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        let relative = validate_entry_path(&name).map_err(|reason| VsixError::UnsafeEntry {
            path: name.clone(),
            reason,
        })?;
        if !seen.insert(relative.to_string_lossy().to_lowercase()) {
            return Err(VsixError::DuplicateEntry);
        }
        if entry.is_symlink() {
            return Err(VsixError::UnsafeEntry {
                path: name,
                reason: UnsafeEntry::NonRegularFile,
            });
        }
        if entry.size() > limits.max_entry_bytes {
            return Err(VsixError::EntryTooLarge {
                path: name,
                size: entry.size(),
                limit: limits.max_entry_bytes,
            });
        }
        total = total.saturating_add(entry.size());
        if total > limits.max_total_bytes {
            return Err(VsixError::ArchiveTooLarge {
                total,
                limit: limits.max_total_bytes,
            });
        }
    }
    Ok(())
}

pub fn read_vsix_manifest(archive_path: &Path) -> Result<serde_json::Value, VsixError> {
    let mut archive = ZipArchive::new(File::open(archive_path)?)?;
    validate_archive(&mut archive, DEFAULT_VSIX_LIMITS)?;
    let entry = archive
        .by_name(VSIX_MANIFEST_PATH)
        .map_err(|error| match error {
            zip::result::ZipError::FileNotFound => VsixError::ManifestMissing,
            other => VsixError::Zip(other),
        })?;
    let mut bytes = Vec::new();
    entry
        .take(MAX_VSIX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_VSIX_MANIFEST_BYTES {
        return Err(VsixError::EntryTooLarge {
            path: VSIX_MANIFEST_PATH.into(),
            size: bytes.len() as u64,
            limit: MAX_VSIX_MANIFEST_BYTES,
        });
    }
    serde_json::from_slice(&bytes).map_err(|error| VsixError::InvalidManifest(error.to_string()))
}

/// Preflight rejects unsafe metadata before output. I/O/CRC failures may leave
/// partial output; callers must discard the private staging directory.
pub fn extract_vsix(
    archive_path: &Path,
    dest: &Path,
    limits: VsixLimits,
) -> Result<Vec<String>, VsixError> {
    let mut archive = ZipArchive::new(File::open(archive_path)?)?;
    validate_archive(&mut archive, limits)?;
    match fs::symlink_metadata(dest) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            if fs::read_dir(dest)?.next().is_some() {
                return Err(VsixError::InvalidDestination);
            }
        }
        Ok(_) => return Err(VsixError::InvalidDestination),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(dest)?,
        Err(error) => return Err(error.into()),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dest, fs::Permissions::from_mode(0o700))?;
    }
    let mut written = Vec::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        let relative = validate_entry_path(&name).map_err(|reason| VsixError::UnsafeEntry {
            path: name.clone(),
            reason,
        })?;
        let target = dest.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(target)?;
            continue;
        }
        let remaining = limits.max_total_bytes.saturating_sub(total);
        let read_limit = limits.max_entry_bytes.min(remaining);
        let mut bytes = Vec::new();
        entry
            .take(read_limit.saturating_add(1))
            .read_to_end(&mut bytes)?;
        let size = bytes.len() as u64;
        if size > limits.max_entry_bytes {
            return Err(VsixError::EntryTooLarge {
                path: name,
                size,
                limit: limits.max_entry_bytes,
            });
        }
        total = total.saturating_add(size);
        if total > limits.max_total_bytes {
            return Err(VsixError::ArchiveTooLarge {
                total,
                limit: limits.max_total_bytes,
            });
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let mut output = options.open(target)?;
        output.write_all(&bytes)?;
        output.sync_all()?;
        written.push(relative.to_string_lossy().into_owned());
    }
    Ok(written)
}
