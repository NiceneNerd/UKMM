use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::SortedDeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct LazyTraverseList(pub SortedDeleteMap<String, SortedDeleteMap<u32, String>>);

impl TryFrom<&Byml> for LazyTraverseList {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_map()?
                .iter()
                .map(
                    |(key, list)| -> Result<(String, SortedDeleteMap<u32, String>)> {
                        Ok((
                            key.clone(),
                            list.as_array()?
                                .iter()
                                .map(|unit| -> Result<(u32, String)> {
                                    let unit = unit.as_map()?;
                                    let id = unit
                                        .get("HashId")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Lazy traverse list unit missing hash ID",
                                        ))?
                                        .as_int()?;
                                    let name = unit
                                        .get("UnitConfigName")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Lazy traverse list unit missing unit name",
                                        ))?
                                        .as_string()?;
                                    Ok((id, name.clone()))
                                })
                                .collect::<Result<_>>()?,
                        ))
                    },
                )
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<LazyTraverseList> for Byml {
    fn from(val: LazyTraverseList) -> Self {
        val.0
            .into_iter()
            .map(|(key, list)| {
                (
                    key.to_string(),
                    list.into_iter()
                        .map(|(id, name)| -> Byml {
                            map!(
                                "HashId" => Byml::U32(id),
                                "UnitConfigName" => Byml::String(name),
                            )
                        })
                        .collect(),
                )
            })
            .collect()
    }
}

impl Mergeable for LazyTraverseList {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl Resource for LazyTraverseList {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("LazyTraverseList")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_lazy() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/MainField/LazyTraverseList.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_lazy() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/MainField/LazyTraverseList.mod.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_lazy();
        let lazy = super::LazyTraverseList::try_from(&byml).unwrap();
        let data = Byml::from(lazy.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let lazy2 = super::LazyTraverseList::try_from(&byml2).unwrap();
        assert_eq!(lazy, lazy2);
    }

    #[test]
    fn diff() {
        let byml = load_lazy();
        let lazy = super::LazyTraverseList::try_from(&byml).unwrap();
        let byml2 = load_mod_lazy();
        let lazy2 = super::LazyTraverseList::try_from(&byml2).unwrap();
        let _diff = lazy.diff(&lazy2);
    }

    #[test]
    fn merge() {
        let byml = load_lazy();
        let lazy = super::LazyTraverseList::try_from(&byml).unwrap();
        let byml2 = load_mod_lazy();
        let lazy2 = super::LazyTraverseList::try_from(&byml2).unwrap();
        let diff = lazy.diff(&lazy2);
        let merged = lazy.merge(&diff);
        assert_eq!(merged, lazy2);
    }

    #[test]
    fn identify() {
        let path =
            std::path::Path::new("content/Pack/Bootup.pack//Map/MainField/LazyTraverseList.smubin");
        assert!(super::LazyTraverseList::path_matches(path));
    }
}
