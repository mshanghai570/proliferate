use super::*;
use crate::domains::agents::route_auth::{
    self,
    state::{AgentAuthState, HarnessAuth},
};
struct TestDirectory(std::path::PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("local-provider-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cipher() -> DataCipher {
    DataCipher::from_env_value("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=").unwrap()
}
fn input() -> ProviderInput {
    ProviderInput {
        name: "Local models".into(),
        endpoint: "http://localhost:1234/v1".into(),
        models: vec!["org/model-a".into(), "model-b".into()],
        api_format: "chat".into(),
        api_key: Some("private-test-marker".into()),
    }
}
fn manager(home: &TestDirectory) -> LocalProviderManager {
    LocalProviderManager::new(home.0.clone(), Some(cipher()))
}

#[test]
fn local_provider_storage_and_profile_keep_keys_encrypted_and_scoped() {
    let home = TestDirectory::new();
    let manager = manager(&home);
    let before = basis_revision(&home.0);
    let row = manager.create(input()).unwrap();
    let bytes = std::fs::read(store::directory(&home.0).join("providers.json")).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("private-test-marker"));
    assert_ne!(before, basis_revision(&home.0));
    assert_eq!(manager.list().unwrap().providers[0].model_count, 2);
    let AgentRuntimeAuthProfile::Sources(profile) = manager.profile().unwrap().unwrap() else {
        panic!("expected local sources")
    };
    assert!(profile.revision < 0);
    let ResolvedSource::ProviderConfig(config) = &profile.sources[0] else {
        panic!("expected provider config")
    };
    let json: serde_json::Value =
        serde_json::from_str(&config.env["OPENCODE_CONFIG_CONTENT"]).unwrap();
    let provider = &json["provider"][format!("custom-local-{}", row.id)];
    assert_eq!(provider["options"]["baseURL"], "http://localhost:1234/v1");
    assert!(provider["models"].get("org/model-a").is_some());
    assert!(!config.env["OPENCODE_CONFIG_CONTENT"].contains("private-test-marker"));
    assert!(config.env.iter().any(
        |(name, value)| name.starts_with("PROLIFERATE_CUSTOM_LOCAL_")
            && value == "private-test-marker"
    ));
    assert!(!format!("{profile:?}").contains("private-test-marker"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(store::directory(&home.0).join("providers.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn local_provider_keyless_connections_disable_ambient_key_fallback() {
    let home = TestDirectory::new();
    let manager = manager(&home);
    let mut config = input();
    config.api_key = None;
    config.api_format = "responses".into();
    manager.create(config).unwrap();
    let AgentRuntimeAuthProfile::Sources(profile) = manager.profile().unwrap().unwrap() else {
        panic!("expected local sources")
    };
    let ResolvedSource::ProviderConfig(config) = &profile.sources[0] else {
        panic!("expected provider config")
    };
    assert_eq!(config.env.len(), 1);
    let json: serde_json::Value =
        serde_json::from_str(&config.env["OPENCODE_CONFIG_CONTENT"]).unwrap();
    let provider = json["provider"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(provider["options"]["apiKey"], "");
    assert_eq!(provider["npm"], "@ai-sdk/openai");
}

#[test]
fn local_provider_missing_key_and_corruption_fail_closed_but_allow_removal() {
    let home = TestDirectory::new();
    let manager = manager(&home);
    let row = manager.create(input()).unwrap();
    use base64::Engine;
    let wrong_key =
        DataCipher::from_env_value(&base64::engine::general_purpose::STANDARD.encode([9_u8; 32]))
            .unwrap();
    let wrong = LocalProviderManager::new(home.0.clone(), Some(wrong_key));
    assert!(matches!(
        wrong.profile(),
        Err(LocalProviderError::Unavailable)
    ));
    let missing = LocalProviderManager::new(home.0.clone(), None);
    assert!(matches!(
        missing.profile(),
        Err(LocalProviderError::MissingKey)
    ));
    missing.set_enabled(&row.id, false).unwrap();
    assert!(missing.profile().unwrap().is_none());
    assert!(matches!(
        missing.set_enabled(&row.id, true),
        Err(LocalProviderError::MissingKey)
    ));
    manager.set_enabled(&row.id, true).unwrap();
    let mut catalog = store::read(&home.0).unwrap();
    catalog.providers[0].ciphertext = "v1:broken".into();
    store::write(&home.0, &mut catalog).unwrap();
    assert!(matches!(
        manager.profile(),
        Err(LocalProviderError::Unavailable)
    ));
    missing.remove(&row.id).unwrap();
    assert!(missing.list().unwrap().providers.is_empty());
}

#[test]
fn local_provider_validation_and_capacity_preserve_existing_connections() {
    let home = TestDirectory::new();
    let manager = manager(&home);
    for endpoint in [
        "file:///tmp/models",
        "https://user:secret@example.com/v1",
        "http://localhost:0/v1",
        "https://example.com/v1?key=secret",
        "https://example.com/{env:URL}",
    ] {
        let mut config = input();
        config.endpoint = endpoint.into();
        assert!(matches!(
            manager.create(config),
            Err(LocalProviderError::Invalid(_))
        ));
    }
    let mut config = input();
    config.models = vec!["model with spaces".into()];
    assert!(matches!(
        manager.create(config),
        Err(LocalProviderError::Invalid(_))
    ));
    for _ in 0..store::MAX_PROVIDERS {
        manager.create(input()).unwrap();
    }
    assert!(matches!(
        manager.create(input()),
        Err(LocalProviderError::Invalid(_))
    ));
    assert_eq!(
        manager.list().unwrap().providers.len(),
        store::MAX_PROVIDERS
    );
}

#[test]
fn local_provider_sources_never_replace_applied_managed_auth_or_other_harnesses() {
    let home = TestDirectory::new();
    let manager = manager(&home);
    let row = manager.create(input()).unwrap();
    let document = AgentAuthState {
        version: 2,
        revision: 7,
        user_id: None,
        issuing_server_origin: None,
        harnesses: vec![HarnessAuth {
            harness_kind: "opencode".into(),
            sources: vec![],
            settings: None,
        }],
    };
    route_auth::apply_state_file(&home.0, &document).unwrap();
    assert!(manager.list().unwrap().managed_auth_active);
    assert!(matches!(
        manager.create(input()),
        Err(LocalProviderError::ManagedAuth)
    ));
    assert!(matches!(
        manager.set_enabled(&row.id, true),
        Err(LocalProviderError::ManagedAuth)
    ));
    assert!(route_auth::resolve_profile_for_launch(&home.0, Some(&document), "opencode").is_err());
    assert!(matches!(
        route_auth::resolve_profile_for_launch(&home.0, None, "codex").unwrap(),
        AgentRuntimeAuthProfile::Native
    ));
    manager.set_enabled(&row.id, false).unwrap();
    manager.remove(&row.id).unwrap();
}

#[test]
fn local_provider_ciphertext_cannot_be_reassigned_to_another_identity() {
    let home = TestDirectory::new();
    let manager = manager(&home);
    manager.create(input()).unwrap();
    manager.create(input()).unwrap();
    let mut catalog = store::read(&home.0).unwrap();
    catalog.providers[0].ciphertext = catalog.providers[1].ciphertext.clone();
    store::write(&home.0, &mut catalog).unwrap();
    assert!(matches!(
        manager.profile(),
        Err(LocalProviderError::Unavailable)
    ));
}
