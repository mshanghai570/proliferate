use crate::api::http::{custom_agents, extensions, local_providers, project_mcp, skills};
use crate::app::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/skills",
            get(skills::list_skills).post(skills::import_skill),
        )
        .route("/skills/{id}", delete(skills::remove_skill))
        .route(
            "/skills/{id}/enabled",
            axum::routing::put(skills::set_skill_enabled),
        )
        .route(
            "/agent-auth/local-providers",
            get(local_providers::list).post(local_providers::create),
        )
        .route(
            "/agent-auth/local-providers/{id}",
            axum::routing::patch(local_providers::set_enabled).delete(local_providers::remove),
        )
        .route("/extensions", get(extensions::list_extensions))
        .route(
            "/extensions/{namespace}/{name}/{version}/color-themes",
            get(extensions::list_color_themes),
        )
        .route(
            "/extensions/install",
            post(extensions::install_registry_extension),
        )
        .route(
            "/extensions/{namespace}/{name}/{version}",
            delete(extensions::uninstall_extension),
        )
        .route(
            "/agents/custom",
            get(custom_agents::list_custom_agents).post(custom_agents::register_custom_agent),
        )
        .route(
            "/agents/custom/{id}",
            delete(custom_agents::remove_custom_agent),
        )
        .route(
            "/repo-roots/{repo_root_id}/mcp-servers",
            get(project_mcp::list),
        )
        .route(
            "/repo-roots/{repo_root_id}/mcp-servers/{name}",
            put(project_mcp::put)
                .patch(project_mcp::set_enabled)
                .delete(project_mcp::remove),
        )
}
