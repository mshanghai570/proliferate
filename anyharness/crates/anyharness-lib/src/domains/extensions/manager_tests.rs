use std::fs;

use super::installer_tests::{archive, identity, manifest, TestDirectory};
use super::manager::ExtensionManager;
use super::model::ExtensionState;
use crate::persistence::Db;

#[test]
fn color_themes_read_only_declared_core_colors_from_registered_packages() {
    use super::installer_tests::archive_with_content;
    let home = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    assert!(matches!(
        manager.color_themes(&identity("1.0.0")),
        Err(super::manager::ManagementError::NotInstalled)
    ));
    let mut raw = manifest("1.0.0");
    raw["contributes"]["themes"][0]["uiTheme"] = serde_json::json!("vs-dark");
    let (archive, digest) = archive_with_content(&home.0, &raw, "extension/themes/demo.json", br##"{
        "colors":{"editor.background":"#171717","editor.foreground":"#ffffff","secret":"do not expose"},
        "tokenColors":[{"scope":"comment"}]
    }"##);
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let (themes, truncated) = manager.color_themes(&identity("1.0.0")).unwrap();
    assert!(!truncated);
    assert_eq!(themes.len(), 1);
    assert_eq!(themes[0].label, "Demo");
    assert_eq!(themes[0].color_mode, Some("dark"));
    assert!(themes[0].available);
    assert_eq!(themes[0].colors.len(), 2);
    assert_eq!(themes[0].colors["editor.foreground"], "#ffffff");
    fs::write(home.0.join("extensions/packages/example.demo-1.0.0/extension/themes/demo.json"), br##"{"include":"../../outside.json","colors":{"editor.background":"#171717","editor.foreground":"#ffffff"}}"##).unwrap();
    assert!(!manager.color_themes(&identity("1.0.0")).unwrap().0[0].available);
    fs::write(
        home.0
            .join("extensions/packages/example.demo-1.0.0/extension/themes/demo.json"),
        b"// unsupported JSON comments",
    )
    .unwrap();
    let (themes, _) = manager.color_themes(&identity("1.0.0")).unwrap();
    assert!(!themes[0].available);
    assert!(themes[0].colors.is_empty());
    let resource = home
        .0
        .join("extensions/packages/example.demo-1.0.0/extension/themes/demo.json");
    fs::write(&resource, vec![b' '; 1_048_577]).unwrap();
    assert!(!manager.color_themes(&identity("1.0.0")).unwrap().0[0].available);
    #[cfg(unix)]
    {
        let outside = home.0.join("outside.json");
        fs::write(&outside, br##"{"type":"dark","colors":{"editor.background":"#171717","editor.foreground":"#ffffff"}}"##).unwrap();
        fs::remove_file(&resource).unwrap();
        std::os::unix::fs::symlink(outside, resource).unwrap();
        assert!(manager.color_themes(&identity("1.0.0")).is_err());
    }
}

#[test]
fn color_theme_inventory_is_bounded() {
    let home = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    let mut raw = manifest("1.0.0");
    raw["contributes"]["themes"] =
        serde_json::Value::Array(vec![raw["contributes"]["themes"][0].clone(); 33]);
    let (archive, digest) = archive(&home.0, &raw, "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let (themes, truncated) = manager.color_themes(&identity("1.0.0")).unwrap();
    assert_eq!(themes.len(), 32);
    assert!(truncated);
}

#[test]
fn registered_install_survives_restart_and_uninstall_removes_only_selected_version() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    let manager = ExtensionManager::new(db.clone(), home.0.clone());
    for version in ["1.0.0", "2.0.0"] {
        let (archive, digest) = archive(&home.0, &manifest(version), "extension/themes/demo.json");
        let installed = manager
            .install(&archive, &identity(version), &digest)
            .unwrap();
        assert_eq!(installed.state, ExtensionState::Installed);
        assert!(!installed.active);
    }
    drop(manager);
    drop(db);
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    assert_eq!(manager.list().unwrap().len(), 2);
    manager.uninstall(&identity("1.0.0")).unwrap();
    manager.uninstall(&identity("1.0.0")).unwrap();
    let remaining = manager.list().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].identity.version, "2.0.0");
    assert!(!home
        .0
        .join("extensions/packages/example.demo-1.0.0")
        .exists());
    assert!(home
        .0
        .join("extensions/packages/example.demo-2.0.0")
        .exists());
}

#[test]
fn failed_checksum_does_not_register_or_remove_previous_install() {
    let home = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    assert!(manager
        .install(&archive, &identity("2.0.0"), &"0".repeat(64))
        .is_err());
    let rows = manager.list().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].identity.version, "1.0.0");
}

