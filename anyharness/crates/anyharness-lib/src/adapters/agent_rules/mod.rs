//! Bounded, private Markdown storage for user-authored agent instructions.
use anyhow::{ensure, Context};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_RULE_BYTES: usize = 64 * 1024;

fn directory(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) => ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "Invalid rules directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(path)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub fn scope_directory(
    home: &Path,
    repository: Option<&str>,
    create: bool,
) -> anyhow::Result<PathBuf> {
    let mut path = home.join("agent-rules");
    let mut components = vec![path.clone()];
    if let Some(repository) = repository {
        use sha2::{Digest, Sha256};
        path.push("repositories");
        components.push(path.clone());
        path.push(format!("{:x}", Sha256::digest(repository.as_bytes())));
    } else {
        path.push("user");
    }
    components.push(path.clone());
    for component in components {
        if create {
            directory(&component)?;
        } else {
            match fs::symlink_metadata(&component) {
                Ok(meta) => ensure!(
                    meta.is_dir() && !meta.file_type().is_symlink(),
                    "Invalid rules directory"
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(path),
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(path)
}

pub fn read(path: &Path) -> anyhow::Result<Option<String>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "Rules must be a regular file"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure!(file.metadata()?.is_file(), "Rules must be a regular file");
    let mut bytes = Vec::new();
    file.take(MAX_RULE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= MAX_RULE_BYTES, "Rules exceed 64 KiB");
    Ok(Some(
        String::from_utf8(bytes).context("Rules must be UTF-8")?,
    ))
}

pub fn write(path: &Path, text: Option<&str>) -> anyhow::Result<()> {
    if let Some(text) = text {
        ensure!(
            text.len() <= MAX_RULE_BYTES && !text.contains('\0'),
            "Invalid rules text"
        );
    }
    match fs::symlink_metadata(path) {
        Ok(meta) => ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "Rules must be a regular file"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = path.parent().context("Missing rules directory")?;
    let Some(text) = text else {
        match fs::remove_file(path) {
            Ok(()) => fs::File::open(parent)?.sync_all()?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        return Ok(());
    };
    ensure!(
        fs::read_dir(parent)?.take(129).count() < 128 || path.exists(),
        "Rules scope is full"
    );
    let temporary = parent.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let result = (|| -> anyhow::Result<()> {
        let mut file = options.open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
