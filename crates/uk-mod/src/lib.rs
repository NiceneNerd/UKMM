#![feature(result_option_inspect, seek_stream_len, let_chains, lazy_cell)]
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow_ext::Context;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{
    prelude::Endian,
    util::{HashSet, IndexMap},
};
pub mod pack;
pub mod unpack;
pub use zstd;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "content")]
    pub content_files: BTreeSet<String>,
    #[serde(rename = "aoc")]
    pub aoc_files:     BTreeSet<String>,
}

impl Manifest {
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

pub static CATEGORIES: &[&str] = &[
    "Animations",
    "Balance",
    "Crafting",
    "Customization",
    "Difficulty",
    "Enemies",
    "Expansion",
    "Items",
    "Meme/Gimmick",
    "Other",
    "Overhaul",
    "Overworld",
    "Player",
    "QoL",
    "Quest",
    "Shrine",
    "Skin/Texture",
];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ModPlatform {
    Specific(Endian),
    Universal,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub version: String,
    pub author: String,
    pub category: String,
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
                name: "Test Mod".into(),
                description: "A sample UKMM mod".into(),
                category: "Other".into(),
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
