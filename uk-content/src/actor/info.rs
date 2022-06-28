use crate::{
    prelude::*,
    util::{self, BymlHashValue, SortedDeleteMap},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
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
        Byml::Hash(
            [
                (
                    "Actors".to_owned(),
                    Byml::Array(val.0.values().cloned().collect()),
                ),
                (
                    "Hashes".to_owned(),
                    Byml::Array(val.0.keys().map(Byml::from).collect()),
                ),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Mergeable for ActorInfo {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(hash, other_info)| {
                    if let Some(self_info) = self.0.get(hash) {
                        if other_info == self_info {
                            None
                        } else {
                            Some((*hash, util::diff_byml_shallow(self_info, other_info), false))
                        }
                    } else {
                        Some((*hash, other_info.clone(), false))
                    }
                })
                .chain(self.0.keys().filter_map(|hash| {
                    (!other.0.contains_key(hash)).then(|| (*hash, Byml::Null, true))
                }))
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        let keys: BTreeSet<BymlHashValue> = self.0.keys().chain(diff.0.keys()).copied().collect();
        Self(
            keys.into_iter()
                .map(|hash| {
                    if let Some(self_info) = self.0.get(hash) {
                        if let Some(diff_info) = diff.0.get(hash) {
                            (
                                hash,
                                util::merge_byml_shallow(self_info, diff_info),
                                diff.0.is_delete(hash).unwrap(),
                            )
                        } else {
                            (hash, self_info.clone(), false)
                        }
                    } else {
                        (
                            hash,
                            diff.0.get(hash).unwrap().clone(),
                            diff.0.is_delete(hash).unwrap(),
                        )
                    }
                })
                .collect::<SortedDeleteMap<BymlHashValue, Byml>>()
                .and_delete(),
        )
    }
}

impl Resource for ActorInfo {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> roead::Bytes {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("ActorInfo.product")
    }
}

single_path!(ActorInfo, "Actor/ActorInfo.product.sbyml");

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

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
            panic!("merged != actorinfo2");
        }
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Actor/ActorInfo.product.sbyml");
        assert!(super::ActorInfo::path_matches(path));
    }
}
