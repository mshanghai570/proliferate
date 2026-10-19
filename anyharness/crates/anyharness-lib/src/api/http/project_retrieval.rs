//! Runtime-owner access to explicit local project indexing and retrieval.
use super::{blocking::run_blocking, error::ApiError};
use crate::domains::workspaces::operation_gate::{WorkspaceOperationKind, WorkspaceOperationLease};
use crate::{api::auth::AuthContext, app::AppState, domains::sessions::retrieval::model};
use anyharness_contract::v1::retrieval::{
    ProjectContextExcerpt, ProjectContextSearchResponse, ProjectRetrievalStatus,
    SearchProjectContextRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use std::path::PathBuf;

fn failure() -> ApiError {
    ApiError::bad_request("Project retrieval failed. Open a local Git workspace, index it first, and search with 1–32 terms (up to 512 UTF-8 bytes). Check project access and retry.", "PROJECT_RETRIEVAL_FAILED")
}
fn status(value: model::ProjectRetrievalStatus) -> ProjectRetrievalStatus {
    ProjectRetrievalStatus {
        enabled: value.enabled,
        indexed_files: value.indexed_files,
        chunks: value.chunks,
        skipped_files: value.skipped_files,
        truncated: value.truncated,
        indexed_at: value.indexed_at,
    }
}
async fn workspace_root(
    state: &AppState,
    auth: &AuthContext,
    workspace: &str,
) -> Result<(PathBuf, WorkspaceOperationLease), ApiError> {
    if matches!(auth, AuthContext::UserClaim(_)) || !state.agent_runtime.supports_custom_agents() {
        return Err(ApiError::forbidden(
            "Project retrieval requires local runtime owner access.",
            "PROJECT_RETRIEVAL_FORBIDDEN",
        ));
    }
    let lease = state
        .workspace_operation_gate
        .acquire_shared(workspace, WorkspaceOperationKind::MaterializationRead)
        .await;
    let runtime = state.workspace_runtime.clone();
    let workspace = workspace.to_string();
    let record = run_blocking("retrieval_workspace", move || {
        runtime.get_workspace(&workspace)
    })
    .await
    .map_err(|_| failure())?
    .map_err(|_| failure())?
    .ok_or_else(|| ApiError::not_found("Workspace not found", "WORKSPACE_NOT_FOUND"))?;
    Ok((PathBuf::from(record.path), lease))
}

#[utoipa::path(get, path = "/v1/workspaces/{workspace_id}/retrieval", operation_id = "get_project_retrieval", params(("workspace_id" = String, Path)), responses((status = 200, body = ProjectRetrievalStatus)), tag = "workspaces")]
pub async fn get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(workspace): Path<String>,
) -> Result<Json<ProjectRetrievalStatus>, ApiError> {
    let (_, lease) = workspace_root(&state, &auth, &workspace).await?;
    let manager = state.project_retrieval_manager.clone();
    let result = run_blocking("retrieval_status", move || {
        let _lease = lease;
        manager.status(&workspace)
    })
    .await
    .map_err(|_| failure())?
    .map_err(|_| failure())?;
    Ok(Json(status(result)))
}
#[utoipa::path(post, path = "/v1/workspaces/{workspace_id}/retrieval", operation_id = "index_project_retrieval", params(("workspace_id" = String, Path)), responses((status = 200, body = ProjectRetrievalStatus)), tag = "workspaces")]
pub async fn index(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(workspace): Path<String>,
) -> Result<Json<ProjectRetrievalStatus>, ApiError> {
    let (root, lease) = workspace_root(&state, &auth, &workspace).await?;
    let result = state
        .project_retrieval_manager
        .refresh(workspace, root, lease)
        .await
        .map_err(|_| failure())?;
    Ok(Json(status(result)))
}
#[utoipa::path(delete, path = "/v1/workspaces/{workspace_id}/retrieval", operation_id = "clear_project_retrieval", params(("workspace_id" = String, Path)), responses((status = 204)), tag = "workspaces")]
pub async fn clear(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(workspace): Path<String>,
) -> Result<StatusCode, ApiError> {
    let (_, lease) = workspace_root(&state, &auth, &workspace).await?;
    let manager = state.project_retrieval_manager.clone();
    run_blocking("retrieval_clear", move || {
        let _lease = lease;
        manager.clear(&workspace)
    })
    .await
    .map_err(|_| failure())?
    .map_err(|_| failure())?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post, path = "/v1/workspaces/{workspace_id}/retrieval/search", operation_id = "search_project_context", params(("workspace_id" = String, Path)), request_body = SearchProjectContextRequest, responses((status = 200, body = ProjectContextSearchResponse)), tag = "workspaces")]
pub async fn search(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(workspace): Path<String>,
    Json(request): Json<SearchProjectContextRequest>,
) -> Result<Json<ProjectContextSearchResponse>, ApiError> {
    let (root, lease) = workspace_root(&state, &auth, &workspace).await?;
    let result = state
        .project_retrieval_manager
        .search(workspace, root, request.query, lease)
        .await
        .map_err(|_| failure())?;
    Ok(Json(ProjectContextSearchResponse {
        workspace_id: result.workspace_id,
        status: status(result.status),
        excerpts: result
            .excerpts
            .into_iter()
            .map(|excerpt| ProjectContextExcerpt {
                path: excerpt.path,
                start_line: excerpt.start_line,
                end_line: excerpt.end_line,
                content_hash: excerpt.content_hash,
                text: excerpt.text,
            })
            .collect(),
    }))
}
