use rusqlite::{params, OptionalExtension};

use crate::persistence::Db;

#[derive(Clone)]
pub(super) struct ProjectMcpStore(pub Db);

pub(super) struct StoredServer {
    pub name: String,
    pub transport: String,
    pub enabled: bool,
    pub ciphertext: String,
}

impl ProjectMcpStore {
    pub fn list(&self, repo: &str) -> anyhow::Result<Vec<StoredServer>> {
        self.0.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT name, transport, enabled, ciphertext FROM project_mcp_servers WHERE repo_root_id = ?1 ORDER BY name")?;
            let rows = stmt.query_map([repo], |row| Ok(StoredServer {
                name: row.get(0)?, transport: row.get(1)?, enabled: row.get(2)?, ciphertext: row.get(3)?,
            }))?;
            rows.collect()
        })
    }

    pub fn put(&self, repo: &str, row: &StoredServer) -> anyhow::Result<bool> {
        self.0.with_conn(|conn| {
            conn.execute(
                "INSERT INTO project_mcp_servers (repo_root_id, name, transport, enabled, ciphertext)
                 SELECT ?1, ?2, ?3, ?4, ?5 WHERE
                   (SELECT COUNT(*) FROM project_mcp_servers WHERE repo_root_id = ?1) < 32
                   OR EXISTS (SELECT 1 FROM project_mcp_servers WHERE repo_root_id = ?1 AND name = ?2)
                 ON CONFLICT(repo_root_id, name) DO UPDATE SET transport = excluded.transport,
                   enabled = excluded.enabled, ciphertext = excluded.ciphertext",
                params![repo, row.name, row.transport, row.enabled, row.ciphertext],
            ).map(|count| count > 0)
        })
    }

    pub fn set_enabled(
        &self,
        repo: &str,
        name: &str,
        enabled: bool,
    ) -> anyhow::Result<Option<String>> {
        self.0.with_conn(|conn| conn.query_row(
            "UPDATE project_mcp_servers SET enabled = ?3 WHERE repo_root_id = ?1 AND name = ?2 RETURNING transport",
            params![repo, name, enabled], |row| row.get(0),
        ).optional())
    }

    pub fn remove(&self, repo: &str, name: &str) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            conn.execute(
                "DELETE FROM project_mcp_servers WHERE repo_root_id = ?1 AND name = ?2",
                params![repo, name],
            )
            .map(|_| ())
        })
    }
}
