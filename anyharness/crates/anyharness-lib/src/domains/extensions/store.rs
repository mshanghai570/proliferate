use rusqlite::{params, Row};

use super::manifest::ExtensionPackageIdentity;
use super::model::{ExtensionState, RegisteredExtension};
use crate::persistence::Db;

pub(super) struct ExtensionStore(pub Db);

impl ExtensionStore {
    pub fn list(&self) -> anyhow::Result<Vec<RegisteredExtension>> {
        self.0.with_conn(|conn| {
            let mut statement = conn.prepare(
                "SELECT namespace, name, version, sha256, state, error_code, removal_id
                 FROM installed_extensions ORDER BY namespace, name, version",
            )?;
            let rows = statement.query_map([], map_row)?;
            rows.collect()
        })
    }

    pub fn reserve(&self, identity: &ExtensionPackageIdentity, sha256: &str) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            conn.execute(
                "INSERT INTO installed_extensions (namespace, name, version, sha256, state)
                 VALUES (?1, ?2, ?3, ?4, 'installing')",
                params![identity.namespace, identity.name, identity.version, sha256],
            )?;
            Ok(())
        })
    }

    pub fn set_state(
        &self,
        identity: &ExtensionPackageIdentity,
        state: &str,
        error: Option<&str>,
        removal_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE installed_extensions SET state = ?4, error_code = ?5, removal_id = ?6,
                 updated_at = datetime('now') WHERE namespace = ?1 AND name = ?2 AND version = ?3",
                params![
                    identity.namespace,
                    identity.name,
                    identity.version,
                    state,
                    error,
                    removal_id
                ],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    pub fn delete(&self, identity: &ExtensionPackageIdentity) -> anyhow::Result<()> {
        self.0.with_conn(|conn| {
            conn.execute(
                "DELETE FROM installed_extensions WHERE namespace = ?1 AND name = ?2 AND version = ?3",
                params![identity.namespace, identity.name, identity.version],
            )?;
            Ok(())
        })
    }
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<RegisteredExtension> {
    let state: String = row.get(4)?;
    let state = match state.as_str() {
        "installing" => ExtensionState::Installing,
        "installed" => ExtensionState::Installed,
        "removing" => ExtensionState::Removing,
        "error" => ExtensionState::Error,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(RegisteredExtension {
        identity: ExtensionPackageIdentity {
            namespace: row.get(0)?,
            name: row.get(1)?,
            version: row.get(2)?,
        },
        sha256: row.get(3)?,
        state,
        active: false,
        error_code: row.get(5)?,
        removal_id: row.get(6)?,
    })
}
