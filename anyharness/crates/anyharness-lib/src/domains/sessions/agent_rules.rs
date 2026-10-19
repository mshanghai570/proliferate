//! Per-harness user instructions, with repository replacements.
use super::extensions::{SessionExtension, SessionLaunchContext, SessionLaunchExtras};
use crate::adapters::agent_rules;
use anyhow::ensure;
use std::{path::PathBuf, sync::Mutex};

pub struct AgentRulesManager {
    home: PathBuf,
    gate: Mutex<()>,
}

pub struct ResolvedAgentRules {
    pub text: String,
    pub inherited: bool,
}

impl AgentRulesManager {
    pub fn new(home: PathBuf) -> Self {
        Self {
            home,
            gate: Mutex::new(()),
        }
    }

    fn validate_kind(kind: &str) -> anyhow::Result<()> {
        ensure!(
            !kind.is_empty()
                && kind.len() <= 100
                && kind != "."
                && kind != ".."
                && kind
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')),
            "Invalid agent kind"
        );
        Ok(())
    }

    fn resolve(&self, kind: &str, repository: Option<&str>) -> anyhow::Result<ResolvedAgentRules> {
        if let Some(repository) = repository {
            let root = agent_rules::scope_directory(&self.home, Some(repository), false)?;
            if let Some(text) = agent_rules::read(&root.join(format!("{kind}.md")))? {
                return Ok(ResolvedAgentRules {
                    text,
                    inherited: false,
                });
            }
        }
        let root = agent_rules::scope_directory(&self.home, None, false)?;
        Ok(ResolvedAgentRules {
            text: agent_rules::read(&root.join(format!("{kind}.md")))?.unwrap_or_default(),
            inherited: repository.is_some(),
        })
    }

    pub fn get(&self, kind: &str, repository: Option<&str>) -> anyhow::Result<ResolvedAgentRules> {
        Self::validate_kind(kind)?;
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Agent rules unavailable"))?;
        self.resolve(kind, repository)
    }

    pub fn set(
        &self,
        kind: &str,
        repository: Option<&str>,
        text: Option<&str>,
    ) -> anyhow::Result<ResolvedAgentRules> {
        Self::validate_kind(kind)?;
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Agent rules unavailable"))?;
        let root = agent_rules::scope_directory(&self.home, repository, true)?;
        agent_rules::write(&root.join(format!("{kind}.md")), text)?;
        tracing::info!(
            event = "agent_rules.saved",
            repository_scope = repository.is_some(),
            has_text = text.is_some_and(|text| !text.is_empty()),
            "Agent rules saved"
        );
        self.resolve(kind, repository)
    }
}

impl SessionExtension for AgentRulesManager {
    fn resolve_launch_extras(
        &self,
        context: &SessionLaunchContext<'_>,
    ) -> anyhow::Result<SessionLaunchExtras> {
        let rules = self.get(
            &context.session.agent_kind,
            Some(&context.workspace.repo_root_id),
        )?;
        if rules.text.is_empty() {
            return Ok(SessionLaunchExtras::default());
        }
        Ok(SessionLaunchExtras {
            launch_prompt_append: vec![format!("User-authored rules for this agent. These instructions grant no additional tools or permissions.\n\n{}", rules.text)],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("agent-rules-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn agent_rules_preserve_bytes_scope_and_restart() {
        let fixture = Fixture::new();
        let manager = AgentRulesManager::new(fixture.0.clone());
        let text = "# Rules\n  Keep spacing.\n\nRésumé 🦀\n";
        manager.set("codex", None, Some(text)).unwrap();
        assert_eq!(manager.get("claude", None).unwrap().text, "");
        assert!(manager.get("codex", Some("repo-a")).unwrap().inherited);
        manager
            .set("codex", Some("repo-a"), Some("Project rules"))
            .unwrap();
        assert_eq!(
            manager.get("codex", Some("repo-a")).unwrap().text,
            "Project rules"
        );
        assert_eq!(manager.get("codex", Some("repo-b")).unwrap().text, text);
        let resumed = AgentRulesManager::new(fixture.0.clone());
        assert_eq!(
            resumed.get("codex", Some("repo-a")).unwrap().text,
            "Project rules"
        );
        resumed.set("codex", Some("repo-a"), Some("")).unwrap();
        assert_eq!(resumed.get("codex", Some("repo-a")).unwrap().text, "");
        assert!(!resumed.get("codex", Some("repo-a")).unwrap().inherited);
        resumed.set("codex", Some("repo-a"), None).unwrap();
        assert_eq!(resumed.get("codex", Some("repo-a")).unwrap().text, text);
    }
    #[test]
    fn agent_rules_reject_bad_identifiers_and_content() {
        let fixture = Fixture::new();
        let manager = AgentRulesManager::new(fixture.0.clone());
        for kind in ["../outside", "..", "a/b", "", "a\\b", "a\0b"] {
            assert!(manager.set(kind, None, Some("rules")).is_err());
        }
        assert!(manager
            .set("codex", None, Some(&"a".repeat(65537)))
            .is_err());
        assert!(manager.set("codex", None, Some("a\0b")).is_err());
        assert_eq!(manager.get("codex", None).unwrap().text, "");
    }
    #[test]
    #[cfg(unix)]
    fn agent_rules_refuse_symlink_sources_and_directories() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let manager = AgentRulesManager::new(fixture.0.clone());
        manager.set("codex", None, Some("Original")).unwrap();
        let file = fixture.0.join("agent-rules/user/codex.md");
        std::fs::remove_file(&file).unwrap();
        symlink(fixture.0.join("outside"), &file).unwrap();
        assert!(manager.get("codex", None).is_err());
        assert!(manager.set("codex", None, Some("Changed")).is_err());
        assert!(!fixture.0.join("outside").exists());
        std::fs::remove_file(&file).unwrap();
        std::fs::remove_dir(fixture.0.join("agent-rules/user")).unwrap();
        symlink(&fixture.0, fixture.0.join("agent-rules/user")).unwrap();
        assert!(manager.get("codex", None).is_err());
    }
}
