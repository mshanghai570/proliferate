use super::router::build_router;
use crate::app::{test_support, AppState};
use crate::domains::agents::installer::seed::AgentSeedStore;
use crate::domains::extensions::installer_tests::TestDirectory;
use crate::domains::repo_roots::test_support::seed_repo_root_1;
use crate::persistence::Db;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::util::ServiceExt;

fn request(method: &str, path: &str, body: &str, authenticated: bool) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if authenticated {
        builder = builder.header("authorization", "Bearer rules-test-token");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn agent_rules_routes_require_owner_and_preserve_repository_scope() {
    let _lock = test_support::lock_env().await;
    let _token = test_support::set_bearer_token_env(Some("rules-test-token"));
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    seed_repo_root_1(&db);
    let state = AppState::new(
        home.0.clone(),
        "http://127.0.0.1:8457".into(),
        db,
        true,
        AgentSeedStore::not_configured_dev(),
    )
    .unwrap();
    let router = build_router(state);
    let path = "/v1/agent-rules/codex";
    for method in ["GET", "PUT"] {
        assert_eq!(
            router
                .clone()
                .oneshot(request(method, path, "{}", false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        router
            .clone()
            .oneshot(request("PUT", path, "{}", true))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let text = "# User rules\n  Exact spacing.\n";
    let response = router
        .clone()
        .oneshot(request(
            "PUT",
            path,
            &json!({"text":text}).to_string(),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(
        value,
        json!({"agentKind":"codex", "repositoryId":null,"text":text,"inherited":false})
    );
    let project = "/v1/agent-rules/codex?repositoryId=repo-root-1";
    let response = router
        .clone()
        .oneshot(request("GET", project, "", true))
        .await
        .unwrap();
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["text"], text);
    assert_eq!(value["inherited"], true);
    let override_body = json!({"repositoryId":"repo-root-1", "text":""}).to_string();
    assert_eq!(
        router
            .clone()
            .oneshot(request("PUT", path, &override_body, true))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let response = router
        .clone()
        .oneshot(request("GET", project, "", true))
        .await
        .unwrap();
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["text"], "");
    assert_eq!(value["inherited"], false);
    for body in [
        json!({"text":"private-marker", "repositoryId":"missing"}),
        json!({"text":"x".repeat(65537)}),
    ] {
        let response = router
            .clone()
            .oneshot(request("PUT", path, &body.to_string(), true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-marker"));
    }
    assert_eq!(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/agent-rules/codex?repositoryId=missing",
                "",
                true
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        router
            .oneshot(request(
                "PUT",
                path,
                &json!({"repositoryId":"repo-root-1","text":null}).to_string(),
                true
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[test]
fn agent_rules_openapi_and_workspace_claim_denial() {
    use super::auth::{user_route_allowed, ClaimPermissions, UserClaimAuth};
    use axum::http::Method;
    let api: Value = serde_json::from_str(&super::openapi::openapi_json()).unwrap();
    assert_eq!(
        api["paths"]["/v1/agent-rules/{agent_kind}"]["get"]["operationId"],
        "get_agent_rules"
    );
    assert_eq!(
        api["paths"]["/v1/agent-rules/{agent_kind}"]["put"]["operationId"],
        "set_agent_rules"
    );
    let claim = UserClaimAuth {
        user_id: "u".into(),
        organization_id: "o".into(),
        target_id: "t".into(),
        cloud_workspace_id: "cw".into(),
        anyharness_workspace_id: "w".into(),
        cloud_session_id: None,
        anyharness_session_id: None,
        claim_id: "c".into(),
        permissions: ClaimPermissions {
            read: true,
            write: true,
            control: true,
        },
        jti: "j".into(),
        expires_at: i64::MAX,
    };
    for method in [Method::GET, Method::PUT] {
        assert!(user_route_allowed(&method, "/v1/agent-rules/codex", &claim).is_err());
    }
}
