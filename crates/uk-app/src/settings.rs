//! Editable settings drafts: the frontend edits a [`SettingsDraft`], and
//! [`SettingsDraft::validate`] turns it into a core [`Settings`] — validating
//! **all** platform configs before anything is applied (GUI_REF §22.5), with
//! the dump endianness derived from the [`Platform`], never from a path
//! substring (§22.7). The save/wipe rule lives in
//! [`SettingsDraft::wipe_needed`] and the `SaveSettings` command.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow_ext::{Context, Result};
use serde::{Deserialize, Serialize};
use uk_content::{constants::Language, prelude::Endian};
use uk_manager::settings::{DeployConfig, Platform, PlatformSettings, Settings, UpdatePreference};
use uk_reader::ResourceReader;

/// The dump source being edited in a [`PlatformDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DumpDraft {
    Unpacked {
        host_path:   PathBuf,
        content_dir: Option<PathBuf>,
        update_dir:  Option<PathBuf>,
        aoc_dir:     Option<PathBuf>,
    },
    ZArchive {
        content_dir: PathBuf,
        update_dir:  PathBuf,
        aoc_dir:     Option<PathBuf>,
        host_path:   PathBuf,
    },
}

impl DumpDraft {
    pub fn host_path(&self) -> &Path {
        match self {
            DumpDraft::Unpacked { host_path, .. } => host_path.as_path(),
            DumpDraft::ZArchive { host_path, .. } => host_path.as_path(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            DumpDraft::Unpacked {
                content_dir,
                update_dir,
                aoc_dir,
                ..
            } => {
                content_dir
                    .as_ref()
                    .map(|d| d.as_os_str().is_empty())
                    .unwrap_or(true)
                    && update_dir
                        .as_ref()
                        .map(|d| d.as_os_str().is_empty())
                        .unwrap_or(true)
                    && aoc_dir
                        .as_ref()
                        .map(|d| d.as_os_str().is_empty())
                        .unwrap_or(true)
            }
            DumpDraft::ZArchive { host_path, .. } => host_path.as_os_str().is_empty(),
        }
    }
}

impl From<&ResourceReader> for DumpDraft {
    fn from(reader: &ResourceReader) -> Self {
        serde_json::from_str(&reader.source_ser()).expect("Reader source must be a dump draft")
    }
}

/// An editable per-platform configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformDraft {
    pub language: Language,
    pub profile: String,
    pub dump: DumpDraft,
    pub deploy_config: DeployConfig,
}

impl Default for PlatformDraft {
    fn default() -> Self {
        Self {
            language: Language::USen,
            profile: "Default".into(),
            dump: DumpDraft::Unpacked {
                host_path:   Default::default(),
                content_dir: Default::default(),
                update_dir:  Default::default(),
                aoc_dir:     Default::default(),
            },
            deploy_config: Default::default(),
        }
    }
}

/// The dump endianness for a platform. Explicit — never inferred from a path
/// substring (GUI_REF §22.7: the old `content`-in-path hack misdetected Switch
/// dumps under a `content` folder).
pub fn dump_endian(platform: Platform) -> Endian {
    Endian::from(platform)
}

/// Whether two dump drafts point at the same dump (compares the actual
/// dirs/paths, not just the host root).
fn dump_eq(a: &DumpDraft, b: &DumpDraft) -> bool {
    match (a, b) {
        (
            DumpDraft::Unpacked {
                content_dir: c1,
                update_dir: u1,
                aoc_dir: d1,
                ..
            },
            DumpDraft::Unpacked {
                content_dir: c2,
                update_dir: u2,
                aoc_dir: d2,
                ..
            },
        ) => c1 == c2 && u1 == u2 && d1 == d2,
        (DumpDraft::ZArchive { host_path: h1, .. }, DumpDraft::ZArchive { host_path: h2, .. }) => {
            h1 == h2
        }
        _ => false,
    }
}

impl PlatformDraft {
    /// Whether this draft is a real change relative to the saved config. A
    /// draft created for a platform with no saved config only counts as a
    /// change when it actually points at a dump or an output folder (matches
    /// the old `(None, Some(_)) => !dump.is_empty() || !output.is_empty()`
    /// rule).
    pub fn differs_from(&self, saved: Option<&PlatformSettings>) -> bool {
        match saved {
            None => !self.dump.is_empty() || !self.deploy_config.output.as_os_str().is_empty(),
            Some(saved) => {
                self.language != saved.language
                    || match (
                        &saved.deploy_config,
                        self.deploy_config.output.as_os_str().is_empty(),
                    ) {
                        (None, true) => false,
                        (None, false) => true,
                        (Some(d), _) => d != &self.deploy_config,
                    }
                    || !dump_eq(&self.dump, &DumpDraft::from(saved.dump.as_ref()))
            }
        }
    }

