use anyharness_contract::v1::ai_providers::{
    DiscoverProviderModelsRequest, DiscoverProviderModelsResponse, ProviderDiscoveryError,
};
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
};
use tokio_util::sync::CancellationToken;

use super::error::ApiError;
use crate::integrations::openai_compatible::{
    ProviderSecret,
    models::{discover_models, parse_endpoint},
};

const NATIVE_PROVIDER_CREDENTIAL_HEADER: &str = "x-anyharness-native-provider-credential";

#[utoipa::path(
    post,
    path = "/v1/ai-providers/models",
    request_body = DiscoverProviderModelsRequest,
    responses(
        (status = 200, description = "Discovered provider models", body = DiscoverProviderModelsResponse),
        (status = 400, description = "The endpoint or credential header is invalid", body = anyharness_contract::v1::ProblemDetails),
        (status = 401, description = "The provider rejected its credential", body = anyharness_contract::v1::ProblemDetails),
        (status = 502, description = "The provider returned an unusable model list", body = anyharness_contract::v1::ProblemDetails),
        (status = 503, description = "The provider is unreachable", body = anyharness_contract::v1::ProblemDetails),
        (status = 504, description = "The provider request timed out", body = anyharness_contract::v1::ProblemDetails)
    )
)]
pub async fn discover_provider_models(
    headers: HeaderMap,
    Json(request): Json<DiscoverProviderModelsRequest>,
) -> Result<Json<DiscoverProviderModelsResponse>, ApiError> {
    let endpoint = parse_endpoint(&request.endpoint).map_err(map_discovery_error)?;
    let credential = headers
        .get(NATIVE_PROVIDER_CREDENTIAL_HEADER)
        .map(|value| {
            value.to_str().map(str::to_owned).map_err(|_| {
                ApiError::bad_request(
                    "The provider credential header is invalid.",
                    "PROVIDER_CREDENTIAL_INVALID",
                )
            })
        })
        .transpose()?
        .filter(|value| !value.is_empty())
        .map(ProviderSecret::new);

    let models = discover_models(&endpoint, credential.as_ref(), CancellationToken::new())
        .await
        .map_err(map_discovery_error)?;

    Ok(Json(DiscoverProviderModelsResponse {
        models,
        refreshed_at: chrono::Utc::now().to_rfc3339(),
    }))
}

fn map_discovery_error(error: ProviderDiscoveryError) -> ApiError {
    let (status, title, detail, code) = match error {
        ProviderDiscoveryError::InvalidEndpoint => (
            StatusCode::BAD_REQUEST,
            "Bad request",
            "The provider endpoint is invalid.",
            "PROVIDER_DISCOVERY_INVALID_ENDPOINT",
        ),
        ProviderDiscoveryError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "Unauthorized",
            "The provider rejected the supplied credential.",
            "PROVIDER_DISCOVERY_UNAUTHORIZED",
        ),
        ProviderDiscoveryError::Unreachable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "The provider endpoint could not be reached.",
            "PROVIDER_DISCOVERY_UNREACHABLE",
        ),
        ProviderDiscoveryError::Timeout => (
            StatusCode::GATEWAY_TIMEOUT,
            "Gateway timeout",
            "Provider model discovery timed out.",
            "PROVIDER_DISCOVERY_TIMEOUT",
        ),
        ProviderDiscoveryError::InvalidResponse => (
            StatusCode::BAD_GATEWAY,
            "Bad gateway",
            "The provider returned an invalid model list.",
            "PROVIDER_DISCOVERY_INVALID_RESPONSE",
        ),
        ProviderDiscoveryError::EmptyModels => (
            StatusCode::BAD_GATEWAY,
            "Bad gateway",
            "The provider returned no models.",
            "PROVIDER_DISCOVERY_EMPTY_MODELS",
        ),
    };

    ApiError::new(status, title, Some(detail.to_owned()), Some(code))
}

#[cfg(test)]
#[path = "ai_providers_tests.rs"]
mod tests;
