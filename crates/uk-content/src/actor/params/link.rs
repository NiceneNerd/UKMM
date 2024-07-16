use join_str::jstr;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};


use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util::{self, DeleteSet},
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]

pub struct ActorLink {
    pub targets:  ParameterObject,
    pub tags:     Option<DeleteSet<String>>,
    pub fit_tags: Option<DeleteSet<String>>,
}

impl TryFrom<&ParameterIO> for ActorLink {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            targets:  pio
                .object("LinkTarget")
                .ok_or(UKError::MissingAampKey(
                    "Actor link missing link targets",
                    None,
                ))?
                .clone(),
            tags:     pio.object("Tags").map(|tags| {
                tags.0
                    .values()
                    .filter_map(|v| v.as_str().ok().map(|s| (s.into(), false)))
                    .collect()
            }),
            fit_tags: pio.object(1115720914).map(|tags| {
                tags.0
                    .values()
                    .filter_map(|v| v.as_str().ok().map(|s| (s.into(), false)))
                    .collect()
            }),
        })
    }
}

impl TryFrom<ParameterIO> for ActorLink {
    type Error = UKError;

    fn try_from(pio: ParameterIO) -> Result<Self> {
        Self::try_from(&pio)
    }
}

impl From<ActorLink> for ParameterIO {
    fn from(val: ActorLink) -> Self {
        ParameterIO {
            param_root: ParameterList {
                objects: {
                    let mut objects = ParameterObjectMap::default();
                    objects.insert(hash_name("LinkTarget"), val.targets);
                    if let Some(tags) = val.tags {
                        objects.insert(
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
                    if let Some(fit_tags) = val.fit_tags {
                        objects.insert(
                            1115720914,
                            fit_tags
                                .into_iter()
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
            },
            version:    0,
            data_type:  "xml".into(),
        }
    }
}

impl Mergeable for ActorLink {
    fn diff(&self, other: &Self) -> Self {
        Self {
            targets:  util::diff_pobj(&self.targets, &other.targets),
            tags:     other.tags.as_ref().map(|diff_tags| {
                if let Some(self_tags) = self.tags.as_ref() {
                    self_tags.diff(diff_tags)
                } else {
                    diff_tags.clone()
                }
            }),
            fit_tags: other.fit_tags.as_ref().map(|diff_tags| {
                if let Some(self_tags) = self.fit_tags.as_ref() {
                    self_tags.diff(diff_tags)
                } else {
                    diff_tags.clone()
                }
            }),
        }
    }

    fn merge(&self, other: &Self) -> Self {
        Self {
            targets:  self
                .targets
                .0
                .iter()
                .chain(other.targets.0.iter())
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            tags:     {
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
            fit_tags: {
                if let Some(base_tags) = &self.fit_tags {
                    if let Some(other_tags) = &other.fit_tags {
                        Some(base_tags.merge(other_tags))
                    } else {
                        self.fit_tags.clone()
                    }
                } else {
                    other.fit_tags.clone()
                }
            },
        }
    }
}

impl InfoSource for ActorLink {
    fn update_info(&self, info: &mut roead::byml::Map) -> Result<()> {
        crate::actor::info_params!(
            &self.targets,
            info,
            {
                ("actorScale", "ActorScale",  f32),
                ("elink", "ElinkUser", smartstring::alias::String),
                ("profile", "ProfileUser", smartstring::alias::String),
                ("slink", "SlinkUser", smartstring::alias::String),
                ("xlink", "XlinkUser", smartstring::alias::String),
            }
        );
        if self.targets.get("SlinkUser") != Some(&Parameter::StringRef("Dummy".into())) {
            info.insert("bugMask".into(), Byml::I32(2));
        }
        if let Some(tags) = self.tags.as_ref().filter(|t| !t.is_empty()) {
            info.insert(
                "tags".into(),
                tags.iter()
                    .map(|tag| -> (std::string::String, Byml) {
                        let hash = roead::aamp::hash_name(tag.as_str());
                        (
                            format!("tag{:08x}", hash),
                            if hash > 2147483647 {
                                Byml::U32(hash)
                            } else {
                                Byml::I32(hash as i32)
                            },
                        )
                    })
                    .collect(),
            );
        }
        Ok(())
    }
}

impl ParameterResource for ActorLink {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/ActorLink/{name}.bxml")
    }
}

impl Resource for ActorLink {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .to_str()
            .map(|path| path.contains("ActorLink") && path.ends_with("bxml"))
            .unwrap_or(false)
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(actorlink.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let actorlink2 = super::ActorLink::try_from(&pio2).unwrap();
        assert_eq!(actorlink, actorlink2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink2 = super::ActorLink::try_from(&pio2).unwrap();
        let _diff = actorlink.diff(&actorlink2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
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
                .get_data("Actor/ActorLink/Enemy_Guardian_A.bxml")
                .unwrap(),
        )
        .unwrap();
        let actorlink = super::ActorLink::try_from(&pio).unwrap();
        let mut info = roead::byml::Map::default();
        actorlink.update_info(&mut info).unwrap();
        assert_eq!(info["actorScale"], Byml::Float(1.0));
        assert_eq!(info["elink"], Byml::String("Guardian_A".into()));
        assert_eq!(info["profile"], Byml::String("Guardian".into()));
        assert_eq!(info["slink"], Byml::String("Guardian".into()));
        assert_eq!(info["xlink"], Byml::String("Guardian".into()));
        assert_eq!(info["tags"]["tag3a61e2a9"], Byml::I32(979493545));
        assert_eq!(info["tags"]["tag994aef4b"], Byml::U32(0x994aef4b));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/ActorLink/Enemy_Guardian_A.\
             bxml",
        );
        assert!(super::ActorLink::path_matches(path));
    }
}
