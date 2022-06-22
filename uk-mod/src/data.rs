use crate::{platform_prefixes, resource::ResourceData};
use anyhow::{bail, Context, Result};
use join_str::jstr;
use jwalk::WalkDir;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uk_content::prelude::*;

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
                                            ResourceData::Binary(self_bytes.clone()),
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
    pub content: Option<String>,
    pub aoc: Option<String>,
    pub resources: BTreeMap<String, ResourceData>,
}

impl DataTreeBuilder {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        let (content_u, aoc_u) = platform_prefixes(Endian::Big);
        let (content_nx, aoc_nx) = platform_prefixes(Endian::Little);
        Self {
            root: dir.to_owned(),
            content: if dir.join(content_u).exists() {
                Some(content_u.to_owned())
            } else if dir.join(content_nx).exists() {
                Some(content_nx.to_owned())
            } else {
                None
            },
            aoc: if dir.join(aoc_u).exists() {
                Some(aoc_u.to_owned())
            } else if dir.join(aoc_nx).exists() {
                Some(aoc_nx.to_owned())
            } else {
                None
            },
            resources: BTreeMap::new(),
        }
    }

    pub fn build(self) -> Result<DataTree> {
        if let Some(content) = self.content.as_ref()
        // && let Some(aoc) = self.aoc.as_ref()
        {
            let mut resources = Arc::new(Mutex::new(&self.resources));
            let content_files = WalkDir::new(self.root.join(content))
                .into_iter()
                .filter_map(|f| f.ok().and_then(|f| f.file_type().is_file().then(|| f)))
                .map(|path| -> Result<(String, String)> {
                    dbg!(path);
                    Ok((Default::default(), Default::default()))
                })
                .collect::<Result<BTreeMap<String, String>>>()?;
            drop(resources);
            Ok(DataTree {
                content_files,
                aoc_files: Default::default(),
                resources: self.resources,
            })
        } else {
            bail!("No content or aoc directory found")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_mod_wiiu() {
        DataTree::from_files("test").unwrap();
    }
}
