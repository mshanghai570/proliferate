use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRules {
    pub agent_kind: String,
    pub repository_id: Option<String>,
    pub text: String,
    pub inherited: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAgentRulesRequest {
    pub repository_id: Option<String>,
    /// Null removes the scope's file. An empty string explicitly disables rules.
    #[serde(deserialize_with = "required_text")]
    #[schema(required = true)]
    pub text: Option<String>,
}

fn required_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}
