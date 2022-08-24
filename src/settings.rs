use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uk_reader::ResourceReader;

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DeployConfig {
    pub output: PathBuf,
    pub method: DeployMethod,
    pub auto: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeployMethod {
    Copy,
    HardLink,
    Symlink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformSettings {
    pub dump: ResourceReader,
    pub deploy_config: Option<DeployConfig>,
    pub language: Language,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub storage_dir: PathBuf,
    pub wiiu_config: Option<PlatformSettings>,
    pub switch_config: Option<PlatformSettings>,
    pub check_updates: bool,
    pub show_changelog: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            storage_dir: dirs2::config_dir().unwrap(),
            wiiu_config: None,
            switch_config: None,
            check_updates: true,
            show_changelog: true,
        }
    }
}