    /// Validates the draft into a core [`PlatformSettings`]. The dump endianness
    /// comes from `platform` — never inferred from a path substring
    /// (GUI_REF §22.7).
    pub fn validate(self, platform: Platform) -> Result<PlatformSettings> {
        let dump = match self.dump {
            DumpDraft::Unpacked {
                content_dir,
                update_dir,
                aoc_dir,
                ..
            } => {
                let endian = dump_endian(platform);
                Arc::new(ResourceReader::from_unpacked_dirs(
                    content_dir,
                    update_dir,
                    aoc_dir,
                    endian,
                )?)
            }
            DumpDraft::ZArchive { host_path, .. } => {
                Arc::new(ResourceReader::from_zarchive(host_path)?)
            }
        };
        Ok(PlatformSettings {
            language: self.language,
            profile: self.profile.into(),
            dump,
            deploy_config: if self.deploy_config.output.as_os_str().is_empty() {
                None
            } else {
                Some(self.deploy_config)
            },
        })
    }
}

impl From<&PlatformSettings> for PlatformDraft {
    fn from(settings: &PlatformSettings) -> Self {
        Self {
            language: settings.language,
            profile: settings.profile.to_string(),
            dump: (&*settings.dump).into(),
            deploy_config: settings.deploy_config.as_ref().cloned().unwrap_or_default(),
        }
    }
}

/// An editable copy of the core [`Settings`]. UI language is deliberately not
/// part of it: it is frontend state (persisted per frontend, format v2 of
/// `settings.yml` drops it from the core file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsDraft {
    pub current_mode: Platform,
    pub system_7z: bool,
    pub storage_dir: PathBuf,
    pub check_updates: UpdatePreference,
    pub show_changelog: bool,
    pub wiiu: Option<PlatformDraft>,
    pub switch: Option<PlatformDraft>,
}

