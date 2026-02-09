use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    util::{DeleteSet, SortedDeleteMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct BarslistInfo(pub SortedDeleteMap<String, DeleteSet<String>>);

impl TryFrom<&Byml> for BarslistInfo {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        Ok(Self(
            hash.iter()
                .map(|(k, v)| -> Result<(String, DeleteSet<String>)> {
                    Ok((
                        k.clone(),
                        v.as_array()?
                            .iter()
                            .filter_map(|v| v.as_string().ok().cloned())
                            .collect(),
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<BarslistInfo> for Byml {
    fn from(val: BarslistInfo) -> Self {
        val.0
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    v.into_iter().map(|s| Byml::from(s.to_string())).collect(),
                )
            })
            .collect()
    }
}

impl Mergeable for BarslistInfo {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl Resource for BarslistInfo {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("BarslistInfo")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_barslist() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Sound/ResourceList/BarslistInfo.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_barslist() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Sound/ResourceList/BarslistInfo.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_barslist();
        let barslist = super::BarslistInfo::try_from(&byml).unwrap();
        let data = Byml::from(barslist.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let barslist2 = super::BarslistInfo::try_from(&byml2).unwrap();
        assert_eq!(barslist, barslist2);
    }

    #[test]
    fn diff() {
        let byml = load_barslist();
        let barslist = super::BarslistInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_barslist();
        let barslist2 = super::BarslistInfo::try_from(&byml2).unwrap();
        let _diff = barslist.diff(&barslist2);
    }

    #[test]
    fn merge() {
        let byml = load_barslist();
        let barslist = super::BarslistInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_barslist();
        let barslist2 = super::BarslistInfo::try_from(&byml2).unwrap();
        let diff = barslist.diff(&barslist2);
        let merged = barslist.merge(&diff);
        assert_eq!(merged, barslist2);
    }

    #[test]
    fn identify() {
        let path =
            std::path::Path::new("content/Pack/Bootup.pack//Sound/ResourceList/BarslistInfo.sbyml");
        assert!(super::BarslistInfo::path_matches(path));
    }
}
