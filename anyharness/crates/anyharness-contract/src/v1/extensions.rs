use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionInstallationState {
    Installing,
    Installed,
    Removing,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionPackageErrorCode {
    InterruptedInstall,
    PackageMissing,
    PackageInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstalledExtensionSummary {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub state: ExtensionInstallationState,
    /// Package materialization does not imply activation. Currently always false.
    pub active: bool,
    pub error_code: Option<ExtensionPackageErrorCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstallRegistryExtensionRequest {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListInstalledExtensionsResponse {
    pub extensions: Vec<InstalledExtensionSummary>,
}
