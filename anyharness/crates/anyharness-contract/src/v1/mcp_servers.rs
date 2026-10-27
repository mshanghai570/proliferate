use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

/// One env variable or header the user attaches to a server. The value is a
/// secret at rest; it never appears in summaries, logs, or listings.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpServerCredential {
    pub name: String,
    pub value: String,
}

impl std::fmt::Debug for McpServerCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "McpServerCredential({})", self.name)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", tag = "transport", deny_unknown_fields)]
pub enum McpServerConfig {
    /// A server started as a child process by the harness at session launch.
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env: Vec<McpServerCredential>,
    },
    /// A reachable HTTP MCP endpoint.
    Http {
        url: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        headers: Vec<McpServerCredential>,
    },
}

impl std::fmt::Debug for McpServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio { args, env, .. } => f
                .debug_struct("McpServerConfig::Stdio")
                .field("command", &"<redacted>")
                .field("arg_count", &args.len())
                .field("env_count", &env.len())
                .finish(),
            Self::Http { headers, .. } => f
                .debug_struct("McpServerConfig::Http")
                .field("url", &"<redacted>")
                .field("header_count", &headers.len())
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpServerTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateMcpServerRequest {
    pub server_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub config: McpServerConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpServerSummary {
    pub id: String,
    pub server_name: String,
    pub transport: McpServerTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub env_names: Vec<String>,
    pub header_names: Vec<String>,
    /// Null means inherited off.
    pub user_enabled: Option<bool>,
    pub repository_overrides: BTreeMap<String, bool>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListMcpServersResponse {
    pub servers: Vec<McpServerSummary>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SetMcpServerEnabledRequest {
    /// A JSON boolean updates the runtime user's default.
    UserDefault(bool),
    /// A repository-specific boolean overrides the default; null resets it.
    RepositoryOverride(McpServerRepositoryOverride),
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpServerRepositoryOverride {
    pub repo_root_id: String,
    /// Required, but nullable: null removes only this repository override.
    #[serde(deserialize_with = "deserialize_nullable_bool")]
    #[schema(required)]
    pub enabled: Option<bool>,
}

fn deserialize_nullable_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_debug_never_shows_value() {
        let credential = McpServerCredential {
            name: "TAVILY_API_KEY".to_string(),
            value: "tvly-secret".to_string(),
        };
        let rendered = format!("{credential:?}");
        assert!(!rendered.contains("tvly-secret"));
        assert!(rendered.contains("TAVILY_API_KEY"));
    }

    #[test]
    fn stdio_config_debug_redacts_env_values() {
        let config = McpServerConfig::Stdio {
            command: "mcp-server-filesystem".to_string(),
            args: vec!["/workspace".to_string()],
            env: vec![McpServerCredential {
                name: "SECRET".to_string(),
                value: "top-secret".to_string(),
            }],
        };
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("mcp-server-filesystem"));
        assert!(!rendered.contains("/workspace"));
        assert!(!rendered.contains("top-secret"));
    }

    #[test]
    fn http_config_debug_redacts_endpoint_and_header_values() {
        let config = McpServerConfig::Http {
            url: "https://user:password@example.invalid/?token=query-secret".to_string(),
            headers: vec![McpServerCredential {
                name: "Authorization".to_string(),
                value: "header-secret".to_string(),
            }],
        };
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("password"));
        assert!(!rendered.contains("query-secret"));
        assert!(!rendered.contains("header-secret"));
    }

    #[test]
    fn enablement_request_accepts_only_boolean_or_repository_override_shape() {
        assert!(matches!(
            serde_json::from_str::<SetMcpServerEnabledRequest>("true"),
            Ok(SetMcpServerEnabledRequest::UserDefault(true))
        ));
        assert!(matches!(
            serde_json::from_str::<SetMcpServerEnabledRequest>(
                r#"{"repoRootId":"repo-1","enabled":false}"#
            ),
            Ok(SetMcpServerEnabledRequest::RepositoryOverride(
                McpServerRepositoryOverride {
                    repo_root_id,
                    enabled: Some(false),
                }
            )) if repo_root_id == "repo-1"
        ));
        assert!(matches!(
            serde_json::from_str::<SetMcpServerEnabledRequest>(
                r#"{"repoRootId":"repo-1","enabled":null}"#
            ),
            Ok(SetMcpServerEnabledRequest::RepositoryOverride(
                McpServerRepositoryOverride {
                    repo_root_id,
                    enabled: None,
                }
            )) if repo_root_id == "repo-1"
        ));
        assert!(
            serde_json::from_str::<SetMcpServerEnabledRequest>(r#"{"repoRootId":"repo-1"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<SetMcpServerEnabledRequest>(r#"{"enabled":true}"#).is_err());
    }
}
