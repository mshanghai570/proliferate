use super::{blocking::run_blocking, error::ApiError};
use crate::domains::agents::{
    launch_probe::{LaunchProbeService, PokeReason},
    local_providers::{
        model::{ProviderInput, ProviderSummary},
        LocalProviderError, LocalProviderManager,
    },
};
use crate::{api::auth::AuthContext, app::AppState};
use anyharness_contract::v1::local_providers::{
    CreateLocalProviderRequest, LocalProviderSummary, LocalProvidersResponse,
    SetLocalProviderEnabledRequest,
};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Extension, Json,
};

fn authorize(auth: &AuthContext) -> Result<(), ApiError> {
    if matches!(auth, AuthContext::UserClaim(_)) {
        return Err(ApiError::forbidden(
            "Local provider management requires runtime owner access.",
            "LOCAL_PROVIDER_FORBIDDEN",
        ));
    }
    Ok(())
}
fn map_error(error: LocalProviderError) -> ApiError {
    match error {
        LocalProviderError::Invalid(_) => {
            ApiError::bad_request(error.to_string(), "LOCAL_PROVIDER_INVALID")
        }
        LocalProviderError::NotFound => {
            ApiError::not_found(error.to_string(), "LOCAL_PROVIDER_NOT_FOUND")
        }
        LocalProviderError::ManagedAuth => ApiError::new(
            StatusCode::CONFLICT,
            error.to_string(),
            None,
            Some("LOCAL_PROVIDER_MANAGED_AUTH"),
        ),
        _ => unavailable(),
    }
}
fn unavailable() -> ApiError {
    ApiError::service_unavailable(
        "Local provider settings could not be accessed. Restart the desktop app and try again.",
        "LOCAL_PROVIDER_UNAVAILABLE",
    )
}
fn summary(row: ProviderSummary) -> LocalProviderSummary {
    LocalProviderSummary {
        id: row.id,
        name: row.name,
        enabled: row.enabled,
        model_count: row.model_count,
    }
}
fn poke(state: &AppState) {
    LaunchProbeService::poke_harnesses_optional(
        &state.automatic_poke_engine,
        &["opencode".into()],
        PokeReason::AuthApplied,
    );
}

#[utoipa::path(get, operation_id = "list_local_providers", path = "/v1/agent-auth/local-providers", responses((status = 200, body = LocalProvidersResponse)), tag = "agent-auth")]
pub async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<LocalProvidersResponse>, ApiError> {
    authorize(&auth)?;
    let home = state.runtime_home.clone();
    let inventory = run_blocking("local_providers_list", move || {
        LocalProviderManager::from_home(&home)?.list()
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    Ok(Json(LocalProvidersResponse {
        providers: inventory.providers.into_iter().map(summary).collect(),
        encryption_available: inventory.encryption_available,
        managed_auth_active: inventory.managed_auth_active,
    }))
}

#[utoipa::path(post, operation_id = "create_local_provider", path = "/v1/agent-auth/local-providers", request_body = CreateLocalProviderRequest, responses((status = 201, body = LocalProviderSummary)), tag = "agent-auth")]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    body: Result<Json<CreateLocalProviderRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<LocalProviderSummary>), ApiError> {
    authorize(&auth)?;
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request(
            "Enter a valid local provider configuration.",
            "LOCAL_PROVIDER_INVALID",
        )
    })?;
    let input = ProviderInput {
        name: body.name,
        endpoint: body.endpoint,
        models: body.models,
        api_format: body.api_format,
        api_key: body.api_key,
    };
    let home = state.runtime_home.clone();
    let row = run_blocking("local_provider_create", move || {
        LocalProviderManager::from_home(&home)?.create(input)
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    poke(&state);
    Ok((StatusCode::CREATED, Json(summary(row))))
}

#[utoipa::path(patch, operation_id = "set_local_provider_enabled", path = "/v1/agent-auth/local-providers/{id}", params(("id" = String, Path)), request_body = SetLocalProviderEnabledRequest, responses((status = 200, body = LocalProviderSummary)), tag = "agent-auth")]
pub async fn set_enabled(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    body: Result<Json<SetLocalProviderEnabledRequest>, JsonRejection>,
) -> Result<Json<LocalProviderSummary>, ApiError> {
    authorize(&auth)?;
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request(
            "Choose whether the provider is enabled.",
            "LOCAL_PROVIDER_INVALID",
        )
    })?;
    let home = state.runtime_home.clone();
    let row = run_blocking("local_provider_toggle", move || {
        LocalProviderManager::from_home(&home)?.set_enabled(&id, body.enabled)
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    poke(&state);
    Ok(Json(summary(row)))
}

#[utoipa::path(delete, operation_id = "remove_local_provider", path = "/v1/agent-auth/local-providers/{id}", params(("id" = String, Path)), responses((status = 204)), tag = "agent-auth")]
pub async fn remove(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    authorize(&auth)?;
    let home = state.runtime_home.clone();
    run_blocking("local_provider_remove", move || {
        LocalProviderManager::from_home(&home)?.remove(&id)
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    poke(&state);
    Ok(StatusCode::NO_CONTENT)
}