#[test]
fn interrupted_database_registration_recovers_published_receipt() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    db.with_conn(|conn| {
        conn.execute_batch(
        "CREATE TRIGGER fail_extension_registration BEFORE UPDATE OF state ON installed_extensions
         WHEN NEW.state = 'installed' BEGIN SELECT RAISE(ABORT, 'injected'); END;"
    )
    })
    .unwrap();
    let manager = ExtensionManager::new(db.clone(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    assert!(manager
        .install(&archive, &identity("1.0.0"), &digest)
        .is_err());
    assert!(home
        .0
        .join("extensions/packages/example.demo-1.0.0")
        .exists());
    db.with_conn(|conn| conn.execute_batch("DROP TRIGGER fail_extension_registration;"))
        .unwrap();
    drop(manager);
    let manager = ExtensionManager::new(db, home.0.clone());
    let rows = manager.list().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, ExtensionState::Installed);
    assert!(!rows[0].active);
}

#[test]
fn missing_package_is_reported_without_claiming_activation() {
    let home = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    fs::remove_dir_all(home.0.join("extensions/packages/example.demo-1.0.0")).unwrap();
    let rows = manager.list().unwrap();
    assert_eq!(rows[0].state, ExtensionState::Error);
    assert_eq!(rows[0].error_code.as_deref(), Some("package_missing"));
    assert!(!rows[0].active);
    manager.uninstall(&identity("1.0.0")).unwrap();
    assert!(manager.list().unwrap().is_empty());
}

#[test]
fn altered_receipt_cannot_be_uninstalled_as_registered_package() {
    let home = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let package = home.0.join("extensions/packages/example.demo-1.0.0");
    fs::write(package.join(".proliferate-package.json"), b"{}").unwrap();
    assert_eq!(manager.list().unwrap()[0].state, ExtensionState::Error);
    assert!(manager.uninstall(&identity("1.0.0")).is_err());
    assert!(package.join("extension/themes/demo.json").exists());
}

#[test]
fn unregistered_packages_are_never_adopted_or_deleted_by_inventory() {
    let home = TestDirectory::new();
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    super::installer::install_local_vsix(&home.0, &archive, &identity("1.0.0"), &digest).unwrap();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    assert!(manager.list().unwrap().is_empty());
    manager.uninstall(&identity("1.0.0")).unwrap();
    assert!(manager
        .install(&archive, &identity("1.0.0"), &digest)
        .is_err());
    assert!(manager.list().unwrap().is_empty());
    assert!(home
        .0
        .join("extensions/packages/example.demo-1.0.0")
        .exists());
}

#[test]
fn interrupted_install_without_publication_is_visible_and_can_be_removed() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    super::store::ExtensionStore(db.clone())
        .reserve(&identity("1.0.0"), &"a".repeat(64))
        .unwrap();
    let manager = ExtensionManager::new(db, home.0.clone());
    let rows = manager.list().unwrap();
    assert_eq!(rows[0].state, ExtensionState::Error);
    assert_eq!(rows[0].error_code.as_deref(), Some("interrupted_install"));
    assert_eq!(
        manager.list().unwrap()[0].error_code.as_deref(),
        Some("interrupted_install")
    );
    manager.uninstall(&identity("1.0.0")).unwrap();
    assert!(manager.list().unwrap().is_empty());
}

#[test]
fn removal_recovers_after_quarantine_and_partial_cleanup() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    let manager = ExtensionManager::new(db.clone(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let removal_id = uuid::Uuid::new_v4().to_string();
    super::store::ExtensionStore(db.clone())
        .set_state(&identity("1.0.0"), "removing", None, Some(&removal_id))
        .unwrap();
    let packages =
        crate::adapters::extensions::management::PackageDirectory::lock(&home.0).unwrap();
    packages
        .quarantine("example.demo-1.0.0", &removal_id)
        .unwrap();
    drop(packages);
    let quarantine = home
        .0
        .join("extensions/packages")
        .join(format!(".remove-{removal_id}"));
    fs::remove_file(quarantine.join(".proliferate-package.json")).unwrap();
    drop(manager);
    let manager = ExtensionManager::new(db, home.0.clone());
    assert!(manager.list().unwrap().is_empty());
    assert!(!quarantine.exists());
}

#[test]
fn removal_recovers_after_filesystem_cleanup_but_database_failure() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    let manager = ExtensionManager::new(db.clone(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    db.with_conn(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER fail_extension_delete BEFORE DELETE ON installed_extensions
         BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
    })
    .unwrap();
    assert!(manager.uninstall(&identity("1.0.0")).is_err());
    assert!(!home
        .0
        .join("extensions/packages/example.demo-1.0.0")
        .exists());
    db.with_conn(|conn| conn.execute_batch("DROP TRIGGER fail_extension_delete;"))
        .unwrap();
    assert!(manager.list().unwrap().is_empty());
}

#[test]
fn failed_install_intent_never_publishes_a_package() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    db.with_conn(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER fail_extension_intent BEFORE INSERT ON installed_extensions
         BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
    })
    .unwrap();
    let manager = ExtensionManager::new(db, home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    assert!(manager
        .install(&archive, &identity("1.0.0"), &digest)
        .is_err());
    assert!(manager.list().unwrap().is_empty());
    assert!(!home
        .0
        .join("extensions/packages/example.demo-1.0.0")
        .exists());
}

