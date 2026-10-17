use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::vsix::{
    extract_vsix, read_vsix_manifest, UnsafeEntry, VsixError, VsixLimits, VSIX_MANIFEST_PATH,
};

/// A unique scratch directory that removes itself on drop, so a failed
/// assertion cannot leave test debris behind (the same guard the git adapter
/// tests use). Derefs to `Path`, so call sites read as if it were the path.
pub(super) struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    pub(super) fn new(prefix: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("anyharness-vsix-{prefix}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }
}

impl std::ops::Deref for TempDirGuard {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn package_json() -> Vec<u8> {
    br#"{"name":"demo","version":"1.2.3","engines":{"vscode":"^1.95.0"},"main":"./out/extension.js"}"#
        .to_vec()
}

/// Write a stored zip with the given `(name, bytes)` entries.
///
/// Deliberately takes no unix mode: `FileOptions::unix_permissions` masks the
/// value to `0o777`, so this helper could not carry a file-type bit even if a
/// caller asked. The symlink test writes its own archive through
/// `ZipWriter::add_symlink`, which sets `S_IFLNK` internally.
fn write_archive(path: &Path, entries: &[(&str, &[u8])]) {
    let file = File::create(path).expect("create archive");
    let mut writer = ZipWriter::new(file);
    for (name, bytes) in entries {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer.start_file(*name, options).expect("start entry");
        writer.write_all(bytes).expect("write entry");
    }
    writer.finish().expect("finish archive");
}

pub(super) fn package_archive(dir: &Path) -> PathBuf {
    let path = dir.join("demo.vsix");
    write_archive(
        &path,
        &[
            (VSIX_MANIFEST_PATH, package_json().as_slice()),
            ("extension/out/extension.js", b"module.exports = {};"),
            ("extension/README.md", b"# Demo"),
        ],
    );
    path
}

#[test]
fn extracts_every_entry_and_reports_what_it_wrote() {
    let dir = TempDirGuard::new("extract");
    let archive = package_archive(&dir);
    let dest = dir.join("unpacked");

    let written = extract_vsix(&archive, &dest, VsixLimits::default()).expect("extract");

    assert_eq!(
        written,
        vec![
            VSIX_MANIFEST_PATH.to_string(),
            "extension/out/extension.js".to_string(),
            "extension/README.md".to_string(),
        ]
    );
    assert_eq!(
        fs::read(dest.join(VSIX_MANIFEST_PATH)).expect("read manifest"),
        package_json()
    );
    // Nested parents are created for the caller.
    assert!(dest.join("extension/out/extension.js").exists());
}

#[test]
fn reads_the_manifest_without_unpacking() {
    let dir = TempDirGuard::new("manifest");
    let archive = package_archive(&dir);
    let dest = dir.join("untouched");

    let manifest = read_vsix_manifest(&archive).expect("read manifest");

    assert_eq!(manifest["name"], "demo");
    assert_eq!(manifest["version"], "1.2.3");
    assert_eq!(manifest["engines"]["vscode"], "^1.95.0");
    // Inspection must not write.
    assert!(!dest.exists());
}

#[test]
fn reports_a_missing_manifest() {
    let dir = TempDirGuard::new("no-manifest");
    let archive = dir.join("plain.zip");
    write_archive(&archive, &[("extension/other.txt", b"x")]);

    assert!(matches!(
        read_vsix_manifest(&archive),
        Err(VsixError::ManifestMissing)
    ));
}

#[test]
fn rejects_parent_directory_traversal() {
    let dir = TempDirGuard::new("traversal");
    let archive = dir.join("evil.vsix");
    write_archive(
        &archive,
        &[
            ("extension/package.json", package_json().as_slice()),
            ("../escape.txt", b"pwned"),
        ],
    );
    let dest = dir.join("unpacked");

    let error = extract_vsix(&archive, &dest, VsixLimits::default()).expect_err("must refuse");

    match error {
        VsixError::UnsafeEntry { path, reason } => {
            assert_eq!(path, "../escape.txt");
            assert_eq!(reason, UnsafeEntry::TraversalAttempt);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    // The escape target must not exist anywhere near the extraction root.
    assert!(!dir.join("escape.txt").exists());
}

#[test]
fn rejects_an_absolute_entry_path() {
    let dir = TempDirGuard::new("absolute");
    let archive = dir.join("evil.vsix");
    write_archive(&archive, &[("/tmp/escape.txt", b"pwned")]);

    let error = extract_vsix(&archive, &dir.join("unpacked"), VsixLimits::default())
        .expect_err("must refuse");

    assert!(
        matches!(
            error,
            VsixError::UnsafeEntry {
                reason: UnsafeEntry::AbsolutePath,
                ..
            }
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn rejects_a_backslash_separator() {
    let dir = TempDirGuard::new("backslash");
    let archive = dir.join("evil.vsix");
    write_archive(&archive, &[("..\\escape.txt", b"pwned")]);

    let error = extract_vsix(&archive, &dir.join("unpacked"), VsixLimits::default())
        .expect_err("must refuse");

    assert!(
        matches!(
            error,
            VsixError::UnsafeEntry {
                reason: UnsafeEntry::BackslashSeparator,
                ..
            }
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn refuses_to_extract_a_symlink_entry() {
    let dir = TempDirGuard::new("symlink");
    let archive = dir.join("evil.vsix");
    {
        let file = File::create(&archive).expect("create archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        // `add_symlink` is the only way to write an entry that carries S_IFLNK:
        // the public `unix_permissions` masks the file-type bits away.
        writer
            .add_symlink("extension/link", "/etc/passwd", options)
            .expect("add symlink entry");
        writer.finish().expect("finish archive");
    }
    let dest = dir.join("unpacked");

    let error = extract_vsix(&archive, &dest, VsixLimits::default()).expect_err("must refuse");

    assert!(
        matches!(
            error,
            VsixError::UnsafeEntry {
                reason: UnsafeEntry::NonRegularFile,
                ..
            }
        ),
        "unexpected error: {error:?}"
    );
    // The refusal happens before anything is written, so no link (and no copy of
    // the link's target) exists under the extraction root.
    assert!(!dest.join("extension/link").exists());
}

#[test]
fn refuses_more_entries_than_the_limit() {
    let dir = TempDirGuard::new("entries");
    let archive = dir.join("many.vsix");
    write_archive(&archive, &[("a.txt", b"a"), ("b.txt", b"b")]);

    let limits = VsixLimits {
        max_entries: 1,
        ..VsixLimits::default()
    };
    let error = extract_vsix(&archive, &dir.join("unpacked"), limits).expect_err("must refuse");

    assert!(
        matches!(
            error,
            VsixError::TooManyEntries {
                entries: 2,
                limit: 1
            }
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn refuses_an_entry_over_the_per_entry_limit() {
    let dir = TempDirGuard::new("entry-size");
    let archive = dir.join("big.vsix");
    write_archive(&archive, &[("extension/big.bin", &[0u8; 64])]);

    let limits = VsixLimits {
        max_entry_bytes: 16,
        ..VsixLimits::default()
    };
    let error = extract_vsix(&archive, &dir.join("unpacked"), limits).expect_err("must refuse");

    assert!(
        matches!(
            error,
            VsixError::EntryTooLarge {
                size: 64,
                limit: 16,
                ..
            }
        ),
        "unexpected error: {error:?}"
    );
    assert!(!dir.join("unpacked/extension/big.bin").exists());
}

#[test]
fn refuses_an_archive_that_expands_past_the_total_limit() {
    let dir = TempDirGuard::new("total-size");
    let archive = dir.join("bomb.vsix");
    write_archive(&archive, &[("a.bin", &[0u8; 32]), ("b.bin", &[0u8; 32])]);

    let limits = VsixLimits {
        max_entry_bytes: 64,
        max_total_bytes: 48,
        ..VsixLimits::default()
    };
    let error = extract_vsix(&archive, &dir.join("unpacked"), limits).expect_err("must refuse");

    assert!(
        matches!(
            error,
            VsixError::ArchiveTooLarge {
                total: 64,
                limit: 48
            }
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn creates_directory_entries() {
    let dir = TempDirGuard::new("dirs");
    let archive = dir.join("dirs.vsix");
    {
        let file = File::create(&archive).expect("create archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .add_directory("extension/", options)
            .expect("add dir");
        writer
            .start_file("extension/a.txt", options)
            .expect("start file");
        writer.write_all(b"a").expect("write");
        writer.finish().expect("finish");
    }

    let dest = dir.join("unpacked");
    extract_vsix(&archive, &dest, VsixLimits::default()).expect("extract");

    assert!(dest.join("extension").is_dir());
    assert_eq!(fs::read(dest.join("extension/a.txt")).expect("read"), b"a");
}

#[test]
fn rejects_windows_paths_on_every_host_before_writing() {
    for name in [
        "C:/escape.txt",
        "C:escape.txt",
        "extension/a:stream",
        "extension/../x",
        "extension/.. /x",
        "extension/CON.txt",
    ] {
        let dir = TempDirGuard::new("windows-path");
        let archive = dir.join("evil.vsix");
        write_archive(&archive, &[("extension/safe.txt", b"safe"), (name, b"bad")]);
        let dest = dir.join("unpacked");
        assert!(
            extract_vsix(&archive, &dest, VsixLimits::default()).is_err(),
            "accepted {name}"
        );
        assert!(!dest.exists(), "validation wrote output for {name}");
    }
}

#[test]
fn bounds_manifest_inspection() {
    let dir = TempDirGuard::new("large-manifest");
    let archive = dir.join("large.vsix");
    let raw = format!("{{\"name\":\"{}\"}}", "x".repeat(1024 * 1024));
    write_archive(&archive, &[(VSIX_MANIFEST_PATH, raw.as_bytes())]);
    assert!(matches!(
        read_vsix_manifest(&archive),
        Err(VsixError::EntryTooLarge { .. })
    ));
}

#[test]
fn never_overwrites_a_nonempty_destination() {
    let dir = TempDirGuard::new("destination");
    let archive = package_archive(&dir);
    let dest = dir.join("unpacked");
    fs::create_dir(&dest).unwrap();
    fs::write(dest.join("keep.txt"), b"original").unwrap();
    assert!(extract_vsix(&archive, &dest, VsixLimits::default()).is_err());
    assert_eq!(fs::read(dest.join("keep.txt")).unwrap(), b"original");
    assert!(!dest.join("extension").exists());
}

#[cfg(unix)]
#[test]
fn refuses_a_symlink_destination() {
    let dir = TempDirGuard::new("destination-link");
    let archive = package_archive(&dir);
    let outside = dir.join("outside");
    fs::create_dir(&outside).unwrap();
    let dest = dir.join("unpacked");
    std::os::unix::fs::symlink(&outside, &dest).unwrap();
    assert!(extract_vsix(&archive, &dest, VsixLimits::default()).is_err());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[test]
fn rejects_normalized_path_collisions_before_writing() {
    let dir = TempDirGuard::new("duplicate-path");
    let archive = dir.join("duplicate.vsix");
    write_archive(
        &archive,
        &[
            ("extension/a.txt", b"first"),
            ("extension/./A.txt", b"second"),
        ],
    );
    let dest = dir.join("unpacked");
    assert!(matches!(
        extract_vsix(&archive, &dest, VsixLimits::default()),
        Err(VsixError::DuplicateEntry)
    ));
    assert!(!dest.exists());
}
