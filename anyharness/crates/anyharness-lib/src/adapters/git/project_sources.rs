//! Bounded Git source enumeration, respecting tracked and untracked ignores.
use anyhow::{ensure, Context};
use std::{collections::BTreeSet, path::Path, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;
const MAX_LIST_BYTES: usize = 4 * 1024 * 1024;

async fn git_list(root: &Path, ignored: bool) -> anyhow::Result<Vec<String>> {
    let mut command = tokio::process::Command::new("git");
    command
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
        ])
        .arg("ls-files")
        .args(["-z", "--cached", "--exclude-standard", "--deduplicate"]);
    command.arg(if ignored { "--ignored" } else { "--others" });
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_PREFIX",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
    ] {
        command.env_remove(variable);
    }
    let mut child = command
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = child.stdout.take().context("Missing source list")?;
    let bytes = tokio::time::timeout(Duration::from_secs(20), async {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_LIST_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() > MAX_LIST_BYTES {
            let _ = child.kill().await;
            anyhow::bail!("Project file list exceeds its limit");
        }
        ensure!(
            child.wait().await?.success(),
            "Could not discover Git project files"
        );
        Ok::<_, anyhow::Error>(bytes)
    })
    .await
    .context("Project discovery timed out")??;
    Ok(bytes
        .split(|byte| *byte == 0)
        .filter(|bytes| !bytes.is_empty())
        .filter_map(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .collect())
}

pub async fn project_paths(root: &Path) -> anyhow::Result<Vec<String>> {
    let paths = git_list(root, false).await?;
    let ignored: BTreeSet<_> = git_list(root, true).await?.into_iter().collect();
    Ok(paths
        .into_iter()
        .filter(|path| !ignored.contains(path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}