#[test]
fn removal_intent_recovers_before_quarantine_and_preserves_other_versions() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    let manager = ExtensionManager::new(db.clone(), home.0.clone());
    for version in ["1.0.0", "2.0.0"] {
        let (archive, digest) = archive(&home.0, &manifest(version), "extension/themes/demo.json");
        manager
            .install(&archive, &identity(version), &digest.to_uppercase())
            .unwrap();
    }
    super::store::ExtensionStore(db.clone())
        .set_state(
            &identity("1.0.0"),
            "removing",
            None,
            Some(&uuid::Uuid::new_v4().to_string()),
        )
        .unwrap();
    drop(manager);
    let rows = ExtensionManager::new(db, home.0.clone()).list().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].identity.version, "2.0.0");
    assert_eq!(rows[0].sha256, rows[0].sha256.to_ascii_lowercase());
    assert!(!rows[0].active);
}

#[test]
fn concurrent_managers_publish_and_register_only_once() {
    let home = TestDirectory::new();
    let db = Db::open(&home.0).unwrap();
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            ExtensionManager::new(db.clone(), home.0.clone()).install(
                &archive,
                &identity("1.0.0"),
                &digest,
            )
        });
        let second = scope.spawn(|| {
            ExtensionManager::new(db.clone(), home.0.clone()).install(
                &archive,
                &identity("1.0.0"),
                &digest,
            )
        });
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        ExtensionManager::new(db, home.0.clone())
            .list()
            .unwrap()
            .len(),
        1
    );
}

#[cfg(unix)]
#[test]
fn package_symlink_is_reported_and_never_followed_on_uninstall() {
    use std::os::unix::fs::symlink;
    let home = TestDirectory::new();
    let outside = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let package = home.0.join("extensions/packages/example.demo-1.0.0");
    fs::remove_dir_all(&package).unwrap();
    fs::write(outside.0.join("keep"), b"preserve").unwrap();
    symlink(&outside.0, &package).unwrap();
    assert_eq!(
        manager.list().unwrap()[0].error_code.as_deref(),
        Some("package_invalid")
    );
    assert!(manager.uninstall(&identity("1.0.0")).is_err());
    assert_eq!(fs::read(outside.0.join("keep")).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn contribution_resource_parent_symlink_is_rejected_after_restart() {
    use std::os::unix::fs::symlink;
    let home = TestDirectory::new();
    let outside = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    let (archive, digest) = archive(&home.0, &manifest("1.0.0"), "extension/themes/demo.json");
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let themes = home
        .0
        .join("extensions/packages/example.demo-1.0.0/extension/themes");
    fs::remove_dir_all(&themes).unwrap();
    fs::write(outside.0.join("demo.json"), b"{}").unwrap();
    symlink(&outside.0, &themes).unwrap();
    assert_eq!(
        manager.list().unwrap()[0].error_code.as_deref(),
        Some("package_invalid")
    );
    manager.uninstall(&identity("1.0.0")).unwrap();
    assert!(outside.0.join("demo.json").exists());
}

#[test]
fn syntax_theme_reads_require_registered_valid_packages_and_never_follow_includes() {
    use super::installer_tests::archive_with_content;
    let home = TestDirectory::new();
    let manager = ExtensionManager::new(Db::open(&home.0).unwrap(), home.0.clone());
    assert!(matches!(
        manager.syntax_themes(&identity("1.0.0")),
        Err(super::manager::ManagementError::NotInstalled)
    ));
    let mut raw = manifest("1.0.0");
    raw["contributes"]["themes"][0]["uiTheme"] = serde_json::json!("vs-dark");
    let content = br##"{"colors":{"editor.background":"#101820","editor.foreground":"#FFFFFF"},"tokenColors":[{"scope":"comment","settings":{"foreground":"#AA55CC","fontStyle":"italic"}}],"private":"not exposed"}"##;
    let (archive, digest) =
        archive_with_content(&home.0, &raw, "extension/themes/demo.json", content);
    manager
        .install(&archive, &identity("1.0.0"), &digest)
        .unwrap();
    let (themes, truncated) = manager.syntax_themes(&identity("1.0.0")).unwrap();
    assert!(!truncated);
    assert!(themes[0].theme.available);
    assert_eq!(themes[0].rules[0].scopes, vec!["comment"]);
    assert_eq!(themes[0].rules[0].foreground, "#AA55CC");
    let resource = home
        .0
        .join("extensions/packages/example.demo-1.0.0/extension/themes/demo.json");
    fs::write(&resource, br##"{"include":"../../private.json","colors":{"editor.background":"#101820","editor.foreground":"#FFFFFF"},"tokenColors":[{"scope":"comment","settings":{"foreground":"#AA55CC"}}]}"##).unwrap();
    let (themes, _) = manager.syntax_themes(&identity("1.0.0")).unwrap();
    assert!(!themes[0].theme.available);
    assert!(themes[0].rules.is_empty());
    #[cfg(unix)]
    {
        let outside = home.0.join("outside.json");
        fs::write(&outside, content).unwrap();
        fs::remove_file(&resource).unwrap();
        std::os::unix::fs::symlink(outside, resource).unwrap();
        assert!(manager.syntax_themes(&identity("1.0.0")).is_err());
    }
}
