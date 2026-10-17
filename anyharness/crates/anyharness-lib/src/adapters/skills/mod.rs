use anyharness_contract::v1::skills::SkillFile;
use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

pub const MAX_PACKAGE_BYTES: usize = 1024 * 1024;
pub const MAX_FILES: usize = 128;
pub const MAX_INSTRUCTIONS_BYTES: usize = 64 * 1024;

/// Import accepts bytes, never a server-side source path or archive link.
pub fn validate_files(files: &[SkillFile]) -> anyhow::Result<Vec<(PathBuf, Vec<u8>)>> {
    if files.is_empty() || files.len() > MAX_FILES {
        bail!("A skill needs 1–128 files");
    }
    let mut paths = BTreeSet::new();
    let mut total = 0;
    let mut result = Vec::new();
    for file in files {
        let path = Path::new(&file.path);
        if file.path.len() > 240
            || file.path.contains('\\')
            || file.path.contains(':')
            || file.path.chars().any(char::is_control)
            || !path
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
            || file
                .path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || !paths.insert(file.path.to_lowercase())
        {
            bail!("Skill file paths must be unique, relative paths without traversal");
        }
        if file.content_base64.len() > MAX_PACKAGE_BYTES * 4 / 3 + 4 {
            bail!("Skill package exceeds 1 MiB");
        }
        let bytes = STANDARD
            .decode(&file.content_base64)
            .context("Invalid base64 skill resource")?;
        total += bytes.len();
        if total > MAX_PACKAGE_BYTES {
            bail!("Skill package exceeds 1 MiB");
        }
        result.push((path.to_owned(), bytes));
    }
    let instructions = result
        .iter()
        .find(|(path, _)| path == Path::new("SKILL.md"))
        .context("Package must contain SKILL.md at its root")?;
    if instructions.1.is_empty()
        || instructions.1.len() > MAX_INSTRUCTIONS_BYTES
        || std::str::from_utf8(&instructions.1).is_err()
    {
        bail!("SKILL.md must contain 1–65536 bytes of UTF-8 instructions");
    }
    for (path, _) in &result {
        for parent in path
            .ancestors()
            .skip(1)
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if paths.contains(&parent.to_string_lossy().to_lowercase()) {
                bail!("A skill path cannot be both a file and directory");
            }
        }
    }
    Ok(result)
}

/// Each session gets a fresh directory: never merge into user-owned contents.
#[cfg(not(unix))]
pub fn stage_package(root: &Path, files: &[SkillFile]) -> anyhow::Result<()> {
    fs::create_dir(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    }
    for (relative, bytes) in validate_files(files)? {
        let target = root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        use std::io::Write;
        output.write_all(&bytes)?;
    }
    Ok(())
}

pub fn private_directory(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            bail!("Skill staging directory is not a regular directory")
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
            }
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Descriptor-relative writes keep a hostile checkout from redirecting resource
/// writes through symlinks while a launch is being prepared.
#[cfg(unix)]
pub fn stage_in_workspace(
    workspace: &Path,
    packages: &[(String, Vec<SkillFile>)],
) -> anyhow::Result<Vec<PathBuf>> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
    };
    fn directory(parent: &fs::File, name: &str, fresh: bool) -> anyhow::Result<fs::File> {
        anyhow::ensure!(
            !name.is_empty()
                && name != "."
                && name != ".."
                && !name.contains('/')
                && !name.contains('\\'),
            "Invalid staging directory name"
        );
        let name = CString::new(name)?;
        let made = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
        if made != 0 {
            let error = std::io::Error::last_os_error();
            if fresh || error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(error.into());
            }
        }
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(unsafe { fs::File::from_raw_fd(descriptor) })
    }
    let workspace = fs::canonicalize(workspace)?;
    let checkout = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&workspace)?;
    let parent = directory(&checkout, ".proliferate-skills", false)?;
    let launch_id = uuid::Uuid::new_v4().to_string();
    let launch = directory(&parent, &launch_id, true)?;
    let mut roots = Vec::new();
    for (id, files) in packages {
        let root = directory(&launch, id, true)?;
        for (path, bytes) in validate_files(files)? {
            let mut current = root.try_clone()?;
            let parts: Vec<_> = path.iter().collect();
            for part in &parts[..parts.len() - 1] {
                current = directory(
                    &current,
                    part.to_str().context("Invalid resource path")?,
                    false,
                )?;
            }
            let name = CString::new(
                parts
                    .last()
                    .context("Empty resource path")?
                    .as_encoded_bytes(),
            )?;
            let descriptor = unsafe {
                libc::openat(
                    current.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if descriptor < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut file = unsafe { fs::File::from_raw_fd(descriptor) };
            use std::io::Write;
            file.write_all(&bytes)?;
        }
        roots.push(
            workspace
                .join(".proliferate-skills")
                .join(&launch_id)
                .join(id),
        );
    }
    Ok(roots)
}

#[cfg(not(unix))]
pub fn stage_in_workspace(
    workspace: &Path,
    packages: &[(String, Vec<SkillFile>)],
) -> anyhow::Result<Vec<PathBuf>> {
    let parent = workspace.join(".proliferate-skills");
    private_directory(&parent)?;
    let launch_root = parent.join(uuid::Uuid::new_v4().to_string());
    private_directory(&launch_root)?;
    packages
        .iter()
        .map(|(id, files)| {
            let root = launch_root.join(id);
            stage_package(&root, files)?;
            Ok(root)
        })
        .collect()
}
