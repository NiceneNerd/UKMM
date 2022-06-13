use crate::{
    actor::InfoSource,
    prelude::*,
    util::{self, DeleteSet},
    Result, UKError,
};
use join_str::jstr;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorLink {
    pub targets: ParameterObject,
    pub tags: Option<DeleteSet<String>>,
}

impl TryFrom<&ParameterIO> for ActorLink {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            targets: pio
                .object("LinkTarget")
                .ok_or(UKError::MissingAampKey("Actor link missing link targets"))?
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
                        tags.into_iter()
                            .enumerate()
                            .map(|(i, tag)| {
                                (
                                    jstr!("Tag{&lexical::to_string(i)}"),
                                    Parameter::StringRef(tag),
                                )
                            })
                            .collect(),
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
                        .filter_map(|tag| {
                            if !self_tags.contains(tag) {
                                Some((tag.clone(), false))
                            } else {
                                None
                            }
                        })
                        .chain(self_tags.iter().filter_map(|tag| {
                            if diff_tags.contains(tag) {
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

    fn merge(&self, other: &Self) -> Self {
        Self {
            targets: self
                .targets
                .0
                .iter()
                .chain(other.targets.0.iter())
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            tags: {
                if let Some(base_tags) = &self.tags {
                    if let Some(other_tags) = &other.tags {
                        Some(base_tags.merge(other_tags))
                    } else {
                        self.tags.clone()
                    }
                } else {
                    other.tags.clone()
                }
            },
        }
    }
}

impl InfoSource for ActorLink {
    fn update_info(&self, info: &mut roead::byml::Hash) -> Result<()> {
        crate::actor::info_params!(
            &self.targets,
            info,
            {
                ("actorScale", "ActorScale",  f32),
                ("elink", "ElinkUser",  String),
                ("profile", "ProfileUser",  String),
                ("slink", "SlinkUser",  String),
                ("xlink", "XlinkUser",  String),
            }
        );
        if self.targets.param("SlinkUser") != Some(&Parameter::StringRef("Dummy".to_owned())) {
            info.insert("bugMask".to_owned(), Byml::Int(2));
        }
        if let Some(tags) = &self.tags && !tags.is_empty() {
            info.insert(
                "tags".to_owned(),
                tags.iter()
                    .map(|tag| -> (String, Byml) {
                        let hash = roead::aamp::hash_name(tag.as_str());
                        (
                            format!("tag{:08x}", hash),
                            if hash > 2147483647 {
                                Byml::UInt(hash)
                            } else {
                                Byml::Int(hash as i32)
                            },
                        )
                    })
                    .collect(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

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
        let data = roead::aamp::ParameterIO::from(actorlink.clone()).to_binary();
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
        let merged = actorlink.merge(&diff);
        assert_eq!(actorlink2, merged);
    }

    #[test]
    fn info() {
        use roead::byml::Byml;
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::new();
        actorlink.update_info(&mut info).unwrap();
        assert_eq!(info["actorScale"], Byml::Float(1.0));
        assert_eq!(info["elink"], Byml::String("Guardian_A".to_owned()));
        assert_eq!(info["profile"], Byml::String("Guardian".to_owned()));
        assert_eq!(info["slink"], Byml::String("Guardian".to_owned()));
        assert_eq!(info["xlink"], Byml::String("Guardian".to_owned()));
        assert_eq!(info["tags"]["tag3a61e2a9"], Byml::Int(979493545));
        assert_eq!(info["tags"]["tag994aef4b"], Byml::UInt(0x994aef4b));
    }
}
