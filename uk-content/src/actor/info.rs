use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

use crate::{
    prelude::*,
    util::{bhash, BymlHashValue, SortedDeleteMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, Editable)]
pub struct ActorInfo(pub SortedDeleteMap<BymlHashValue, Byml>);

impl TryFrom<&Byml> for ActorInfo {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let actorinfo = byml.as_hash()?;
        let actors = actorinfo
            .get("Actors")
            .ok_or(UKError::MissingBymlKey("Actor info missing Actors"))?
            .as_array()?;
        let hashes = actorinfo
            .get("Hashes")
            .ok_or(UKError::MissingBymlKey("Actor info missing Hashes"))?
            .as_array()?;
        if actors.len() != hashes.len() {
            Err(UKError::Other(
                "Invalid actor info: actor count and hash count not equal",
            ))
        } else {
            Ok(Self(
                actors
                    .iter()
                    .zip(hashes.iter())
                    .map(|(actor, hash)| -> Result<(BymlHashValue, Byml)> {
                        Ok((hash.try_into()?, actor.clone()))
                    })
                    .collect::<Result<_>>()?,
            ))
        }
    }
}

impl From<ActorInfo> for Byml {
    fn from(val: ActorInfo) -> Self {
        bhash!(
            "Actors" => Byml::Array(val.0.values().cloned().collect()),
            "Hashes" => Byml::Array(val.0.keys().map(Byml::from).collect())
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
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("ActorInfo.product")
    }
}

single_path!(ActorInfo, "Actor/ActorInfo.product.sbyml");

#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_actorinfo() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Actor/ActorInfo.product.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_actorinfo() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Actor/ActorInfo.product.mod.sbyml").unwrap(),
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
        let byml2 = Byml::from_binary(&data).unwrap();
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
