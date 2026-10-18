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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionColorTheme {
    pub label: String,
    /// Normalized light/dark mode; absent when the theme cannot be imported.
    pub color_mode: Option<String>,
    /// Only the six supported core app color keys are exposed.
    pub colors: std::collections::BTreeMap<String, String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListExtensionColorThemesResponse {
    pub themes: Vec<ExtensionColorTheme>,
    pub truncated: bool,
}
