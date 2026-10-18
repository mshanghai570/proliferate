use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::installer::{install_local_vsix, InstallError};
use super::manifest::{inspect_install_manifest, ExtensionPackageIdentity, ManifestError};
use crate::adapters::extensions::package::PackageError;

pub(crate) struct TestDirectory(pub(crate) PathBuf);
impl TestDirectory {
    pub(crate) fn new() -> Self {
        let path = std::env::temp_dir().join(format!("extension-install-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn identity(version: &str) -> ExtensionPackageIdentity {
    ExtensionPackageIdentity {
        namespace: "example".into(),
        name: "demo".into(),
        version: version.into(),
    }
}

pub(crate) fn manifest(version: &str) -> serde_json::Value {
    json!({"publisher":"example", "name":"demo", "version":version,
        "engines":{"vscode":"^1.90.0"},
        "contributes":{"themes":[{"label":"Demo", "path":"./themes/demo.json"}]}})
}

pub(crate) fn archive(root: &Path, raw: &serde_json::Value, resource: &str) -> (PathBuf, String) {
    archive_with_content(
        root,
        raw,
        resource,
        br##"{"colors":{"editor.background":"#171717"}}"##,
    )
}

pub(crate) fn archive_with_content(
    root: &Path,
    raw: &serde_json::Value,
    resource: &str,
    content: &[u8],
) -> (PathBuf, String) {
    let path = root.join(format!("{}.vsix", Uuid::new_v4()));
    let mut writer = ZipWriter::new(File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer
        .start_file("extension/package.json", options)
        .unwrap();
    writer.write_all(&serde_json::to_vec(raw).unwrap()).unwrap();
    writer.start_file(resource, options).unwrap();
    writer.write_all(content).unwrap();
    writer.finish().unwrap();
    let digest = format!("{:x}", Sha256::digest(fs::read(&path).unwrap()));
    (path, digest)
}

#[test]
fn installs_verified_data_package_without_executing_scripts() {
    let root = TestDirectory::new();
    let mut raw = manifest("1.0.0");
    raw["scripts"] = json!({"postinstall":format!("touch {}", root.0.join("executed").display())});
    let (archive, digest) = archive(&root.0, &raw, "extension/themes/demo.json");
    let installed = install_local_vsix(&root.0, &archive, &identity("1.0.0"), &digest).unwrap();
    assert_eq!(installed.receipt.sha256, digest);
    assert_eq!(installed.receipt.identity, identity("1.0.0"));
    assert!(installed
        .directory
        .join("extension/themes/demo.json")
        .is_file());
    assert!(!root.0.join("executed").exists());
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(installed.directory.join(".proliferate-package.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["sha256"], digest);
    assert_eq!(receipt["identity"]["namespace"], "example");
    assert_eq!(
        fs::read_dir(installed.directory.parent().unwrap())
            .unwrap()
            .count(),
        2
    ); // package plus lock
}

#[test]
fn rejects_checksum_mismatch_and_discards_staging() {
    let root = TestDirectory::new();
    let (archive, _) = archive(&root.0, &manifest("1.0.0"), "extension/themes/demo.json");
    assert!(matches!(
        install_local_vsix(&root.0, &archive, &identity("1.0.0"), &"0".repeat(64)),
        Err(InstallError::Package(PackageError::ChecksumMismatch))
    ));
    let packages = root.0.join("extensions/packages");
    assert_eq!(fs::read_dir(packages).unwrap().count(), 0);
}

#[test]
fn rejects_manifest_identity_mismatch_without_publishing() {
    let root = TestDirectory::new();
    let (archive, digest) = archive(&root.0, &manifest("1.0.0"), "extension/themes/demo.json");
    assert!(matches!(
        install_local_vsix(&root.0, &archive, &identity("2.0.0"), &digest),
        Err(InstallError::Manifest(ManifestError::IdentityMismatch))
    ));
    assert_eq!(
        fs::read_dir(root.0.join("extensions/packages"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn blocks_declared_node_and_browser_entrypoints() {
    for entrypoint in ["main", "browser"] {
        let root = TestDirectory::new();
        let mut raw = manifest("1.0.0");
        raw[entrypoint] = json!("./extension.js");
        let (archive, digest) = archive(&root.0, &raw, "extension/themes/demo.json");
        assert!(matches!(
            install_local_vsix(&root.0, &archive, &identity("1.0.0"), &digest),
            Err(InstallError::Manifest(ManifestError::ExecutableUnsupported))
        ));
        assert_eq!(
            fs::read_dir(root.0.join("extensions/packages"))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
fn blocks_dependencies_and_unsupported_contributions() {
    let mut raw = manifest("1.0.0");
    raw["extensionDependencies"] = json!(["example.other"]);
    assert!(matches!(
        inspect_install_manifest(&raw, &identity("1.0.0")),
        Err(ManifestError::DependenciesUnsupported)
    ));
    raw.as_object_mut().unwrap().remove("extensionDependencies");
    raw["contributes"]["commands"] = json!([{"command":"demo.run"}]);
    assert!(matches!(
        inspect_install_manifest(&raw, &identity("1.0.0")),
        Err(ManifestError::UnsupportedContribution(_))
    ));
}

#[test]
fn blocks_invalid_manifest_resource_paths() {
    for path in [
        "../outside.json",
        "/etc/passwd",
        "C:/secret",
        "..\\outside",
        "themes/a:stream",
        "themes/.. /x",
    ] {
        let mut raw = manifest("1.0.0");
        raw["contributes"]["themes"][0]["path"] = json!(path);
        assert!(
            matches!(
                inspect_install_manifest(&raw, &identity("1.0.0")),
                Err(ManifestError::InvalidResourcePath)
            ),
            "accepted {path}"
        );
    }
}

#[test]
fn refuses_missing_declared_resources() {
    let root = TestDirectory::new();
    let (archive, digest) = archive(&root.0, &manifest("1.0.0"), "extension/other.json");
    assert!(matches!(
        install_local_vsix(&root.0, &archive, &identity("1.0.0"), &digest),
        Err(InstallError::MissingResource)
    ));
    assert_eq!(
        fs::read_dir(root.0.join("extensions/packages"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn failed_new_version_preserves_existing_package() {
    let root = TestDirectory::new();
    let (first, digest) = archive(&root.0, &manifest("1.0.0"), "extension/themes/demo.json");
    let installed = install_local_vsix(&root.0, &first, &identity("1.0.0"), &digest).unwrap();
    let original = fs::read(installed.directory.join("extension/package.json")).unwrap();
    let (second, digest) = archive(&root.0, &manifest("2.0.0"), "extension/missing.json");
    assert!(install_local_vsix(&root.0, &second, &identity("2.0.0"), &digest).is_err());
    assert_eq!(
        fs::read(installed.directory.join("extension/package.json")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_dir(root.0.join("extensions/packages"))
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn refuses_overwriting_an_installed_version() {
    let root = TestDirectory::new();
    let (archive, digest) = archive(&root.0, &manifest("1.0.0"), "extension/themes/demo.json");
    let installed = install_local_vsix(&root.0, &archive, &identity("1.0.0"), &digest).unwrap();
    let receipt = fs::read(installed.directory.join(".proliferate-package.json")).unwrap();
    assert!(matches!(
        install_local_vsix(&root.0, &archive, &identity("1.0.0"), &digest),
        Err(InstallError::Package(PackageError::AlreadyInstalled))
    ));
    assert_eq!(
        fs::read(installed.directory.join(".proliferate-package.json")).unwrap(),
        receipt
    );
}

#[cfg(unix)]
#[test]
fn refuses_a_symlink_in_owned_install_layout() {
    let root = TestDirectory::new();
    let outside = root.0.join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.0.join("extensions")).unwrap();
    let (archive, digest) = archive(&root.0, &manifest("1.0.0"), "extension/themes/demo.json");
    assert!(install_local_vsix(&root.0, &archive, &identity("1.0.0"), &digest).is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

#[test]
fn rejects_invalid_requested_identity_before_staging() {
    let root = TestDirectory::new();
    let (archive, digest) = archive(&root.0, &manifest("1.0.0"), "extension/themes/demo.json");
    for version in [
        "../1.0.0",
        "01.0.0",
        "1.0",
        "1.0.0-01",
        "1.0.0+",
        "1.0.0+bad+build",
    ] {
        assert!(matches!(
            install_local_vsix(&root.0, &archive, &identity(version), &digest),
            Err(InstallError::Manifest(ManifestError::InvalidIdentity))
        ));
    }
    let mut invalid = identity("1.0.0");
    invalid.namespace = "../outside".into();
    assert!(matches!(
        install_local_vsix(&root.0, &archive, &invalid, &digest),
        Err(InstallError::Manifest(ManifestError::InvalidIdentity))
    ));
    assert!(!root.0.join("extensions").exists());
    assert!(identity("1.2.3-rc.1+build.5").directory_name().is_ok());
}

#[test]
fn refuses_archive_supplied_installation_receipt() {
    let root = TestDirectory::new();
    let path = root.0.join("receipt.vsix");
    let mut writer = ZipWriter::new(File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, data) in [
        (
            "extension/package.json",
            serde_json::to_vec(&manifest("1.0.0")).unwrap(),
        ),
        ("extension/themes/demo.json", b"{}".to_vec()),
        (".proliferate-package.json", b"{\"trusted\":true}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    writer.finish().unwrap();
    let digest = format!("{:x}", Sha256::digest(fs::read(&path).unwrap()));
    assert!(matches!(
        install_local_vsix(&root.0, &path, &identity("1.0.0"), &digest),
        Err(InstallError::Package(PackageError::Io(_)))
    ));
    assert_eq!(
        fs::read_dir(root.0.join("extensions/packages"))
            .unwrap()
            .count(),
        0
    );
}
