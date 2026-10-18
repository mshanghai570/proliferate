use super::*;
use crate::domains::repo_roots::test_support::seed_repo_root;
use crate::domains::sessions::mcp_bindings::model::{SessionMcpHeader, SessionMcpHttpServer};

pub(crate) struct TestHome(pub std::path::PathBuf);
impl TestHome {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("project-mcp-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cipher() -> DataCipher {
    DataCipher::from_env_value("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=").unwrap()
}
fn server() -> SessionMcpServer {
    SessionMcpServer::Http(SessionMcpHttpServer {
        connection_id: "untrusted-id".into(),
        catalog_entry_id: Some("untrusted-catalog".into()),
        server_name: "untrusted-name".into(),
        url: "https://example.invalid/mcp?key=private-url".into(),
        headers: vec![SessionMcpHeader {
            name: "Authorization".into(),
            value: "Bearer private-header".into(),
        }],
    })
}
fn setup() -> (TestHome, Db, ProjectMcpManager) {
    let home = TestHome::new();
    let db = Db::open(&home.0).unwrap();
    seed_repo_root(&db, "repo-1", "/tmp/project-mcp-one");
    seed_repo_root(&db, "repo-2", "/tmp/project-mcp-two");
    let manager = ProjectMcpManager::new(db.clone(), Some(cipher()));
    (home, db, manager)
}

#[test]
fn inventory_never_returns_credentials_and_store_is_encrypted() {
    let (_home, db, manager) = setup();
    manager.put("repo-1", "docs", server()).unwrap();
    let inventory = serde_json::to_string(&manager.list("repo-1").unwrap()).unwrap();
    assert!(!inventory.contains("private-"));
    assert!(inventory.contains("docs"));
    let rows = manager.store.list("repo-1").unwrap();
    assert!(rows[0].ciphertext.starts_with("v1:"));
    assert!(!rows[0].ciphertext.contains("private-"));
    let value: SessionMcpServer =
        serde_json::from_slice(&decrypt_bytes(&cipher(), &rows[0].ciphertext).unwrap()).unwrap();
    let SessionMcpServer::Http(value) = value else {
        panic!("HTTP server");
    };
    assert_eq!(value.server_name, "docs");
    assert_eq!(value.connection_id, "local-project:docs");
    assert_eq!(value.catalog_entry_id, None);
    assert_eq!(value.headers[0].value, "Bearer private-header");
    assert!(manager.list("repo-2").unwrap().servers.is_empty());
    db.with_conn(|conn| {
        conn.execute("DELETE FROM repo_roots WHERE id = 'repo-1'", [])
            .map(|_| ())
    })
    .unwrap();
    assert!(manager.store.list("repo-1").unwrap().is_empty());
}

#[test]
fn replacements_toggles_and_idempotent_removal_preserve_other_projects() {
    let (_home, _db, manager) = setup();
    manager.put("repo-1", "docs", server()).unwrap();
    manager.put("repo-2", "docs", server()).unwrap();
    manager.set_enabled("repo-1", "docs", false).unwrap();
    assert!(!manager.list("repo-1").unwrap().servers[0].enabled);
    assert!(manager.list("repo-2").unwrap().servers[0].enabled);
    manager.put("repo-1", "docs", server()).unwrap();
    assert!(manager.list("repo-1").unwrap().servers[0].enabled);
    for _ in 0..2 {
        manager.remove("repo-1", "docs").unwrap();
    }
    assert_eq!(manager.list("repo-2").unwrap().servers.len(), 1);
    assert!(matches!(
        manager.set_enabled("repo-1", "missing", true),
        Err(ProjectMcpError::NotFound)
    ));
    assert!(matches!(
        manager.put("missing", "docs", server()),
        Err(ProjectMcpError::NotFound)
    ));
}

#[test]
fn missing_key_never_writes_plaintext() {
    let (_home, db, _manager) = setup();
    let manager = ProjectMcpManager::new(db, None);
    assert!(!manager.list("repo-1").unwrap().encryption_available);
    assert!(matches!(
        manager.put("repo-1", "docs", server()),
        Err(ProjectMcpError::MissingKey)
    ));
    assert!(manager.store.list("repo-1").unwrap().is_empty());
}

#[test]
fn validation_rejects_reserved_names_bad_urls_and_duplicate_headers() {
    for name in ["", "a/b", "Proliferate_workspace", "bad name"] {
        assert!(validation::validate_and_normalize(name, &mut server()).is_err());
    }
    for url in [
        "file:///tmp/a",
        "https://user:secret@example.com/mcp",
        "https://example.com/mcp#fragment",
        "not a url",
    ] {
        let mut binding = server();
        if let SessionMcpServer::Http(ref mut value) = binding {
            value.url = url.into();
        }
        assert!(validation::validate_and_normalize("docs", &mut binding).is_err());
    }
    let mut binding = server();
    if let SessionMcpServer::Http(ref mut value) = binding {
        value.headers.push(SessionMcpHeader {
            name: "authorization".into(),
            value: "duplicate-secret".into(),
        });
    }
    assert!(validation::validate_and_normalize("docs", &mut binding).is_err());
}

#[test]
fn project_server_limit_is_atomic_and_allows_replacement() {
    let (_home, _db, manager) = setup();
    for i in 0..32 {
        manager
            .put("repo-1", &format!("docs-{i}"), server())
            .unwrap();
    }
    assert!(matches!(
        manager.put("repo-1", "overflow", server()),
        Err(ProjectMcpError::Invalid(_))
    ));
    manager.put("repo-1", "docs-0", server()).unwrap();
    assert_eq!(manager.list("repo-1").unwrap().servers.len(), 32);
}

#[test]
fn stdio_validation_and_serialized_size_are_bounded() {
    let (_home, _db, manager) = setup();
    let valid = serde_json::json!({"transport":"stdio", "connectionId":"ignored", "serverName":"ignored", "command":"mcp-server", "args":["project with spaces"], "env":[{"name":"API_KEY","value":"secret"}]});
    manager
        .put(
            "repo-1",
            &"n".repeat(50),
            serde_json::from_value(valid.clone()).unwrap(),
        )
        .unwrap();
    for patch in [
        serde_json::json!({"command":""}),
        serde_json::json!({"args":["nul\0"]}),
        serde_json::json!({"env":[{"name":"BAD-NAME","value":"secret"}]}),
        serde_json::json!({"env":[{"name":"KEY","value":"a"},{"name":"KEY","value":"b"}]}),
    ] {
        let mut value = valid.clone();
        for (key, entry) in patch.as_object().unwrap() {
            value[key] = entry.clone();
        }
        assert!(manager
            .put("repo-1", "bad", serde_json::from_value(value).unwrap())
            .is_err());
    }
    let mut oversized = valid;
    oversized["env"] = serde_json::Value::Array(
        (0..32)
            .map(|i| serde_json::json!({"name":format!("VAR_{i}"), "value":"s".repeat(4096)}))
            .collect(),
    );
    assert!(matches!(
        manager.put(
            "repo-1",
            "too-large",
            serde_json::from_value(oversized).unwrap()
        ),
        Err(ProjectMcpError::Invalid(_))
    ));
    assert_eq!(manager.list("repo-1").unwrap().servers.len(), 1);
}
