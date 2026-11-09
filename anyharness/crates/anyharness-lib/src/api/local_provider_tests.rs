use super::router::build_router;
use crate::app::{test_support, AppState};
use crate::domains::{
    agents::{installer::seed::AgentSeedStore, route_auth},
    extensions::installer_tests::TestDirectory,
};
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
        builder = builder.header("authorization", "Bearer local-provider-test-token");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn local_provider_routes_use_owner_auth_and_deliver_launch_credentials_without_cloud() {
    let _lock = test_support::lock_env().await;
    let _token = test_support::set_bearer_token_env(Some("local-provider-test-token"));
    let _key = test_support::set_data_key_env(Some("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY="));
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    let state = AppState::new(
        home.0.clone(),
        "http://127.0.0.1:8457".into(),
        db,
        true,
        AgentSeedStore::not_configured_dev(),
    )
    .unwrap();
    let router = build_router(state);
    let base = "/v1/agent-auth/local-providers";
    let input = json!({"name":"My endpoint", "endpoint":"http://localhost:1234/v1", "models":["model-a"], "apiFormat":"chat", "apiKey":"private-provider-marker"}).to_string();
    for method in ["GET", "POST"] {
        assert_eq!(
            router
                .clone()
                .oneshot(request(method, base, &input, false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let saved = router
        .clone()
        .oneshot(request("POST", base, &input, true))
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::CREATED);
    let bytes = to_bytes(saved.into_body(), 65_536).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("private-provider-marker"));
    let saved: Value = serde_json::from_slice(&bytes).unwrap();
    let path = format!("{base}/{}", saved["id"].as_str().unwrap());
    assert!(route_auth::launch_route_provides_credentials(
        &home.0, "opencode"
    ));
    let profile = route_auth::resolve_launch_auth_profile(&home.0, "opencode").unwrap();
    assert!(matches!(
        profile,
        route_auth::AgentRuntimeAuthProfile::Sources(_)
    ));
    let inventory = router
        .clone()
        .oneshot(request("GET", base, "", true))
        .await
        .unwrap();
    let inventory: Value =
        serde_json::from_slice(&to_bytes(inventory.into_body(), 65_536).await.unwrap()).unwrap();
    assert_eq!(inventory["managedAuthActive"], false);
    assert_eq!(inventory["encryptionAvailable"], true);
    assert_eq!(inventory["providers"][0]["modelCount"], 1);
    assert!(inventory["providers"][0].get("apiKey").is_none());
    for method in ["PATCH", "DELETE"] {
        assert_eq!(
            router
                .clone()
                .oneshot(request(method, &path, r#"{"enabled":false}"#, false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        router
            .clone()
            .oneshot(request("PATCH", &path, r#"{"enabled":false}"#, true))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert!(!route_auth::launch_route_provides_credentials(
        &home.0, "opencode"
    ));
    assert_eq!(
        router
            .clone()
            .oneshot(request("DELETE", &path, "", true))
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let rejected = router
        .oneshot(request(
            "POST",
            base,
            r#"{"apiKey":"private-provider-marker"}"#,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    assert!(
        !String::from_utf8_lossy(&to_bytes(rejected.into_body(), 65_536).await.unwrap())
            .contains("private-provider-marker")
    );
}

#[test]
fn local_provider_management_is_not_a_workspace_claim_capability() {
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
        (Method::POST, ""),
        (Method::PATCH, "/one"),
        (Method::DELETE, "/one"),
    ] {
        assert!(user_route_allowed(
            &method,
            &format!("/v1/agent-auth/local-providers{suffix}"),
            &claim
        )
        .is_err());
    }
}
