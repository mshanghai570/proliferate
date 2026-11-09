//! Local, user-owned ACP commands. They never enter managed distribution.
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::*;
use crate::integrations::agent_cli::executable::{find_in_path, is_valid_executable};

const MAX_BYTES: u64 = 256 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomAgent {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub fn is_custom_id(id: &str) -> bool {
    id.strip_prefix("custom-").is_some_and(|value| {
        uuid::Uuid::parse_str(value).is_ok_and(|uuid| uuid.to_string() == value)
    })
}

impl CustomAgent {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(is_custom_id(&self.id), "invalid custom agent id");
        anyhow::ensure!(
            !self.name.trim().is_empty()
                && self.name.len() <= 120
                && !self.name.chars().any(char::is_control),
            "name must contain 1–120 printable bytes"
        );
        anyhow::ensure!(
            !self.command.trim().is_empty()
                && self.command.len() <= 4096
                && !self.command.contains('\0'),
            "invalid command"
        );
        let path = Path::new(&self.command);
        anyhow::ensure!(
            path.is_absolute() || path.components().count() == 1,
            "command must be an absolute path or executable name"
        );
        anyhow::ensure!(
            self.args.len() <= 128
                && self
                    .args
                    .iter()
                    .all(|value| value.len() <= 8192 && !value.contains('\0')),
            "invalid arguments"
        );
        anyhow::ensure!(
            self.env.len() <= 128
                && self.env.iter().all(|(key, value)| {
                    !key.is_empty()
                        && key.len() <= 256
                        && key.bytes().enumerate().all(|(i, byte)| {
                            byte == b'_'
                                || byte.is_ascii_alphabetic()
                                || (i > 0 && byte.is_ascii_digit())
                        })
                        && value.len() <= 32768
                        && !value.contains('\0')
                }),
            "invalid environment"
        );
        anyhow::ensure!(
            serde_json::to_vec(self)?.len() as u64 <= MAX_BYTES,
            "custom agent configuration is too large"
        );
        Ok(())
    }

    pub fn descriptor(&self) -> AgentDescriptor {
        AgentDescriptor {
            kind: AgentKind::Custom {
                id: self.id.clone(),
                name: self.name.clone(),
            },
            native: None,
            agent_process: AgentProcessArtifactSpec {
                install: AgentProcessInstallSpec::PathOnly {
                    candidate_binaries: vec![self.command.clone()],
                    default_args: self.args.clone(),
                    docs_url: None,
                },
            },
            launch: LaunchSpecTemplate {
                executable_name: self.command.clone(),
                default_args: self.args.clone(),
            },
            auth: AuthSpec {
                readiness_policy: AuthReadinessPolicy::None,
                slots: Vec::new(),
            },
            docs_url: None,
            self_update_neutralization: SelfUpdateNeutralization {
                mechanism: SelfUpdateMechanism::NotApplicable,
                detail: "User-owned ACP command; not managed by the runtime".into(),
                env: Vec::new(),
            },
        }
    }

    pub fn resolve(&self) -> ResolvedAgent {
        let program = if Path::new(&self.command).is_absolute() {
            is_valid_executable(Path::new(&self.command)).then(|| PathBuf::from(&self.command))
        } else if let Some(path) = self.env.get("PATH") {
            std::env::split_paths(path)
                .map(|dir| dir.join(&self.command))
                .find(|path| path.is_absolute() && is_valid_executable(path))
        } else {
            find_in_path(&self.command)
        };
        let installed = program.is_some();
        let spawn = program.clone().map(|program| SpawnSpec {
            program,
            args: self.args.clone(),
            env: self.env.clone().into_iter().collect(),
            cwd: None,
        });
        ResolvedAgent {
            descriptor: self.descriptor(),
            status: if installed {
                ResolvedAgentStatus::Ready
            } else {
                ResolvedAgentStatus::InstallRequired
            },
            credential_state: CredentialState::Ready,
            auth_slots: Vec::new(),
            cli_auth_state: Some(CliAuthState::Unsupported),
            native: None,
            agent_process: ResolvedArtifact {
                role: ArtifactRole::AgentProcess,
                installed,
                source: Some("custom".into()),
                version: None,
                path: program,
                message: (!installed)
                    .then(|| "Custom ACP executable was not found or is not executable.".into()),
            },
            spawn,
            credentials_from_route: false,
        }
    }
}

fn path(runtime_home: &Path, id: &str) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(is_custom_id(id), "invalid custom agent id");
    Ok(runtime_home
        .join("custom-agents")
        .join(format!("{id}.json")))
}

pub fn load(runtime_home: &Path, id: &str) -> anyhow::Result<Option<CustomAgent>> {
    let path = path(runtime_home, id)?;
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    anyhow::ensure!(
        metadata.file_type().is_file() && metadata.len() <= MAX_BYTES,
        "invalid custom agent file"
    );
    let config: CustomAgent = serde_json::from_slice(&std::fs::read(path)?)?;
    config.validate()?;
    anyhow::ensure!(config.id == id, "custom agent id mismatch");
    Ok(Some(config))
}

pub fn list(runtime_home: &Path) -> anyhow::Result<Vec<CustomAgent>> {
    let entries = match std::fs::read_dir(runtime_home.join("custom-agents")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut agents = Vec::new();
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let Some(id) = filename
            .to_str()
            .and_then(|value| value.strip_suffix(".json"))
        else {
            continue;
        };
        if !is_custom_id(id) {
            continue;
        }
        match load(runtime_home, id) {
            Ok(Some(config)) => agents.push(config),
            Ok(None) => {}
            Err(_) => tracing::warn!(agent_kind = %id, "ignoring invalid custom ACP registration"),
        }
    }
    agents.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(agents)
}

pub fn register(runtime_home: &Path, config: &CustomAgent) -> anyhow::Result<()> {
    config.validate()?;
    let target = path(runtime_home, &config.id)?;
    let root = target.parent().expect("custom agent path has parent");
    std::fs::create_dir_all(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?;
    }
    let temporary = root.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> anyhow::Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(&serde_json::to_vec(config)?)?;
        file.sync_all()?;
        std::fs::rename(&temporary, target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

pub fn remove(runtime_home: &Path, id: &str) -> anyhow::Result<bool> {
    match std::fs::remove_file(path(runtime_home, id)?) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
