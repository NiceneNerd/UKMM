use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    util::{BymlHashValue, SortedDeleteMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct ShopGameDataInfo {
    pub area_info:     SortedDeleteMap<BymlHashValue, Byml>,
    pub sold_out_info: SortedDeleteMap<BymlHashValue, Byml>,
}

impl TryFrom<&Byml> for ShopGameDataInfo {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        let area_info = hash
            .get("ShopAreaInfo")
            .ok_or(UKError::MissingBymlKey(
                "Shop game data info missing ShopAreaInfo",
            ))?
            .as_map()?;
        let sold_out_info = hash
            .get("SoldOutInfo")
            .ok_or(UKError::MissingBymlKey(
                "Shop game data info missing SoldOutInfo",
            ))?
            .as_map()?;
        Ok(Self {
            area_info:     area_info
                .get("Hashes")
                .ok_or(UKError::MissingBymlKey(
                    "Shop game data info missing area info hashes",
                ))?
                .as_array()?
                .iter()
                .zip(
                    area_info
                        .get("Values")
                        .ok_or(UKError::MissingBymlKey(
                            "Shop game data info missing area info values",
                        ))?
                        .as_array()?
                        .iter(),
                )
                .map(|(hash, value)| Ok((hash.try_into()?, value.clone())))
                .collect::<Result<_>>()?,
            sold_out_info: sold_out_info
                .get("Hashes")
                .ok_or(UKError::MissingBymlKey(
                    "Shop game data info missing sold out info hashes",
                ))?
                .as_array()?
                .iter()
                .zip(
                    sold_out_info
                        .get("Values")
                        .ok_or(UKError::MissingBymlKey(
                            "Shop game data info missing sold out info values",
                        ))?
                        .as_array()?
                        .iter(),
                )
                .map(|(hash, value)| Ok((hash.try_into()?, value.clone())))
                .collect::<Result<_>>()?,
        })
    }
}

impl From<ShopGameDataInfo> for Byml {
    fn from(val: ShopGameDataInfo) -> Self {
        map!(
            "ShopAreaInfo" => {
                let (hashes, values): (Vec<Byml>, Vec<Byml>) = val
                    .area_info
                    .into_iter()
                    .map(|(hash, value)| (hash.into(), value))
                    .unzip();
                map!(
                    "Hashes" => Byml::Array(hashes),
                    "Values" => Byml::Array(values),
                )
            },
            "SoldOutInfo" => {
                let (hashes, values): (Vec<Byml>, Vec<Byml>) = val
                    .sold_out_info
                    .into_iter()
                    .map(|(hash, value)| (hash.into(), value))
                    .unzip();
                map!(
                    "Hashes" => Byml::Array(hashes),
                    "Values" => Byml::Array(values),
                )
            }
        )
    }
}

impl Mergeable for ShopGameDataInfo {
    fn diff(&self, other: &Self) -> Self {
        Self {
            area_info:     self.area_info.diff(&other.area_info),
            sold_out_info: self.sold_out_info.diff(&other.sold_out_info),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            area_info:     self.area_info.merge(&diff.area_info),
            sold_out_info: self.sold_out_info.merge(&diff.sold_out_info),
        }
    }
}

impl Resource for ShopGameDataInfo {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("ShopGameDataInfo")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_shopinfo() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/GameData/ShopGameDataInfo.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_shopinfo() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/GameData/ShopGameDataInfo.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_shopinfo();
        let shopinfo = super::ShopGameDataInfo::try_from(&byml).unwrap();
        let data = Byml::from(shopinfo.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let shopinfo2 = super::ShopGameDataInfo::try_from(&byml2).unwrap();
        assert_eq!(shopinfo, shopinfo2);
    }

    #[test]
    fn diff() {
        let byml = load_shopinfo();
        let shopinfo = super::ShopGameDataInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_shopinfo();
        let shopinfo2 = super::ShopGameDataInfo::try_from(&byml2).unwrap();
        let _diff = shopinfo.diff(&shopinfo2);
    }

    #[test]
    fn merge() {
        let byml = load_shopinfo();
        let shopinfo = super::ShopGameDataInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_shopinfo();
        let shopinfo2 = super::ShopGameDataInfo::try_from(&byml2).unwrap();
        let diff = shopinfo.diff(&shopinfo2);
        let merged = shopinfo.merge(&diff);
        assert_eq!(merged, shopinfo2);
    }

    #[test]
    fn identify() {
        let path =
            std::path::Path::new("content/Pack/Bootup.pack//GameData/ShopGameDataInfo.sbyml");
        assert!(super::ShopGameDataInfo::path_matches(path));
    }
}
