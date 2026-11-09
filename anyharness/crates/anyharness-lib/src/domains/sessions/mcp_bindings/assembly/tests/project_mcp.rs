use super::*;
use crate::domains::extensions::installer_tests::TestDirectory;
use crate::domains::repo_roots::test_support::seed_repo_root_1;
use crate::domains::sessions::mcp_bindings::project::ProjectMcpManager;
use crate::persistence::Db;

#[test]
fn project_mcp_servers_reach_launch_and_acp_and_respect_policy() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    seed_repo_root_1(&db);
    let manager = Arc::new(ProjectMcpManager::new(db.clone(), Some(sample_cipher())));
    manager.put("repo-root-1", "docs", serde_json::from_value(serde_json::json!({
        "transport": "http", "connectionId": "discard", "serverName": "discard", "url": "http://localhost:8000/mcp",
        "headers": [{"name": "Authorization", "value": "Bearer project-secret"}]
    })).unwrap()).unwrap();
    manager.put("repo-root-1", "files", serde_json::from_value(serde_json::json!({
        "transport": "stdio", "connectionId": "discard", "serverName": "discard", "command": "/usr/bin/env",
        "args": ["mcp-server"], "env": [{"name": "API_KEY", "value": "stdio-secret"}]
    })).unwrap()).unwrap();
    let extensions: Vec<Arc<dyn SessionExtension>> = vec![manager.clone()];
    let assemble = |record: &SessionRecord, workspace: &WorkspaceRecord| {
        assemble_session_mcp_launch(
            None,
            &extensions,
            &ProductMcpLaunchCatalog::disabled(),
            workspace,
            record,
            None,
        )
    };
    let mut workspace = workspace_record();
    let mut record = session_record();
    let launch = assemble(&record, &workspace).unwrap();
    assert_eq!(launch.mcp_servers.len(), 2);
    let acp = crate::domains::sessions::mcp_bindings::acp::to_acp_servers(&launch.mcp_servers);
    let wire = serde_json::to_value(&acp).unwrap();
    assert_eq!(wire[0]["name"], "docs");
    assert_eq!(wire[0]["headers"][0]["value"], "Bearer project-secret");
    assert_eq!(wire[1]["command"], "/usr/bin/env");
    assert_eq!(wire[1]["env"][0]["value"], "stdio-secret");
    manager.set_enabled("repo-root-1", "docs", false).unwrap();
    assert_eq!(assemble(&record, &workspace).unwrap().mcp_servers.len(), 1);
    record.mcp_binding_policy = SessionMcpBindingPolicy::InternalOnly;
    assert!(assemble(&record, &workspace)
        .unwrap()
        .mcp_servers
        .is_empty());
    record.mcp_binding_policy = SessionMcpBindingPolicy::InheritWorkspace;
    workspace.repo_root_id = "another-project".into();
    assert!(assemble(&record, &workspace)
        .unwrap()
        .mcp_servers
        .is_empty());
    workspace.repo_root_id = "repo-root-1".into();
    db.with_conn(|conn| {
        conn.execute(
            "UPDATE project_mcp_servers SET ciphertext = 'broken' WHERE name = 'files'",
            [],
        )
        .map(|_| ())
    })
    .unwrap();
    let error = assemble(&record, &workspace).unwrap_err();
    assert!(!format!("{error:?}").contains("secret"));
    manager.set_enabled("repo-root-1", "files", false).unwrap();
    assert!(assemble(&record, &workspace)
        .unwrap()
        .mcp_servers
        .is_empty());
}
