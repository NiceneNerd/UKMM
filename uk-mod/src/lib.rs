#![feature(seek_stream_len)]
use anyhow::Context;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
};
use uk_content::prelude::Endian;
use uk_content::util::{IndexMap, IndexSet};
pub mod pack;
pub mod unpack;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "content")]
    pub content_files: BTreeSet<String>,
    #[serde(rename = "aoc")]
    pub aoc_files: BTreeSet<String>,
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
    fn options(&self) -> &IndexSet<ModOption>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusiveOptionGroup {
    pub name: String,
    pub description: String,
    pub default: Option<PathBuf>,
    pub options: IndexSet<ModOption>,
}

impl ModOptionGroup for ExclusiveOptionGroup {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn options(&self) -> &IndexSet<ModOption> {
        &self.options
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipleOptionGroup {
    pub name: String,
    pub description: String,
    pub defaults: HashSet<PathBuf>,
    pub options: IndexSet<ModOption>,
}

impl ModOptionGroup for MultipleOptionGroup {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn options(&self) -> &IndexSet<ModOption> {
        &self.options
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
    pub description: String,
    pub platform: Endian,
    pub url: Option<String>,
    #[serde(rename = "option_groups")]
    pub options: Vec<OptionGroup>,
    pub masters: IndexMap<String, f32>,
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
    fn save_dump() {
        let reader = uk_reader::ResourceReader::from_unpacked_dirs(
            Some("/games/Cemu/mlc01/usr/title/00050000/101C9400/content"),
            Some("/games/Cemu/mlc01/usr/title/0005000E/101C9400/content"),
            Some("/games/Cemu/mlc01/usr/title/0005000C/101C9400/content/0010"),
        )
        .unwrap();
        std::fs::write(
            "../.vscode/dump.yml",
            serde_yaml::to_string(&reader).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn read_meta() {
        dbg!(Meta::read("test/wiiu.zip").unwrap());
    }
}
