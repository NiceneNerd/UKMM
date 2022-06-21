use anyhow::{Context, Result};
use join_str::jstr;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use uk_content::prelude::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Resource {
    Binary(Vec<u8>),
    Mergeable(crate::resource::MergeableResource),
}

impl Resource {
    pub fn to_binary(&self, endian: Endian) -> Vec<u8> {
        match self {
            Resource::Binary(data) => data.clone(),
            Resource::Mergeable(resource) => resource.clone().into_binary(endian),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DataTree {
    pub file_map: BTreeMap<String, String>,
    pub resources: BTreeMap<String, Resource>,
}

impl Mergeable for DataTree {
    fn diff(&self, other: &Self) -> Self {
        Self {
            file_map: other
                .file_map
                .iter()
                .filter_map(|(k, v)| {
                    (self.file_map.get(k.as_str()) != Some(v)).then(|| (k.clone(), v.clone()))
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
                                (Resource::Binary(self_bytes), Resource::Binary(other_bytes)) => {
                                    if self_bytes == other_bytes {
                                        None
                                    } else {
                                        Some((name.clone(), Resource::Binary(self_bytes.clone())))
                                    }
                                }
                                (Resource::Mergeable(self_res), Resource::Mergeable(other_res)) => {
                                    if self_res == other_res {
                                        None
                                    } else {
                                        Some((
                                            name.clone(),
                                            Resource::Mergeable(self_res.diff(other_res)),
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
            file_map: self
                .file_map
                .keys()
                .chain(diff.file_map.keys())
                .map(|k| {
                    (
                        k.clone(),
                        diff.file_map
                            .get(k)
                            .or_else(|| self.file_map.get(k))
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
                            (Some(Resource::Mergeable(v1)), Some(Resource::Mergeable(v2))) => {
                                Resource::Mergeable(v1.merge(v2))
                            }
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
        for (path, canon) in self.file_map {
            let out = dir.as_ref().join(path);
            if !out.parent().unwrap().exists() {
                std::fs::create_dir_all(out.parent().unwrap())?;
            }
            std::fs::write(
                out,
                self.resources
                    .get(&canon)
                    .with_context(|| jstr!("Mod missing needed resource {&canon}"))?
                    .to_binary(endian),
            )?;
        }
        Ok(())
    }
}
