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
        builder = builder.header("authorization", "Bearer mcp-test-token");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn project_mcp_routes_manage_encrypted_write_only_configuration() {
    let _lock = test_support::lock_env().await;
    let _token = test_support::set_bearer_token_env(Some("mcp-test-token"));
    let _key = test_support::set_data_key_env(Some("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY="));
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
    let base = "/v1/repo-roots/repo-root-1/mcp-servers";
    let path = format!("{base}/docs");
    let body = json!({"transport":"http", "connectionId":"ignored", "serverName":"ignored", "url":"http://localhost:8000/mcp?key=private-marker", "headers":[{"name":"Authorization","value":"Bearer private-marker"}]}).to_string();
    for method in ["PUT", "PATCH", "DELETE"] {
        assert_eq!(
            router
                .clone()
                .oneshot(request(method, &path, &body, false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        router
            .clone()
            .oneshot(request("GET", base, "", false))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let saved = router
        .clone()
        .oneshot(request("PUT", &path, &body, true))
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::OK);
    let saved: Value =
        serde_json::from_slice(&to_bytes(saved.into_body(), 65_536).await.unwrap()).unwrap();
    assert_eq!(
        saved,
        json!({"name":"docs", "transport":"http", "enabled":true})
    );
    let response = router
        .clone()
        .oneshot(request("GET", base, "", true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 65_536).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("private-marker"));
    let inventory: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        inventory["servers"],
        json!([{"name":"docs", "transport":"http", "enabled":true}])
    );
    assert_eq!(inventory["encryptionAvailable"], true);
    assert_eq!(
        router
            .clone()
            .oneshot(request("PATCH", &path, r#"{"enabled":false}"#, true))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    for bad in [
        r#"{"transport":"private-marker"}"#,
        r#"{"transport":"stdio","command":null,"secret":"private-marker"}"#,
        "{private-marker",
    ] {
        let response = router
            .clone()
            .oneshot(request("PUT", &path, bad, true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(response.into_body(), 65_536).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-marker"));
    }
    for _ in 0..2 {
        assert_eq!(
            router
                .clone()
                .oneshot(request("DELETE", &path, "", true))
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
    }
    let response = router
        .oneshot(request("GET", base, "", true))
        .await
        .unwrap();
    let inventory: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65_536).await.unwrap()).unwrap();
    assert_eq!(inventory["servers"], json!([]));
}

#[test]
fn project_mcp_management_is_not_a_workspace_claim_capability() {
    use crate::api::auth::{user_route_allowed, ClaimPermissions, UserClaimAuth};
    use axum::http::Method;
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
    for (method, suffix) in [
        (Method::GET, ""),
        (Method::PUT, "/docs"),
        (Method::PATCH, "/docs"),
        (Method::DELETE, "/docs"),
    ] {
        assert!(user_route_allowed(
            &method,
            &format!("/v1/repo-roots/repo-root-1/mcp-servers{suffix}"),
            &claim
        )
        .is_err());
    }
}
