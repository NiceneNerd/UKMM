#![allow(clippy::unwrap_used)]

use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use anyhow_ext::{Context, Result};
use fs_err as fs;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_with::{DefaultOnError, serde_as};
use smartstring::alias::String;
use uk_content::{constants::Language, prelude::Endian};
use uk_localization::LocLang;
use uk_reader::ResourceReader;

use crate::util;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    #[default]
    WiiU,
    Switch,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WiiU => "Wii U",
            Self::Switch => "Switch",
        }
        .fmt(f)
    }
}

impl std::str::FromStr for Platform {
    type Err = anyhow_ext::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "wiiu" | "be" | "wii u" => Ok(Self::WiiU),
            "switch" | "nx" => Ok(Self::Switch),
            _ => anyhow_ext::bail!("Invalid platform"),
        }
    }
}

impl From<roead::Endian> for Platform {
    fn from(e: roead::Endian) -> Self {
        match e {
            roead::Endian::Big => Self::WiiU,
            roead::Endian::Little => Self::Switch,
        }
    }
}

impl From<Platform> for roead::Endian {
    fn from(p: Platform) -> Self {
        match p {
            Platform::WiiU => Self::Big,
            Platform::Switch => Self::Little,
        }
    }
}

impl From<Endian> for Platform {
    fn from(e: Endian) -> Self {
        match e {
            Endian::Big => Self::WiiU,
            Endian::Little => Self::Switch,
        }
    }
}

impl From<Platform> for Endian {
    fn from(p: Platform) -> Self {
        match p {
            Platform::WiiU => Self::Big,
            Platform::Switch => Self::Little,
        }
    }
}

impl From<rstb::Endian> for Platform {
    fn from(e: rstb::Endian) -> Self {
        match e {
            rstb::Endian::Big => Self::WiiU,
            rstb::Endian::Little => Self::Switch,
        }
    }
}

