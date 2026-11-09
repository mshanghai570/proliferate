use super::super::profile::ApiKeyProfile;
use super::*;

fn source(id: &str) -> ResolvedSource {
    ResolvedSource::ProviderConfig(ProviderConfigProfile {
        config_kind: "openai_compatible".into(),
        env: BTreeMap::from([(
            "OPENCODE_CONFIG_CONTENT".into(),
            json!({
                "provider": {id: {
                    "npm": "@ai-sdk/openai-compatible",
                    "options": {"baseURL": "http://127.0.0.1:1234/v1"},
                    "models": {"model-a": {}}
                }}
            })
            .to_string(),
        )]),
    })
}

#[test]
fn custom_providers_compose_with_each_other_and_existing_keys() {
    let profile = AgentRuntimeAuthProfile::Sources(HarnessSources {
        harness_kind: "opencode".into(),
        revision: 1,
        sources: vec![
            source("custom-first"),
            source("custom-second"),
            ResolvedSource::ApiKey(ApiKeyProfile {
                env_var_name: "ANTHROPIC_API_KEY".into(),
                value: "test-key".into(),
            }),
        ],
    });
    let rendered = render_profile(
        &profile,
        "opencode",
        &GatewayModelPlan::default(),
        Path::new("/unused"),
    )
    .unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&rendered.set["OPENCODE_CONFIG_CONTENT"]).unwrap();
    assert_eq!(config["provider"].as_object().unwrap().len(), 2);
    assert!(config["provider"].get("custom-first").is_some());
    assert!(config["provider"].get("custom-second").is_some());
    assert!(rendered.set.contains_key("ANTHROPIC_API_KEY"));
    assert!(rendered.files.is_empty());
}

#[test]
fn duplicate_custom_provider_ids_fail_closed() {
    let profile = AgentRuntimeAuthProfile::Sources(HarnessSources {
        harness_kind: "opencode".into(),
        revision: 1,
        sources: vec![source("custom-same"), source("custom-same")],
    });
    assert!(render_profile(
        &profile,
        "opencode",
        &GatewayModelPlan::default(),
        Path::new("/unused")
    )
    .is_err());
}

#[test]
fn malformed_custom_config_fails_without_echoing_its_content() {
    let profile = AgentRuntimeAuthProfile::Sources(HarnessSources {
        harness_kind: "opencode".into(),
        revision: 1,
        sources: vec![ResolvedSource::ProviderConfig(ProviderConfigProfile {
            config_kind: "openai_compatible".into(),
            env: BTreeMap::from([(
                "OPENCODE_CONFIG_CONTENT".into(),
                "secret-invalid-json".into(),
            )]),
        })],
    });
    let error = render_profile(
        &profile,
        "opencode",
        &GatewayModelPlan::default(),
        Path::new("/unused"),
    )
    .unwrap_err();
    assert!(!error.to_string().contains("secret-invalid-json"));
}

#[test]
fn custom_config_cannot_be_overwritten_by_an_api_key_binding() {
    for config_first in [true, false] {
        let mut sources = vec![
            source("custom-first"),
            ResolvedSource::ApiKey(ApiKeyProfile {
                env_var_name: "OPENCODE_CONFIG_CONTENT".into(),
                value: "{}".into(),
            }),
        ];
        if !config_first {
            sources.reverse();
        }
        let profile = AgentRuntimeAuthProfile::Sources(HarnessSources {
            harness_kind: "opencode".into(),
            revision: 1,
            sources,
        });
        assert!(render_profile(
            &profile,
            "opencode",
            &GatewayModelPlan::default(),
            Path::new("/unused")
        )
        .is_err());
    }
}
