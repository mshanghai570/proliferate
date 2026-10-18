use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateLocalProviderRequest {
    pub name: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub api_format: String,
    pub api_key: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalProviderSummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub model_count: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalProvidersResponse {
    pub providers: Vec<LocalProviderSummary>,
    pub encryption_available: bool,
    pub managed_auth_active: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetLocalProviderEnabledRequest {
    pub enabled: bool,
}
