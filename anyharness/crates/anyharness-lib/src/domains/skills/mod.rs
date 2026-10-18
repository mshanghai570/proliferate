use crate::adapters::skills::{private_directory, stage_in_workspace, validate_files, SkillFile};
use crate::domains::sessions::extensions::{
    SessionExtension, SessionLaunchContext, SessionLaunchExtras,
};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::Mutex};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub user_enabled: bool,
    pub repository_overrides: BTreeMap<String, bool>,
    pub file_count: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct Package {
    summary: SkillSummary,
    files: Vec<SkillFile>,
}

/// Runtime-local user settings. Repository overrides use stable repo-root ids,
/// so a worktree inherits its repository's selection.
pub struct SkillManager {
    home: PathBuf,
    gate: Mutex<()>,
}

impl SkillManager {
    pub fn new(runtime_home: PathBuf) -> Self {
        Self {
            home: runtime_home.join("skills"),
            gate: Mutex::new(()),
        }
    }

    fn read(&self) -> anyhow::Result<Vec<Package>> {
        let path = self.home.join("packages.json");
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        use std::io::Read;
        const MAX_CATALOG_BYTES: u64 = 96 * 1024 * 1024;
        let mut bytes = Vec::new();
        file.take(MAX_CATALOG_BYTES + 1).read_to_end(&mut bytes)?;
        anyhow::ensure!(
            bytes.len() as u64 <= MAX_CATALOG_BYTES,
            "Skill catalog exceeds its size limit"
        );
        let packages: Vec<Package> =
            serde_json::from_slice(&bytes).context("Skill catalog is invalid")?;
        anyhow::ensure!(
            packages.len() <= 64,
            "Skill catalog exceeds its package limit"
        );
        Ok(packages)
    }

    fn write(&self, packages: &[Package]) -> anyhow::Result<()> {
        private_directory(&self.home)?;
        let temporary = self.home.join(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        use std::io::Write;
        let result = (|| -> anyhow::Result<()> {
            file.write_all(&serde_json::to_vec(packages)?)?;
            file.sync_all()?;
            fs::rename(&temporary, self.home.join("packages.json"))?;
            fs::File::open(&self.home)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub fn list(&self) -> anyhow::Result<Vec<SkillSummary>> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Skill catalog unavailable"))?;
        Ok(self
            .read()?
            .into_iter()
            .map(|package| package.summary)
            .collect())
    }

    pub fn import(&self, name: String, files: Vec<SkillFile>) -> anyhow::Result<SkillSummary> {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 || name.chars().any(char::is_control) {
            bail!("Skill name must contain 1–100 printable bytes");
        }
        validate_files(&files)?;
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Skill catalog unavailable"))?;
        let mut packages = self.read()?;
        if packages.len() >= 64 {
            bail!("Remove a skill before importing more than 64 packages");
        }
        let summary = SkillSummary {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
            user_enabled: false,
            repository_overrides: BTreeMap::new(),
            file_count: files.len(),
        };
        packages.push(Package {
            summary: summary.clone(),
            files,
        });
        self.write(&packages)?;
        tracing::info!(
            event = "skill.imported",
            file_count = summary.file_count,
            "Skill package imported"
        );
        Ok(summary)
    }

    pub fn set_enabled(
        &self,
        id: &str,
        repository_id: Option<String>,
        enabled: Option<bool>,
    ) -> anyhow::Result<SkillSummary> {
        if repository_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 128 || id.chars().any(char::is_control))
        {
            bail!("Invalid repository id");
        }
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Skill catalog unavailable"))?;
        let mut packages = self.read()?;
        let package = packages
            .iter_mut()
            .find(|package| package.summary.id == id)
            .context("Unknown skill")?;
        match repository_id {
            Some(repository) => match enabled {
                Some(enabled) => {
                    package
                        .summary
                        .repository_overrides
                        .insert(repository, enabled);
                }
                None => {
                    package.summary.repository_overrides.remove(&repository);
                }
            },
            None => {
                package.summary.user_enabled =
                    enabled.context("User enablement requires a boolean")?
            }
        }
        let summary = package.summary.clone();
        self.write(&packages)?;
        tracing::info!(
            event = "skill.enablement_changed",
            "Skill enablement changed"
        );
        Ok(summary)
    }

    pub fn remove(&self, id: &str) -> anyhow::Result<()> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Skill catalog unavailable"))?;
        let mut packages = self.read()?;
        packages.retain(|package| package.summary.id != id);
        self.write(&packages)?;
        Ok(())
    }
}

impl SessionExtension for SkillManager {
    fn resolve_launch_extras(
        &self,
        ctx: &SessionLaunchContext<'_>,
    ) -> anyhow::Result<SessionLaunchExtras> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("Skill catalog unavailable"))?;
        let packages: Vec<_> = self
            .read()?
            .into_iter()
            .filter(|package| {
                package
                    .summary
                    .repository_overrides
                    .get(&ctx.workspace.repo_root_id)
                    .copied()
                    .unwrap_or(package.summary.user_enabled)
            })
            .collect();
        if packages.is_empty() {
            return Ok(SessionLaunchExtras::default());
        }
        let mut prompts = Vec::new();
        let mut instructions = Vec::new();
        let mut total = 0;
        for package in &packages {
            let files = validate_files(&package.files)?;
            let (_, bytes) = files
                .into_iter()
                .find(|(path, _)| path == std::path::Path::new("SKILL.md"))
                .context("Missing skill instructions")?;
            total += bytes.len();
            anyhow::ensure!(
                total <= 256 * 1024,
                "Enabled skill instructions exceed the 256 KiB launch limit"
            );
            instructions.push(String::from_utf8(bytes)?);
        }
        let resources: Vec<_> = packages
            .iter()
            .map(|package| (package.summary.id.clone(), package.files.clone()))
            .collect();
        let roots = stage_in_workspace(std::path::Path::new(&ctx.workspace.path), &resources)?;
        for ((package, root), instructions) in packages.into_iter().zip(roots).zip(instructions) {
            prompts.push(format!("Enabled skill: {}\nSkill package directory: {}\nResolve relative skill resource paths against this directory. Skill instructions grant no additional tools or permissions.\n\n{}", package.summary.name, root.display(), instructions));
        }
        tracing::info!(
            event = "skill.launch_prepared",
            skill_count = prompts.len(),
            "Enabled skills prepared"
        );
        Ok(SessionLaunchExtras {
            launch_prompt_append: prompts,
            ..Default::default()
        })
    }
}
