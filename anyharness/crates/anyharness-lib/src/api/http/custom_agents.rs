//! Runtime-local ACP command registration transport.
use anyharness_contract::v1::{CustomAcpAgent, ProblemDetails, RegisterCustomAcpAgentRequest};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use super::error::ApiError;
use crate::app::AppState;
use crate::domains::agents::{custom, launch_probe::PokeReason};

fn ensure_local(state: &AppState) -> Result<(), ApiError> {
    if state.agent_runtime.supports_custom_agents() {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "Custom ACP registrations require a local runtime",
            "CUSTOM_AGENT_LOCAL_ONLY",
        ))
    }
}

fn summary(config: custom::CustomAgent) -> CustomAcpAgent {
    CustomAcpAgent {
        id: config.id,
        name: config.name,
        command: config.command,
        args: config.args,
        env_names: config.env.into_keys().collect(),
    }
}

#[utoipa::path(get, path = "/v1/agents/custom", responses((status = 200, description = "Local custom ACP registrations", body = Vec<CustomAcpAgent>)), tag = "agents")]
pub async fn list_custom_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<CustomAcpAgent>>, ApiError> {
    ensure_local(&state)?;
    let runtime = state.agent_runtime.clone();
    let agents = tokio::task::spawn_blocking(move || runtime.list_custom_agents())
        .await
        .map_err(|_| ApiError::internal("Custom ACP registration task failed"))?
        .map_err(|_| ApiError::internal("Could not read custom ACP registrations"))?;
    Ok(Json(agents.into_iter().map(summary).collect()))
}

#[utoipa::path(post, path = "/v1/agents/custom", request_body = RegisterCustomAcpAgentRequest, responses((status = 201, description = "Custom ACP command registered", body = CustomAcpAgent), (status = 400, description = "Invalid registration", body = ProblemDetails)), tag = "agents")]
pub async fn register_custom_agent(
    State(state): State<AppState>,
    Json(request): Json<RegisterCustomAcpAgentRequest>,
) -> Result<(StatusCode, Json<CustomAcpAgent>), ApiError> {
    ensure_local(&state)?;
    let config = custom::CustomAgent {
        id: format!("custom-{}", uuid::Uuid::new_v4()),
        name: request.name.trim().to_string(),
        command: request.command,
        args: request.args,
        env: request.env,
    };
    config
        .validate()
        .map_err(|error| ApiError::bad_request(error.to_string(), "CUSTOM_AGENT_INVALID"))?;
    let id = config.id.clone();
    let runtime = state.agent_runtime.clone();
    let config = tokio::task::spawn_blocking(move || {
        runtime.register_custom_agent(&config)?;
        Ok::<_, anyhow::Error>(config)
    })
    .await
    .map_err(|_| ApiError::internal("Custom ACP registration task failed"))?
    .map_err(|_| ApiError::internal("Could not save custom ACP registration"))?;
    state
        .launch_probe_service
        .clone()
        .poke_harness(&id, PokeReason::Manual);
    Ok((StatusCode::CREATED, Json(summary(config))))
}

#[utoipa::path(delete, path = "/v1/agents/custom/{id}", params(("id" = String, Path, description = "Custom ACP registration id")), responses((status = 204, description = "Registration removed"), (status = 404, description = "Registration not found", body = ProblemDetails)), tag = "agents")]
pub async fn remove_custom_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_local(&state)?;
    if !custom::is_custom_id(&id) {
        return Err(ApiError::bad_request(
            "Invalid custom ACP registration id",
            "CUSTOM_AGENT_INVALID",
        ));
    }
    let runtime = state.agent_runtime.clone();
    let removed = tokio::task::spawn_blocking(move || runtime.remove_custom_agent(&id))
        .await
        .map_err(|_| ApiError::internal("Custom ACP registration task failed"))?
        .map_err(|_| ApiError::internal("Could not remove custom ACP registration"))?;
    if !removed {
        return Err(ApiError::not_found(
            "Custom ACP registration not found",
            "CUSTOM_AGENT_NOT_FOUND",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}
