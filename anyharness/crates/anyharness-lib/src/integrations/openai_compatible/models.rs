use std::time::Duration;

use anyharness_contract::v1::ai_providers::{ProviderDiscoveryError, ProviderModel};
use futures::StreamExt;
use reqwest::{Client, StatusCode, redirect::Policy};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::ProviderSecret;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub(crate) fn parse_endpoint(endpoint: &str) -> Result<Url, ProviderDiscoveryError> {
    if has_user_info_in_input(endpoint) {
        return Err(ProviderDiscoveryError::InvalidEndpoint);
    }
    let endpoint = Url::parse(endpoint).map_err(|_| ProviderDiscoveryError::InvalidEndpoint)?;
    validate_endpoint(&endpoint)?;
    Ok(endpoint)
}

pub(crate) async fn discover_models(
    endpoint: &Url,
    api_key: Option<&ProviderSecret>,
    cancel: CancellationToken,
) -> Result<Vec<ProviderModel>, ProviderDiscoveryError> {
    discover_models_with_timeouts(endpoint, api_key, cancel, CONNECT_TIMEOUT, REQUEST_TIMEOUT).await
}

async fn discover_models_with_timeouts(
    endpoint: &Url,
    api_key: Option<&ProviderSecret>,
    cancel: CancellationToken,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<Vec<ProviderModel>, ProviderDiscoveryError> {
    validate_endpoint(endpoint)?;
    let models_url = models_url(endpoint)?;

    if cancel.is_cancelled() {
        return Err(ProviderDiscoveryError::Unreachable);
    }

    let client = Client::builder()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .redirect(Policy::none())
        .build()
        .map_err(|_| ProviderDiscoveryError::Unreachable)?;

    let request = async {
        let mut request = client
            .get(models_url)
            .header(reqwest::header::ACCEPT, "application/json");
        if let Some(api_key) = api_key {
            request = request.bearer_auth(api_key.expose());
        }

        let response = request.send().await.map_err(classify_request_error)?;
        if response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::FORBIDDEN
        {
            return Err(ProviderDiscoveryError::Unauthorized);
        }
        if !response.status().is_success() {
            return Err(ProviderDiscoveryError::InvalidResponse);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(ProviderDiscoveryError::InvalidResponse);
        }

        let mut body = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(0)
                .min(MAX_RESPONSE_BYTES as u64) as usize,
        );
        let mut chunks = response.bytes_stream();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.map_err(classify_request_error)?;
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(ProviderDiscoveryError::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }

        parse_models(&body)
    };

    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ProviderDiscoveryError::Unreachable),
        result = tokio::time::timeout(request_timeout, request) => {
            result.unwrap_or(Err(ProviderDiscoveryError::Timeout))
        }
    }
}

fn validate_endpoint(endpoint: &Url) -> Result<(), ProviderDiscoveryError> {
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none_or(str::is_empty)
        || has_user_info(endpoint)
        || endpoint.fragment().is_some()
    {
        return Err(ProviderDiscoveryError::InvalidEndpoint);
    }
    Ok(())
}

fn has_user_info(endpoint: &Url) -> bool {
    let authority = endpoint
        .as_str()
        .split_once("://")
        .map(|(_, remainder)| remainder)
        .and_then(|remainder| remainder.split(['/', '?', '#']).next());
    !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || authority.is_some_and(|s| s.contains('@'))
}

fn has_user_info_in_input(endpoint: &str) -> bool {
    endpoint
        .trim()
        .split_once("://")
        .map(|(_, remainder)| remainder)
        .and_then(|remainder| remainder.split(['/', '?', '#']).next())
        .is_some_and(|authority| authority.contains('@'))
}

fn models_url(endpoint: &Url) -> Result<Url, ProviderDiscoveryError> {
    let mut models_url = endpoint.clone();
    models_url
        .path_segments_mut()
        .map_err(|_| ProviderDiscoveryError::InvalidEndpoint)?
        .pop_if_empty()
        .push("models");
    Ok(models_url)
}

fn classify_request_error(error: reqwest::Error) -> ProviderDiscoveryError {
    if error.is_timeout() {
        ProviderDiscoveryError::Timeout
    } else {
        ProviderDiscoveryError::Unreachable
    }
}

fn parse_models(body: &[u8]) -> Result<Vec<ProviderModel>, ProviderDiscoveryError> {
    let response: ModelsResponse =
        serde_json::from_slice(body).map_err(|_| ProviderDiscoveryError::InvalidResponse)?;
    if response.data.is_empty() {
        return Err(ProviderDiscoveryError::EmptyModels);
    }
    if response.data.iter().any(|model| model.id.is_empty()) {
        return Err(ProviderDiscoveryError::InvalidResponse);
    }

    Ok(response
        .data
        .into_iter()
        .map(|model| ProviderModel {
            id: model.id,
            display_name: None,
        })
        .collect())
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelRow>,
}

