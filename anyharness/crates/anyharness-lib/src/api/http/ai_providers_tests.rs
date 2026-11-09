use std::path::PathBuf;

use anyharness_contract::v1::ai_providers::{
    DiscoverProviderModelsRequest, ProviderDiscoveryError,
};
use axum::{
    Json,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode, header},
    response::IntoResponse,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tower::util::ServiceExt;
use uuid::Uuid;

use super::{NATIVE_PROVIDER_CREDENTIAL_HEADER, discover_provider_models, map_discovery_error};
use crate::{
    api::router::build_router,
    app::{AppState, test_support},
    domains::agents::installer::seed::AgentSeedStore,
    integrations::openai_compatible::ProviderSecret,
    persistence::Db,
};

struct RuntimeHome(PathBuf);

impl RuntimeHome {
    fn new() -> Self {
        Self(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("ai-provider-route-test-{}", Uuid::new_v4())),
        )
    }
}

impl Drop for RuntimeHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        assert_ne!(read, 0, "client closed before sending request headers");
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        assert!(request.len() < 16 * 1024, "request headers too large");
    }
    String::from_utf8(request).unwrap()
}

async fn mock_provider(status: &str, body: &str) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.to_owned();
    let status = status.to_owned();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream).await;
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        request
    });
    (format!("http://{address}/v1"), server)
}

fn test_state(runtime_home: &RuntimeHome) -> AppState {
    AppState::new(
        runtime_home.0.clone(),
        "http://127.0.0.1:8457".to_owned(),
        Db::open_in_memory().expect("open in-memory database"),
        false,
        AgentSeedStore::not_configured_dev(),
    )
    .expect("construct app state")
}

#[tokio::test]
async fn discovers_exact_model_ids_and_sends_the_native_credential_only_upstream() {
    let (endpoint, server) = mock_provider(
        "200 OK",
        r#"{"data":[{"id":"org/model:latest"},{"id":"model-2"}]}"#,
    )
    .await;
    let credential = "native-provider-key-never-return-this";
    let mut headers = HeaderMap::new();
    headers.insert(
        NATIVE_PROVIDER_CREDENTIAL_HEADER,
        credential.parse().unwrap(),
    );

    let Json(response) =
        discover_provider_models(headers, Json(DiscoverProviderModelsRequest { endpoint }))
            .await
            .expect("discovery succeeds");
    let request = server.await.unwrap();

    assert!(request.starts_with("GET /v1/models HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {credential}\r\n"))
    );
    assert_eq!(
        response
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        ["org/model:latest", "model-2"]
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(&response.refreshed_at).is_ok(),
        "the response includes a valid discovery timestamp"
    );
    let serialized = serde_json::to_string(&response).unwrap();
    assert!(!serialized.contains(credential));
    assert!(!format!("{response:?}").contains(credential));
}

#[tokio::test]
async fn every_discovery_error_has_a_stable_sanitized_http_mapping() {
    let cases = [
        (
            ProviderDiscoveryError::InvalidEndpoint,
            StatusCode::BAD_REQUEST,
            "PROVIDER_DISCOVERY_INVALID_ENDPOINT",
        ),
        (
            ProviderDiscoveryError::Unauthorized,
            StatusCode::UNAUTHORIZED,
            "PROVIDER_DISCOVERY_UNAUTHORIZED",
        ),
        (
            ProviderDiscoveryError::Unreachable,
            StatusCode::SERVICE_UNAVAILABLE,
            "PROVIDER_DISCOVERY_UNREACHABLE",
        ),
        (
            ProviderDiscoveryError::Timeout,
            StatusCode::GATEWAY_TIMEOUT,
            "PROVIDER_DISCOVERY_TIMEOUT",
        ),
        (
            ProviderDiscoveryError::InvalidResponse,
            StatusCode::BAD_GATEWAY,
            "PROVIDER_DISCOVERY_INVALID_RESPONSE",
        ),
        (
            ProviderDiscoveryError::EmptyModels,
            StatusCode::BAD_GATEWAY,
            "PROVIDER_DISCOVERY_EMPTY_MODELS",
        ),
    ];

    for (discovery_error, status, code) in cases {
        let error = map_discovery_error(discovery_error);
        assert_eq!(error.status(), status);
        assert_eq!(error.code(), Some(code));
        let response = error.into_response();
        assert_eq!(response.status(), status);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read sanitized error body");
        let body = String::from_utf8(body.to_vec()).expect("error body is UTF-8");
        assert!(body.contains(code));
        assert!(!body.contains("private-provider-credential"));
        assert!(!body.contains("private-provider-response"));
        assert!(!body.contains("private-endpoint-query"));
    }
}

#[tokio::test]
async fn provider_error_body_and_credential_are_not_echoed() {
    let provider_body = "private-upstream-response";
    let credential = "private-provider-credential";
    let (endpoint, server) = mock_provider("401 Unauthorized", provider_body).await;
    let mut headers = HeaderMap::new();
    headers.insert(
        NATIVE_PROVIDER_CREDENTIAL_HEADER,
        credential.parse().unwrap(),
    );

    let error = discover_provider_models(
        headers,
        Json(DiscoverProviderModelsRequest {
            endpoint: format!("{endpoint}?token=private-endpoint-query"),
        }),
    )
    .await
    .expect_err("provider rejects its credential");
    let request = server.await.unwrap();
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(request.contains(credential));
    assert!(!body.contains(credential));
    assert!(!body.contains(provider_body));
    assert!(!body.contains("private-endpoint-query"));
}

#[tokio::test]
async fn provider_discovery_route_rejects_missing_runtime_bearer_auth() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some("runtime-route-token"));
    let home = RuntimeHome::new();
    let app = build_router(test_state(&home));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/ai-providers/models")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"endpoint":"http://127.0.0.1:1/v1"}"#))
                .expect("build request"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn provider_secret_debug_is_redacted() {
    let key = "private-provider-credential";
    let secret = ProviderSecret::new(key.to_owned());

    assert!(!format!("{secret:?}").contains(key));
}
