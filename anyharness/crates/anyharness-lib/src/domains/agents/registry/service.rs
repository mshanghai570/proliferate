//! Static-truth queries over the bundled agent registry.

use super::built_in_registry;
use crate::domains::agents::model::AgentDescriptor;

/// Look up one agent's descriptor by kind — the single sanctioned way to
/// resolve a kind into a descriptor outside this domain. No inline registry
/// scans at call sites.
pub fn descriptor(kind: &str) -> Option<AgentDescriptor> {
    built_in_registry()
        .into_iter()
        .find(|descriptor| descriptor.kind.as_str() == kind)
}

/// Local custom registrations supplement the immutable bundled registry.
pub fn descriptor_at(runtime_home: &std::path::Path, kind: &str) -> Option<AgentDescriptor> {
    descriptor(kind).or_else(|| {
        super::super::custom::load(runtime_home, kind)
            .ok()
            .flatten()
            .map(|agent| agent.descriptor())
    })
}

pub fn runtime_registry(runtime_home: &std::path::Path) -> Vec<AgentDescriptor> {
    let mut registry = built_in_registry();
    match super::super::custom::list(runtime_home) {
        Ok(agents) => registry.extend(agents.into_iter().map(|agent| agent.descriptor())),
        Err(_) => tracing::warn!("could not read custom ACP registrations"),
    }
    registry
}
