use super::manifest::ExtensionPackageIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionState {
    Installing,
    Installed,
    Removing,
    Error,
}

#[derive(Debug, Clone)]
pub struct RegisteredExtension {
    pub identity: ExtensionPackageIdentity,
    pub sha256: String,
    pub state: ExtensionState,
    /// No contribution loader exists. Materialized packages always stay inactive.
    pub active: bool,
    pub error_code: Option<String>,
    pub(crate) removal_id: Option<String>,
}
