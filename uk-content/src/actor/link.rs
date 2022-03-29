use crate::{prelude::*, util, Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorLink {
    pub targets: ParameterObject,
    pub tags: Option<IndexMap<String, bool>>,
}

impl TryFrom<&ParameterIO> for ActorLink {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            targets: pio
                .object("LinkTarget")
                .ok_or_else(|| {
                    UKError::MissingAampKey("Actor link missing link targets".to_owned())
                })?
                .clone(),
            tags: pio.object("Tags").map(|tags| {
                tags.0
                    .values()
                    .filter_map(|v| v.as_string().ok().map(|s| (s.to_owned(), false)))
                    .collect()
            }),
        })
    }
}

impl From<ActorLink> for ParameterIO {
    fn from(val: ActorLink) -> Self {
        ParameterIO {
            objects: {
                let mut objects = ParameterObjectMap::default();
                objects.0.insert(hash_name("LinkTarget"), val.targets);
                if let Some(tags) = val.tags {
                    objects.0.insert(
                        hash_name("Tags"),
                        ParameterObject(
                            tags.into_iter()
                                .enumerate()
                                .filter_map(|(i, (tag, delete))| {
                                    if delete {
                                        None
                                    } else {
                                        Some((
                                            hash_name(&format!("Tag{}", i)),
                                            Parameter::StringRef(tag),
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

impl Mergeable for ActorLink {
    fn diff(&self, other: &Self) -> Self {
        Self {
            targets: util::diff_pobj(&self.targets, &other.targets),
            tags: other.tags.as_ref().map(|diff_tags| {
                if let Some(self_tags) = self.tags.as_ref() {
                    diff_tags
                        .iter()
                        .filter_map(|(tag, _)| {
                            if !self_tags.contains_key(tag.as_str()) {
                                Some((tag.clone(), false))
                            } else {
                                None
                            }
                        })
                        .chain(self_tags.iter().filter_map(|(tag, _)| {
                            if diff_tags.contains_key(tag.as_str()) {
                                None
                            } else {
                                Some((tag.clone(), true))
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
                                .collect::<IndexMap<&String, &bool>>()
                                .into_iter()
                                .filter_map(|(tag, delete)| {
                                    if *delete {
                                        None
                                    } else {
                                        Some((tag.clone(), false))
                                    }
                                })
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

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let data = actorlink.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let actorlink2 = super::ActorLink::try_from(&pio2).unwrap();
        assert_eq!(actorlink, actorlink2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink2 = super::ActorLink::try_from(&pio2).unwrap();
        let diff = actorlink.diff(&actorlink2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink2 = super::ActorLink::try_from(&pio2).unwrap();
        let diff = actorlink.diff(&actorlink2);
        let merged = super::ActorLink::merge(&actorlink, &diff);
        assert_eq!(actorlink2, merged);
    }
}
