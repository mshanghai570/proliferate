use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(super) const MAX_PROVIDERS: usize = 32;
const MAX_CATALOG_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub model_count: usize,
    pub ciphertext: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Catalog {
    pub version: u8,
    pub revision: i64,
    pub providers: Vec<Record>,
}

pub(super) fn directory(home: &Path) -> PathBuf {
    home.join("agent-auth").join("local-providers")
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
}

pub(super) fn lock(home: &Path) -> anyhow::Result<File> {
    let dir = directory(home);
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&dir)?;
    anyhow::ensure!(
        !fs::symlink_metadata(&dir)?.file_type().is_symlink(),
        "invalid vault directory"
    );
    let file = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("vault.lock"))?;
    fs2::FileExt::lock_exclusive(&file)?;
    Ok(file)
}

pub(super) fn read(home: &Path) -> anyhow::Result<Catalog> {
    let file = match options()
        .read(true)
        .open(directory(home).join("providers.json"))
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Catalog {
                version: 1,
                revision: 0,
                providers: vec![],
            })
        }
        Err(error) => return Err(error.into()),
    };
    anyhow::ensure!(file.metadata()?.is_file(), "invalid vault file");
    let mut bytes = Vec::new();
    file.take(MAX_CATALOG_BYTES + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_CATALOG_BYTES,
        "vault exceeds limit"
    );
    let catalog: Catalog = serde_json::from_slice(&bytes)?;
    if catalog.version != 1
        || catalog.revision < 0
        || (!catalog.providers.is_empty() && catalog.revision == 0)
        || catalog.providers.len() > MAX_PROVIDERS
    {
        bail!("invalid vault catalog");
    }
    let mut ids = std::collections::BTreeSet::new();
    for row in &catalog.providers {
        uuid::Uuid::parse_str(&row.id).context("invalid provider identity")?;
        anyhow::ensure!(
            ids.insert(&row.id) && row.name.len() <= 255 && row.model_count <= 100,
            "invalid provider metadata"
        );
    }
    Ok(catalog)
}

pub(super) fn write(home: &Path, catalog: &mut Catalog) -> anyhow::Result<()> {
    catalog.revision = catalog
        .revision
        .checked_add(1)
        .context("vault revision exhausted")?;
    let bytes = serde_json::to_vec(catalog)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_CATALOG_BYTES,
        "vault exceeds limit"
    );
    let dir = directory(home);
    let temporary = dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> anyhow::Result<()> {
        let mut file = options().write(true).create_new(true).open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, dir.join("providers.json"))?;
        File::open(&dir)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
