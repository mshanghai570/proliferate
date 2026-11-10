use std::collections::BTreeMap;

use rusqlite::Row;

use super::model::{UserMcpServer, MAX_MCP_SERVERS};
use anyharness_contract::v1::mcp_servers::McpServerTransport;
use crate::persistence::Db;

pub(super) struct McpServerStore(pub Db);

impl McpServerStore {
    pub fn list(&self) -> anyhow::Result<Vec<UserMcpServer>> {
        self.0.with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT id, server_name, transport, description, stdio_command, stdio_args_json,
                        env_sealed, url, headers_sealed, enabled_user_default,
                        repository_overrides_json, created_at, updated_at
                 FROM user_mcp_servers ORDER BY server_name",
            )?;
            let rows = statement.query_map([], map_row)?;
            rows.collect()
        })
    }

    pub fn insert(
        &self,
        record: &UserMcpServer,
        args_json: &str,
        overrides_json: &str,
        env_sealed: Option<&str>,
        headers_sealed: Option<&str>,
    ) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT count(*) FROM user_mcp_servers",
                [],
                |row| row.get(0),
            )?;
            if count >= i64::try_from(MAX_MCP_SERVERS).expect("fit") {
                return Err(rusqlite::Error::ToSqlConversionFailure(
                    format!(
                        "Remove a server before managing more than {MAX_MCP_SERVERS} records"
                    )
                    .into(),
                ));
            }
            let transport_text: &'static str = match record.transport {
                McpServerTransport::Stdio => "stdio",
                McpServerTransport::Http => "http",
            };
            conn.execute(
                "INSERT INTO user_mcp_servers
                    (id, server_name, transport, description, stdio_command, stdio_args_json,
                     env_sealed, url, headers_sealed, enabled_user_default, repository_overrides_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    record.id,
                    record.server_name,
                    transport_text,
                    record.description,
                    record.stdio_command,
                    args_json,
                    env_sealed,
                    record.url,
                    headers_sealed,
                    record.enabled_user_default,
                    overrides_json,
                ],
            )?;
            Ok(())
        })
    }

    pub fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            let changed = conn.execute(
                "DELETE FROM user_mcp_servers WHERE id = ?1",
                rusqlite::params![id],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    pub fn set_user_default(&self, id: &str, enabled: bool) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE user_mcp_servers SET enabled_user_default = ?2,
                    updated_at = datetime('now') WHERE id = ?1",
                rusqlite::params![id, enabled as i64],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    pub fn set_repository_override(
        &self,
        id: &str,
        overrides_json: &str,
    ) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE user_mcp_servers SET repository_overrides_json = ?2,
                    updated_at = datetime('now') WHERE id = ?1",
                rusqlite::params![id, overrides_json],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<UserMcpServer> {
    let transport_text: String = row.get("transport")?;
    let transport = match transport_text.as_str() {
        "stdio" => McpServerTransport::Stdio,
        "http" => McpServerTransport::Http,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                "unknown user MCP transport".into(),
            ))
        }
    };
    let args_json: String = row.get("stdio_args_json")?;
    let overrides_json: String = row.get("repository_overrides_json")?;
    let decode_string_list = |value: &str, what: &'static str| -> rusqlite::Result<Vec<String>> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(trimmed)
            .map_err(|error| invalid_state(what, error))
    };
    let decode_overrides =
        |value: &str| -> rusqlite::Result<BTreeMap<String, bool>> {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(BTreeMap::new());
            }
            serde_json::from_str(trimmed).map_err(|error| invalid_state("Repository overrides", error))
        };
    // Corruption fails the read rather than silently dropping durable
    // selections.
    let stdio_args = decode_string_list(&args_json, "Command arguments")?;
    let repository_overrides = decode_overrides(&overrides_json)?;
    Ok(UserMcpServer {
        id: row.get("id")?,
        server_name: row.get("server_name")?,
        transport,
        description: row.get("description")?,
        stdio_command: row.get("stdio_command")?,
        stdio_args,
        env_sealed: row.get("env_sealed")?,
        url: row.get("url")?,
        headers_sealed: row.get("headers_sealed")?,
        enabled_user_default: row
            .get::<_, Option<i64>>("enabled_user_default")?
            .map(|value| value != 0),
        repository_overrides,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn invalid_state(what: &str, error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("{what} is invalid: {error}").into(),
    )
}
