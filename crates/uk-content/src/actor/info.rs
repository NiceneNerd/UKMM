use anyhow::Context;
use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::SortedDeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct ActorInfo(pub SortedDeleteMap<u32, Byml>);

impl TryFrom<&Byml> for ActorInfo {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let actorinfo = byml.as_map()?;
        let actors = actorinfo
            .get("Actors")
            .ok_or(UKError::MissingBymlKey("Actor info missing Actors"))?
            .as_array()?;

        Ok(Self(
            actors
                .iter()
                .map(|actor| -> Result<(u32, Byml)> {
                    let name = actor
                        .as_map()
                        .context("Actor info entry isn't a hash?")?
                        .get("name")
                        .ok_or(UKError::MissingBymlKey("Actor info entry missing name"))?
                        .as_string()
                        .context("Actor info entry name isn't a string")?;
                    let hash = roead::aamp::hash_name(name);
                    Ok((hash, actor.clone()))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<ActorInfo> for Byml {
    fn from(val: ActorInfo) -> Self {
        let (hashes, actors) = val
            .0
            .into_iter()
            .map(|(hash, actor)| {
                (
                    if hash > i32::MAX as u32 {
                        Byml::U32(hash)
                    } else {
                        Byml::I32(hash as i32)
                    },
                    actor,
                )
            })
            .unzip();
        map!(
            "Actors" => Byml::Array(actors),
            "Hashes" => Byml::Array(hashes)
        )
    }
}

impl Mergeable for ActorInfo {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl Resource for ActorInfo {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("ActorInfo.product")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_actorinfo() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Actor/ActorInfo.product.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_actorinfo() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Actor/ActorInfo.product.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_actorinfo();
        let actorinfo = super::ActorInfo::try_from(&byml).unwrap();
        let data = Byml::from(actorinfo.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let actorinfo2 = super::ActorInfo::try_from(&byml2).unwrap();
        assert_eq!(actorinfo, actorinfo2);
    }

    #[test]
    fn diff() {
        let byml = load_actorinfo();
        let actorinfo = super::ActorInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_actorinfo();
        let actorinfo2 = super::ActorInfo::try_from(&byml2).unwrap();
        let _diff = actorinfo.diff(&actorinfo2);
    }

    #[test]
    fn merge() {
        let byml = load_actorinfo();
        let actorinfo = super::ActorInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_actorinfo();
        let actorinfo2 = super::ActorInfo::try_from(&byml2).unwrap();
        let diff = actorinfo.diff(&actorinfo2);
        let merged = actorinfo.merge(&diff);
        if merged != actorinfo2 {
            dbg!(merged.diff(&actorinfo2));
            panic!("merged != actorinfo2");
        }
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Actor/ActorInfo.product.sbyml");
        assert!(super::ActorInfo::path_matches(path));
    }
}
