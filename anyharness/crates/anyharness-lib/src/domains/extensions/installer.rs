use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::manifest::{inspect_install_manifest, ExtensionPackageIdentity, ManifestError};
use crate::adapters::extensions::package::{PackageError, StagedVsix};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageReceipt {
    pub schema_version: u8,
    pub identity: ExtensionPackageIdentity,
    pub sha256: String,
}

pub struct InstalledPackage {
    pub receipt: PackageReceipt,
    pub directory: PathBuf,
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("package preparation or publication failed: {0}")]
    Package(#[from] PackageError),
    #[error("package is not eligible for installation: {0}")]
    Manifest(#[from] ManifestError),
    #[error("a declared contribution resource is missing or is not a regular file")]
    MissingResource,
    #[error("package receipt could not be serialized: {0}")]
    Receipt(#[from] serde_json::Error),
}

/// Blocking filesystem work: runtime transports must dispatch this off their
/// async executor. Publication never enables or executes an extension.
#[tracing::instrument(skip_all)]
pub fn install_local_vsix(
    runtime_home: &Path,
    archive: &Path,
    expected: &ExtensionPackageIdentity,
    expected_sha256: &str,
) -> Result<InstalledPackage, InstallError> {
    let directory_name = expected.directory_name()?;
    let staged = StagedVsix::prepare(archive, runtime_home, expected_sha256)?;
    let resources = inspect_install_manifest(staged.manifest(), expected)?;
    for resource in resources {
        if !staged.resource_is_file(&resource) {
            return Err(InstallError::MissingResource);
        }
    }
    let receipt = PackageReceipt {
        schema_version: 1,
        identity: expected.clone(),
        sha256: staged.sha256().into(),
    };
    staged.write_receipt(&serde_json::to_value(&receipt)?)?;
    let directory = staged.publish(&directory_name)?;
    Ok(InstalledPackage { receipt, directory })
}
