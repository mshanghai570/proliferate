use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use fs2::FileExt;
use serde_json::Value;
use uuid::Uuid;

use super::package::{private_directory, sync_directory, PackageError, PACKAGE_RECEIPT_FILE};
use super::vsix::{validate_entry_path, MAX_VSIX_MANIFEST_BYTES};

/// Serializes durable lifecycle operations across manager instances/processes.
/// The low-level installer separately locks publication, always acquired second.
pub struct PackageDirectory {
    root: PathBuf,
    _lock: File,
}

impl PackageDirectory {
    pub fn lock(home: &Path) -> Result<Self, PackageError> {
        let home = fs::canonicalize(home)?;
        let extensions = home.join("extensions");
        private_directory(&extensions)?;
        let root = extensions.join("packages");
        private_directory(&root)?;
        let path = root.join(".management.lock");
        if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
            return Err(PackageError::InvalidLayout);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let lock = options.open(path)?;
        if !lock.metadata()?.is_file() {
            return Err(PackageError::InvalidLayout);
        }
        lock.lock_exclusive()?;
        Ok(Self { root, _lock: lock })
    }

    pub fn exists(&self, name: &str) -> Result<bool, PackageError> {
        let path = self.package_path(name)?;
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(true),
            Ok(_) => Err(PackageError::InvalidLayout),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub fn read_receipt(&self, name: &str) -> Result<Value, PackageError> {
        self.read_json(name, Path::new(PACKAGE_RECEIPT_FILE), 64 * 1024)
    }

    pub fn read_manifest(&self, name: &str) -> Result<Value, PackageError> {
        self.read_json(
            name,
            Path::new("extension/package.json"),
            MAX_VSIX_MANIFEST_BYTES,
        )
    }

    pub fn resource_is_file(&self, name: &str, relative: &Path) -> bool {
        self.regular_file(name, &Path::new("extension").join(relative))
            .is_ok()
    }

    pub fn quarantine(&self, name: &str, removal_id: &str) -> Result<(), PackageError> {
        let source = self.package_path(name)?;
        let target = self.removal_path(removal_id)?;
        if self.exists(name)? {
            match fs::symlink_metadata(&target) {
                Ok(_) => return Err(PackageError::AlreadyInstalled),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            fs::rename(source, target)?;
        }
        sync_directory(&self.root)?;
        Ok(())
    }

    pub fn delete_quarantine(&self, removal_id: &str) -> Result<(), PackageError> {
        let path = self.removal_path(removal_id)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                fs::remove_dir_all(path)?;
            }
            Ok(_) => return Err(PackageError::InvalidLayout),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        sync_directory(&self.root)?;
        Ok(())
    }

    fn removal_path(&self, id: &str) -> Result<PathBuf, PackageError> {
        let id = Uuid::parse_str(id).map_err(|_| PackageError::InvalidDestination)?;
        Ok(self.root.join(format!(".remove-{id}")))
    }

    fn package_path(&self, name: &str) -> Result<PathBuf, PackageError> {
        let relative = validate_entry_path(name).map_err(|_| PackageError::InvalidDestination)?;
        if relative.components().count() != 1
            || relative.to_str() != Some(name)
            || name.starts_with('.')
        {
            return Err(PackageError::InvalidDestination);
        }
        Ok(self.root.join(relative))
    }

    fn regular_file(&self, name: &str, relative: &Path) -> Result<PathBuf, PackageError> {
        if !self.exists(name)? {
            return Err(io::Error::from(io::ErrorKind::NotFound).into());
        }
        let mut path = self.package_path(name)?;
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(component) = component else {
                return Err(PackageError::InvalidLayout);
            };
            path.push(component);
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink()
                || (components.peek().is_some() && !metadata.is_dir())
                || (components.peek().is_none() && !metadata.is_file())
            {
                return Err(PackageError::InvalidLayout);
            }
        }
        Ok(path)
    }

    fn read_json(&self, name: &str, relative: &Path, limit: u64) -> Result<Value, PackageError> {
        let path = self.regular_file(name, relative)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(path)?;
        if !file.metadata()?.is_file() {
            return Err(PackageError::InvalidSource);
        }
        let mut bytes = Vec::new();
        file.take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(PackageError::TooLarge);
        }
        serde_json::from_slice(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e).into())
    }
}
