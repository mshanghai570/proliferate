use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverProviderModelsRequest {
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModel {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverProviderModelsResponse {
    pub models: Vec<ProviderModel>,
    pub refreshed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDiscoveryError {
    InvalidEndpoint,
    Unauthorized,
    Unreachable,
    Timeout,
    InvalidResponse,
    EmptyModels,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        DiscoverProviderModelsRequest, DiscoverProviderModelsResponse, ProviderDiscoveryError,
        ProviderModel,
    };

    #[test]
    fn provider_model_round_trips_opaque_id() {
        let model = ProviderModel {
            id: "org/model:latest".to_owned(),
            display_name: Some("Latest model".to_owned()),
        };

        let serialized = serde_json::to_string(&model).unwrap();
        let deserialized: ProviderModel = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, "org/model:latest");
        assert_eq!(deserialized.display_name.as_deref(), Some("Latest model"));
    }

    #[test]
    fn request_uses_camel_case_and_preserves_endpoint_without_credentials() {
        let endpoint = "http://localhost:1234/base/v1?region=local";
        let request = DiscoverProviderModelsRequest {
            endpoint: endpoint.to_owned(),
        };

        let serialized = serde_json::to_value(&request).unwrap();

        assert_eq!(
            serialized,
            json!({
                "endpoint": endpoint
            })
        );
        assert_eq!(serialized["endpoint"], endpoint);
        assert!(serialized.get("apiKey").is_none());
        assert!(serialized.get("api_key").is_none());
        assert!(serialized.get("credential").is_none());
    }

    #[test]
    fn response_uses_camel_case_and_round_trips() {
        let response = DiscoverProviderModelsResponse {
            models: vec![ProviderModel {
                id: "org/model:latest".to_owned(),
                display_name: None,
            }],
            refreshed_at: "2026-10-17T16:00:00Z".to_owned(),
        };

        let serialized = serde_json::to_value(&response).unwrap();

        assert_eq!(
            serialized,
            json!({
                "models": [{ "id": "org/model:latest" }],
                "refreshedAt": "2026-10-17T16:00:00Z"
            })
        );

        let deserialized: DiscoverProviderModelsResponse =
            serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized.models[0].id, "org/model:latest");
        assert_eq!(deserialized.refreshed_at, "2026-10-17T16:00:00Z");
    }

    #[test]
    fn discovery_errors_have_stable_snake_case_vocabulary() {
        let errors = [
            (ProviderDiscoveryError::InvalidEndpoint, "invalid_endpoint"),
            (ProviderDiscoveryError::Unauthorized, "unauthorized"),
            (ProviderDiscoveryError::Unreachable, "unreachable"),
            (ProviderDiscoveryError::Timeout, "timeout"),
            (ProviderDiscoveryError::InvalidResponse, "invalid_response"),
            (ProviderDiscoveryError::EmptyModels, "empty_models"),
        ];

        for (error, expected) in errors {
            assert_eq!(
                serde_json::to_string(&error).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }
}
