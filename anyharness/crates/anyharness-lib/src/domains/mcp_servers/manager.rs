use std::collections::BTreeMap;

use anyhow::{bail, Context};
use anyharness_contract::v1::mcp_servers::McpServerCredential;
use serde::{Deserialize, Serialize};

use super::admission::AdmittedMcpServer;
use super::model::UserMcpServer;
use super::store::McpServerStore;
use crate::domains::sessions::mcp_bindings::crypto::{
    decrypt_bytes, encrypt_bytes, SessionDataCipher,
};
use crate::persistence::Db;

const MISSING_DATA_KEY_DETAIL: &str =
    "A runtime data key (ANYHARNESS_DATA_KEY) is required to store MCP server secrets.";

#[derive(Serialize, Deserialize)]
pub(super) struct SealedCredential {
    pub(super) name: String,
    pub(super) value: String,
}

pub(super) fn seal_credentials(
    cipher: Option<&SessionDataCipher>,
    credentials: &[anyharness_contract::v1::mcp_servers::McpServerCredential],
) -> anyhow::Result<Option<String>> {
    if credentials.is_empty() {
        return Ok(None);
    }
    let Some(cipher) = cipher else {
        bail!("{MISSING_DATA_KEY_DETAIL}");
    };
    let sealed = credentials
        .iter()
        .map(|credential| {
            Ok(SealedCredential {
                name: credential.name.clone(),
                value: encrypt_bytes(cipher, credential.value.as_bytes())?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Some(serde_json::to_string(&sealed)?))
}

pub(super) fn credential_names(sealed: Option<&str>) -> anyhow::Result<Vec<String>> {
    let Some(sealed) = sealed else {
        return Ok(Vec::new());
    };
    let encrypted_metadata: Vec<SealedCredential> =
        serde_json::from_str(sealed).context("MCP credential metadata is invalid")?;
    Ok(encrypted_metadata
        .into_iter()
        .map(|credential| credential.name)
        .collect())
}

pub(super) fn unseal_credentials(
    cipher: &SessionDataCipher,
    sealed: &str,
) -> anyhow::Result<Vec<McpServerCredential>> {
    let encrypted_metadata: Vec<SealedCredential> =
        serde_json::from_str(sealed).context("MCP credential metadata is invalid")?;
    encrypted_metadata
        .into_iter()
        .map(|credential| {
            let plaintext = decrypt_bytes(cipher, &credential.value)?;
            Ok(McpServerCredential {
                name: credential.name,
                value: String::from_utf8(plaintext).context("MCP credential is not UTF-8")?,
            })
        })
        .collect()
}

pub struct UserMcpServerManager {
    store: McpServerStore,
    cipher: Option<SessionDataCipher>,
}

impl UserMcpServerManager {
    pub fn new(db: Db, cipher: Option<SessionDataCipher>) -> Self {
        Self {
            store: McpServerStore(db),
            cipher,
        }
    }

    pub fn list(&self) -> anyhow::Result<Vec<UserMcpServer>> {
        self.store.list()
    }

    /// Listing surface: metadata plus secret NAMES only. Values are never
    /// decrypted outside the launch path.
    pub fn summaries(
        &self,
    ) -> anyhow::Result<Vec<anyharness_contract::v1::mcp_servers::McpServerSummary>> {
        let records = self.store.list()?;
        let mut out = Vec::with_capacity(records.len());
        for record in records {
            let env_names = credential_names(record.env_sealed.as_deref())?;
            let header_names = credential_names(record.headers_sealed.as_deref())?;
            out.push(record.summary(&env_names, &header_names));
        }
        Ok(out)
    }

    pub fn create(&self, config: &AdmittedMcpServer) -> anyhow::Result<UserMcpServer> {
        let env_sealed = seal_credentials(self.cipher.as_ref(), &config.env)?;
        let headers_sealed = seal_credentials(self.cipher.as_ref(), &config.headers)?;
        let record = UserMcpServer {
            id: uuid::Uuid::new_v4().to_string(),
            server_name: config.server_name.clone(),
            transport: config.transport,
            description: config.description.clone(),
            stdio_command: config.stdio_command.clone(),
            stdio_args: config.stdio_args.clone(),
            env_sealed,
            url: config.url.clone(),
            headers_sealed,
            // A new record starts disabled: an agent tool grant must be an
            // explicit enablement.
            enabled_user_default: None,
            repository_overrides: BTreeMap::new(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let args_json = serde_json::to_string(&record.stdio_args)?;
        self.store.insert(
            &record,
            &args_json,
            "{}",
            record.env_sealed.as_deref(),
            record.headers_sealed.as_deref(),
        )?;
        tracing::info!(
            event = "mcp_server.created",
            transport = ?record.transport,
            secret_count = config.env.len() + config.headers.len(),
            "User MCP server created"
        );
        self.find_by_id(&record.id)
    }

    fn find_by_id(&self, id: &str) -> anyhow::Result<UserMcpServer> {
        self.list()?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| anyhow::anyhow!("Unknown MCP server"))
    }

    pub fn set_enabled(
        &self,
        id: &str,
        repository_id: Option<&str>,
        enabled: Option<bool>,
    ) -> anyhow::Result<UserMcpServer> {
        match (repository_id, enabled) {
            (None, Some(enabled)) => {
                self.store.set_user_default(id, enabled)?;
            }
            (Some(repository), enabled) => {
                let record = self.find_by_id(id)?;
                let mut overrides = record.repository_overrides.clone();
                match enabled {
                    Some(enabled) => overrides.insert(repository.to_owned(), enabled),
                    None => overrides.remove(repository),
                };
                let overrides_json = serde_json::to_string(&overrides)?;
                self.store.set_repository_override(id, &overrides_json)?;
            }
            (None, None) => {
                bail!("Setting the user default requires a boolean");
            }
        }
        tracing::info!(
            event = "mcp_server.enablement_changed",
            "User MCP enablement changed"
        );
        self.find_by_id(id)
    }

    pub fn remove(&self, id: &str) -> anyhow::Result<()> {
        self.store.delete(id)?;
        tracing::info!(event = "mcp_server.removed", "User MCP server removed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::mcp_servers::admission::admit_create;
    use crate::persistence::Db;
    use std::fs;
    use std::path::PathBuf;

    const TEST_KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";

    fn stdio_config() -> AdmittedMcpServer {
        AdmittedMcpServer {
            server_name: "filesystem".to_string(),
            transport: anyharness_contract::v1::mcp_servers::McpServerTransport::Stdio,
            description: Some("Local files".to_string()),
            stdio_command: Some("mcp-server-filesystem".to_string()),
            stdio_args: vec!["/workspace".to_string()],
            env: vec![McpServerCredential {
                name: "FILESYSTEM_ACCESS_TOKEN".to_string(),
                value: "top-secret".to_string(),
            }],
            url: None,
            headers: Vec::new(),
        }
    }

    struct RuntimeHome(PathBuf);

    impl RuntimeHome {
        fn new() -> Self {
            let path = std::env::current_dir()
                .expect("current directory")
                .join(format!(".mcp-server-reopen-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).expect("create project-local test database directory");
            Self(path)
        }
    }

    impl Drop for RuntimeHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn create_requires_data_cipher_for_secret_bearing_servers() {
        let manager = UserMcpServerManager::new(Db::open_in_memory().unwrap(), None);
        let attempted = manager.create(&stdio_config());
        assert!(attempted.is_err());

        let mut no_secrets = stdio_config();
        no_secrets.env.clear();
        assert!(manager.create(&no_secrets).is_ok());
    }

    #[test]
    fn create_seals_secret_values_at_rest_and_summaries_only_expose_names() {
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let db = Db::open_in_memory().unwrap();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher.clone()));
        let mut config = stdio_config();
        config.stdio_command = Some("/Users/private/bin/mcp-filesystem".to_string());
        config.stdio_args = vec!["/Users/private/project".to_string()];
        let created = manager.create(&config).unwrap();
        assert!(created.enabled_user_default.is_none());
        assert!(created.env_sealed.is_some());
        assert!(!format!("{created:?}").contains("top-secret"));
        assert!(!format!("{created:?}").contains("/Users/private"));

        let stored: String = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT env_sealed FROM user_mcp_servers WHERE id = ?1",
                    [&created.id],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert!(!stored.contains("top-secret"));
        let sealed: Vec<SealedCredential> = serde_json::from_str(&stored).unwrap();
        assert_eq!(sealed.len(), 1);
        assert_eq!(sealed[0].name, "FILESYSTEM_ACCESS_TOKEN");
        assert!(sealed[0].value.starts_with("v1:"));
        assert_eq!(
            String::from_utf8(decrypt_bytes(&cipher, &sealed[0].value).unwrap()).unwrap(),
            "top-secret"
        );

        let summaries = manager.summaries().unwrap();
        assert_eq!(
            summaries[0].env_names,
            vec!["FILESYSTEM_ACCESS_TOKEN".to_string()]
        );
        assert!(summaries[0].command.is_none());
        assert!(summaries[0].args.is_none());
        assert!(!format!("{summaries:?}").contains("top-secret"));
        assert!(!serde_json::to_string(&summaries)
            .unwrap()
            .contains("/Users/private"));

        let no_key_manager = UserMcpServerManager::new(db, None);
        let no_key_summaries = no_key_manager.summaries().unwrap();
        assert_eq!(
            no_key_summaries[0].env_names,
            vec!["FILESYSTEM_ACCESS_TOKEN".to_string()]
        );
        assert!(!format!("{no_key_summaries:?}").contains("top-secret"));
    }

    #[test]
    fn sqlite_reopen_preserves_records_selections_and_sealed_credentials() {
        let home = RuntimeHome::new();
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let created_id;
        {
            let db = Db::open(&home.0).unwrap();
            let manager = UserMcpServerManager::new(db, Some(cipher.clone()));
            let mut config = stdio_config();
            config.server_name = "reopen-safe".to_string();
            config.stdio_args = vec!["/workspace".to_string()];
            created_id = manager.create(&config).unwrap().id;
            manager
                .set_enabled(&created_id, None, Some(true))
                .unwrap();
            manager
                .set_enabled(&created_id, Some("repo-1"), Some(false))
                .unwrap();
        }

        {
            let db = Db::open(&home.0).unwrap();
            let manager = UserMcpServerManager::new(db, Some(cipher));
            let reopened = manager.list().unwrap();
            assert_eq!(reopened.len(), 1);
            assert_eq!(reopened[0].id, created_id);
            assert_eq!(reopened[0].server_name, "reopen-safe");
            assert_eq!(reopened[0].stdio_args, vec!["/workspace".to_string()]);
            assert_eq!(reopened[0].enabled_user_default, Some(true));
            assert_eq!(
                reopened[0].repository_overrides.get("repo-1"),
                Some(&false)
            );
            assert!(reopened[0].env_sealed.as_deref().unwrap().contains("v1:"));
            assert!(!format!("{reopened:?}").contains("top-secret"));
        }
    }

    #[test]
    fn selection_precedence_defaults_overrides_and_reset() {
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let manager = UserMcpServerManager::new(Db::open_in_memory().unwrap(), Some(cipher.clone()));
        let created = manager.create(&stdio_config()).unwrap();
        assert!(!created.is_selected_for("repo-1"));

        let set_default = manager
            .set_enabled(&created.id, Some("repo-1"), Some(true))
            .unwrap();
        assert_eq!(set_default.repository_overrides.get("repo-1"), Some(&true));
        assert!(!set_default.is_selected_for("repo-2"), "other repos inherit off");

        let reset = manager.set_enabled(&created.id, Some("repo-1"), None).unwrap();
        assert!(!reset.repository_overrides.contains_key("repo-1"));

        let enabled = manager.set_enabled(&created.id, None, Some(true)).unwrap();
        assert!(enabled.is_selected_for("repo-2"));
        let overridden_off = manager
            .set_enabled(&created.id, Some("repo-1"), Some(false))
            .unwrap();
        assert!(!overridden_off.is_selected_for("repo-1"));
        assert!(overridden_off.is_selected_for("repo-2"));
    }

    #[test]
    fn capacity_is_32_and_delete_releases_a_slot() {
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let manager = UserMcpServerManager::new(Db::open_in_memory().unwrap(), Some(cipher.clone()));
        for index in 0..crate::domains::mcp_servers::model::MAX_MCP_SERVERS {
            let mut config = stdio_config();
            config.server_name = format!("tool-{index}");
            config.stdio_args = Vec::new();
            config.env = Vec::new();
            manager.create(&config).unwrap();
        }
        let mut overflow = stdio_config();
        overflow.server_name = "overflow".into();
        assert!(manager.create(&overflow).is_err());

        let first = &manager.list().unwrap()[0];
        manager.remove(&first.id).unwrap();
        let mut after_delete = stdio_config();
        after_delete.server_name = "replacement".to_string();
        after_delete.env.clear();
        manager.create(&after_delete).unwrap();
        assert_eq!(manager.list().unwrap().len(), 32);
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let mut config = stdio_config();
        config.env = Vec::new();
        let manager = UserMcpServerManager::new(Db::open_in_memory().unwrap(), Some(cipher));
        manager.create(&config).unwrap();
        assert!(manager.create(&config).is_err());
    }

    #[test]
    fn malformed_stored_json_fails_catalog_reads() {
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let db = Db::open_in_memory().unwrap();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher));
        let created = manager.create(&stdio_config()).unwrap();

        db.with_conn(|conn| {
            conn.execute(
                "UPDATE user_mcp_servers SET stdio_args_json = 'not-json' WHERE id = ?1",
                [&created.id],
            )?;
            Ok(())
        })
        .unwrap();
        assert!(manager.list().is_err());

        db.with_conn(|conn| {
            conn.execute(
                "UPDATE user_mcp_servers SET stdio_args_json = '[]',
                    repository_overrides_json = 'not-json' WHERE id = ?1",
                [&created.id],
            )?;
            Ok(())
        })
        .unwrap();
        assert!(manager.summaries().is_err());
    }

    #[test]
    fn stored_transport_discriminators_decode_for_stdio_and_http() {
        let cipher = SessionDataCipher::from_env_value(TEST_KEY).unwrap();
        let manager = UserMcpServerManager::new(Db::open_in_memory().unwrap(), Some(cipher));
        let stdio = manager.create(&stdio_config()).unwrap();
        let http = admit_create(
            "http-service",
            None,
            &anyharness_contract::v1::mcp_servers::McpServerConfig::Http {
                url: "https://example.invalid/mcp".to_string(),
                headers: Vec::new(),
            },
        )
        .unwrap();
        let http = manager.create(&http).unwrap();
        let records = manager.list().unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == stdio.id)
                .unwrap()
                .transport,
            anyharness_contract::v1::mcp_servers::McpServerTransport::Stdio
        );
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == http.id)
                .unwrap()
                .transport,
            anyharness_contract::v1::mcp_servers::McpServerTransport::Http
        );
        let summaries = manager.summaries().unwrap();
        let http_summary = summaries
            .iter()
            .find(|summary| summary.id == http.id)
            .unwrap();
        assert!(http_summary.url.is_none());
    }
}
