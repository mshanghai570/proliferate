use std::path::PathBuf;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use serde_json::{Value, json};
use tower::util::ServiceExt;
use uuid::Uuid;

use super::{auth, router::build_router};
use crate::{
    app::{AppState, test_support},
    domains::agents::installer::seed::AgentSeedStore,
    persistence::Db,
};

const BEARER: &str = "mcp-server-api-test-token";
const DATA_KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
const STDIO_SECRET: &str = "stdio-secret-never-return-this";
const HTTP_SECRET: &str = "header-secret-never-return-this";

struct RuntimeHome(PathBuf);

impl RuntimeHome {
    fn new() -> Self {
        Self(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!("mcp-server-api-test-{}", Uuid::new_v4())),
        )
    }
}

impl Drop for RuntimeHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn state(home: &RuntimeHome) -> AppState {
    AppState::new(
        home.0.clone(),
        "http://127.0.0.1:8457".to_owned(),
        Db::open_in_memory().expect("open in-memory database"),
        true,
        AgentSeedStore::not_configured_dev(),
    )
    .expect("construct app state")
}

fn request(method: &str, path: &str, body: Option<Value>, authenticated: bool) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if authenticated {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {BEARER}"));
    }
    let body = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    builder.body(body).expect("build request")
}

async fn response_text(response: Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    String::from_utf8(body.to_vec()).expect("response body is UTF-8")
}

async fn assert_bounded_error(
    response: Response,
    status: StatusCode,
    code: &str,
    absent_text: &[&str],
) {
    assert_eq!(response.status(), status);
    let body = response_text(response).await;
    let problem: Value = serde_json::from_str(&body).expect("structured problem response");
    assert_eq!(problem["code"], code);
    for text in absent_text {
        assert!(!body.contains(text), "error body leaked {text:?}");
    }
}

fn create_stdio(name: &str, secret: &str) -> Value {
    json!({
        "serverName": name,
        "description": "Filesystem tools",
        "config": {
            "transport": "stdio",
            "command": "mcp-filesystem",
            "args": ["/private/user/home"],
            "env": [{"name": "ACCESS_TOKEN", "value": secret}]
        }
    })
}

#[tokio::test]
async fn all_management_routes_are_behind_the_runtime_bearer_boundary() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some(BEARER));
    let _key = test_support::set_data_key_env(Some(DATA_KEY));
    let home = RuntimeHome::new();
    let router = build_router(state(&home));

    for (method, path, body) in [
        ("GET", "/v1/mcp-servers", None),
        ("POST", "/v1/mcp-servers", Some(json!({}))),
        (
            "PUT",
            "/v1/mcp-servers/server-id/enabled",
            Some(json!(true)),
        ),
        ("DELETE", "/v1/mcp-servers/server-id", None),
    ] {
        let response = router
            .clone()
            .oneshot(request(method, path, body, false))
            .await
            .expect("router responds");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {path} must require runtime bearer auth"
        );
    }
}

