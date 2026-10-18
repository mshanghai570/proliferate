use std::path::{Path, PathBuf};

use thiserror::Error;
use uuid::Uuid;

use super::installer::{install_local_vsix, InstallError, PackageReceipt};
use super::manifest::{inspect_install_manifest, ExtensionPackageIdentity, ManifestError};
use super::model::{ExtensionState, RegisteredExtension};
use super::store::ExtensionStore;
use crate::adapters::extensions::management::PackageDirectory;
use crate::adapters::extensions::package::PackageError;
use crate::persistence::Db;

#[derive(Debug, Error)]
pub enum ManagementError {
    #[error("extension database operation failed: {0}")]
    Database(#[from] anyhow::Error),
    #[error("extension package operation failed: {0}")]
    Package(#[from] PackageError),
    #[error("extension installation failed: {0}")]
    Install(#[from] InstallError),
    #[error("invalid extension identity: {0}")]
    Manifest(#[from] ManifestError),
    #[error("extension version is already registered or has an unfinished operation")]
    AlreadyRegistered,
    #[error("package receipt or manifest does not match its registration")]
    InvalidPackage,
    #[error("extension version is not installed")]
    NotInstalled,
}

/// Blocking use cases. Transports must call these off their async executor.
pub struct ExtensionManager {
    store: ExtensionStore,
    home: PathBuf,
}

impl ExtensionManager {
    pub fn new(db: Db, home: PathBuf) -> Self {
        Self {
            store: ExtensionStore(db),
            home,
        }
    }

    #[tracing::instrument(skip_all)]
    /// Runtime home that holds the extension package directory.
    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn list(&self) -> Result<Vec<RegisteredExtension>, ManagementError> {
        let packages = PackageDirectory::lock(&self.home)?;
        self.recover(&packages)?;
        Ok(self.store.list()?)
    }

    #[tracing::instrument(skip_all)]
    pub fn color_themes(
        &self,
        identity: &ExtensionPackageIdentity,
    ) -> Result<(Vec<super::themes::PackageColorTheme>, bool), ManagementError> {
        let name = identity.directory_name()?;
        let packages = PackageDirectory::lock(&self.home)?;
        self.recover(&packages)?;
        let row = self
            .store
            .list()?
            .into_iter()
            .find(|row| row.identity == *identity && row.state == ExtensionState::Installed)
            .ok_or(ManagementError::NotInstalled)?;
        self.validate_package(&packages, &row)?;
        Ok(super::themes::read_color_themes(
            &packages,
            &name,
            &packages.read_manifest(&name)?,
        ))
    }

    #[tracing::instrument(skip_all)]
    pub fn syntax_themes(
        &self,
        identity: &ExtensionPackageIdentity,
    ) -> Result<(Vec<super::syntax_themes::PackageSyntaxTheme>, bool), ManagementError> {
        let name = identity.directory_name()?;
        let packages = PackageDirectory::lock(&self.home)?;
        self.recover(&packages)?;
        let row = self
            .store
            .list()?
            .into_iter()
            .find(|row| row.identity == *identity && row.state == ExtensionState::Installed)
            .ok_or(ManagementError::NotInstalled)?;
        self.validate_package(&packages, &row)?;
        Ok(super::syntax_themes::read_syntax_themes(
            &packages,
            &name,
            &packages.read_manifest(&name)?,
        ))
    }

    #[tracing::instrument(skip_all)]
    pub fn install(
        &self,
        archive: &Path,
        identity: &ExtensionPackageIdentity,
        sha256: &str,
    ) -> Result<RegisteredExtension, ManagementError> {
        let name = identity.directory_name()?;
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PackageError::InvalidChecksum.into());
        }
        let packages = PackageDirectory::lock(&self.home)?;
        self.recover(&packages)?;
        if self
            .store
            .list()?
            .iter()
            .any(|row| row.identity == *identity)
        {
            return Err(ManagementError::AlreadyRegistered);
        }
        if packages.exists(&name)? {
            return Err(PackageError::AlreadyInstalled.into());
        }
        let sha256 = sha256.to_ascii_lowercase();
        self.store.reserve(identity, &sha256)?;
        if let Err(error) = install_local_vsix(&self.home, archive, identity, &sha256) {
            // An incomplete rollback retains its intent for inspection/recovery.
            if !packages.exists(&name)? {
                self.store.delete(identity)?;
            }
            return Err(error.into());
        }
        self.store.set_state(identity, "installed", None, None)?;
        Ok(RegisteredExtension {
            identity: identity.clone(),
            sha256,
            state: ExtensionState::Installed,
            active: false,
            error_code: None,
            removal_id: None,
        })
    }

    #[tracing::instrument(skip_all)]
    pub fn uninstall(&self, identity: &ExtensionPackageIdentity) -> Result<(), ManagementError> {
        let name = identity.directory_name()?;
        let packages = PackageDirectory::lock(&self.home)?;
        self.recover(&packages)?;
        let Some(row) = self
            .store
            .list()?
            .into_iter()
            .find(|row| row.identity == *identity)
        else {
            return Ok(());
        };
        if packages.exists(&name)? {
            self.validate_receipt(&packages, &row)?;
        }
        let removal_id = Uuid::new_v4().to_string();
        self.store
            .set_state(identity, "removing", None, Some(&removal_id))?;
        packages.quarantine(&name, &removal_id)?;
        packages.delete_quarantine(&removal_id)?;
        self.store.delete(identity)?;
        Ok(())
    }

    fn recover(&self, packages: &PackageDirectory) -> Result<(), ManagementError> {
        for row in self.store.list()? {
            let name = row.identity.directory_name()?;
            if row.state == ExtensionState::Removing {
                let removal_id = row
                    .removal_id
                    .as_deref()
                    .ok_or(ManagementError::InvalidPackage)?;
                if packages.exists(&name)? {
                    self.validate_receipt(packages, &row)?;
                }
                packages.quarantine(&name, removal_id)?;
                packages.delete_quarantine(removal_id)?;
                self.store.delete(&row.identity)?;
                continue;
            }
            let validation = self.validate_package(packages, &row);
            let error_code = match validation {
                Ok(()) => None,
                Err(ManagementError::Package(PackageError::Io(error)))
                    if error.kind() != std::io::ErrorKind::NotFound
                        && error.kind() != std::io::ErrorKind::InvalidData =>
                {
                    return Err(PackageError::Io(error).into());
                }
                Err(_) => {
                    let missing = matches!(packages.exists(&name), Ok(false));
                    Some(
                        if missing
                            && (row.state == ExtensionState::Installing
                                || row.error_code.as_deref() == Some("interrupted_install"))
                        {
                            "interrupted_install"
                        } else if missing {
                            "package_missing"
                        } else {
                            "package_invalid"
                        },
                    )
                }
            };
            let state = if error_code.is_some() {
                ExtensionState::Error
            } else {
                ExtensionState::Installed
            };
            if row.state != state || row.error_code.as_deref() != error_code {
                self.store.set_state(
                    &row.identity,
                    if error_code.is_some() {
                        "error"
                    } else {
                        "installed"
                    },
                    error_code,
                    None,
                )?;
            }
        }
        Ok(())
    }

    fn validate_receipt(
        &self,
        packages: &PackageDirectory,
        row: &RegisteredExtension,
    ) -> Result<(), ManagementError> {
        let name = row.identity.directory_name()?;
        let receipt: PackageReceipt = serde_json::from_value(packages.read_receipt(&name)?)
            .map_err(|_| ManagementError::InvalidPackage)?;
        if receipt.schema_version != 1
            || receipt.identity != row.identity
            || receipt.sha256 != row.sha256
        {
            return Err(ManagementError::InvalidPackage);
        }
        Ok(())
    }

    fn validate_package(
        &self,
        packages: &PackageDirectory,
        row: &RegisteredExtension,
    ) -> Result<(), ManagementError> {
        self.validate_receipt(packages, row)?;
        let name = row.identity.directory_name()?;
        let resources = inspect_install_manifest(&packages.read_manifest(&name)?, &row.identity)?;
        if resources
            .iter()
            .any(|path| !packages.resource_is_file(&name, path))
        {
            return Err(ManagementError::InvalidPackage);
        }
        Ok(())
    }
}
