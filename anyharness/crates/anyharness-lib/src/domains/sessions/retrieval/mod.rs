//! Local, explicit project context retrieval; no embedding provider or network requests.
mod chunks;
pub mod model;
mod store;
#[cfg(test)]
mod tests;
use self::model::{ProjectContextSearchResult, ProjectRetrievalStatus};
use crate::adapters::files::retrieval::eligible_path;
use crate::adapters::git::project_sources::project_paths;
use crate::domains::workspaces::operation_gate::WorkspaceOperationLease;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct ProjectRetrievalManager {
    home: PathBuf,
    gate: Mutex<()>,
}
impl ProjectRetrievalManager {
    pub fn new(home: PathBuf) -> Self {
        Self {
            home,
            gate: Mutex::new(()),
        }
    }
    pub fn status(&self, workspace: &str) -> anyhow::Result<ProjectRetrievalStatus> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Retrieval cache unavailable"))?;
        let connection = store::open(&self.home, workspace, false)?;
        store::status(connection.as_ref())
    }
    pub fn clear(&self, workspace: &str) -> anyhow::Result<()> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Retrieval cache unavailable"))?;
        store::clear(&self.home, workspace)
    }
    pub async fn refresh(
        self: &Arc<Self>,
        workspace: String,
        root: PathBuf,
        lease: WorkspaceOperationLease,
    ) -> anyhow::Result<ProjectRetrievalStatus> {
        anyhow::ensure!(
            lease.workspace_id() == workspace,
            "Workspace lease mismatch"
        );
        let root = tokio::fs::canonicalize(root).await?;
        let paths: Vec<_> = project_paths(&root)
            .await?
            .into_iter()
            .filter(|path| eligible_path(path))
            .collect();
        let manager = self.clone();
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            let _guard = manager
                .gate
                .lock()
                .map_err(|_| anyhow::anyhow!("Retrieval cache unavailable"))?;
            let mut connection = store::open(&manager.home, &workspace, true)?
                .ok_or_else(|| anyhow::anyhow!("Retrieval cache unavailable"))?;
            let status = store::refresh(&mut connection, &root, &paths)?;
            tracing::info!(
                event = "project_retrieval.indexed",
                indexed_files = status.indexed_files,
                chunks = status.chunks,
                skipped_files = status.skipped_files,
                truncated = status.truncated,
                "Project retrieval index refreshed"
            );
            Ok(status)
        })
        .await?
    }
    pub async fn search(
        self: &Arc<Self>,
        workspace: String,
        root: PathBuf,
        query: String,
        lease: WorkspaceOperationLease,
    ) -> anyhow::Result<ProjectContextSearchResult> {
        chunks::query(&query)?;
        anyhow::ensure!(
            lease.workspace_id() == workspace,
            "Workspace lease mismatch"
        );
        let root = tokio::fs::canonicalize(root).await?;
        let paths: Vec<_> = project_paths(&root)
            .await?
            .into_iter()
            .filter(|path| eligible_path(path))
            .collect();
        let manager = self.clone();
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            let _guard = manager
                .gate
                .lock()
                .map_err(|_| anyhow::anyhow!("Retrieval cache unavailable"))?;
            let mut connection = store::open(&manager.home, &workspace, false)?
                .ok_or_else(|| anyhow::anyhow!("Index the project before searching"))?;
            anyhow::ensure!(
                store::status(Some(&connection))?.enabled,
                "Index the project before searching"
            );
            let status = store::refresh(&mut connection, &root, &paths)?;
            let excerpts = store::search(&connection, &query)?;
            tracing::info!(
                event = "project_retrieval.searched",
                result_count = excerpts.len(),
                "Project context retrieved"
            );
            Ok(ProjectContextSearchResult {
                workspace_id: workspace,
                status,
                excerpts,
            })
        })
        .await?
    }
}

/// Purge and materialization cleanup call this under the exclusive workspace lease.
pub fn clear_workspace_cache(home: &std::path::Path, workspace: &str) -> anyhow::Result<()> {
    store::clear(home, workspace)
}
