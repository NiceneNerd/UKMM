use crate::{
    prelude::*,
    util::{self, SortedDeleteMap},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct AreaData(pub SortedDeleteMap<usize, Byml>);

impl TryFrom<&Byml> for AreaData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .iter()
                .map(|area| -> Result<(usize, Byml)> {
                    let hash = area.as_hash()?;
                    Ok((
                        hash.get("AreaNumber")
                            .ok_or(UKError::MissingBymlKey(
                                "Area data entry missing area number",
                            ))?
                            .as_int()? as usize,
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
        Self(
            other
                .0
                .iter()
                .filter_map(|(num, diff_area)| {
                    if let Some(self_area) = self.0.get(num) {
                        if self_area != diff_area {
                            Some((*num, util::diff_byml_shallow(self_area, diff_area), false))
                        } else {
                            None
                        }
                    } else {
                        Some((*num, diff_area.clone(), false))
                    }
                })
                .chain(self.0.keys().filter_map(|num| {
                    (!other.0.contains_key(num)).then(|| (*num, Byml::Null, true))
                }))
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        let keys: BTreeSet<usize> = self.0.keys().chain(diff.0.keys()).copied().collect();
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
                .collect::<SortedDeleteMap<usize, Byml>>()
                .and_delete(),
        )
    }
}

impl Resource for AreaData {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("AreaData")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_areadata() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Ecosystem/AreaData.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_areadata() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Ecosystem/AreaData.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_areadata();
        let areadata = super::AreaData::try_from(&byml).unwrap();
        let data = Byml::from(areadata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
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
}
