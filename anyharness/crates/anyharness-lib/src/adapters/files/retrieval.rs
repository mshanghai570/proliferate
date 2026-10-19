//! Read-only, bounded project source discovery for local retrieval.
use anyhow::ensure;
use std::{
    io::Read,
    path::{Component, Path},
};

pub const MAX_FILE_BYTES: usize = 256 * 1024;

pub fn eligible_path(path: &str) -> bool {
    if path.is_empty()
        || path.len() > 1024
        || path.chars().any(char::is_control)
        || path.contains(['\\', ':'])
    {
        return false;
    }
    let components: Vec<_> = Path::new(path).components().collect();
    if components
        .iter()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return false;
    }
    let lower = path.to_ascii_lowercase();
    let denied = [
        ".git",
        ".svn",
        ".hg",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        ".turbo",
        ".venv",
        "venv",
        "__pycache__",
        ".cache",
        ".aws",
        ".ssh",
        ".codex",
        ".proliferate-skills",
    ];
    if lower.split('/').any(|part| {
        denied.contains(&part)
            || part == ".env"
            || part.starts_with(".env.")
            || part.ends_with(".env")
            || part.starts_with("credentials")
            || part.starts_with("secrets")
            || [
                ".netrc",
                ".npmrc",
                ".pypirc",
                ".git-credentials",
                "id_rsa",
                "id_ed25519",
            ]
            .contains(&part)
    }) {
        return false;
    }
    let extension = Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    [
        "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "swift", "md", "mdx", "txt",
        "html", "css", "scss", "json", "toml", "yaml", "yml", "sql", "sh", "bash", "zsh", "java",
        "kt", "c", "h", "cpp", "hpp", "rb", "php", "vue", "svelte", "xml", "proto",
    ]
    .contains(&extension)
        || Path::new(&lower)
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| ["readme", "makefile", "dockerfile", "justfile"].contains(&name))
}

#[cfg(unix)]
fn open_source(root: &Path, path: &str) -> anyhow::Result<std::fs::File> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        },
    };
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let mut directory = options.open(root)?;
    let parts: Vec<_> = Path::new(path).components().collect();
    for (index, part) in parts.iter().enumerate() {
        let Component::Normal(part) = part else {
            anyhow::bail!("Invalid source path")
        };
        let name = CString::new(part.as_bytes())?;
        let leaf = index + 1 == parts.len();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if leaf {
                libc::O_NONBLOCK
            } else {
                libc::O_DIRECTORY
            };
        // Each descriptor is opened relative to the previously verified directory.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        if leaf {
            return Ok(file);
        }
        directory = file;
    }
    anyhow::bail!("Empty source path")
}

#[cfg(not(unix))]
fn open_source(root: &Path, path: &str) -> anyhow::Result<std::fs::File> {
    let mut candidate = root.to_path_buf();
    for component in Path::new(path).components() {
        candidate.push(component);
        ensure!(
            !std::fs::symlink_metadata(&candidate)?
                .file_type()
                .is_symlink(),
            "Source symlinks are excluded"
        );
    }
    Ok(std::fs::File::open(candidate)?)
}

pub fn read_source(root: &Path, path: &str) -> anyhow::Result<Option<String>> {
    ensure!(eligible_path(path), "Source path is excluded");
    let file = open_source(root, path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES as u64 {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    Read::take(file, MAX_FILE_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FILE_BYTES || bytes.contains(&0) {
        return Ok(None);
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(None);
    };
    if text.contains("PRIVATE KEY-----") {
        return Ok(None);
    }
    Ok(Some(text))
}
