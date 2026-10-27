use std::collections::BTreeMap;
use std::fmt;

use anyharness_contract::v1::mcp_servers::McpServerTransport;

pub const MAX_MCP_SERVERS: usize = 32;

/// A durable runtime-local user MCP server record. Secrets live only in the
/// sealed envelopes; every plaintext field is safe to render and log.
#[derive(Clone, PartialEq, Eq)]
pub struct UserMcpServer {
    pub id: String,
    pub server_name: String,
    pub transport: McpServerTransport,
    pub description: Option<String>,
    pub stdio_command: Option<String>,
    pub stdio_args: Vec<String>,
    /// JSON credential metadata with each secret value sealed by SessionDataCipher.
    pub env_sealed: Option<String>,
    pub url: Option<String>,
    /// JSON credential metadata with each secret value sealed by SessionDataCipher.
    pub headers_sealed: Option<String>,
    pub enabled_user_default: Option<bool>,
    pub repository_overrides: BTreeMap<String, bool>,
    pub created_at: String,
    pub updated_at: String,
}

impl fmt::Debug for UserMcpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserMcpServer")
            .field("id", &self.id)
            .field("server_name", &self.server_name)
            .field("transport", &self.transport)
            .field("description_configured", &self.description.is_some())
            .field("stdio_command_configured", &self.stdio_command.is_some())
            .field("stdio_arg_count", &self.stdio_args.len())
            .field("env_configured", &self.env_sealed.is_some())
            .field("url_configured", &self.url.is_some())
            .field("headers_configured", &self.headers_sealed.is_some())
            .field("enabled_user_default", &self.enabled_user_default)
            .field("repository_overrides", &self.repository_overrides)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl UserMcpServer {
    /// Skills' precedence verbatim: an explicit repository boolean wins over
    /// the user default; absence inherits the default, which may be off.
    pub fn is_selected_for(&self, repo_root_id: &str) -> bool {
        self.repository_overrides
            .get(repo_root_id)
            .copied()
            .unwrap_or(self.enabled_user_default.unwrap_or(false))
    }

    pub fn summary(
        &self,
        env_names: &[String],
        header_names: &[String],
    ) -> anyharness_contract::v1::mcp_servers::McpServerSummary {
        use anyharness_contract::v1::mcp_servers::McpServerSummary;
        McpServerSummary {
            id: self.id.clone(),
            server_name: self.server_name.clone(),
            transport: match self.transport {
                McpServerTransport::Stdio => McpServerTransport::Stdio,
                McpServerTransport::Http => McpServerTransport::Http,
            },
            description: self.description.clone(),
            // Launch configuration can contain local paths or embedded secrets.
            command: None,
            args: None,
            url: None,
            env_names: env_names.to_vec(),
            header_names: header_names.to_vec(),
            user_enabled: self.enabled_user_default,
            repository_overrides: self.repository_overrides.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}
