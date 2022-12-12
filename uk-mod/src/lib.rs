#![feature(seek_stream_len, let_chains)]
use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{prelude::Endian, util::IndexMap};
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusiveOptionGroup {
    pub name: String,
    pub description: String,
    pub default: Option<PathBuf>,
    pub options: Vec<ModOption>,
}

impl ModOptionGroup for ExclusiveOptionGroup {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn options(&self) -> &Vec<ModOption> {
        &self.options
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn description_mut(&mut self) -> &mut String {
        &mut self.description
    }

    fn options_mut(&mut self) -> &mut Vec<ModOption> {
        &mut self.options
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipleOptionGroup {
    pub name: String,
    pub description: String,
    pub defaults: HashSet<PathBuf>,
    pub options: Vec<ModOption>,
}

impl ModOptionGroup for MultipleOptionGroup {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn options(&self) -> &Vec<ModOption> {
        &self.options
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn description_mut(&mut self) -> &mut String {
        &mut self.description
    }

    fn options_mut(&mut self) -> &mut Vec<ModOption> {
        &mut self.options
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[enum_dispatch::enum_dispatch]
#[serde(tag = "type")]
pub enum OptionGroup {
    Exclusive(ExclusiveOptionGroup),
    Multiple(MultipleOptionGroup),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub version: f32,
    pub author: String,
    pub category: String,
    pub description: String,
    pub platform: Endian,
    pub url: Option<String>,
    #[serde(rename = "option_groups")]
    pub options: Vec<OptionGroup>,
    pub masters: IndexMap<usize, (String, f32)>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for Meta {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.version.to_bits().hash(state);
        self.author.hash(state);
        self.platform.hash(state);
        self.description.hash(state);
    }
}

impl Meta {
    pub fn read(mod_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        use std::io::Read;
        let mod_path = mod_path.as_ref();
        let mut zip = zip::ZipArchive::new(std::io::BufReader::new(fs_err::File::open(mod_path)?))?;
        let mut meta = zip.by_name("meta.toml").context("Mod missing meta file")?;
        let mut buffer = vec![0; meta.size() as usize];
        let read = meta.read(&mut buffer)?;
        if read != meta.size() as usize {
            anyhow::bail!("Failed to read meta file")
        } else {
            Ok(toml::from_slice(&buffer).context("Failed to parse meta file")?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_meta() {
        dbg!(Meta::read("test/wiiu.zip").unwrap());
    }
}
