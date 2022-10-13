use anyhow::{Context, Result};
use enum_iterator::Sequence;
use fs_err as fs;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_reader::ResourceReader;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
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
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "wiiu" | "be" | "wii u" => Ok(Self::WiiU),
            "switch" | "nx" => Ok(Self::Switch),
            _ => anyhow::bail!("Invalid platform"),
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

impl From<uk_content::prelude::Endian> for Platform {
    fn from(e: uk_content::prelude::Endian) -> Self {
        match e {
            uk_content::prelude::Endian::Big => Self::WiiU,
            uk_content::prelude::Endian::Little => Self::Switch,
        }
    }
}

impl From<Platform> for uk_content::prelude::Endian {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Serialize, Deserialize)]
pub enum Language {
    USen,
    EUen,
    USfr,
    USes,
    EUde,
    EUes,
    EUfr,
    EUit,
    EUnl,
    EUru,
    CNzh,
    JPja,
    KRko,
    TWzh,
}

impl Language {
    pub fn to_str(self) -> &'static str {
        self.into()
    }
}

impl From<Language> for &str {
    fn from(lang: Language) -> Self {
        match lang {
            Language::USen => "USen",
            Language::EUen => "EUen",
            Language::USfr => "USfr",
            Language::USes => "USes",
            Language::EUde => "EUde",
            Language::EUes => "EUes",
            Language::EUfr => "EUfr",
            Language::EUit => "EUit",
            Language::EUnl => "EUnl",
            Language::EUru => "EUru",
            Language::CNzh => "CNzh",
            Language::JPja => "JPja",
            Language::KRko => "KRko",
            Language::TWzh => "TWzh",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployConfig {
    pub output: PathBuf,
    pub method: DeployMethod,
    pub auto: bool,
}

impl Default for DeployConfig {
    fn default() -> Self {
        DeployConfig {
            output: "".into(),
            method: DeployMethod::Copy,
            auto: false,
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
    pub fn name(&self) -> &str {
        match self {
            DeployMethod::Copy => "Copy",
            DeployMethod::HardLink => "Hard Links",
            DeployMethod::Symlink => "Symlink",
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSettings {
    pub dark: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self { dark: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub current_mode: Platform,
    pub storage_dir: PathBuf,
    pub unpack_mods: bool,
    pub check_updates: bool,
    pub show_changelog: bool,
    pub wiiu_config: Option<PlatformSettings>,
    pub switch_config: Option<PlatformSettings>,
    #[serde(rename = "ui")]
    pub ui_config: UiSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            current_mode: Platform::WiiU,
            storage_dir: dirs2::data_local_dir().unwrap().join("ukmm"),
            unpack_mods: false,
            wiiu_config: None,
            switch_config: None,
            check_updates: true,
            show_changelog: true,
            ui_config: Default::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Arc<RwLock<Settings>> {
        Arc::new(RwLock::new(match Settings::read(&SETTINGS_PATH) {
            Ok(settings) => {
                log::debug!("{:#?}", settings);
                settings
            }
            Err(e) => {
                log::error!("Failed to read settings file:\n{}", e);
                log::info!("Loading default settings instead");
                Settings::default()
            }
        }))
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
        if !SETTINGS_PATH.parent().unwrap().exists() {
            fs::create_dir_all(SETTINGS_PATH.parent().unwrap())?;
        }
        log::debug!("Saving settings:\n{:#?}", self);
        fs::write(SETTINGS_PATH.as_path(), serde_yaml::to_string(self)?)?;
        log::info!("Settings saved");
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
                entries
                    .filter_map(std::result::Result::ok)
                    .filter_map(|entry| {
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

    #[inline]
    pub fn state_file(&self) -> PathBuf {
        self.storage_dir.join("ui.json")
    }
}

static SETTINGS_PATH: Lazy<PathBuf> = Lazy::new(|| {
    if std::env::args().any(|a| a == "--portable") {
        std::env::current_dir().unwrap().join("settings.yml")
    } else {
        dirs2::config_dir().unwrap().join("ukmm/settings.yml")
    }
});

/*
SAMPLE SETTINGS

current_mode = "WiiU"
storage_dir = "/home/nn/.config/ukmm"
unpack_mods = false
check_updates = true
show_changelog = true

[wiiu_config]
language = "USen"

[wiiu_config.dump]
bin_type = "Nintendo"

[wiiu_config.dump.source]
type = "Unpacked"
host_path = "/games/Cemu/mlc01/usr/title"
content_dir = "/games/Cemu/mlc01/usr/title/00050000/101C9400/content"
update_dir = "/games/Cemu/mlc01/usr/title/0005000E/101C9400/content"
aoc_dir = "/games/Cemu/mlc01/usr/title/0005000C/101C9400/content/0010"

[wiiu_config.deploy_config]
output = "/tmp/BreathOfTheWild_UKMM"
method = "Copy"
auto = false
*/
