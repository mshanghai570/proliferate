//! Session-launch extension injecting the runtime user's MCP servers.
//!
//! Resolves user default + repository-override selection for the launch's
//! repo root, honors `InternalOnly`, decrypts sealed credentials, and emits
//! external `SessionMcpServer` values. The `mcp_bindings` assembly merges
//! them with product MCPs and owns the launch-order summary merge.

use std::fmt;

use super::manager::unseal_credentials;
use super::model::UserMcpServer;
use super::store::McpServerStore;
use anyharness_contract::v1::mcp_servers::{McpServerCredential, McpServerTransport};
use anyharness_contract::v1::{
    SessionMcpBindingOutcome, SessionMcpBindingNotAppliedReason, SessionMcpBindingSummary,
    SessionMcpTransport,
};
use crate::domains::sessions::extensions::{
    SessionExtension, SessionLaunchContext, SessionLaunchExtras,
};
use crate::domains::sessions::mcp_bindings::crypto::SessionDataCipher;
use crate::domains::sessions::mcp_bindings::model::{
    SessionMcpEnvVar, SessionMcpHeader, SessionMcpHttpServer, SessionMcpServer, SessionMcpStdioServer,
};
use crate::domains::sessions::model::SessionMcpBindingPolicy;

pub struct UserMcpServersLaunchExtension {
    store: McpServerStore,
    cipher: Option<SessionDataCipher>,
}

impl fmt::Debug for UserMcpServersLaunchExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserMcpServersLaunchExtension")
            .field("cipher_configured", &self.cipher.is_some())
            .finish()
    }
}

impl UserMcpServersLaunchExtension {
    pub fn new(db: crate::persistence::Db, cipher: Option<SessionDataCipher>) -> Self {
        Self {
            store: McpServerStore(db),
            cipher,
        }
    }
}

struct ResolvedSeal {
    credentials: Option<Vec<McpServerCredential>>,
    seal_problem: Option<SessionMcpBindingNotAppliedReason>,
}

/// Decrypt a record's transport-matching sealed envelope. Credentials ride
/// only through the launch path; summaries carry names at most, and neither
/// carries a value.
fn resolve_seal(record: &UserMcpServer, cipher: Option<&SessionDataCipher>) -> ResolvedSeal {
    let sealed = match record.transport {
        McpServerTransport::Stdio => record.env_sealed.as_deref(),
        McpServerTransport::Http => record.headers_sealed.as_deref(),
    };
    let Some(sealed) = sealed else {
        return ResolvedSeal {
            credentials: Some(Vec::new()),
            seal_problem: None,
        };
    };
    let Some(cipher) = cipher else {
        return ResolvedSeal {
            credentials: None,
            seal_problem: Some(SessionMcpBindingNotAppliedReason::MissingSecret),
        };
    };
    match unseal_credentials(cipher, sealed) {
        Ok(credentials) => ResolvedSeal {
            credentials: Some(credentials),
            seal_problem: None,
        },
        Err(_) => ResolvedSeal {
            credentials: None,
            seal_problem: Some(SessionMcpBindingNotAppliedReason::ResolverError),
        },
    }
}

