use super::{blocking::run_blocking, error::ApiError};
use crate::app::AppState;
use anyharness_contract::v1::skills::{
    ImportSkillRequest, ListSkillsResponse, SetSkillEnabledRequest, SkillSummary,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

fn invalid(error: anyhow::Error) -> ApiError {
    if error
        .chain()
        .any(|cause| cause.is::<std::io::Error>() || cause.is::<serde_json::Error>())
    {
        return unavailable();
    }
    ApiError::bad_request(
        "Skill operation failed. Check the package size, root SKILL.md, relative paths, and enablement scope.",
        "SKILL_OPERATION_FAILED",
    )
}
fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "Skill management unavailable",
        None,
        Some("SKILLS_UNAVAILABLE"),
    )
}

#[utoipa::path(get, path = "/v1/skills", responses((status = 200, body = ListSkillsResponse)), tag = "skills")]
pub async fn list_skills(
    State(state): State<AppState>,
) -> Result<Json<ListSkillsResponse>, ApiError> {
    let manager = state.skill_manager.clone();
    let skills = run_blocking("skills_list", move || manager.list())
        .await
        .map_err(|_| unavailable())?
        .map_err(invalid)?;
    Ok(Json(ListSkillsResponse { skills }))
}

#[utoipa::path(post, path = "/v1/skills", request_body = ImportSkillRequest, responses((status = 201, body = SkillSummary)), tag = "skills")]
pub async fn import_skill(
    State(state): State<AppState>,
    Json(request): Json<ImportSkillRequest>,
) -> Result<(StatusCode, Json<SkillSummary>), ApiError> {
    let manager = state.skill_manager.clone();
    let skill = run_blocking("skill_import", move || manager.import(request))
        .await
        .map_err(|_| unavailable())?
        .map_err(invalid)?;
    Ok((StatusCode::CREATED, Json(skill)))
}

#[utoipa::path(put, path = "/v1/skills/{id}/enabled", params(("id" = String, Path)), request_body = SetSkillEnabledRequest, responses((status = 200, body = SkillSummary)), tag = "skills")]
pub async fn set_skill_enabled(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<SetSkillEnabledRequest>,
) -> Result<Json<SkillSummary>, ApiError> {
    let manager = state.skill_manager.clone();
    let repositories = state.repo_root_service.clone();
    let skill = run_blocking("skill_enable", move || {
        if let Some(repository_id) = &request.repository_id {
            anyhow::ensure!(
                repositories.get_repo_root(repository_id)?.is_some(),
                "Unknown repository"
            );
        }
        manager.set_enabled(&id, request)
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(invalid)?;
    Ok(Json(skill))
}

#[utoipa::path(delete, path = "/v1/skills/{id}", params(("id" = String, Path)), responses((status = 204)), tag = "skills")]
pub async fn remove_skill(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let manager = state.skill_manager.clone();
    run_blocking("skill_remove", move || manager.remove(&id))
        .await
        .map_err(|_| unavailable())?
        .map_err(invalid)?;
    Ok(StatusCode::NO_CONTENT)
}