impl From<Platform> for rstb::Endian {
    fn from(p: Platform) -> Self {
        match p {
            Platform::WiiU => Self::Big,
            Platform::Switch => Self::Little,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployConfig {
    pub output: PathBuf,
    pub method: DeployMethod,
    pub auto: bool,
    #[serde(default)]
    pub cemu_rules: bool,
    #[serde(default)]
    pub executable: Option<std::string::String>,
    #[serde(default)]
    pub layout: DeployLayout,
}

impl DeployConfig {
    pub fn final_output_paths(&self, endian: Endian) -> (PathBuf, PathBuf) {
        match endian {
            Endian::Little => {
                match self.layout {
                    DeployLayout::WithoutName => {
                        (
                            self.output.join("01007EF00011E000").join("romfs"),
                            self.output.join("01007EF00011F001").join("romfs"),
                        )
                    }
                    DeployLayout::WithName => {
                        (
                            self.output
                                .join("01007EF00011E000")
                                .join("BreathOfTheWild_UKMM")
                                .join("romfs"),
                            self.output
                                .join("01007EF00011F001")
                                .join("BreathOfTheWild_UKMM")
                                .join("romfs"),
                        )
                    }
                }
            }
            Endian::Big => {
                match self.layout {
                    DeployLayout::WithoutName => {
                        (
                            self.output.join("content"),
                            self.output.join("aoc").join("0010"),
                        )
                    }
                    DeployLayout::WithName => {
                        (
                            self.output.join("BreathOfTheWild_UKMM").join("content"),
                            self.output
                                .join("BreathOfTheWild_UKMM")
                                .join("aoc")
                                .join("0010"),
                        )
                    }
                }
            }
        }
    }
}

impl Default for DeployConfig {
    fn default() -> Self {
        DeployConfig {
            output: "".into(),
            method: DeployMethod::Copy,
            auto: false,
            cemu_rules: false,
            executable: None,
            layout: DeployLayout::WithoutName,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployMethod {
    Copy,
    HardLink,
    Symlink,
}

impl DeployMethod {
    #[inline(always)]
    pub fn name(&self) -> &'static str {
        match self {
            DeployMethod::Copy => "Settings_Platform_Deploy_Method_Copy",
            DeployMethod::HardLink => "Settings_Platform_Deploy_Method_HardLink",
            DeployMethod::Symlink => "Settings_Platform_Deploy_Method_Symlink",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeployLayout {
    #[default]
    WithoutName,
    WithName,
}

impl DeployLayout {
    #[inline(always)]
    pub fn name(&self) -> &str {
        match self {
            DeployLayout::WithoutName => "SD Card",
            DeployLayout::WithName => "Emulator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformSettings {
    pub language: Language,
    pub profile: String,
    pub dump: Arc<ResourceReader>,
    pub deploy_config: Option<DeployConfig>,
}

#[inline]
fn default_storage() -> PathBuf {
    if std::env::args().any(|a| a == "--portable") {
        std::env::current_exe().unwrap().with_file_name("data")
    } else {
        dirs2::data_local_dir().unwrap().join("ukmm")
    }
}

#[inline]
fn default_file_path() -> PathBuf {
    Settings::config_dir().join("settings.yml")
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdatePreference {
    None,
    #[default]
    Stable,
    Beta,
}

/// Current `settings.yml` format version. v1 had the UI language (`lang`)
/// inside; v2 moves it to the per-frontend state file and omits it here.
pub const FORMAT_VERSION: u32 = 2;

#[inline(always)]
fn default_format_version() -> u32 {
    1
}

#[inline(always)]
fn default_lang() -> LocLang {
    LocLang::English
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[serde_as]
pub struct Settings {
    /// Format version of `settings.yml`. Old files (no field) read as 1;
    /// saves always write [`FORMAT_VERSION`].
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub current_mode: Platform,
    pub system_7z: bool,
    #[serde(default = "default_storage")]
    pub storage_dir: PathBuf,
    #[serde(deserialize_with = "serde_with::As::<DefaultOnError>::deserialize")]
    pub check_updates: UpdatePreference,
    pub show_changelog: bool,
    pub last_version: Option<String>,
    pub wiiu_config: Option<PlatformSettings>,
    pub switch_config: Option<PlatformSettings>,
    /// Legacy v1 field: the UI language used to live in `settings.yml`. Still
    /// read from old files (frontends seed their state file from it once),
    /// never written — v2+ keeps language in the frontend state file.
    #[serde(default = "default_lang", skip_serializing)]
    pub lang: LocLang,
    /// The file this instance is loaded from and saved to. Not serialized;
    /// defaults to [`Settings::path`]. Use [`Settings::load_from`] to point a
    /// settings instance at a different file (e.g. a temp dir in tests).
    #[serde(skip, default = "default_file_path")]
    file_path: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            current_mode: Platform::WiiU,
            system_7z: true,
            storage_dir: default_storage(),
            wiiu_config: None,
            switch_config: None,
            check_updates: UpdatePreference::Stable,
            show_changelog: true,
            last_version: None,
            lang: LocLang::English,
            file_path: default_file_path(),
        }
    }
}

impl Settings {
    pub fn path() -> &'static Path {
        static PATH: LazyLock<PathBuf> =
            LazyLock::new(|| Settings::config_dir().join("settings.yml"));
        PATH.as_path()
    }

    pub fn config_dir() -> &'static Path {
        static PATH: LazyLock<PathBuf> = LazyLock::new(|| {
            if std::env::args().any(|a| a == "--portable") {
                std::env::current_exe()
                    .expect("No current executable???")
                    .parent()
                    .expect("Executable has no parent???")
                    .join("config")
            } else {
                dirs2::config_dir().unwrap().join("ukmm")
            }
        });
        PATH.as_path()
    }

    /// The file this settings instance was loaded from and saves to.
    #[inline]
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn load() -> Arc<RwLock<Settings>> {
        Self::load_from(Self::path())
    }

    /// Loads settings from `path` instead of the default config location. If
    /// the file is missing or invalid, defaults are used with `storage_dir`
    /// redirected next to `path`, so loading a custom config (e.g. a temp dir
    /// in tests) never reads or writes the real user directories.
    pub fn load_from(path: &Path) -> Arc<RwLock<Settings>> {
        let mut settings = match Settings::read(path) {
            Ok(settings) => settings,
            Err(e) => {
                log::error!("Failed to read settings file:\n{}", e);
                log::info!("Loading default settings instead");
                let mut settings = Settings::default();
                settings.storage_dir = path
                    .parent()
                    .map(|p| p.join("data"))
                    .unwrap_or_else(default_storage);
                settings
            }
        };
        settings.file_path = path.to_path_buf();
        log::debug!("{:#?}", settings);
        util::USE_SZ.store(settings.system_7z, std::sync::atomic::Ordering::Release);
        Arc::new(RwLock::new(settings))
    }

    pub fn reload(&mut self) {
        let file_path = self.file_path.clone();
        *self = match Settings::read(&file_path) {
            Ok(settings) => {
                log::debug!("{:#?}", settings);
                settings
            }
            Err(e) => {
                log::error!("Failed to read settings file:\n{}", e);
                log::info!("Loading default settings instead");
                Settings::default()
            }
        };
        self.file_path = file_path;
    }

    pub fn read(path: &Path) -> Result<Self> {
        Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn apply(&mut self, apply_fn: impl Fn(&mut Self)) -> Result<()> {
        apply_fn(self);
        self.save().context("Failed to save settings file")?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        if !self.file_path.parent().unwrap().exists() {
            fs::create_dir_all(self.file_path.parent().unwrap())?;
        }
        log::debug!("Saving settings:\n{:#?}", self);
        let _ = util::USE_SZ.compare_exchange_weak(
            !self.system_7z,
            self.system_7z,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
        // Always write the current format version (v2 drops the legacy `lang`).
        let mut serialized = self.clone();
        serialized.format_version = FORMAT_VERSION;
        fs::write(&self.file_path, serde_yaml::to_string(&serialized)?)?;
        log::info!("Settings saved");
        Ok(())
    }

    pub fn wipe_output(&self, endian: Endian) -> Result<()> {
        let (content, aoc) = match endian {
            Endian::Big => {
                self.wiiu_config
                    .as_ref()
                    .unwrap()
                    .deploy_config
                    .as_ref()
                    .unwrap()
                    .final_output_paths(endian)
            }
            Endian::Little => {
                self.switch_config
                    .as_ref()
                    .unwrap()
                    .deploy_config
                    .as_ref()
                    .unwrap()
                    .final_output_paths(endian)
            }
        };
        if util::is_symlink(content.as_ref()) {
            util::remove_symlink(content)?;
        } else if content.exists() {
            util::remove_dir_all(content)?;
        }
        if util::is_symlink(aoc.as_ref()) {
            util::remove_symlink(aoc)?;
        } else if aoc.exists() {
            util::remove_dir_all(aoc)?;
        }
        Ok(())
    }

    #[inline]
    pub fn platform_dir(&self) -> PathBuf {
        self.get_platform_dir(self.current_mode)
    }

    #[inline]
    pub fn get_platform_dir(&self, platform: Platform) -> PathBuf {
        match platform {
            Platform::Switch => self.storage_dir.join("nx"),
            Platform::WiiU => self.storage_dir.join("wiiu"),
        }
    }

    #[inline]
    pub fn profiles_dir(&self) -> PathBuf {
        self.platform_dir().join("profiles")
    }

    #[inline]
    pub fn profile_dir(&self) -> PathBuf {
        self.platform_dir().join("profiles").join(
            self.platform_config()
                .map(|c| c.profile.as_str())
                .unwrap_or("Default"),
        )
    }

    #[inline]
    pub fn profiles(&self) -> impl Iterator<Item = String> {
        fs::read_dir(self.profiles_dir())
            .into_iter()
            .flat_map(|entries| {
                entries.filter_map(Result::ok).filter_map(|entry| {
                    entry
                        .path()
                        .is_dir()
                        .then(|| entry.file_name().to_string_lossy().into())
                })
            })
    }

    #[inline]
    pub fn mods_dir(&self) -> PathBuf {
        self.platform_dir().join("mods")
    }

    #[inline]
    pub fn dump(&self) -> Option<Arc<ResourceReader>> {
        match self.current_mode {
            Platform::Switch => self.switch_config.as_ref().map(|c| c.dump.clone()),
            Platform::WiiU => self.wiiu_config.as_ref().map(|c| c.dump.clone()),
        }
    }

    #[inline(always)]
    pub fn platform_config(&self) -> Option<&PlatformSettings> {
        match self.current_mode {
            Platform::Switch => self.switch_config.as_ref(),
            Platform::WiiU => self.wiiu_config.as_ref(),
        }
    }

    #[inline(always)]
    pub fn platform_config_mut(&mut self) -> Option<&mut PlatformSettings> {
        match self.current_mode {
            Platform::Switch => self.switch_config.as_mut(),
            Platform::WiiU => self.wiiu_config.as_mut(),
        }
    }

    #[inline]
    pub fn merged_dir(&self) -> PathBuf {
        self.profile_dir().join("merged")
    }

    #[inline]
    pub fn deploy_dir(&self) -> Option<&Path> {
        let config = self.platform_config();
        config
            .and_then(|c| c.deploy_config.as_ref())
            .map(|c| c.output.as_ref())
    }

    /// The per-frontend UI state file (`ui-<frontend>.json`). Each frontend
    /// keeps its own theme/layout/language here, so two frontends never
    /// overwrite each other.
    #[inline]
    pub fn frontend_state_file(frontend: &str) -> PathBuf {
        Self::config_dir().join(format!("ui-{frontend}.json"))
    }

    /// The legacy shared UI state file (`ui.json`, pre per-frontend files).
    /// Read once for migration, then ignored.
    #[inline]
    pub fn legacy_state_file() -> PathBuf {
        Self::config_dir().join("ui.json")
    }

    #[inline]
    pub fn projects_dir(&self) -> PathBuf {
        self.storage_dir.join("projects")
    }
}
