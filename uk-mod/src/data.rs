use anyhow::{bail, Context, Result};
use join_str::jstr;
use jwalk::WalkDir;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use uk_content::{canonicalize, platform_prefixes, prelude::*, resource::ResourceData};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DataTree {
    pub content_files: BTreeMap<String, String>,
    pub aoc_files: BTreeMap<String, String>,
    pub resources: BTreeMap<String, ResourceData>,
}

impl Mergeable for DataTree {
    fn diff(&self, other: &Self) -> Self {
        Self {
            content_files: other
                .content_files
                .iter()
                .filter_map(|(k, v)| {
                    (self.content_files.get(k.as_str()) != Some(v)).then(|| (k.clone(), v.clone()))
                })
                .collect(),
            aoc_files: other
                .aoc_files
                .iter()
                .filter_map(|(k, v)| {
                    (self.aoc_files.get(k.as_str()) != Some(v)).then(|| (k.clone(), v.clone()))
                })
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
                .keys()
                .chain(diff.content_files.keys())
                .map(|k| {
                    (
                        k.clone(),
                        diff.content_files
                            .get(k)
                            .or_else(|| self.content_files.get(k))
                            .cloned()
                            .unwrap(),
                    )
                })
                .collect(),
            aoc_files: self
                .aoc_files
                .keys()
                .chain(diff.aoc_files.keys())
                .map(|k| {
                    (
                        k.clone(),
                        diff.aoc_files
                            .get(k)
                            .or_else(|| self.aoc_files.get(k))
                            .cloned()
                            .unwrap(),
                    )
                })
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
            .try_for_each(|(path, canon)| -> Result<()> {
                let out = dir.join(content).join(path);
                if !out.parent().unwrap().exists() {
                    std::fs::create_dir_all(out.parent().unwrap())?;
                }
                std::fs::write(
                    out,
                    self.resources
                        .get(&canon)
                        .with_context(|| jstr!("Mod missing needed resource {&canon}"))?
                        .to_binary(endian, &self.resources)?,
                )?;
                Ok(())
            })?;
        self.aoc_files
            .into_par_iter()
            .try_for_each(|(path, canon)| -> Result<()> {
                let out = dir.join(aoc).join(path);
                if !out.parent().unwrap().exists() {
                    std::fs::create_dir_all(out.parent().unwrap())?;
                }
                std::fs::write(
                    out,
                    self.resources
                        .get(&canon)
                        .with_context(|| jstr!("Mod missing needed resource {&canon}"))?
                        .to_binary(endian, &self.resources)?,
                )?;
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
            root: dir.as_ref().to_owned(),
            resources: BTreeMap::new(),
        }
    }

    fn collect_resources(&mut self, dir: impl AsRef<Path>) -> Result<BTreeMap<String, String>> {
        let resources = Arc::new(RwLock::new(&mut self.resources));
        WalkDir::new(dir.as_ref())
            .into_iter()
            .filter_map(|f| {
                f.ok()
                    .and_then(|f| f.file_type().is_file().then(|| f.path()))
            })
            .collect::<BTreeSet<PathBuf>>()
            .into_par_iter()
            .map(|path| -> Result<(String, String)> {
                let name = path
                    .strip_prefix(&self.root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let canon = canonicalize(&name);
                if !resources.read().unwrap().contains_key(&canon) {
                    let mut resources = resources.write().unwrap();
                    let resource =
                        ResourceData::from_binary(&name, std::fs::read(&path)?, &mut resources)
                            .with_context(|| jstr!("Error parsing resource at {&name}"))?;
                    resources.insert(canon.clone(), resource);
                };
                Ok((name, canon))
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
        let tree = DataTree::from_files("test/wiiu").unwrap();
        dbg!(tree.content_files);
        dbg!(tree.aoc_files);
        dbg!(tree.resources.keys().count());
    }
}
