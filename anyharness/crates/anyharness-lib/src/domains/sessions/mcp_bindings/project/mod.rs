pub mod model;
mod store;
#[cfg(test)]
mod tests;
mod validation;

use self::model::{ProjectMcpInventory, ProjectMcpServerMetadata, ProjectMcpTransport};
use super::model::SessionMcpServer;
use crate::domains::repo_roots::store::RepoRootStore;
use crate::domains::sessions::extensions::{
    SessionExtension, SessionLaunchContext, SessionLaunchExtras,
};
use crate::domains::sessions::model::SessionMcpBindingPolicy;
use crate::persistence::data_cipher::{decrypt_bytes, encrypt_bytes, DataCipher};
use crate::persistence::Db;
use store::{ProjectMcpStore, StoredServer};

#[derive(Debug, thiserror::Error)]
pub enum ProjectMcpError {
    #[error("Invalid MCP configuration. {0}")]
    Invalid(&'static str),
    #[error("Project or MCP server was not found.")]
    NotFound,
    #[error("The runtime encryption key is unavailable. Restart the desktop app and try again.")]
    MissingKey,
    #[error("Project MCP settings could not be accessed.")]
    Unavailable,
}

#[derive(Clone)]
pub struct ProjectMcpManager {
    store: ProjectMcpStore,
    repos: RepoRootStore,
    cipher: Option<DataCipher>,
}

impl ProjectMcpManager {
    pub fn new(db: Db, cipher: Option<DataCipher>) -> Self {
        Self {
            store: ProjectMcpStore(db.clone()),
            repos: RepoRootStore::new(db),
            cipher,
        }
    }

    fn assert_repo(&self, repo: &str) -> Result<(), ProjectMcpError> {
        if self
            .repos
            .find_by_id(repo)
            .map_err(|_| ProjectMcpError::Unavailable)?
            .is_none()
        {
            return Err(ProjectMcpError::NotFound);
        }
        Ok(())
    }

    pub fn list(&self, repo: &str) -> Result<ProjectMcpInventory, ProjectMcpError> {
        self.assert_repo(repo)?;
        let servers = self
            .store
            .list(repo)
            .map_err(|_| ProjectMcpError::Unavailable)?
            .into_iter()
            .map(|row| ProjectMcpServerMetadata {
                name: row.name,
                enabled: row.enabled,
                transport: if row.transport == "http" {
                    ProjectMcpTransport::Http
                } else {
                    ProjectMcpTransport::Stdio
                },
            })
            .collect();
        Ok(ProjectMcpInventory {
            servers,
            encryption_available: self.cipher.is_some(),
        })
    }

    /// Replacing a server supplies all credentials again and enables it.
    pub fn put(
        &self,
        repo: &str,
        name: &str,
        mut server: SessionMcpServer,
    ) -> Result<ProjectMcpServerMetadata, ProjectMcpError> {
        self.assert_repo(repo)?;
        validation::validate_and_normalize(name, &mut server)?;
        let cipher = self.cipher.as_ref().ok_or(ProjectMcpError::MissingKey)?;
        let transport = match &server {
            SessionMcpServer::Http(_) => "http",
            SessionMcpServer::Stdio(_) => "stdio",
        };
        let bytes = serde_json::to_vec(&server).map_err(|_| ProjectMcpError::Unavailable)?;
        if bytes.len() > 65_536 {
            return Err(ProjectMcpError::Invalid("Use at most 64 KiB per server."));
        }
        let ciphertext = encrypt_bytes(cipher, &bytes).map_err(|_| ProjectMcpError::Unavailable)?;
        if !self
            .store
            .put(
                repo,
                &StoredServer {
                    name: name.into(),
                    transport: transport.into(),
                    enabled: true,
                    ciphertext,
                },
            )
            .map_err(|_| ProjectMcpError::Unavailable)?
        {
            return Err(ProjectMcpError::Invalid(
                "Use at most 32 servers per project.",
            ));
        }
        tracing::info!(target: "anyharness.project_mcp.changed", action = "replace", "project MCP settings changed");
        Ok(ProjectMcpServerMetadata {
            name: name.into(),
            enabled: true,
            transport: if transport == "http" {
                ProjectMcpTransport::Http
            } else {
                ProjectMcpTransport::Stdio
            },
        })
    }

    pub fn set_enabled(
        &self,
        repo: &str,
        name: &str,
        enabled: bool,
    ) -> Result<ProjectMcpServerMetadata, ProjectMcpError> {
        self.assert_repo(repo)?;
        let transport = self
            .store
            .set_enabled(repo, name, enabled)
            .map_err(|_| ProjectMcpError::Unavailable)?
            .ok_or(ProjectMcpError::NotFound)?;
        tracing::info!(target: "anyharness.project_mcp.changed", action = "toggle", "project MCP settings changed");
        Ok(ProjectMcpServerMetadata {
            name: name.into(),
            enabled,
            transport: if transport == "http" {
                ProjectMcpTransport::Http
            } else {
                ProjectMcpTransport::Stdio
            },
        })
    }

    pub fn remove(&self, repo: &str, name: &str) -> Result<(), ProjectMcpError> {
        self.assert_repo(repo)?;
        self.store
            .remove(repo, name)
            .map_err(|_| ProjectMcpError::Unavailable)?;
        tracing::info!(target: "anyharness.project_mcp.changed", action = "remove", "project MCP settings changed");
        Ok(())
    }
}

impl SessionExtension for ProjectMcpManager {
    fn resolve_launch_extras(
        &self,
        ctx: &SessionLaunchContext<'_>,
    ) -> anyhow::Result<SessionLaunchExtras> {
        if ctx.session.mcp_binding_policy == SessionMcpBindingPolicy::InternalOnly {
            return Ok(SessionLaunchExtras::default());
        }
        let rows = self
            .store
            .list(&ctx.workspace.repo_root_id)
            .map_err(|_| ProjectMcpError::Unavailable)?;
        let mut extras = SessionLaunchExtras::default();
        for row in rows.into_iter().filter(|row| row.enabled) {
            let cipher = self.cipher.as_ref().ok_or(ProjectMcpError::MissingKey)?;
            let bytes =
                decrypt_bytes(cipher, &row.ciphertext).map_err(|_| ProjectMcpError::Unavailable)?;
            let mut server: SessionMcpServer =
                serde_json::from_slice(&bytes).map_err(|_| ProjectMcpError::Unavailable)?;
            validation::validate_and_normalize(&row.name, &mut server)?;
            extras.mcp_servers.push(server);
        }
        Ok(extras)
    }
}
