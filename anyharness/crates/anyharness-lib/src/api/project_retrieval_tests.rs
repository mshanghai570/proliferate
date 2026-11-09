use super::router::build_router;
use crate::{
    app::{test_support, AppState},
    domains::{
        agents::installer::seed::AgentSeedStore, extensions::installer_tests::TestDirectory,
    },
    persistence::Db,
};
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
        builder = builder.header("authorization", "Bearer retrieval-test-token");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}
#[tokio::test]
async fn project_retrieval_routes_index_search_clear_and_reject_unauthorized_reads() {
    let _lock = test_support::lock_env().await;
    let _token = test_support::set_bearer_token_env(Some("retrieval-test-token"));
    let home = TestDirectory::new();
    let project = TestDirectory::new();
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(&project.0)
        .status()
        .unwrap()
        .success());
    std::fs::write(project.0.join("account.rs"), "fn account_login() {}\n").unwrap();
    let state = AppState::new(
        home.0.clone(),
        "http://127.0.0.1:8457".into(),
        Db::open(&home.0).unwrap(),
        true,
        AgentSeedStore::not_configured_dev(),
    )
    .unwrap();
    let workspace = state
        .workspace_runtime
        .create_workspace(&project.0.display().to_string())
        .unwrap()
        .workspace;
    let path = format!("/v1/workspaces/{}/retrieval", workspace.id);
    let search = format!("{path}/search");
    let router = build_router(state.clone());
    for (method, target) in [
        ("GET", &path),
        ("POST", &path),
        ("DELETE", &path),
        ("POST", &search),
    ] {
        assert_eq!(
            router
                .clone()
                .oneshot(request(method, target, "{}", false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        router
            .clone()
            .oneshot(request("POST", &search, r#"{"query":"account"}"#, true))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let response = router
        .clone()
        .oneshot(request("POST", &path, "", true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = router
        .clone()
        .oneshot(request("POST", &search, r#"{"query":"account"}"#, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap();
    assert_eq!(value["workspaceId"], workspace.id);
    assert_eq!(value["excerpts"][0]["path"], "account.rs");
    assert_eq!(value["excerpts"][0]["startLine"], 1);
    assert_eq!(value["excerpts"][0]["endLine"], 1);
    assert_eq!(
        router
            .clone()
            .oneshot(request("DELETE", &path, "", true))
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let response = router
        .clone()
        .oneshot(request("GET", &path, "", true))
        .await
        .unwrap();
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["enabled"], false);
    assert_eq!(
        router
            .oneshot(request("POST", &path, "", true))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    state
        .workspace_runtime
        .delete_workspace_record(&workspace.id)
        .unwrap();
    assert!(
        !state
            .project_retrieval_manager
            .status(&workspace.id)
            .unwrap()
            .enabled
    );
}
#[test]
fn project_retrieval_api_is_owner_only_and_has_unique_operation_ids() {
    use super::auth::{user_route_allowed, ClaimPermissions, UserClaimAuth};
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
        (Method::DELETE, ""),
        (Method::POST, "/search"),
    ] {
        assert!(user_route_allowed(
            &method,
            &format!("/v1/workspaces/w/retrieval{suffix}"),
            &claim
        )
        .is_err());
    }
    let api: Value = serde_json::from_str(&super::openapi::openapi_json()).unwrap();
    assert_eq!(
        api["paths"]["/v1/workspaces/{workspace_id}/retrieval/search"]["post"]["operationId"],
        "search_project_context"
    );
}
