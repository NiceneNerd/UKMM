//! Tests that run against a temporary config dir via `Settings::load_from` +
//! `Manager::init_with` — never the real user config.

use uk_manager::{core::Manager, settings::Settings};

fn temp_manager() -> (tempfile::TempDir, Manager) {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings::load_from(&dir.path().join("config").join("settings.yml"));
    let core = Manager::init_with(settings).unwrap();
    (dir, core)
}

#[test]
fn init_with_creates_profile_in_temp_storage() {
    let (dir, core) = temp_manager();
    let storage = dir.path().join("config").join("data");
    assert!(storage.join("wiiu/profiles/Default/profile.yml").exists());
    assert!(core.settings().profiles_dir().starts_with(&storage));
    assert!(core.settings().mods_dir().starts_with(&storage));
}

#[test]
fn settings_save_goes_to_temp_file_not_real_config() {
    let (dir, core) = temp_manager();
    let settings_file = dir.path().join("config").join("settings.yml");
    core.settings_mut().last_version = Some("9.9.9".into());
    core.settings().save().unwrap();
    let reloaded = Settings::read(&settings_file).unwrap();
    assert_eq!(reloaded.last_version.as_deref(), Some("9.9.9"));
    // the internal file path must not leak into the serialized format
    let raw = std::fs::read_to_string(&settings_file).unwrap();
    assert!(!raw.contains("file_path"));
}

#[test]
fn reload_keeps_custom_file_path() {
    let (dir, core) = temp_manager();
    let settings_file = dir.path().join("config").join("settings.yml");
    core.settings_mut().last_version = Some("1.2.3".into());
    core.settings().save().unwrap();
    core.settings_mut().reload();
    assert_eq!(core.settings().last_version.as_deref(), Some("1.2.3"));
    assert_eq!(core.settings().file_path(), settings_file.as_path());
}

/// Without a configured game dump, a remerge must fail with a clear,
/// typed-context error — and must not touch the real config.
#[test]
fn remerge_without_dump_fails_with_clear_error() {
    let (_dir, core) = temp_manager();
    let err = uk_app::apply_changes(&core, vec![], None).unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("No dump available"), "unexpected error: {msg}");
}
