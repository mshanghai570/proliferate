use anyharness_contract::v1::{
    ProjectMcpServerSummary, ProjectMcpServersResponse, SessionMcpServer, SessionMcpTransport,
    SetProjectMcpServerEnabledRequest,
};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Extension, Json,
};

use super::{blocking::run_blocking, error::ApiError};
use crate::api::auth::AuthContext;
use crate::app::AppState;
use crate::domains::sessions::mcp_bindings::contract::bindings_from_contract;
use crate::domains::sessions::mcp_bindings::project::model::{
    ProjectMcpInventory, ProjectMcpServerMetadata, ProjectMcpTransport,
};
use crate::domains::sessions::mcp_bindings::project::ProjectMcpError;

fn summary(value: ProjectMcpServerMetadata) -> ProjectMcpServerSummary {
    ProjectMcpServerSummary {
        name: value.name,
        enabled: value.enabled,
        transport: match value.transport {
            ProjectMcpTransport::Http => SessionMcpTransport::Http,
            ProjectMcpTransport::Stdio => SessionMcpTransport::Stdio,
        },
    }
}

fn inventory(value: ProjectMcpInventory) -> ProjectMcpServersResponse {
    ProjectMcpServersResponse {
        servers: value.servers.into_iter().map(summary).collect(),
        encryption_available: value.encryption_available,
    }
}

fn authorize(auth: &AuthContext) -> Result<(), ApiError> {
    if matches!(auth, AuthContext::UserClaim(_)) {
        return Err(ApiError::forbidden(
            "Project MCP management requires runtime owner access.",
            "PROJECT_MCP_FORBIDDEN",
        ));
    }
    Ok(())
}

fn map_error(error: ProjectMcpError) -> ApiError {
    match error {
        ProjectMcpError::Invalid(_) => {
            ApiError::bad_request(error.to_string(), "PROJECT_MCP_INVALID")
        }
        ProjectMcpError::NotFound => {
            ApiError::not_found(error.to_string(), "PROJECT_MCP_NOT_FOUND")
        }
        ProjectMcpError::MissingKey => {
            ApiError::service_unavailable(error.to_string(), "PROJECT_MCP_KEY_UNAVAILABLE")
        }
        ProjectMcpError::Unavailable => unavailable(),
    }
}

fn unavailable() -> ApiError {
    ApiError::service_unavailable(
        "Project MCP settings could not be accessed.",
        "PROJECT_MCP_UNAVAILABLE",
    )
}

#[utoipa::path(get, path = "/v1/repo-roots/{repo_root_id}/mcp-servers", params(("repo_root_id" = String, Path)),
    responses((status = 200, description = "Write-only MCP inventory for a project", body = ProjectMcpServersResponse)), tag = "repo-roots")]
pub async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(repo): Path<String>,
) -> Result<Json<ProjectMcpServersResponse>, ApiError> {
    authorize(&auth)?;
    let manager = state.project_mcp_manager.clone();
    Ok(Json(inventory(
        run_blocking("project_mcp_list", move || manager.list(&repo))
            .await
            .map_err(|_| unavailable())?
            .map_err(map_error)?,
    )))
}

#[utoipa::path(put, path = "/v1/repo-roots/{repo_root_id}/mcp-servers/{name}", params(("repo_root_id" = String, Path), ("name" = String, Path)), request_body = SessionMcpServer,
    responses((status = 200, description = "Server replaced and enabled; used at next session process launch", body = ProjectMcpServerSummary)), tag = "repo-roots")]
pub async fn put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((repo, name)): Path<(String, String)>,
    body: Result<Json<SessionMcpServer>, JsonRejection>,
) -> Result<Json<ProjectMcpServerSummary>, ApiError> {
    authorize(&auth)?;
    let Json(server) = body.map_err(|_| {
        ApiError::bad_request(
            "Supply a valid HTTP or stdio MCP server object.",
            "PROJECT_MCP_INVALID",
        )
    })?;
    let manager = state.project_mcp_manager.clone();
    let server = bindings_from_contract(vec![server])
        .pop()
        .ok_or_else(unavailable)?;
    Ok(Json(summary(
        run_blocking("project_mcp_put", move || manager.put(&repo, &name, server))
            .await
            .map_err(|_| unavailable())?
            .map_err(map_error)?,
    )))
}

#[utoipa::path(patch, path = "/v1/repo-roots/{repo_root_id}/mcp-servers/{name}", params(("repo_root_id" = String, Path), ("name" = String, Path)), request_body = SetProjectMcpServerEnabledRequest,
    responses((status = 200, description = "Server enabled state updated; used at next session process launch", body = ProjectMcpServerSummary)), tag = "repo-roots")]
pub async fn set_enabled(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((repo, name)): Path<(String, String)>,
    body: Result<Json<SetProjectMcpServerEnabledRequest>, JsonRejection>,
) -> Result<Json<ProjectMcpServerSummary>, ApiError> {
    authorize(&auth)?;
    let Json(body) = body
        .map_err(|_| ApiError::bad_request("Supply an enabled boolean.", "PROJECT_MCP_INVALID"))?;
    let manager = state.project_mcp_manager.clone();
    Ok(Json(summary(
        run_blocking("project_mcp_toggle", move || {
            manager.set_enabled(&repo, &name, body.enabled)
        })
        .await
        .map_err(|_| unavailable())?
        .map_err(map_error)?,
    )))
}

#[utoipa::path(delete, path = "/v1/repo-roots/{repo_root_id}/mcp-servers/{name}", params(("repo_root_id" = String, Path), ("name" = String, Path)),
    responses((status = 204, description = "Server removed; live sessions keep their current configuration")), tag = "repo-roots")]
pub async fn remove(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((repo, name)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    authorize(&auth)?;
    let manager = state.project_mcp_manager.clone();
    run_blocking("project_mcp_remove", move || manager.remove(&repo, &name))
        .await
        .map_err(|_| unavailable())?
        .map_err(map_error)?;
    Ok(StatusCode::NO_CONTENT)
}
