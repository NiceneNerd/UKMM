use crate::{prelude::*, util, Result, UKError};
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub inner: String,
    pub delete: bool,
}

impl From<String> for Tag {
    fn from(string: String) -> Self {
        Tag {
            inner: string,
            delete: false,
        }
    }
}

impl From<&str> for Tag {
    fn from(string: &str) -> Self {
        string.to_owned().into()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorLink {
    pub targets: ParameterObject,
    pub tags: Option<HashSet<Tag>>,
}

impl Mergeable for ActorLink {
    fn diff(&self, other: &Self) -> Self {
        Self {
            targets: util::diff_pobj(&self.targets, &other.targets),
            tags: other.tags.as_ref().map(|diff_tags| {
                if let Some(self_tags) = self.tags.as_ref() {
                    diff_tags
                        .iter()
                        .filter(|tag| !self_tags.contains(tag))
                        .cloned()
                        .chain(self_tags.iter().filter_map(|tag| {
                            if diff_tags.contains(tag) {
                                None
                            } else {
                                Some(Tag {
                                    inner: tag.inner.clone(),
                                    delete: true,
                                })
                            }
                        }))
                        .collect()
                } else {
                    diff_tags.clone()
                }
            }),
        }
    }

    fn merge(base: &Self, other: &Self) -> Self {
        Self {
            targets: ParameterObject(
                base.targets
                    .0
                    .iter()
                    .chain(other.targets.0.iter())
                    .map(|(k, v)| (*k, v.clone()))
                    .collect(),
            ),
            tags: {
                if let Some(base_tags) = &base.tags {
                    if let Some(other_tags) = &other.tags {
                        Some(
                            base_tags
                                .iter()
                                .chain(other_tags.iter())
                                .filter_map(|tag| if tag.delete { None } else { Some(tag.clone()) })
                                .collect(),
                        )
                    } else {
                        base.tags.clone()
                    }
                } else {
                    other.tags.clone()
                }
            },
        }
    }
}

impl TryFrom<&ParameterIO> for ActorLink {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            targets: pio
                .object("LinKTarget")
                .ok_or_else(|| {
                    UKError::MissingAampKey("Actor link missing link targets".to_owned())
                })?
                .clone(),
            tags: pio.object("Tags").map(|tags| {
                tags.0
                    .values()
                    .filter_map(|v| v.as_string().ok().map(|s| s.into()))
                    .collect()
            }),
        })
    }
}

impl IntoParameterIO for ActorLink {
    fn into_pio(self) -> ParameterIO {
        ParameterIO {
            objects: {
                let mut objects = ParameterObjectMap::default();
                objects.0.insert(hash_name("LinkTarget"), self.targets);
                if let Some(tags) = self.tags {
                    objects.0.insert(
                        hash_name("Tags"),
                        ParameterObject(
                            tags.into_iter()
                                .enumerate()
                                .filter_map(|(i, tag)| {
                                    if tag.delete {
                                        None
                                    } else {
                                        Some((
                                            hash_name(&format!("Tag{}", i)),
                                            Parameter::StringRef(tag.inner),
                                        ))
                                    }
                                })
                                .collect(),
                        ),
                    );
                }
                objects
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {}