impl SessionExtension for UserMcpServersLaunchExtension {
    fn resolve_launch_extras(
        &self,
        ctx: &SessionLaunchContext<'_>,
    ) -> anyhow::Result<SessionLaunchExtras> {
        if ctx.session.mcp_binding_policy == SessionMcpBindingPolicy::InternalOnly {
            return Ok(SessionLaunchExtras::default());
        }
        let records: Vec<UserMcpServer> = self
            .store
            .list()?
            .into_iter()
            .filter(|record| record.is_selected_for(&ctx.workspace.repo_root_id))
            .collect();
        if records.is_empty() {
            return Ok(SessionLaunchExtras::default());
        }
        let mut servers = Vec::with_capacity(records.len());
        let mut summaries = Vec::with_capacity(records.len());
        for record in &records {
            let summary_id = super::admission::connection_id(&record.server_name);
            let seal = resolve_seal(record, self.cipher.as_ref());
            let server = match (&seal.credentials, record.transport, &record.stdio_command, &record.url) {
                (Some(credentials), McpServerTransport::Stdio, Some(command), _) => {
                    Some(SessionMcpServer::Stdio(SessionMcpStdioServer {
                        connection_id: summary_id.clone(),
                        catalog_entry_id: None,
                        server_name: record.server_name.clone(),
                        command: command.clone(),
                        args: record.stdio_args.clone(),
                        env: credentials
                            .iter()
                            .map(|credential| SessionMcpEnvVar {
                                name: credential.name.clone(),
                                value: credential.value.clone(),
                            })
                            .collect(),
                    }))
                }
                (Some(credentials), McpServerTransport::Http, _, Some(url)) => {
                    Some(SessionMcpServer::Http(SessionMcpHttpServer {
                        connection_id: summary_id.clone(),
                        catalog_entry_id: None,
                        server_name: record.server_name.clone(),
                        url: url.clone(),
                        headers: credentials
                            .iter()
                            .map(|credential| SessionMcpHeader {
                                name: credential.name.clone(),
                                value: credential.value.clone(),
                            })
                            .collect(),
                    }))
                }
                _ => None,
            };
            let (outcome, reason) = match (&server, &seal.seal_problem) {
                (Some(_), _) => (SessionMcpBindingOutcome::Applied, None),
                (None, Some(reason)) => (SessionMcpBindingOutcome::NotApplied, Some(*reason)),
                (None, None) => (
                    SessionMcpBindingOutcome::NotApplied,
                    Some(SessionMcpBindingNotAppliedReason::UnsupportedTarget),
                ),
            };
            summaries.push(SessionMcpBindingSummary {
                id: summary_id,
                server_name: record.server_name.clone(),
                display_name: record.description.clone(),
                transport: match record.transport {
                    McpServerTransport::Stdio => SessionMcpTransport::Stdio,
                    McpServerTransport::Http => SessionMcpTransport::Http,
                },
                outcome,
                reason,
            });
            if let Some(server) = server {
                servers.push(server);
            }
        }
        tracing::info!(
            event = "mcp_server.launch_prepared",
            server_count = servers.len(),
            "User MCP servers prepared"
        );
        Ok(SessionLaunchExtras {
            mcp_servers: servers,
            mcp_binding_summaries: summaries,
            ..SessionLaunchExtras::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::mcp_servers::admission::admit_create;
    use crate::domains::mcp_servers::manager::UserMcpServerManager;
    use crate::domains::sessions::model::SessionRecord;
    use crate::domains::workspaces::model::{
        WorkspaceKind, WorkspaceLifecycleState, WorkspaceRecord, WorkspaceSurface,
    };
    use crate::persistence::Db;
    use anyharness_contract::v1::mcp_servers::McpServerConfig;

    const TEST_KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";

    fn filesystem_config() -> McpServerConfig {
        McpServerConfig::Stdio {
            command: "mcp-server-filesystem".to_string(),
            args: vec!["/workspace".to_string()],
            env: vec![anyharness_contract::v1::mcp_servers::McpServerCredential {
                name: "FILESYSTEM_ACCESS_TOKEN".to_string(),
                value: "top-secret".to_string(),
            }],
        }
    }

    fn workspace_record() -> WorkspaceRecord {
        WorkspaceRecord {
            id: "workspace-1".to_string(),
            kind: WorkspaceKind::Local,
            repo_root_id: "repo-root-1".to_string(),
            path: "/tmp/workspace".to_string(),
            surface: WorkspaceSurface::Standard,
            original_branch: None,
            current_branch: None,
            display_name: None,
            origin: None,
            creator_context: None,
            lifecycle_state: WorkspaceLifecycleState::Active,
            archived_head_sha: None,
            archived_branch: None,
            archived_at: None,
            partial_capture_json: None,
            created_at: "2026-03-25T00:00:00Z".to_string(),
            updated_at: "2026-03-25T00:00:00Z".to_string(),
        }
    }

    fn session_record(policy: SessionMcpBindingPolicy) -> SessionRecord {
        SessionRecord {
            id: "session-1".to_string(),
            workspace_id: "workspace-1".to_string(),
            agent_kind: "claude".to_string(),
            native_session_id: None,
            agent_auth_contexts: None,
            requested_model_id: None,
            current_model_id: None,
            requested_mode_id: None,
            current_mode_id: None,
            title: None,
            thinking_level_id: None,
            thinking_budget_tokens: None,
            status: "idle".to_string(),
            created_at: "2026-03-25T00:00:00Z".to_string(),
            updated_at: "2026-03-25T00:00:00Z".to_string(),
            last_prompt_at: None,
            closed_at: None,
            dismissed_at: None,
            mcp_bindings_ciphertext: None,
            mcp_binding_summaries_json: None,
            mcp_binding_policy: policy,
            system_prompt_append: None,
            subagents_enabled: true,
            action_capabilities_json: None,
            origin: None,
        }
    }

    fn launch(
        extension: &UserMcpServersLaunchExtension,
        workspace: &WorkspaceRecord,
        session: &SessionRecord,
    ) -> SessionLaunchExtras {
        extension
            .resolve_launch_extras(&SessionLaunchContext {
                workspace,
                session,
            })
            .expect("launch extras resolve")
    }

    fn cipher() -> SessionDataCipher {
        SessionDataCipher::from_env_value(TEST_KEY).unwrap()
    }

    #[test]
    fn internal_only_and_unselected_repos_get_default_extras() {
        let db = Db::open_in_memory().unwrap();
        let cipher = cipher();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher.clone()));
        let extension = UserMcpServersLaunchExtension::new(db, Some(cipher));
        let workspace = workspace_record();
        let session = session_record(SessionMcpBindingPolicy::InheritWorkspace);

        assert!(
            launch(&extension, &workspace, &session).mcp_servers.is_empty(),
            "no rows: nothing injects"
        );

        let admitted = admit_create("filesystem", Some("Local files"), &filesystem_config()).unwrap();
        let created = manager.create(&admitted).unwrap();
        manager
            .set_enabled(&created.id, Some("other-repo"), Some(true))
            .unwrap();

        let other = WorkspaceRecord {
            repo_root_id: "repo-root-1".to_string(),
            ..workspace_record()
        };
        assert!(
            launch(&extension, &other, &session)
                .mcp_servers
                .is_empty(),
            "override targets a different repo: nothing injects"
        );

        let internal_session = session_record(SessionMcpBindingPolicy::InternalOnly);
        manager.set_enabled(&created.id, None, Some(true)).unwrap();
        let internal_extras = launch(&extension, &workspace, &internal_session);
        assert!(internal_extras.mcp_servers.is_empty());
        assert!(internal_extras.mcp_binding_summaries.is_empty());
    }

    #[test]
    fn enabled_stdio_server_launches_with_decrypted_credentials_and_summary() {
        let db = Db::open_in_memory().unwrap();
        let cipher = cipher();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher.clone()));
        let admitted = admit_create("filesystem", Some("Local files"), &filesystem_config()).unwrap();
        let created = manager.create(&admitted).unwrap();
        manager.set_enabled(&created.id, None, Some(true)).unwrap();

