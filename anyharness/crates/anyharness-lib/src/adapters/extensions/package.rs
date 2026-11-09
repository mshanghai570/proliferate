//! Private verified staging and immutable package publication. No activation.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use fs2::FileExt;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::vsix::{extract_vsix, read_vsix_manifest, validate_entry_path, VsixError, VsixLimits};

pub const MAX_PACKAGE_BYTES: u64 = 128 * 1024 * 1024;
pub const PACKAGE_RECEIPT_FILE: &str = ".proliferate-package.json";

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("package filesystem operation failed: {0}")]
    Io(#[from] io::Error),
    #[error("invalid VSIX package: {0}")]
    Vsix(#[from] VsixError),
    #[error("SHA-256 must contain exactly 64 hexadecimal characters")]
    InvalidChecksum,
    #[error("package checksum does not match the expected SHA-256")]
    ChecksumMismatch,
    #[error("package exceeds the 128 MiB compressed size limit")]
    TooLarge,
    #[error("install layout contains a symlink or a non-directory")]
    InvalidLayout,
    #[error("package source must be a regular file")]
    InvalidSource,
    #[error("package destination must be a single safe directory name")]
    InvalidDestination,
    #[error("this package version already exists; it was not overwritten")]
    AlreadyInstalled,
    #[error("publication failed and rollback could not complete: {0}")]
    RollbackFailed(io::Error),
}

pub struct StagedVsix {
    root: PathBuf,
    staging: PathBuf,
    payload: PathBuf,
    manifest: serde_json::Value,
    sha256: String,
}

impl StagedVsix {
    pub fn prepare(
        archive: &Path,
        runtime_home: &Path,
        expected_sha256: &str,
    ) -> Result<Self, PackageError> {
        if expected_sha256.len() != 64 || !expected_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PackageError::InvalidChecksum);
        }
        let home = fs::canonicalize(runtime_home)?;
        if !home.is_dir() {
            return Err(PackageError::InvalidLayout);
        }
        let extensions = home.join("extensions");
        private_directory(&extensions)?;
        let root = extensions.join("packages");
        private_directory(&root)?;
        let staging = root.join(format!(".stage-{}", Uuid::new_v4()));
        // create_dir is exclusive: never reuse or clean up somebody else's stage.
        fs::create_dir(&staging)?;
        let mut prepared = Self {
            payload: staging.join("payload"),
            root,
            staging,
            manifest: serde_json::Value::Null,
            sha256: String::new(),
        };
        private_directory(&prepared.staging)?;
        let snapshot = prepared.staging.join("package.vsix");
        prepared.sha256 = copy_verified_snapshot(archive, &snapshot, expected_sha256)?;
        prepared.manifest = read_vsix_manifest(&snapshot)?;
        extract_vsix(&snapshot, &prepared.payload, VsixLimits::default())?;
        Ok(prepared)
    }

    pub fn manifest(&self) -> &serde_json::Value {
        &self.manifest
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn payload(&self) -> &Path {
        &self.payload
    }

    pub fn resource_is_file(&self, relative: &Path) -> bool {
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return false;
        }
        fs::symlink_metadata(self.payload.join("extension").join(relative))
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
    }

    pub fn write_receipt(&self, receipt: &serde_json::Value) -> Result<(), PackageError> {
        let mut file = private_file(&self.payload.join(PACKAGE_RECEIPT_FILE))?;
        serde_json::to_writer(&mut file, receipt).map_err(io::Error::other)?;
        file.sync_all()?;
        Ok(())
    }

    pub fn publish(self, directory_name: &str) -> Result<PathBuf, PackageError> {
        self.publish_with_sync(directory_name, sync_directory)
    }

    pub(super) fn publish_with_sync(
        self,
        directory_name: &str,
        sync_parent: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<PathBuf, PackageError> {
        let name =
            validate_entry_path(directory_name).map_err(|_| PackageError::InvalidDestination)?;
        if name.components().count() != 1 {
            return Err(PackageError::InvalidDestination);
        }
        let destination = self.root.join(name);
        let lock_path = self.root.join(".install.lock");
        if fs::symlink_metadata(&lock_path)
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err(PackageError::InvalidLayout);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let lock = options.open(lock_path)?;
        lock.lock_exclusive()?;
        match fs::symlink_metadata(&destination) {
            Ok(_) => return Err(PackageError::AlreadyInstalled),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        sync_tree(&self.payload)?;
        fs::rename(&self.payload, &destination)?;
        if let Err(error) = sync_parent(&self.root) {
            // No registration points here yet, so an unpublished install can be undone.
            fs::rename(&destination, &self.payload).map_err(PackageError::RollbackFailed)?;
            sync_directory(&self.root)?;
            return Err(error.into());
        }
        Ok(destination)
    }
}

impl Drop for StagedVsix {
    fn drop(&mut self) {
        if fs::remove_dir_all(&self.staging).is_err() {
            tracing::warn!(
                error_classification = "extension_staging_cleanup_failed",
                "extension staging cleanup failed"
            );
        }
    }
}

pub(super) fn private_directory(path: &Path) -> Result<(), PackageError> {
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PackageError::InvalidLayout);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub(super) fn private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

fn copy_verified_snapshot(
    source: &Path,
    destination: &Path,
    expected: &str,
) -> Result<String, PackageError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut input = options.open(source)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() {
        return Err(PackageError::InvalidSource);
    }
    if metadata.len() > MAX_PACKAGE_BYTES {
        return Err(PackageError::TooLarge);
    }
    let mut output = private_file(destination)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        if bytes > MAX_PACKAGE_BYTES {
            return Err(PackageError::TooLarge);
        }
        hasher.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    let sha256 = format!("{:x}", hasher.finalize());
    if !sha256.eq_ignore_ascii_case(expected) {
        return Err(PackageError::ChecksumMismatch);
    }
    output.sync_all()?;
    Ok(sha256)
}

fn sync_tree(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            sync_tree(&entry.path())?;
        } else if kind.is_file() {
            let mut options = OpenOptions::new();
            options.write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW);
            }
            options.open(entry.path())?.sync_all()?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "staging contains a non-regular entry",
            ));
        }
    }
    sync_directory(path)
}

#[cfg(unix)]
pub(super) fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}
#[cfg(not(unix))]
pub(super) fn sync_directory(_path: &Path) -> io::Result<()> {
    // Windows has no supported directory fsync; individual files are synced.
    Ok(())
}
