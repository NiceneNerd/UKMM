use anyhow::{bail, Context, Result};
use join_str::jstr;
use jwalk::WalkDir;
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::BufWriter,
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{canonicalize, platform_prefixes, prelude::*, resource::ResourceData};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DataTree {
    pub content_files: BTreeSet<String>,
    pub aoc_files: BTreeSet<String>,
    pub resources: BTreeMap<String, ResourceData>,
}

impl Mergeable for DataTree {
    fn diff(&self, other: &Self) -> Self {
        Self {
            content_files: other
                .content_files
                .iter()
                .filter(|f| !self.content_files.contains(*f))
                .cloned()
                .collect(),
            aoc_files: other
                .aoc_files
                .iter()
                .filter(|f| !self.aoc_files.contains(*f))
                .cloned()
                .collect(),
            resources: other
                .resources
                .iter()
                .filter_map(|(name, other_res)| {
                    if let Some(self_res) = self.resources.get(name) {
                        if self_res == other_res {
                            None
                        } else {
                            match (self_res, other_res) {
                                (
                                    ResourceData::Binary(self_bytes),
                                    ResourceData::Binary(other_bytes),
                                ) => {
                                    if self_bytes == other_bytes {
                                        None
                                    } else {
                                        Some((
                                            name.clone(),
                                            ResourceData::Binary(other_bytes.clone()),
                                        ))
                                    }
                                }
                                (
                                    ResourceData::Mergeable(self_res),
                                    ResourceData::Mergeable(other_res),
                                ) => {
                                    if self_res == other_res {
                                        None
                                    } else {
                                        Some((
                                            name.clone(),
                                            ResourceData::Mergeable(self_res.diff(other_res)),
                                        ))
                                    }
                                }
                                _ => Some((name.clone(), other_res.clone())),
                            }
                        }
                    } else {
                        Some((name.clone(), other_res.clone()))
                    }
                })
                .collect(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            content_files: self
                .content_files
                .iter()
                .chain(diff.content_files.iter())
                .cloned()
                .collect(),
            aoc_files: self
                .aoc_files
                .iter()
                .chain(diff.aoc_files.iter())
                .cloned()
                .collect(),
            resources: self
                .resources
                .keys()
                .chain(diff.resources.keys())
                .map(|k| {
                    (k.clone(), {
                        let v1 = self.resources.get(k);
                        let v2 = diff.resources.get(k);
                        match (v1, v2) {
                            (
                                Some(ResourceData::Mergeable(v1)),
                                Some(ResourceData::Mergeable(v2)),
                            ) => ResourceData::Mergeable(v1.merge(v2)),
                            _ => v2.or(v1).cloned().unwrap(),
                        }
                    })
                })
                .collect(),
        }
    }
}

impl DataTree {
    pub fn write_files(self, dir: impl AsRef<Path>, endian: Endian) -> Result<()> {
        let (content, aoc) = platform_prefixes(endian);
        let dir = dir.as_ref();
        self.content_files
            .into_par_iter()
            .try_for_each(|path| -> Result<()> {
                println!("Writing {}", &path);
                let canon = canonicalize(&path);
                let out = dir.join(content).join(path);
                let compress = out
                    .extension()
                    .and_then(|ext| ext.to_str().map(|ext| ext.starts_with('s')))
                    .unwrap_or(false);
                if !out.parent().unwrap().exists() {
                    std::fs::create_dir_all(out.parent().unwrap())?;
                }
                let data = self
                    .resources
                    .get(&canon)
                    .with_context(|| jstr!("Mod missing needed resource {&canon}"))?
                    .to_binary(endian, &self.resources)?;
                use std::io::Write;
                let mut writer = BufWriter::new(std::fs::File::create(&out)?);
                if compress {
                    let data = Binary::Bytes(roead::yaz0::compress(data));
                    writer.write_all(&data)?;
                } else {
                    writer.write_all(&data)?;
                };
                Ok(())
            })?;
        self.aoc_files
            .into_par_iter()
            .try_for_each(|path| -> Result<()> {
                let canon = canonicalize(&path);
                let out = dir.join(aoc).join(path);
                let compress = out
                    .extension()
                    .and_then(|ext| ext.to_str().map(|ext| ext.starts_with('s')))
                    .unwrap_or(false);
                if !out.parent().unwrap().exists() {
                    std::fs::create_dir_all(out.parent().unwrap())?;
                }
                let data = self
                    .resources
                    .get(&canon)
                    .with_context(|| jstr!("Mod missing needed resource {&canon}"))?
                    .to_binary(endian, &self.resources)?;
                use std::io::Write;
                let mut writer = BufWriter::new(std::fs::File::create(&out)?);
                if compress {
                    let data = Binary::Bytes(roead::yaz0::compress(data));
                    writer.write_all(&data)?;
                } else {
                    writer.write_all(&data)?;
                };
                Ok(())
            })?;
        Ok(())
    }