#[tokio::test]
async fn management_contract_round_trips_selection_without_returning_secrets_or_paths() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some(BEARER));
    let _key = test_support::set_data_key_env(Some(DATA_KEY));
    let home = RuntimeHome::new();
    let state = state(&home);
    test_support::seed_repo_root(
        &state.db,
        "repo-root-existing",
        home.0.to_str().expect("runtime home path"),
    );
    let router = build_router(state.clone());

    let created_stdio = router
        .clone()
        .oneshot(request(
            "POST",
            "/v1/mcp-servers",
            Some(create_stdio("filesystem", STDIO_SECRET)),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(created_stdio.status(), StatusCode::CREATED);
    let created_stdio_body = response_text(created_stdio).await;
    assert!(!created_stdio_body.contains(STDIO_SECRET));
    assert!(!created_stdio_body.contains(home.0.to_str().unwrap()));
    let created_stdio: Value = serde_json::from_str(&created_stdio_body).unwrap();
    let stdio_id = created_stdio["id"].as_str().unwrap().to_owned();
    assert_eq!(created_stdio["serverName"], "filesystem");
    assert_eq!(created_stdio["envNames"], json!(["ACCESS_TOKEN"]));
    assert!(created_stdio["command"].is_null());
    assert!(created_stdio["args"].is_null());

    let created_http = router
        .clone()
        .oneshot(request(
            "POST",
            "/v1/mcp-servers",
            Some(json!({
                "serverName": "remote-tools",
                "config": {
                    "transport": "http",
                    "url": "https://mcp.example.invalid/mcp",
                    "headers": [{"name": "Authorization", "value": HTTP_SECRET}]
                }
            })),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(created_http.status(), StatusCode::CREATED);
    let created_http_body = response_text(created_http).await;
    assert!(!created_http_body.contains(HTTP_SECRET));
    assert!(!created_http_body.contains(home.0.to_str().unwrap()));

    let listed = router
        .clone()
        .oneshot(request("GET", "/v1/mcp-servers", None, true))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = response_text(listed).await;
    assert!(!listed_body.contains(STDIO_SECRET));
    assert!(!listed_body.contains(HTTP_SECRET));
    assert!(!listed_body.contains(home.0.to_str().unwrap()));
    let listed: Value = serde_json::from_str(&listed_body).unwrap();
    assert_eq!(listed["servers"].as_array().unwrap().len(), 2);

    let user_default = router
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/v1/mcp-servers/{stdio_id}/enabled"),
            Some(json!(true)),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(user_default.status(), StatusCode::OK);
    let user_default_body = response_text(user_default).await;
    assert!(!user_default_body.contains(STDIO_SECRET));
    let user_default: Value = serde_json::from_str(&user_default_body).unwrap();
    assert_eq!(user_default["userEnabled"], true);
    assert_eq!(user_default["repositoryOverrides"], json!({}));

    let repo_override = router
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/v1/mcp-servers/{stdio_id}/enabled"),
            Some(json!({"repoRootId": "repo-root-existing", "enabled": false})),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(repo_override.status(), StatusCode::OK);
    let repo_override_body = response_text(repo_override).await;
    assert!(!repo_override_body.contains(STDIO_SECRET));
    let repo_override: Value = serde_json::from_str(&repo_override_body).unwrap();
    assert_eq!(repo_override["userEnabled"], true);
    assert_eq!(
        repo_override["repositoryOverrides"]["repo-root-existing"],
        false
    );

    let reset_override = router
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/v1/mcp-servers/{stdio_id}/enabled"),
            Some(json!({"repoRootId": "repo-root-existing", "enabled": null})),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(reset_override.status(), StatusCode::OK);
    let reset_override_body = response_text(reset_override).await;
    assert!(!reset_override_body.contains(STDIO_SECRET));
    let reset_override: Value = serde_json::from_str(&reset_override_body).unwrap();
    assert_eq!(reset_override["userEnabled"], true);
    assert_eq!(reset_override["repositoryOverrides"], json!({}));

    for id in [
        stdio_id,
        created_http_body.parse::<Value>().unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned(),
    ] {
        let deleted = router
            .clone()
            .oneshot(request(
                "DELETE",
                &format!("/v1/mcp-servers/{id}"),
                None,
                true,
            ))
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
        assert!(response_text(deleted).await.is_empty());
    }
}

#[tokio::test]
async fn create_rejects_bounds_with_a_typed_error_without_echoing_input() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some(BEARER));
    let _key = test_support::set_data_key_env(Some(DATA_KEY));
    let home = RuntimeHome::new();
    let router = build_router(state(&home));

    let oversized_args = json!({
        "serverName": "too-many-args",
        "config": {
            "transport": "stdio",
            "command": "private-command-path",
            "args": (0..33).map(|index| format!("argument-{index}")).collect::<Vec<_>>(),
            "env": []
        }
    });
    let too_many_env = json!({
        "serverName": "too-many-env",
        "config": {
            "transport": "stdio",
            "command": "private-command-path",
            "args": [],
            "env": (0..17).map(|index| json!({"name": format!("ENV_{index}"), "value": STDIO_SECRET})).collect::<Vec<_>>()
        }
    });
    let too_many_headers = json!({
        "serverName": "too-many-headers",
        "config": {
            "transport": "http",
            "url": "https://private.invalid",
            "headers": (0..17).map(|index| json!({"name": format!("X-{index}"), "value": HTTP_SECRET})).collect::<Vec<_>>()
        }
    });
    let oversized_name = json!({
        "serverName": "x".repeat(101),
        "config": {"transport": "stdio", "command": "mcp-server"}
    });
    let oversized_command = json!({
        "serverName": "too-long-command",
        "config": {"transport": "stdio", "command": "x".repeat(4097)}
    });

    for body in [
        oversized_args,
        too_many_env,
        too_many_headers,
        oversized_name,
        oversized_command,
    ] {
        let response = router
            .clone()
            .oneshot(request("POST", "/v1/mcp-servers", Some(body), true))
            .await
            .unwrap();
        assert_bounded_error(
            response,
            StatusCode::BAD_REQUEST,
            "MCP_SERVER_OPERATION_FAILED",
            &[
                "private-command-path",
                "private.invalid",
                STDIO_SECRET,
                HTTP_SECRET,
                "argument-32",
            ],
        )
        .await;
    }
}

#[tokio::test]
async fn missing_data_key_has_a_stable_bounded_error() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some(BEARER));
    let _key = test_support::set_data_key_env(None);
    let home = RuntimeHome::new();
    let router = build_router(state(&home));

    let response = router
        .oneshot(request(
            "POST",
            "/v1/mcp-servers",
            Some(create_stdio("requires-key", STDIO_SECRET)),
            true,
        ))
        .await
        .unwrap();
    assert_bounded_error(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "MCP_SERVER_DATA_KEY_REQUIRED",
        &[STDIO_SECRET, home.0.to_str().unwrap()],
    )
    .await;
}

