use super::*;
use crate::adapters::files::retrieval::{eligible_path, read_source};
use std::{fs, path::Path, process::Command};

struct Fixture {
    base: PathBuf,
    home: PathBuf,
    project: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("project-retrieval-{}", uuid::Uuid::new_v4()));
        let home = base.join("home");
        let project = base.join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir(&project).unwrap();
        assert!(Command::new("git")
            .args(["init", "-q"])
            .arg(&project)
            .status()
            .unwrap()
            .success());
        Self {
            base,
            home,
            project,
        }
    }
    fn manager(&self) -> Arc<ProjectRetrievalManager> {
        Arc::new(ProjectRetrievalManager::new(self.home.clone()))
    }
    fn git(&self, args: &[&str]) {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&self.project)
            .status()
            .unwrap()
            .success());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[tokio::test]
async fn project_retrieval_indexes_source_and_refreshes_changes_without_stale_results() {
    let fixture = Fixture::new();
    let manager = fixture.manager();
    fs::write(
        fixture.project.join("auth.rs"),
        "// Account flow\nfn account_login() {}\n",
    )
    .unwrap();
    assert!(!manager.status("workspace-a").unwrap().enabled);
    assert!(manager
        .search(
            "workspace-a".into(),
            fixture.project.clone(),
            "account".into(),
            lease("workspace-a").await
        )
        .await
        .is_err());
    let status = manager
        .refresh(
            "workspace-a".into(),
            fixture.project.clone(),
            lease("workspace-a").await,
        )
        .await
        .unwrap();
    assert_eq!(status.indexed_files, 1);
    assert_eq!(status.chunks, 1);
    let first = manager
        .search(
            "workspace-a".into(),
            fixture.project.clone(),
            "account".into(),
            lease("workspace-a").await,
        )
        .await
        .unwrap();
    assert_eq!(first.excerpts.len(), 1);
    assert_eq!(first.excerpts[0].path, "auth.rs");
    assert_eq!(
        (first.excerpts[0].start_line, first.excerpts[0].end_line),
        (1, 2)
    );
    let hash = first.excerpts[0].content_hash.clone();
    fs::write(
        fixture.project.join("auth.rs"),
        "// Session flow\nfn session_resume() {}\n",
    )
    .unwrap();
    assert!(manager
        .search(
            "workspace-a".into(),
            fixture.project.clone(),
            "account".into(),
            lease("workspace-a").await
        )
        .await
        .unwrap()
        .excerpts
        .is_empty());
    let updated = manager
        .search(
            "workspace-a".into(),
            fixture.project.clone(),
            "session".into(),
            lease("workspace-a").await,
        )
        .await
        .unwrap();
    assert_ne!(updated.excerpts[0].content_hash, hash);
    assert_eq!(
        fixture
            .manager()
            .status("workspace-a")
            .unwrap()
            .indexed_files,
        1
    );
    assert!(!fixture.manager().status("workspace-b").unwrap().enabled);
    fs::remove_file(fixture.project.join("auth.rs")).unwrap();
    assert!(manager
        .search(
            "workspace-a".into(),
            fixture.project.clone(),
            "session".into(),
            lease("workspace-a").await
        )
        .await
        .unwrap()
        .excerpts
        .is_empty());
    manager.clear("workspace-a").unwrap();
    assert!(!manager.status("workspace-a").unwrap().enabled);
    assert!(fs::read_dir(fixture.home.join("project-retrieval"))
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn project_retrieval_honors_tracked_and_untracked_ignores_and_sensitive_exclusions() {
    let fixture = Fixture::new();
    let manager = fixture.manager();
    for name in [
        "visible.rs",
        "ignored.rs",
        "untracked.rs",
        "credentials.json",
        ".mcp.json",
        "auth.json",
        ".env.local",
    ] {
        fs::write(fixture.project.join(name), "retrieval_marker").unwrap();
    }
    fixture.git(&["add", "ignored.rs"]);
    fs::write(
        fixture.project.join(".gitignore"),
        "ignored.rs\nuntracked.rs\n",
    )
    .unwrap();
    fs::write(fixture.project.join("binary.rs"), [0, 255, 1]).unwrap();
    fs::write(fixture.project.join("huge.txt"), "x".repeat(256 * 1024 + 1)).unwrap();
    fs::write(
        fixture.project.join("key.txt"),
        "-----BEGIN PRIVATE KEY-----\nsecret\n",
    )
    .unwrap();
    manager
        .refresh(
            "workspace".into(),
            fixture.project.clone(),
            lease("workspace").await,
        )
        .await
        .unwrap();
    let results = manager
        .search(
            "workspace".into(),
            fixture.project.clone(),
            "retrieval_marker".into(),
            lease("workspace").await,
        )
        .await
        .unwrap();
    assert_eq!(results.excerpts.len(), 1);
    assert_eq!(results.excerpts[0].path, "visible.rs");
    fixture.git(&["add", "visible.rs"]);
    fs::write(
        fixture.project.join(".gitignore"),
        "visible.rs\nignored.rs\nuntracked.rs\n",
    )
    .unwrap();
    assert!(manager
        .search(
            "workspace".into(),
            fixture.project.clone(),
            "retrieval_marker".into(),
            lease("workspace").await
        )
        .await
        .unwrap()
        .excerpts
        .is_empty());
}

#[test]
fn project_retrieval_path_policy_is_bounded_and_source_only() {
    for path in [
        "../outside.rs",
        "/outside.rs",
        "a\\b.rs",
        ".git/config",
        "node_modules/a.ts",
        "a/.env",
        "a/.env.production",
        "a/credentials.json",
        ".claude/settings.json",
        ".cursor/mcp.json",
        ".mcp.json",
        "config/auth.json",
        "a/secret.pem",
        "a/id_rsa",
        "a.bin",
        "a\nb.rs",
    ] {
        assert!(!eligible_path(path), "{path}");
    }
    for path in [
        "src/auth.rs",
        "docs/design.md",
        "README",
        "package.json",
        "src/résumé.ts",
    ] {
        assert!(eligible_path(path), "{path}");
    }
}

#[tokio::test]
#[cfg(unix)]
async fn project_retrieval_never_reads_symlink_sources_or_cache_targets() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let manager = fixture.manager();
    fs::write(fixture.base.join("outside.rs"), "private_outside_marker").unwrap();
    symlink(
        fixture.base.join("outside.rs"),
        fixture.project.join("linked.rs"),
    )
    .unwrap();
    symlink(&fixture.base, fixture.project.join("linked-directory")).unwrap();
    assert!(read_source(&fixture.project, "linked.rs").is_err());
    assert!(read_source(&fixture.project, "linked-directory/outside.rs").is_err());
    manager
        .refresh(
            "workspace".into(),
            fixture.project.clone(),
            lease("workspace").await,
        )
        .await
        .unwrap();
    assert!(manager
        .search(
            "workspace".into(),
            fixture.project.clone(),
            "private_outside_marker".into(),
            lease("workspace").await
        )
        .await
        .unwrap()
        .excerpts
        .is_empty());
    manager.clear("workspace").unwrap();
    let path = store::path(&fixture.home, "workspace", false)
        .unwrap()
        .unwrap();
    symlink(fixture.base.join("outside.rs"), &path).unwrap();
    assert!(manager.status("workspace").is_err());
    assert_eq!(
        fs::read_to_string(fixture.base.join("outside.rs")).unwrap(),
        "private_outside_marker"
    );
    assert!(Path::new(&path).is_symlink());
}

async fn lease(
    workspace: &str,
) -> crate::domains::workspaces::operation_gate::WorkspaceOperationLease {
    use crate::domains::workspaces::operation_gate::{
        WorkspaceOperationGate, WorkspaceOperationKind,
    };
    WorkspaceOperationGate::new()
        .acquire_shared(workspace, WorkspaceOperationKind::MaterializationRead)
        .await
}

#[test]
fn project_retrieval_rejects_negative_cached_counts() {
    let fixture = Fixture::new();
    let connection = store::open(&fixture.home, "workspace", true)
        .unwrap()
        .unwrap();
    connection.execute("INSERT INTO retrieval_meta(id,indexed_at,skipped,truncated) VALUES(1,'2026-10-19T00:00:00Z',-1,0)", []).unwrap();
    assert!(store::status(Some(&connection)).is_err());
}