#[derive(Deserialize)]
struct ModelRow {
    id: String,
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
    };

    use super::*;

    const TEST_TIMEOUT: Duration = Duration::from_millis(150);

    async fn listen() -> (Url, TcpListener) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = Url::parse(&format!(
            "http://{address}/base/v1/?region=local&token=query-secret"
        ))
        .unwrap();
        (endpoint, listener)
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
            assert!(
                request.len() < 16 * 1024,
                "request headers unexpectedly large"
            );
        }
        String::from_utf8(request).unwrap()
    }

    fn response(status: &str, body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    async fn serve_response(response: Vec<u8>) -> (Url, JoinHandle<String>) {
        let (endpoint, listener) = listen().await;
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            stream.write_all(&response).await.unwrap();
            request
        });
        (endpoint, task)
    }

    async fn assert_body_cancellation_or_timeout(
        cancel: Option<CancellationToken>,
    ) -> Result<Vec<ProviderModel>, ProviderDiscoveryError> {
        let (endpoint, listener) = listen().await;
        let (body_started_tx, body_started_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_request(&mut stream).await;
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 200\r\n\r\n{\"data\":[",
                )
                .await
                .unwrap();
            let _ = body_started_tx.send(());
            std::future::pending::<()>().await;
        });

        let should_cancel = cancel.is_some();
        let token = cancel.unwrap_or_default();
        let request_endpoint = endpoint.clone();
        let request_token = token.clone();
        let request = tokio::spawn(async move {
            discover_models_with_timeouts(
                &request_endpoint,
                None,
                request_token,
                Duration::from_secs(1),
                TEST_TIMEOUT,
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(1), body_started_rx)
            .await
            .unwrap()
            .unwrap();
        if should_cancel {
            tokio::time::sleep(Duration::from_millis(25)).await;
            token.cancel();
        }
        let result = tokio::time::timeout(Duration::from_secs(1), request)
            .await
            .unwrap()
            .unwrap();
        server.abort();
        result
    }

    #[test]
    fn accepts_http_https_and_local_or_lan_hosts() {
        for endpoint in [
            "http://127.0.0.1:8080/v1",
            "http://192.168.1.20:8080/custom?tenant=local",
            "https://api.example.test/v1",
        ] {
            assert!(parse_endpoint(endpoint).is_ok(), "{endpoint}");
        }
    }

    #[test]
    fn rejects_unsupported_or_ambiguous_endpoints_without_echoing_them() {
        for endpoint in [
            "file:///models",
            "ftp://localhost/models",
            "http://",
            "http://user:pass@localhost/v1?token=query-secret",
            "http://@localhost/v1",
            "http://:pass@localhost/v1",
            "http://localhost/v1#fragment-secret",
        ] {
            let error = parse_endpoint(endpoint).unwrap_err();
            assert_eq!(error, ProviderDiscoveryError::InvalidEndpoint);
            assert!(!format!("{error:?}").contains("secret"));
        }
    }

    #[tokio::test]
    async fn appends_models_to_the_base_path_and_preserves_the_query() {
        let body = r#"{"data":[{"id":"org/model:latest?variant=blue"}]}"#;
        let (endpoint, server) = serve_response(response("200 OK", body)).await;

        let models = discover_models(&endpoint, None, CancellationToken::new())
            .await
            .unwrap();
        let request = server.await.unwrap();

        assert!(
            request.starts_with("GET /base/v1/models?region=local&token=query-secret HTTP/1.1")
        );
        assert_eq!(models[0].id, "org/model:latest?variant=blue");
        assert_eq!(models[0].display_name, None);
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
    }

    #[tokio::test]
    async fn sends_optional_bearer_auth_only_on_the_configured_request() {
        let (endpoint, server) =
            serve_response(response("200 OK", r#"{"data":[{"id":"model"}]}"#)).await;
        let secret = ProviderSecret::new("test-api-key".to_owned());

        discover_models(&endpoint, Some(&secret), CancellationToken::new())
            .await
            .unwrap();
        let request = server.await.unwrap();

        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-api-key\r\n")
        );
        assert!(!request.contains("test-api-key?"));
        assert!(!format!("{secret:?}").contains("test-api-key"));
    }

    #[tokio::test]
    async fn rejects_invalid_json_shape_rows_and_oversized_bodies() {
        for (body, expected) in [
            ("not-json", ProviderDiscoveryError::InvalidResponse),
            ("{}", ProviderDiscoveryError::InvalidResponse),
            (r#"{"data":{}}"#, ProviderDiscoveryError::InvalidResponse),
            (r#"{"data":[{}]}"#, ProviderDiscoveryError::InvalidResponse),
            (
                r#"{"data":[{"id":null}]}"#,
                ProviderDiscoveryError::InvalidResponse,
            ),
            (
                r#"{"data":[{"id":""}]}"#,
                ProviderDiscoveryError::InvalidResponse,
            ),
        ] {
            let (endpoint, server) = serve_response(response("200 OK", body)).await;
            let result = discover_models(&endpoint, None, CancellationToken::new()).await;
            assert_eq!(result.unwrap_err(), expected, "body: {body}");
            server.await.unwrap();
        }

        let (endpoint, server) = serve_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\nConnection: close\r\n\r\n".to_vec(),
        )
        .await;
        assert_eq!(
            discover_models(&endpoint, None, CancellationToken::new())
                .await
                .unwrap_err(),
            ProviderDiscoveryError::InvalidResponse
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn distinguishes_empty_models_and_unauthorized_responses() {
        let (endpoint, server) = serve_response(response("200 OK", r#"{"data":[]}"#)).await;
        assert_eq!(
            discover_models(&endpoint, None, CancellationToken::new())
                .await
                .unwrap_err(),
            ProviderDiscoveryError::EmptyModels
        );
        server.await.unwrap();

        let (endpoint, server) =
            serve_response(response("401 Unauthorized", "secret response body")).await;
        let error = discover_models(&endpoint, None, CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(error, ProviderDiscoveryError::Unauthorized);
        assert!(!format!("{error:?}").contains("secret"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn classifies_unreachable_endpoints() {
        let (endpoint, listener) = listen().await;
        drop(listener);
        let result = discover_models(&endpoint, None, CancellationToken::new()).await;

        assert_eq!(result.unwrap_err(), ProviderDiscoveryError::Unreachable);
    }

    #[tokio::test]
    async fn times_out_while_waiting_for_response_headers() {
        let (endpoint, listener) = listen().await;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_request(&mut stream).await;
            std::future::pending::<()>().await;
        });

        let result = discover_models_with_timeouts(
            &endpoint,
            None,
            CancellationToken::new(),
            Duration::from_secs(1),
            TEST_TIMEOUT,
        )
        .await;

        assert_eq!(result.unwrap_err(), ProviderDiscoveryError::Timeout);
        server.abort();
    }

    #[tokio::test]
    async fn observes_cancellation_during_request_and_body_reading() {
        let (endpoint, listener) = listen().await;
        let (request_seen_tx, request_seen_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_request(&mut stream).await;
            let _ = request_seen_tx.send(());
            std::future::pending::<()>().await;
        });
        let cancel = CancellationToken::new();
        let request_endpoint = endpoint.clone();
        let request_cancel = cancel.clone();
        let request = tokio::spawn(async move {
            discover_models_with_timeouts(
                &request_endpoint,
                None,
                request_cancel,
                Duration::from_secs(1),
                Duration::from_secs(5),
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(1), request_seen_rx)
            .await
            .unwrap()
            .unwrap();
        cancel.cancel();
        assert_eq!(
            request.await.unwrap().unwrap_err(),
            ProviderDiscoveryError::Unreachable
        );
        server.abort();

        assert_eq!(
            assert_body_cancellation_or_timeout(Some(CancellationToken::new()))
                .await
                .unwrap_err(),
            ProviderDiscoveryError::Unreachable
        );
    }

    #[tokio::test]
    async fn times_out_while_reading_a_response_body() {
        assert_eq!(
            assert_body_cancellation_or_timeout(None).await.unwrap_err(),
            ProviderDiscoveryError::Timeout
        );
    }

    #[tokio::test]
    async fn does_not_follow_cross_origin_redirects_or_forward_credentials() {
        let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_address: SocketAddr = target_listener.local_addr().unwrap();
        let (endpoint, source_listener) = listen().await;
        let source_server = tokio::spawn(async move {
            let (mut stream, _) = source_listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            let redirect = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/steal\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(redirect.as_bytes()).await.unwrap();
            request
        });
        let target_server = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_millis(250), target_listener.accept()).await
        });
        let secret = ProviderSecret::new("must-not-forward".to_owned());

        assert_eq!(
            discover_models(&endpoint, Some(&secret), CancellationToken::new())
                .await
                .unwrap_err(),
            ProviderDiscoveryError::InvalidResponse
        );
        let source_request = source_server.await.unwrap();
        assert!(
            source_request
                .to_ascii_lowercase()
                .contains("authorization: bearer must-not-forward\r\n")
        );
        assert!(target_server.await.unwrap().is_err());
    }
}