impl SettingsDraft {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            current_mode: settings.current_mode,
            system_7z: settings.system_7z,
            storage_dir: settings.storage_dir.clone(),
            check_updates: settings.check_updates.clone(),
            show_changelog: settings.show_changelog,
            wiiu: settings.wiiu_config.as_ref().map(PlatformDraft::from),
            switch: settings.switch_config.as_ref().map(PlatformDraft::from),
        }
    }

    /// The draft for `platform`, creating a default one if absent (the settings
    /// tab renders both platform sections even when unconfigured).
    pub fn platform_draft_mut(&mut self, platform: Platform) -> &mut PlatformDraft {
        let slot = match platform {
            Platform::WiiU => &mut self.wiiu,
            Platform::Switch => &mut self.switch,
        };
        slot.get_or_insert_with(PlatformDraft::default)
    }

    /// Whether anything in the draft differs from the saved settings.
    pub fn differs_from(&self, saved: &Settings) -> bool {
        fn platform_differs(
            draft: &Option<PlatformDraft>,
            saved: Option<&PlatformSettings>,
        ) -> bool {
            match (draft, saved) {
                (Some(d), s) => d.differs_from(s),
                // Removing a saved config counts only when it was non-empty
                // (mirrors the old changed-check).
                (None, Some(s)) => {
                    !DumpDraft::from(s.dump.as_ref()).is_empty()
                        || s.deploy_config
                            .as_ref()
                            .is_some_and(|d| !d.output.as_os_str().is_empty())
                }
                (None, None) => false,
            }
        }
        self.current_mode != saved.current_mode
            || self.system_7z != saved.system_7z
            || self.storage_dir != saved.storage_dir
            || self.check_updates != saved.check_updates
            || self.show_changelog != saved.show_changelog
            || platform_differs(&self.wiiu, saved.wiiu_config.as_ref())
            || platform_differs(&self.switch, saved.switch_config.as_ref())
    }

    /// Validates **both** platform drafts before building anything, so a bad
    /// platform config can never leave a half-applied settings file
    /// (GUI_REF §22.5). Unchanged platforms keep their saved config as-is
    /// (an untouched default draft for an unconfigured platform stays `None`).
    /// Scalar fields and the file path come from `old`.
    pub fn validate(self, old: &Settings) -> Result<Settings> {
        fn platform_into(
            draft: Option<PlatformDraft>,
            saved: Option<&PlatformSettings>,
            platform: Platform,
            label: &str,
        ) -> Result<Option<PlatformSettings>> {
            match draft {
                None => Ok(saved.cloned()),
                Some(d) if !d.differs_from(saved) => Ok(saved.cloned()),
                Some(d) => {
                    d.validate(platform)
                        .map(Some)
                        .with_context(|| format!("Invalid {label} configuration"))
                }
            }
        }
        let wiiu = platform_into(self.wiiu, old.wiiu_config.as_ref(), Platform::WiiU, "Wii U")?;
        let switch = platform_into(
            self.switch,
            old.switch_config.as_ref(),
            Platform::Switch,
            "Switch",
        )?;
        let mut settings = old.clone();
        settings.current_mode = self.current_mode;
        settings.system_7z = self.system_7z;
        settings.storage_dir = self.storage_dir;
        settings.check_updates = self.check_updates;
        settings.show_changelog = self.show_changelog;
        settings.wiiu_config = wiiu;
        settings.switch_config = switch;
        Ok(settings)
    }

    /// Whether saving this draft must wipe the old deploy output: the current
    /// platform's deploy layout/method/output changed (GUI_REF §22.37's wipe
    /// rule, computed here so every frontend gets it).
    pub fn wipe_needed(&self, old: &Settings) -> bool {
        let old_config = match self.current_mode {
            Platform::WiiU => old.wiiu_config.as_ref(),
            Platform::Switch => old.switch_config.as_ref(),
        };
        let old_dep = old_config.and_then(|c| c.deploy_config.as_ref());
        let new_draft = match self.current_mode {
            Platform::WiiU => self.wiiu.as_ref(),
            Platform::Switch => self.switch.as_ref(),
        };
        match (old_dep, new_draft) {
            (Some(old), Some(new)) => {
                let new = &new.deploy_config;
                old.layout != new.layout || old.method != new.method || old.output != new.output
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings(dir: &tempfile::TempDir) -> Settings {
        Settings::load_from(&dir.path().join("settings.yml"))
            .read()
            .clone()
    }

    /// Creates a fake unpacked dump (the reader checks these files exist).
    fn fake_dump(root: &Path) -> PathBuf {
        let content = root.join("content");
        std::fs::create_dir_all(content.join("Map/MainField/A-1")).unwrap();
        std::fs::write(
            content.join("Map/MainField/A-1/A-1.00_Clustering.sblwp"),
            b"",
        )
        .unwrap();
        content
    }

    #[test]
    fn endian_comes_from_platform_not_path() {
        // GUI_REF §22.7: a "content"-named folder must not imply Big endian.
        assert_eq!(dump_endian(Platform::WiiU), Endian::Big);
        assert_eq!(dump_endian(Platform::Switch), Endian::Little);
        // And validate() accepts a dump under a "content" folder for Switch
        // (the old substring hack would have picked Big here).
        let dir = tempfile::tempdir().unwrap();
        let content = fake_dump(dir.path());
        let mut draft = PlatformDraft::default();
        draft.dump = DumpDraft::Unpacked {
            host_path:   "/".into(),
            content_dir: Some(content),
            update_dir:  None,
            aoc_dir:     None,
        };
        assert!(draft.validate(Platform::Switch).is_ok());
    }

    #[test]
    fn validate_checks_all_platforms_before_applying() {
        let dir = tempfile::tempdir().unwrap();
        let old = test_settings(&dir);
        let mut draft = SettingsDraft::from_settings(&old);
        // Broken Switch config (ZArchive pointing nowhere) + valid scalar edit.
        draft.switch = Some(PlatformDraft {
            dump: DumpDraft::ZArchive {
                content_dir: Default::default(),
                update_dir:  Default::default(),
                aoc_dir:     None,
                host_path:   dir.path().join("missing.wua"),
            },
            ..Default::default()
        });
        draft.system_7z = !old.system_7z;
        // Must fail, and must not produce a half-validated Settings.
        assert!(draft.validate(&old).is_err());
    }

    #[test]
    fn validate_rejects_missing_base_dir() {
        let dir = tempfile::tempdir().unwrap();
        let old = test_settings(&dir);
        let mut draft = SettingsDraft::from_settings(&old);
        // A dump draft pointing at a nonexistent folder must fail validation.
        draft.wiiu = Some(PlatformDraft {
            dump: DumpDraft::Unpacked {
                host_path:   "/".into(),
                content_dir: Some(dir.path().join("no-such-dump")),
                update_dir:  None,
                aoc_dir:     None,
            },
            ..Default::default()
        });
        assert!(draft.validate(&old).is_err());
    }

    #[test]
    fn untouched_default_draft_stays_none() {
        let dir = tempfile::tempdir().unwrap();
        let old = test_settings(&dir);
        let mut draft = SettingsDraft::from_settings(&old);
        // Rendering the platform section materializes a default draft; an
        // untouched one must not be validated into an error (or saved).
        let _ = draft.platform_draft_mut(Platform::WiiU);
        let validated = draft.clone().validate(&old).unwrap();
        assert!(validated.wiiu_config.is_none());
        assert!(!draft.differs_from(&old));
    }

    #[test]
    fn wipe_needed_only_on_deploy_change() {
        let dir = tempfile::tempdir().unwrap();
        let old = test_settings(&dir);
        let mut draft = SettingsDraft::from_settings(&old);
        assert!(!draft.wipe_needed(&old));
        draft.storage_dir = dir.path().join("elsewhere");
        assert!(
            !draft.wipe_needed(&old),
            "storage change alone must not wipe"
        );
    }

    #[test]
    fn differs_from_matches_saved() {
        let dir = tempfile::tempdir().unwrap();
        let old = test_settings(&dir);
        let draft = SettingsDraft::from_settings(&old);
        assert!(!draft.differs_from(&old));
        let mut changed = draft.clone();
        changed.show_changelog = !old.show_changelog;
        assert!(changed.differs_from(&old));
    }
}
