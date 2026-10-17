use std::collections::BTreeMap;

use serde_json::Value;

use super::render::RenderedRouteAuth;
use super::RouteAuthError;

pub(super) fn compose(
    env: &BTreeMap<String, String>,
    rendered: &mut RenderedRouteAuth,
) -> Result<(), RouteAuthError> {
    let invalid = || RouteAuthError::SelectionIncomplete {
        harness_kind: "opencode".into(),
        detail: "custom provider configuration is invalid or contains duplicate provider IDs"
            .into(),
    };
    let content = env.get("OPENCODE_CONFIG_CONTENT").ok_or_else(invalid)?;
    let incoming: Value = serde_json::from_str(content).map_err(|_| invalid())?;
    let incoming = incoming
        .as_object()
        .filter(|root| root.len() == 1)
        .ok_or_else(invalid)?;
    let providers = incoming
        .get("provider")
        .and_then(Value::as_object)
        .filter(|providers| !providers.is_empty())
        .ok_or_else(invalid)?;
    let mut combined: Value = match rendered.set.get("OPENCODE_CONFIG_CONTENT") {
        Some(content) => serde_json::from_str(content).map_err(|_| invalid())?,
        None => serde_json::json!({"provider": {}}),
    };
    let combined_providers = combined
        .get_mut("provider")
        .and_then(Value::as_object_mut)
        .ok_or_else(invalid)?;
    for (id, provider) in providers {
        if !id.starts_with("custom-")
            || combined_providers.contains_key(id)
            || !provider.is_object()
        {
            return Err(invalid());
        }
        combined_providers.insert(id.clone(), provider.clone());
    }
    for (key, value) in env {
        if key != "OPENCODE_CONFIG_CONTENT" {
            if !key.starts_with("PROLIFERATE_CUSTOM_") || rendered.set.contains_key(key) {
                return Err(invalid());
            }
            rendered.set.insert(key.clone(), value.clone());
        }
    }
    rendered.set.insert(
        "OPENCODE_CONFIG_CONTENT".into(),
        serde_json::to_string(&combined).map_err(|_| invalid())?,
    );
    Ok(())
}
