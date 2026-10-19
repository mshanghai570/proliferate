use super::chunks;
use super::model::{ProjectContextExcerpt, ProjectRetrievalStatus};
use crate::adapters::files::retrieval::read_source;
use anyhow::{ensure, Context};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn directory(home: &Path, create: bool) -> anyhow::Result<Option<PathBuf>> {
    let path = home.join("project-retrieval");
    match fs::symlink_metadata(&path) {
        Ok(meta) => ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "Invalid retrieval cache directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if !create {
                return Ok(None);
            }
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(&path)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(Some(path))
}

pub fn path(home: &Path, workspace: &str, create: bool) -> anyhow::Result<Option<PathBuf>> {
    Ok(directory(home, create)?.map(|directory| {
        directory.join(format!("{:x}.sqlite", Sha256::digest(workspace.as_bytes())))
    }))
}

pub fn open(home: &Path, workspace: &str, create: bool) -> anyhow::Result<Option<Connection>> {
    let Some(path) = path(home, workspace, create)? else {
        return Ok(None);
    };
    match fs::symlink_metadata(&path) {
        Ok(meta) => ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "Invalid retrieval cache file"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if !create {
                return Ok(None);
            }
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
            }
            options.open(&path)?;
        }
        Err(error) => return Err(error.into()),
    }
    let mut journal = path.as_os_str().to_os_string();
    journal.push("-journal");
    if let Ok(meta) = fs::symlink_metadata(Path::new(&journal)) {
        ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "Invalid retrieval journal"
        );
    }
    let connection = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    connection.busy_timeout(Duration::from_secs(2))?;
    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    ensure!(
        version <= 1,
        "Retrieval cache was created by a newer runtime"
    );
    connection.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA secure_delete=ON;
        CREATE TABLE IF NOT EXISTS retrieval_files(path TEXT PRIMARY KEY, hash TEXT NOT NULL);
        CREATE VIRTUAL TABLE IF NOT EXISTS retrieval_chunks USING fts5(path UNINDEXED, start_line UNINDEXED, end_line UNINDEXED, hash UNINDEXED, text, tokenize='unicode61');
        CREATE TABLE IF NOT EXISTS retrieval_meta(id INTEGER PRIMARY KEY CHECK(id=1), indexed_at TEXT NOT NULL, skipped INTEGER NOT NULL, truncated INTEGER NOT NULL); PRAGMA user_version=1;")?;
    Ok(Some(connection))
}

pub fn status(connection: Option<&Connection>) -> anyhow::Result<ProjectRetrievalStatus> {
    let Some(connection) = connection else {
        return Ok(ProjectRetrievalStatus::default());
    };
    let meta = connection
        .query_row(
            "SELECT indexed_at, skipped, truncated FROM retrieval_meta WHERE id=1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, usize>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((indexed_at, skipped_files, truncated)) = meta else {
        return Ok(ProjectRetrievalStatus::default());
    };
    Ok(ProjectRetrievalStatus {
        enabled: true,
        indexed_files: connection
            .query_row("SELECT count(*) FROM retrieval_files", [], |row| row.get(0))?,
        chunks: connection.query_row("SELECT count(*) FROM retrieval_chunks", [], |row| {
            row.get(0)
        })?,
        skipped_files,
        truncated,
        indexed_at: Some(indexed_at),
    })
}

pub fn refresh(
    connection: &mut Connection,
    root: &Path,
    paths: &[String],
) -> anyhow::Result<ProjectRetrievalStatus> {
    let started = Instant::now();
    let transaction = connection.transaction()?;
    let mut seen = BTreeSet::new();
    let mut skipped = 0usize;
    let mut bytes = 0usize;
    let mut count = 0usize;
    let mut truncated = paths.len() > 10000;
    let previous = {
        let mut statement = transaction.prepare("SELECT path, hash FROM retrieval_files")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<std::collections::BTreeMap<_, _>, _>>()?
    };
    for path in paths.iter().take(10000) {
        if started.elapsed() > Duration::from_secs(20) {
            truncated = true;
            break;
        }
        let text = match read_source(root, path) {
            Ok(Some(text)) => text,
            _ => {
                skipped += 1;
                continue;
            }
        };
        bytes += text.len();
        if bytes > 32 * 1024 * 1024 {
            truncated = true;
            break;
        }
        let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
        let chunks = chunks::split(&text);
        if chunks.is_empty() {
            skipped += 1;
            continue;
        }
        count += chunks.len();
        if count > 20000 {
            truncated = true;
            break;
        }
        seen.insert(path.clone());
        if previous.get(path) == Some(&hash) {
            continue;
        }
        transaction.execute("DELETE FROM retrieval_chunks WHERE path=?1", [path])?;
        transaction.execute("INSERT INTO retrieval_files(path, hash) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET hash=excluded.hash", params![path, hash])?;
        for chunk in chunks {
            transaction.execute("INSERT INTO retrieval_chunks(path,start_line,end_line,hash,text) VALUES(?1,?2,?3,?4,?5)", params![path,chunk.start_line,chunk.end_line,hash,chunk.text])?;
        }
    }
    for path in previous.keys().filter(|path| !seen.contains(*path)) {
        transaction.execute("DELETE FROM retrieval_chunks WHERE path=?1", [path])?;
        transaction.execute("DELETE FROM retrieval_files WHERE path=?1", [path])?;
    }
    transaction.execute("INSERT INTO retrieval_meta(id,indexed_at,skipped,truncated) VALUES(1,?1,?2,?3) ON CONFLICT(id) DO UPDATE SET indexed_at=excluded.indexed_at, skipped=excluded.skipped, truncated=excluded.truncated", params![chrono::Utc::now().to_rfc3339(), skipped, truncated])?;
    transaction.commit()?;
    status(Some(connection))
}

pub fn search(connection: &Connection, query: &str) -> anyhow::Result<Vec<ProjectContextExcerpt>> {
    let query = chunks::query(query)?;
    let mut statement = connection.prepare("SELECT path,start_line,end_line,hash,text FROM retrieval_chunks WHERE retrieval_chunks MATCH ?1 ORDER BY bm25(retrieval_chunks), path, start_line LIMIT 8")?;
    let rows = statement.query_map([query], |row| {
        Ok(ProjectContextExcerpt {
            path: row.get(0)?,
            start_line: row.get(1)?,
            end_line: row.get(2)?,
            content_hash: row.get(3)?,
            text: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn clear(home: &Path, workspace: &str) -> anyhow::Result<()> {
    let Some(path) = path(home, workspace, false)? else {
        return Ok(());
    };
    for suffix in ["", "-journal"] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        match fs::symlink_metadata(Path::new(&name)) {
            Ok(meta) => {
                ensure!(
                    meta.is_file() && !meta.file_type().is_symlink(),
                    "Invalid retrieval cache"
                );
                fs::remove_file(&name)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    fs::File::open(path.parent().context("Missing retrieval directory")?)?.sync_all()?;
    Ok(())
}
