#![allow(unstable_name_collisions)]
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow_ext::Context;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{
    constants::Language,
    prelude::Endian,
    util::{HashSet, IndexMap},
};
pub mod pack;
pub mod unpack;
pub use zstd;

static DICTIONARY: &[u8] = include_bytes!("../data/zsdic");

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "content")]
    pub content_files: BTreeSet<String>,
    #[serde(rename = "aoc")]
    pub aoc_files:     BTreeSet<String>,
}

impl Manifest {
    pub fn languages(&self) -> Vec<Language> {
        self.content_files
            .iter()
            .filter_map(|file| Language::from_path(Path::new(file.as_str())))
            .collect::<Vec<_>>()
    }

    pub fn resources(&self) -> impl Iterator<Item = String> + '_ {
        self.content_files
            .iter()
            .map(|s| s.replace(".s", ".").into())
            .chain(
                self.aoc_files
                    .iter()
                    .map(|s| ["Aoc/0010/", &s.replace(".s", ".")].join("").into()),
            )
    }

    pub fn extend(&mut self, other: &Manifest) {
        self.content_files
            .extend(other.content_files.iter().cloned());
        self.aoc_files.extend(other.aoc_files.iter().cloned());
    }

    pub fn clear(&mut self) {
        self.content_files.clear();
        self.aoc_files.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.content_files.is_empty() && self.aoc_files.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModOption {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub requires: Vec<PathBuf>,
}

impl ModOption {
    #[inline(always)]
    pub fn manifest_path(&self) -> PathBuf {
        Path::new("options").join(&self.path).join("manifest.yml")
    }
}

#[enum_dispatch::enum_dispatch(OptionGroup)]
pub trait ModOptionGroup {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn options(&self) -> &Vec<ModOption>;
    fn name_mut(&mut self) -> &mut String;
    fn description_mut(&mut self) -> &mut String;
    fn options_mut(&mut self) -> &mut Vec<ModOption>;
    fn required(&self) -> bool;
    fn required_mut(&mut self) -> &mut bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusiveOptionGroup {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<PathBuf>,
    pub options: Vec<ModOption>,
}

impl ModOptionGroup for ExclusiveOptionGroup {
    #[inline(always)]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline(always)]
    fn description(&self) -> &str {
        &self.description
    }

    #[inline(always)]
    fn options(&self) -> &Vec<ModOption> {
        &self.options
    }

    #[inline(always)]
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    #[inline(always)]
    fn description_mut(&mut self) -> &mut String {
        &mut self.description
    }

    #[inline(always)]
    fn options_mut(&mut self) -> &mut Vec<ModOption> {
        &mut self.options
    }

    #[inline(always)]
    fn required(&self) -> bool {
        self.required
    }

    #[inline(always)]
    fn required_mut(&mut self) -> &mut bool {
        &mut self.required
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipleOptionGroup {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub defaults: HashSet<PathBuf>,
    pub options: Vec<ModOption>,
}

impl ModOptionGroup for MultipleOptionGroup {
    #[inline(always)]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline(always)]
    fn description(&self) -> &str {
        &self.description
    }

    #[inline(always)]
    fn options(&self) -> &Vec<ModOption> {
        &self.options
    }

    #[inline(always)]
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    #[inline(always)]
    fn description_mut(&mut self) -> &mut String {
        &mut self.description
    }

    #[inline(always)]
    fn options_mut(&mut self) -> &mut Vec<ModOption> {
        &mut self.options
    }

    #[inline(always)]
    fn required(&self) -> bool {
        self.required
    }

    #[inline(always)]
    fn required_mut(&mut self) -> &mut bool {
        &mut self.required
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[enum_dispatch::enum_dispatch]
#[serde(tag = "type")]
pub enum OptionGroup {
    Exclusive(ExclusiveOptionGroup),
    Multiple(MultipleOptionGroup),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModCategory {
    #[serde(alias = "")]
    None,
    Animations,
    Balance,
    Crafting,
    Customization,
    Difficulty,
    Enemies,
    Expansion,
    Items,
    #[serde(alias = "Meme/Gimmick")]
    Meme,
    Other,
    Overhaul,
    Overworld,
    Player,
    QoL,
    Quest,
    Shrine,
    #[serde(alias = "Skin/Texture")]
    Texture
}

impl ModCategory {
    pub fn iter() -> std::slice::Iter<'static, Self> {
        [
            Self::Animations,
            Self::Balance,
            Self::Crafting,
            Self::Customization,
            Self::Difficulty,
            Self::Enemies,
            Self::Expansion,
            Self::Items,
            Self::Meme,
            Self::Other,
            Self::Overhaul,
            Self::Overworld,
            Self::Player,
            Self::QoL,
            Self::Quest,
            Self::Shrine,
            Self::Texture,
            Self::None,
        ]
        .iter()
    }

    pub fn to_loc_str(&self) -> &'static str {
        match self {
            ModCategory::None => "Mod_Category_None",
            ModCategory::Animations => "Mod_Category_Anim",
            ModCategory::Balance => "Mod_Category_Balance",
            ModCategory::Crafting => "Mod_Category_Crafting",
            ModCategory::Customization => "Mod_Category_Customization",
            ModCategory::Difficulty => "Mod_Category_Difficulty",
            ModCategory::Enemies => "Mod_Category_Enemies",
            ModCategory::Expansion => "Mod_Category_Expansion",
            ModCategory::Items => "Mod_Category_Items",
            ModCategory::Meme => "Mod_Category_Meme",
            ModCategory::Other => "Mod_Category_Other",
            ModCategory::Overhaul => "Mod_Category_Overhaul",
            ModCategory::Overworld => "Mod_Category_Overworld",
            ModCategory::Player => "Mod_Category_Player",
            ModCategory::QoL => "Mod_Category_QoL",
            ModCategory::Quest => "Mod_Category_Quest",
            ModCategory::Shrine => "Mod_Category_Shrine",
            ModCategory::Texture => "Mod_Category_Texture",
        }
    }

    #[inline(always)]
    pub fn to_str(&self) -> &'static str {
        <&'static str>::from(*self)
    }

    #[inline(always)]
    pub fn u8(&self) -> u8 {
        *self as u8
    }
}

impl From<ModCategory> for &'static str {
    fn from(value: ModCategory) -> Self {
        match value {
            ModCategory::None => "None",
            ModCategory::Animations => "Animations",
            ModCategory::Balance => "Balance",
            ModCategory::Crafting => "Crafting",
            ModCategory::Customization => "Customization",
            ModCategory::Difficulty => "Difficulty",
            ModCategory::Enemies => "Enemies",
            ModCategory::Expansion => "Expansion",
            ModCategory::Items => "Items",
            ModCategory::Meme => "Meme/Gimmick",
            ModCategory::Other => "Other",
            ModCategory::Overhaul => "Overhaul",
            ModCategory::Overworld => "Overworld",
            ModCategory::Player => "Player",
            ModCategory::QoL => "QoL",
            ModCategory::Quest => "Quest",
            ModCategory::Shrine => "Shrine",
            ModCategory::Texture => "Skin/Texture",
        }
    }
}

impl From<&str> for ModCategory {
    fn from(value: &str) -> Self {
        match value {
            "Animations" => Self::Animations,
            "Balance" => Self::Balance,
            "Crafting" => Self::Crafting,
            "Customization" => Self::Customization,
            "Difficulty" => Self::Difficulty,
            "Enemies" => Self::Enemies,
            "Expansion" => Self::Expansion,
            "Items" => Self::Items,
            "Meme/Gimmick" => Self::Meme,
            "Other" => Self::Other,
            "Overhaul" => Self::Overhaul,
            "Overworld" => Self::Overworld,
            "Player" => Self::Player,
            "QoL" => Self::QoL,
            "Quest" => Self::Quest,
            "Shrine" => Self::Shrine,
            "Skin/Texture" => Self::Texture,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ModPlatform {
    Specific(Endian),
    Universal,
}

impl std::fmt::Display for ModPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModPlatform::Specific(Endian::Big) => "Wii U".fmt(f),
            ModPlatform::Specific(Endian::Little) => "Switch".fmt(f),
            ModPlatform::Universal => "any platform".fmt(f),
        }
    }
}

#[inline(always)]
fn default_api() -> String {
    env!("CARGO_PKG_VERSION").into()
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default = "default_api")]
    pub api: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub category: ModCategory,
    pub description: String,
    pub platform: ModPlatform,
    pub url: Option<String>,
    #[serde(rename = "option_groups")]
    pub options: Vec<OptionGroup>,
    pub masters: IndexMap<usize, (String, String)>,
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl std::hash::Hash for Meta {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.version.hash(state);
        self.author.hash(state);
        self.platform.hash(state);
        self.description.hash(state);
    }
}

impl Meta {
    pub fn from_mod(mod_path: impl AsRef<Path>) -> anyhow_ext::Result<Self> {
        use std::io::Read;
        let mod_path = mod_path.as_ref();
        let mut zip = zip::ZipArchive::new(std::io::BufReader::new(fs_err::File::open(mod_path)?))?;
        let mut meta = zip.by_name("meta.yml").context("Mod missing meta file")?;
        let mut buffer = vec![0; meta.size() as usize];
        let read = meta.read(&mut buffer)?;
        if read != meta.size() as usize {
            anyhow_ext::bail!("Failed to read meta file")
        } else {
            Ok(serde_yaml::from_slice(&buffer).context("Failed to parse meta file")?)
        }
    }

    #[inline(always)]
    pub fn parse(file: impl AsRef<Path>) -> anyhow_ext::Result<Self> {
        fs_err::read_to_string(file.as_ref())
            .context("Failed to read meta file")
            .and_then(|s| serde_yaml::from_str(&s).context("Failed to parse meta file"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_meta() {
        dbg!(Meta::from_mod("test/wiiu.zip").unwrap());
    }

    #[test]
    fn create_meta() {
        println!(
            "{}",
            serde_yaml::to_string(&Meta {
                api: env!("CARGO_PKG_VERSION").into(),
                name: "Test Mod".into(),
                description: "A sample UKMM mod".into(),
                category: ModCategory::Other,
                author: "Nicene Nerd".into(),
                platform: ModPlatform::Universal,
                url: None,
                version: "1.0.0".into(),
                masters: Default::default(),
                options: Default::default(),
            })
            .unwrap()
        );
    }
}
