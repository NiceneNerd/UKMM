use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::SortedDeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct AreaData(pub SortedDeleteMap<usize, Byml>);

impl TryFrom<&Byml> for AreaData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .iter()
                .map(|area| -> Result<(usize, Byml)> {
                    let hash = area.as_map()?;
                    Ok((
                        hash.get("AreaNumber")
                            .ok_or(UKError::MissingBymlKey(
                                "Area data entry missing area number",
                            ))?
                            .as_int()?,
                        area.clone(),
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<AreaData> for Byml {
    fn from(val: AreaData) -> Self {
        val.0.values().cloned().collect()
    }
}

impl Mergeable for AreaData {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl Resource for AreaData {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("AreaData")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_areadata() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Ecosystem/AreaData.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_areadata() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Ecosystem/AreaData.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_areadata();
        let areadata = super::AreaData::try_from(&byml).unwrap();
        let data = Byml::from(areadata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let areadata2 = super::AreaData::try_from(&byml2).unwrap();
        assert_eq!(areadata, areadata2);
    }

    #[test]
    fn diff() {
        let byml = load_areadata();
        let areadata = super::AreaData::try_from(&byml).unwrap();
        let byml2 = load_mod_areadata();
        let areadata2 = super::AreaData::try_from(&byml2).unwrap();
        let _diff = areadata.diff(&areadata2);
    }

    #[test]
    fn merge() {
        let byml = load_areadata();
        let areadata = super::AreaData::try_from(&byml).unwrap();
        let byml2 = load_mod_areadata();
        let areadata2 = super::AreaData::try_from(&byml2).unwrap();
        let diff = areadata.diff(&areadata2);
        let merged = areadata.merge(&diff);
        assert_eq!(merged, areadata2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//Ecosystem/AreaData.sbyml");
        assert!(super::AreaData::path_matches(path));
    }
}
