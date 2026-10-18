use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderInput {
    pub name: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub api_format: String,
    pub api_key: Option<String>,
}

#[derive(Clone)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub model_count: usize,
}

pub struct ProviderInventory {
    pub providers: Vec<ProviderSummary>,
    pub encryption_available: bool,
    pub managed_auth_active: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SealedProvider {
    pub id: String,
    pub input: ProviderInput,
}
