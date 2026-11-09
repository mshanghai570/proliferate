use super::{blocking::run_blocking, error::ApiError};
use crate::app::AppState;
use anyharness_contract::v1::agent_rules::{AgentRules, SetAgentRulesRequest};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RulesQuery {
    repository_id: Option<String>,
}

fn failure() -> ApiError {
    ApiError::bad_request("Could not access agent rules. Check the agent, repository, UTF-8 text, and 64 KiB size limit.", "AGENT_RULES_UNAVAILABLE")
}

#[utoipa::path(get, path = "/v1/agent-rules/{agent_kind}", operation_id = "get_agent_rules", params(("agent_kind" = String, Path), ("repositoryId" = Option<String>, Query)), responses((status = 200, body = AgentRules)), tag = "agents")]
pub async fn get(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Query(query): Query<RulesQuery>,
) -> Result<Json<AgentRules>, ApiError> {
    let manager = state.agent_rules_manager.clone();
    let repositories = state.repo_root_service.clone();
    let result = run_blocking("agent_rules_get", move || -> anyhow::Result<AgentRules> {
        if let Some(id) = &query.repository_id {
            anyhow::ensure!(
                repositories.get_repo_root(id)?.is_some(),
                "Unknown repository"
            );
        }
        let rules = manager.get(&kind, query.repository_id.as_deref())?;
        Ok(AgentRules {
            agent_kind: kind,
            repository_id: query.repository_id,
            text: rules.text,
            inherited: rules.inherited,
        })
    })
    .await
    .map_err(|_| failure())?
    .map_err(|_| failure())?;
    Ok(Json(result))
}

#[utoipa::path(put, path = "/v1/agent-rules/{agent_kind}", operation_id = "set_agent_rules", params(("agent_kind" = String, Path)), request_body = SetAgentRulesRequest, responses((status = 200, body = AgentRules)), tag = "agents")]
pub async fn put(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Json(request): Json<SetAgentRulesRequest>,
) -> Result<Json<AgentRules>, ApiError> {
    let manager = state.agent_rules_manager.clone();
    let repositories = state.repo_root_service.clone();
    let result = run_blocking("agent_rules_set", move || -> anyhow::Result<AgentRules> {
        if let Some(id) = &request.repository_id {
            anyhow::ensure!(
                repositories.get_repo_root(id)?.is_some(),
                "Unknown repository"
            );
        }
        let rules = manager.set(
            &kind,
            request.repository_id.as_deref(),
            request.text.as_deref(),
        )?;
        Ok(AgentRules {
            agent_kind: kind,
            repository_id: request.repository_id,
            text: rules.text,
            inherited: rules.inherited,
        })
    })
    .await
    .map_err(|_| failure())?
    .map_err(|_| failure())?;
    Ok(Json(result))
}
