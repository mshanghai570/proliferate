use super::blocking::run_blocking;
use super::error::ApiError;
use crate::app::AppState;
use anyharness_contract::v1::mcp_servers::{
    CreateMcpServerRequest, ListMcpServersResponse, McpServerSummary, SetMcpServerEnabledRequest,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

const DATA_KEY_REQUIRED_DETAIL: &str =
    "A runtime data key (ANYHARNESS_DATA_KEY) is required to store MCP server secrets.";

fn invalid(error: anyhow::Error) -> ApiError {
    if error
        .chain()
        .any(|cause| cause.is::<std::io::Error>() || cause.is::<serde_json::Error>())
    {
        return unavailable();
    }
    if error
        .chain()
        .any(|cause| cause.to_string() == DATA_KEY_REQUIRED_DETAIL)
    {
        return ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            DATA_KEY_REQUIRED_DETAIL,
            None,
            Some("MCP_SERVER_DATA_KEY_REQUIRED"),
        );
    }
    ApiError::bad_request(
        "MCP server operation failed. Check the name, transport, bounded sizes, and enablement scope.",
        "MCP_SERVER_OPERATION_FAILED",
    )
}

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "MCP server management unavailable",
        None,
        Some("MCP_SERVERS_UNAVAILABLE"),
    )
}

#[utoipa::path(get, path = "/v1/mcp-servers", responses((status = 200, body = ListMcpServersResponse)), tag = "mcp_servers")]
pub async fn list_mcp_servers(
    State(state): State<AppState>,
) -> Result<Json<ListMcpServersResponse>, ApiError> {
    let manager = state.mcp_servers_manager.clone();
    let servers = run_blocking("mcp_servers_list", move || manager.summaries())
        .await
        .map_err(|_| unavailable())?
        .map_err(invalid)?;
    Ok(Json(ListMcpServersResponse { servers }))
}

#[utoipa::path(post, path = "/v1/mcp-servers", request_body = CreateMcpServerRequest, responses((status = 201, body = McpServerSummary)), tag = "mcp_servers")]
pub async fn create_mcp_server(
    State(state): State<AppState>,
    Json(request): Json<CreateMcpServerRequest>,
) -> Result<(StatusCode, Json<McpServerSummary>), ApiError> {
    let manager = state.mcp_servers_manager.clone();
    let created = run_blocking("mcp_server_create", move || {
        let admitted = crate::domains::mcp_servers::admission::admit_create(
            &request.server_name,
            request.description.as_deref(),
            &request.config,
        )?;
        manager.create(&admitted)?;
        manager
            .summaries()?
            .into_iter()
            .find(|summary| summary.server_name == admitted.server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server vanished after creation"))
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(invalid)?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(put, path = "/v1/mcp-servers/{id}/enabled", params(("id" = String, Path)), request_body = SetMcpServerEnabledRequest, responses((status = 200, body = McpServerSummary)), tag = "mcp_servers")]
pub async fn set_mcp_server_enabled(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<SetMcpServerEnabledRequest>,
) -> Result<Json<McpServerSummary>, ApiError> {
    let (repo_root_id, enabled) = match request {
        SetMcpServerEnabledRequest::UserDefault(enabled) => (None, Some(enabled)),
        SetMcpServerEnabledRequest::RepositoryOverride(request) => {
            (Some(request.repo_root_id), request.enabled)
        }
    };
    let manager = state.mcp_servers_manager.clone();
    let repositories = state.repo_root_service.clone();
    let server = run_blocking("mcp_server_enable", move || {
        if let Some(repo_root_id) = &repo_root_id {
            anyhow::ensure!(
                !repo_root_id.trim().is_empty()
                    && repo_root_id.len() <= 128
                    && !repo_root_id.chars().any(char::is_control),
                "Invalid repository root id"
            );
            repositories
                .get_repo_root(repo_root_id)?
                .is_some()
                .then_some(())
                .ok_or_else(|| anyhow::anyhow!("Unknown repository root"))?;
        }
        let updated = manager.set_enabled(&id, repo_root_id.as_deref(), enabled)?;
        manager
            .summaries()?
            .into_iter()
            .find(|summary| summary.id == updated.id)
            .ok_or_else(|| anyhow::anyhow!("MCP server vanished after enablement"))
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(invalid)?;
    Ok(Json(server))
}

#[utoipa::path(delete, path = "/v1/mcp-servers/{id}", params(("id" = String, Path)), responses((status = 204)), tag = "mcp_servers")]
pub async fn remove_mcp_server(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let manager = state.mcp_servers_manager.clone();
    run_blocking("mcp_server_remove", move || manager.remove(&id))
        .await
        .map_err(|_| unavailable())?
        .map_err(invalid)?;
    Ok(StatusCode::NO_CONTENT)
}