        let extension = UserMcpServersLaunchExtension::new(db, Some(cipher));
        let workspace = workspace_record();
        let session = session_record(SessionMcpBindingPolicy::InheritWorkspace);
        let extras = launch(&extension, &workspace, &session);

        assert_eq!(extras.mcp_servers.len(), 1);
        assert_eq!(extras.mcp_binding_summaries.len(), 1);
        let SessionMcpServer::Stdio(stdio) = &extras.mcp_servers[0] else {
            panic!("one stdio server");
        };
        assert_eq!(stdio.connection_id, "user:filesystem");
        assert_eq!(stdio.command, "mcp-server-filesystem");
        assert_eq!(stdio.env.len(), 1);
        assert_eq!(stdio.env[0].name, "FILESYSTEM_ACCESS_TOKEN");
        assert_eq!(stdio.env[0].value, "top-secret");

        let summary = &extras.mcp_binding_summaries[0];
        assert_eq!(summary.outcome, SessionMcpBindingOutcome::Applied);
        assert_eq!(summary.transport, SessionMcpTransport::Stdio);

        // Secrets stay redacted outside the launch payload: Debug output of
        // the summary list and every summary rendered alone must not leak.
        let rendered = format!("{:?}", extras.mcp_binding_summaries);
        assert!(!rendered.contains("top-secret"));
        assert!(!format!("{summary:?}").contains("top-secret"));
        assert!(!serde_json::to_string(&extras.mcp_binding_summaries)
            .unwrap()
            .contains("top-secret"));
    }

    #[test]
    fn enabled_http_server_launches_with_decrypted_headers_and_redacted_summary() {
        let db = Db::open_in_memory().unwrap();
        let cipher = cipher();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher.clone()));
        let admitted = admit_create(
            "remote",
            None,
            &McpServerConfig::Http {
                url: "https://example.invalid/mcp".to_string(),
                headers: vec![anyharness_contract::v1::mcp_servers::McpServerCredential {
                    name: "Authorization".to_string(),
                    value: "header-secret".to_string(),
                }],
            },
        )
        .unwrap();
        let created = manager.create(&admitted).unwrap();
        manager.set_enabled(&created.id, None, Some(true)).unwrap();

        let extension = UserMcpServersLaunchExtension::new(db, Some(cipher));
        let extras = launch(
            &extension,
            &workspace_record(),
            &session_record(SessionMcpBindingPolicy::InheritWorkspace),
        );
        let [SessionMcpServer::Http(server)] = extras.mcp_servers.as_slice() else {
            panic!("one HTTP server");
        };
        assert_eq!(server.url, "https://example.invalid/mcp");
        assert_eq!(server.headers[0].name, "Authorization");
        assert_eq!(server.headers[0].value, "header-secret");
        assert_eq!(
            extras.mcp_binding_summaries[0].outcome,
            SessionMcpBindingOutcome::Applied
        );
        assert!(!format!("{:?}", extras.mcp_binding_summaries).contains("header-secret"));
    }

    #[test]
    fn selection_changes_and_deletion_only_affect_future_launches() {
        let db = Db::open_in_memory().unwrap();
        let cipher = cipher();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher.clone()));
        let extension = UserMcpServersLaunchExtension::new(db, Some(cipher));
        let created = manager
            .create(&admit_create("filesystem", None, &filesystem_config()).unwrap())
            .unwrap();
        let workspace = workspace_record();
        let session = session_record(SessionMcpBindingPolicy::InheritWorkspace);

        manager.set_enabled(&created.id, None, Some(true)).unwrap();
        assert_eq!(launch(&extension, &workspace, &session).mcp_servers.len(), 1);
        manager
            .set_enabled(&created.id, Some("repo-root-1"), Some(false))
            .unwrap();
        assert!(launch(&extension, &workspace, &session).mcp_servers.is_empty());
        manager
            .set_enabled(&created.id, Some("repo-root-1"), None)
            .unwrap();
        assert_eq!(launch(&extension, &workspace, &session).mcp_servers.len(), 1);

        manager.remove(&created.id).unwrap();
        assert!(launch(&extension, &workspace, &session).mcp_servers.is_empty());
    }

    #[test]
    fn missing_data_key_marks_the_server_not_applied_instead_of_aborting_launch() {
        // Seal exists (created under a cipher) but the launch path runs
        // without a data key: the server is not silently dropped — the
        // summary names MissingSecret.
        let db = Db::open_in_memory().unwrap();
        let cipher = cipher();
        let manager = UserMcpServerManager::new(db.clone(), Some(cipher));
        let admitted = admit_create("filesystem", None, &filesystem_config()).unwrap();
        let created = manager.create(&admitted).unwrap();
        manager.set_enabled(&created.id, None, Some(true)).unwrap();

        let extension = UserMcpServersLaunchExtension::new(db, None);
        let workspace = workspace_record();
        let session = session_record(SessionMcpBindingPolicy::InheritWorkspace);
        let extras = launch(&extension, &workspace, &session);

        assert!(extras.mcp_servers.is_empty());
        assert_eq!(extras.mcp_binding_summaries.len(), 1);
        let summary = &extras.mcp_binding_summaries[0];
        assert_eq!(summary.outcome, SessionMcpBindingOutcome::NotApplied);
        assert_eq!(
            summary.reason,
            Some(SessionMcpBindingNotAppliedReason::MissingSecret)
        );
    }
}
