use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt;

use super::router::build_router;
use crate::adapters::extensions::registry_tests::{Route, hex_sha256, serve, standard_routes};
use crate::app::{AppState, test_support};
use crate::domains::agents::installer::seed::AgentSeedStore;
use crate::domains::extensions::installer_tests::{TestDirectory, archive, identity, manifest};
use crate::domains::extensions::manager::ExtensionManager;
use crate::persistence::Db;

fn state(home: &TestDirectory) -> AppState {
    AppState::new(
        home.0.clone(),
        "http://127.0.0.1:8457".into(),
        Db::open(&home.0).unwrap(),
        true,
        AgentSeedStore::not_configured_dev(),
    )
    .unwrap()
}

fn request(method: &str, path: &str, authenticated: bool) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if authenticated {
        builder = builder.header("authorization", "Bearer extension-test-token");
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn extensions_list_and_uninstall_use_real_registered_state() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some("extension-test-token"));
    let _key = test_support::set_data_key_env(None);
    let home = TestDirectory::new();
    let state = state(&home);
    let manager = ExtensionManager::new(state.db.clone(), home.0.clone());
    for version in ["1.0.0", "2.0.0"] {
        let (archive, digest) = archive(&home.0, &manifest(version), "extension/themes/demo.json");
        manager
            .install(&archive, &identity(version), &digest)
            .unwrap();
    }
    let router = build_router(state);
    let response = router
        .clone()
        .oneshot(request("GET", "/v1/extensions", true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains(home.0.to_str().unwrap()));
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["extensions"].as_array().unwrap().len(), 2);
    assert_eq!(body["extensions"][0]["active"], false);
    assert_eq!(body["extensions"][0]["state"], "installed");
    for _ in 0..2 {
        let response = router
            .clone()
            .oneshot(request("DELETE", "/v1/extensions/example/demo/1.0.0", true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
    assert_eq!(manager.list().unwrap().len(), 1);
    assert_eq!(manager.list().unwrap()[0].identity.version, "2.0.0");
}

#[tokio::test]
async fn extensions_routes_require_authentication_and_sanitize_errors() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some("extension-test-token"));
    let _key = test_support::set_data_key_env(None);
    let home = TestDirectory::new();
    let state = state(&home);
    let manager = ExtensionManager::new(state.db.clone(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let router = build_router(state);
    for method in ["GET", "DELETE"] {
        let path = if method == "GET" {
            "/v1/extensions"
        } else {
            "/v1/extensions/example/demo/1.0.0"
        };
        assert_eq!(
            router
                .clone()
                .oneshot(request(method, path, false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(manager.list().unwrap().len(), 1);
    let response = router
        .clone()
        .oneshot(request(
            "DELETE",
            "/v1/extensions/example/demo/invalid",
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    std::fs::write(
        home.0
            .join("extensions/packages/example.demo-1.0.0/.proliferate-package.json"),
        "{}",
    )
    .unwrap();
    let response = router
        .oneshot(request("DELETE", "/v1/extensions/example/demo/1.0.0", true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!text.contains(home.0.to_str().unwrap()));
    assert!(text.contains("EXTENSION_PACKAGE_INVALID"));
    assert!(
        home.0
            .join("extensions/packages/example.demo-1.0.0")
            .exists()
    );
}

fn install_request(body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/extensions/install")
        .header("authorization", "Bearer extension-test-token")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn install_route_fetches_verifies_and_registers_registry_package() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some("extension-test-token"));
    let _key = test_support::set_data_key_env(None);
    let home = TestDirectory::new();
    let mut state = state(&home);
    let (archive_path, _) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    let bytes = std::fs::read(&archive_path).unwrap();
    state.extension_registry_base_url = serve(standard_routes(
        format!("{}  demo.vsix\n", hex_sha256(&bytes)),
        &bytes,
    ));
    let manager = ExtensionManager::new(state.db.clone(), home.0.clone());
    let router = build_router(state);
    let response = router
        .oneshot(install_request(serde_json::json!({
            "namespace": "example",
            "name": "demo",
            "version": "1.0.0",
        })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["version"], "1.0.0");
    assert_eq!(manager.list().unwrap().len(), 1);
}

#[tokio::test]
async fn install_route_reports_registry_miss_without_registering() {
    let _lock = test_support::lock_env().await;
    let _bearer = test_support::set_bearer_token_env(Some("extension-test-token"));
    let _key = test_support::set_data_key_env(None);
    let home = TestDirectory::new();
    let mut state = state(&home);
    state.extension_registry_base_url = serve(Vec::<Route>::new());
    let manager = ExtensionManager::new(state.db.clone(), home.0.clone());
    let router = build_router(state);
    let response = router
        .oneshot(install_request(serde_json::json!({
            "namespace": "example",
            "name": "demo",
            "version": "9.9.9",
        })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("EXTENSION_NOT_FOUND"));
    assert!(manager.list().unwrap().is_empty());
}

#[test]
fn extensions_openapi_declares_routes_and_typed_inactive_state() {
    let api: Value = serde_json::from_str(&super::openapi::openapi_json()).unwrap();
    assert!(api["paths"]["/v1/extensions"]["get"].is_object());
    assert!(api["paths"]["/v1/extensions/{namespace}/{name}/{version}"]["delete"].is_object());
    assert!(
        api["components"]["schemas"]["InstalledExtensionSummary"]["properties"]["active"]
            .is_object()
    );
}

#[test]
fn extensions_global_management_remains_denied_to_workspace_claims() {
    use super::auth::{AuthError, ClaimPermissions, UserClaimAuth, user_route_allowed};
    use axum::http::Method;
    let claim = UserClaimAuth {
        user_id: "user".into(),
        organization_id: "org".into(),
        target_id: "target".into(),
        cloud_workspace_id: "cloud-workspace".into(),
        anyharness_workspace_id: "workspace".into(),
        cloud_session_id: None,
        anyharness_session_id: None,
        claim_id: "claim".into(),
        permissions: ClaimPermissions {
            read: true,
            write: true,
            control: true,
        },
        jti: "claim-test".into(),
        expires_at: i64::MAX,
    };
    for (method, path) in [
        (Method::GET, "/v1/extensions"),
        (Method::DELETE, "/v1/extensions/example/demo/1.0.0"),
    ] {
        assert!(matches!(
            user_route_allowed(&method, path, &claim),
            Err(AuthError::UnsupportedRoute)
        ));
    }
}