#[tokio::test]
async fn repository_overrides_validate_existing_root_and_id_bounds() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some(BEARER));
    let _key = test_support::set_data_key_env(None);
    let home = RuntimeHome::new();
    let state = state(&home);
    test_support::seed_repo_root(&state.db, "repo-root-existing", "/private/repo");
    let router = build_router(state);
    let created = router
        .clone()
        .oneshot(request(
            "POST",
            "/v1/mcp-servers",
            Some(json!({
                "serverName": "no-secrets",
                "config": {"transport": "stdio", "command": "mcp-server"}
            })),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: Value = serde_json::from_str(&response_text(created).await).unwrap();
    let id = created["id"].as_str().unwrap();

    for repo_root_id in [
        "missing-root".to_owned(),
        " ".to_owned(),
        "x".repeat(129),
        "bad\u{1}root".to_owned(),
    ] {
        let response = router
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/v1/mcp-servers/{id}/enabled"),
                Some(json!({"repoRootId": repo_root_id, "enabled": true})),
                true,
            ))
            .await
            .unwrap();
        assert_bounded_error(
            response,
            StatusCode::BAD_REQUEST,
            "MCP_SERVER_OPERATION_FAILED",
            &["/private/repo", "missing-root", "bad\u{1}root"],
        )
        .await;
    }

    let accepted = router
        .oneshot(request(
            "PUT",
            &format!("/v1/mcp-servers/{id}/enabled"),
            Some(json!({"repoRootId": "repo-root-existing", "enabled": true})),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
}

#[tokio::test]
async fn malformed_sealed_metadata_fails_the_listing_instead_of_disappearing() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some(BEARER));
    let _key = test_support::set_data_key_env(Some(DATA_KEY));
    let home = RuntimeHome::new();
    let state = state(&home);
    let router = build_router(state.clone());
    let created = router
        .clone()
        .oneshot(request(
            "POST",
            "/v1/mcp-servers",
            Some(create_stdio("corrupt-record", STDIO_SECRET)),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    state
        .db
        .with_conn(|connection| {
            connection.execute(
                "UPDATE user_mcp_servers SET env_sealed = ?1 WHERE server_name = ?2",
                rusqlite::params!["not-json-private-diagnostic", "corrupt-record"],
            )?;
            Ok(())
        })
        .unwrap();

    let response = router
        .oneshot(request("GET", "/v1/mcp-servers", None, true))
        .await
        .unwrap();
    assert_bounded_error(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "MCP_SERVERS_UNAVAILABLE",
        &[
            "not-json-private-diagnostic",
            STDIO_SECRET,
            home.0.to_str().unwrap(),
        ],
    )
    .await;
}

#[test]
fn workspace_scoped_claims_cannot_manage_runtime_global_mcp_servers() {
    use axum::http::Method;

    let claim = auth::UserClaimAuth {
        user_id: "user".into(),
        organization_id: "org".into(),
        target_id: "target".into(),
        cloud_workspace_id: "cloud-workspace".into(),
        anyharness_workspace_id: "workspace".into(),
        cloud_session_id: None,
        anyharness_session_id: None,
        claim_id: "claim".into(),
        permissions: auth::ClaimPermissions {
            read: true,
            write: true,
            control: true,
        },
        jti: "claim-test".into(),
        expires_at: i64::MAX,
    };

    for (method, path) in [
        (Method::GET, "/v1/mcp-servers"),
        (Method::POST, "/v1/mcp-servers"),
        (Method::PUT, "/v1/mcp-servers/server-id/enabled"),
        (Method::DELETE, "/v1/mcp-servers/server-id"),
    ] {
        assert!(matches!(
            auth::user_route_allowed(&method, path, &claim),
            Err(auth::AuthError::UnsupportedRoute)
        ));
    }
}
