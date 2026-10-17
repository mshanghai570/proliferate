use sha2::{Digest, Sha256};
use std::fs;
use std::io;

use super::package::{PackageError, StagedVsix};
use super::vsix_tests::{package_archive, TempDirGuard};

#[test]
fn publication_sync_failure_rolls_back_and_cleans_staging() {
    let root = TempDirGuard::new("publish-rollback");
    let archive = package_archive(&root);
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    let staged = StagedVsix::prepare(&archive, &root, &digest).unwrap();
    let result = staged.publish_with_sync("example.demo-1.0.0", |_| {
        Err(io::Error::other("injected sync failure"))
    });
    assert!(matches!(result, Err(PackageError::Io(_))));
    let packages = root.join("extensions/packages");
    assert!(!packages.join("example.demo-1.0.0").exists());
    assert_eq!(fs::read_dir(packages).unwrap().count(), 1); // lock only
}

#[cfg(unix)]
#[test]
fn staging_and_package_files_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let root = TempDirGuard::new("permissions");
    let archive = package_archive(&root);
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    let staged = StagedVsix::prepare(&archive, &root, &digest).unwrap();
    assert_eq!(
        fs::metadata(staged.payload()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(staged.payload().join("extension/package.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn invalid_checksum_format_is_rejected_before_creating_install_layout() {
    let root = TempDirGuard::new("invalid-digest");
    let archive = package_archive(&root);
    assert!(matches!(
        StagedVsix::prepare(&archive, &root, "bad digest"),
        Err(PackageError::InvalidChecksum)
    ));
    assert!(!root.join("extensions").exists());
}

#[test]
fn rejects_oversized_compressed_package_without_copying_it() {
    use super::package::MAX_PACKAGE_BYTES;
    let root = TempDirGuard::new("compressed-limit");
    let archive = root.join("large.vsix");
    fs::File::create(&archive)
        .unwrap()
        .set_len(MAX_PACKAGE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        StagedVsix::prepare(&archive, &root, &"0".repeat(64)),
        Err(PackageError::TooLarge)
    ));
    assert_eq!(
        fs::read_dir(root.join("extensions/packages"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn corrupt_archive_content_is_rejected_even_with_a_matching_package_digest() {
    let root = TempDirGuard::new("crc");
    let archive = package_archive(&root);
    let mut bytes = fs::read(&archive).unwrap();
    let marker = b"module.exports = {};";
    let position = bytes
        .windows(marker.len())
        .position(|slice| slice == marker)
        .unwrap();
    bytes[position] ^= 1;
    fs::write(&archive, &bytes).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    assert!(matches!(
        StagedVsix::prepare(&archive, &root, &digest),
        Err(PackageError::Vsix(_))
    ));
    assert_eq!(
        fs::read_dir(root.join("extensions/packages"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn uses_verified_snapshot_when_original_archive_changes() {
    let root = TempDirGuard::new("snapshot");
    let archive = package_archive(&root);
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    let staged = StagedVsix::prepare(&archive, &root, &digest).unwrap();
    fs::write(archive, b"changed after verification").unwrap();
    let installed = staged.publish("example.demo-1.0.0").unwrap();
    assert_eq!(
        fs::read(installed.join("extension/README.md")).unwrap(),
        b"# Demo"
    );
}

#[test]
fn concurrent_publications_do_not_overwrite_each_other() {
    let root = TempDirGuard::new("concurrent");
    let archive = package_archive(&root);
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    let first = StagedVsix::prepare(&archive, &root, &digest).unwrap();
    let second = StagedVsix::prepare(&archive, &root, &digest).unwrap();
    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| first.publish("example.demo-1.0.0"));
        let b = scope.spawn(|| second.publish("example.demo-1.0.0"));
        (a.join().unwrap(), b.join().unwrap())
    });
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(PackageError::AlreadyInstalled)) | (Err(PackageError::AlreadyInstalled), Ok(_))
    ));
    assert_eq!(
        fs::read_dir(root.join("extensions/packages"))
            .unwrap()
            .count(),
        2
    );
}

#[cfg(unix)]
#[test]
fn refuses_staging_symlinks_before_publication() {
    let root = TempDirGuard::new("staging-link");
    let archive = package_archive(&root);
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
    let staged = StagedVsix::prepare(&archive, &root, &digest).unwrap();
    let outside = root.join("credential.txt");
    fs::write(&outside, b"private").unwrap();
    std::os::unix::fs::symlink(&outside, staged.payload().join("sneaky")).unwrap();
    assert!(matches!(
        staged.publish("example.demo-1.0.0"),
        Err(PackageError::Io(_))
    ));
    assert_eq!(fs::read(outside).unwrap(), b"private");
    assert!(!root.join("extensions/packages/example.demo-1.0.0").exists());
}