    pub fn from_files(dir: impl AsRef<Path>) -> Result<Self> {
        DataTreeBuilder::new(dir).build()
    }
}

#[derive(Debug)]
struct DataTreeBuilder {
    pub root: PathBuf,
    pub resources: BTreeMap<String, ResourceData>,
}

impl DataTreeBuilder {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            root: dir.as_ref().into(),
            resources: BTreeMap::new(),
        }
    }

    fn collect_resources(&mut self, dir: impl AsRef<Path>) -> Result<BTreeSet<String>> {
        let resources = Arc::new(RwLock::new(&mut self.resources));
        WalkDir::new(dir.as_ref())
            .into_iter()
            .filter_map(|f| {
                f.ok()
                    .and_then(|f| f.file_type().is_file().then(|| f.path()))
            })
            .collect::<BTreeSet<PathBuf>>()
            .into_par_iter()
            .map(|path| -> Result<String> {
                let name = path
                    .strip_prefix(&self.root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let canon = canonicalize(&name);
                if !resources.read().unwrap().contains_key(&canon) {
                    let mut resources = resources.write().unwrap();
                    let resource =
                        ResourceData::from_binary(&name, std::fs::read(&path)?, &mut resources)
                            .with_context(|| jstr!("Error parsing resource at {&name}"))?;
                    resources.insert(canon, resource);
                };
                Ok(name)
            })
            .collect::<Result<_>>()
    }

    pub fn build(mut self) -> Result<DataTree> {
        let (content_u, aoc_u) = platform_prefixes(Endian::Big);
        let (content_nx, aoc_nx) = platform_prefixes(Endian::Little);
        let content = self
            .root
            .join(content_u)
            .exists()
            .then(|| content_u)
            .or_else(|| self.root.join(content_nx).exists().then(|| content_nx));
        let aoc = self
            .root
            .join(aoc_u)
            .exists()
            .then(|| aoc_u)
            .or_else(|| self.root.join(aoc_nx).exists().then(|| aoc_nx));
        if content.is_none() && aoc.is_none() {
            bail!("No content or aoc directory found in mod")
        };
        Ok(DataTree {
            content_files: content
                .map(|content| self.collect_resources(self.root.join(content)))
                .transpose()?
                .unwrap_or_default(),
            aoc_files: aoc
                .map(|aoc| self.collect_resources(self.root.join(aoc)))
                .transpose()?
                .unwrap_or_default(),
            resources: self.resources,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_wiiu() {
        println!("Creating tree...");
        let tree = DataTree::from_files("test/wiiu").unwrap();
        println!("Cloning tree...");
        let tmp_tree = tree.clone();
        println!("Writing files...");
        let ex_dir = tempfile::tempdir().unwrap();
        tmp_tree.write_files(ex_dir.path(), Endian::Big).unwrap();
        println!("Creating seconday tree...");
        let tree2 = DataTree::from_files(ex_dir.path()).unwrap();
        println!("Comparing...");
        assert_eq!(tree, tree2);
    }
}
