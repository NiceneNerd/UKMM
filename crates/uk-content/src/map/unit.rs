use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::SortedDeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct MapUnit {
    pub pos_x:   Option<f32>,
    pub pos_z:   Option<f32>,
    pub size:    Option<f32>,
    pub objects: SortedDeleteMap<u32, Byml>,
    pub rails:   SortedDeleteMap<u32, Byml>,
}

impl TryFrom<&Byml> for MapUnit {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        Ok(Self {
            pos_x:   hash
                .get("LocationPosX")
                .map(|v| -> Result<f32> { Ok(v.as_float()?) })
                .transpose()?,
            pos_z:   hash
                .get("LocationPosZ")
                .map(|v| -> Result<f32> { Ok(v.as_float()?) })
                .transpose()?,
            size:    hash
                .get("LocationSize")
                .map(|v| -> Result<f32> { Ok(v.as_float()?) })
                .transpose()?,
            objects: hash
                .get("Objs")
                .ok_or(UKError::MissingBymlKey("Map unit missing objs"))?
                .as_array()?
                .iter()
                .map(|obj| -> Result<(u32, Byml)> {
                    let hash = obj.as_map()?;
                    let id = hash
                        .get("HashId")
                        .ok_or(UKError::MissingBymlKey("Map unit object missing hash ID"))?
                        .as_int()?;
                    Ok((id, obj.clone()))
                })
                .collect::<Result<_>>()?,
            rails:   hash
                .get("Rails")
                .ok_or(UKError::MissingBymlKey("Map unit missing rails"))?
                .as_array()?
                .iter()
                .map(|obj| -> Result<(u32, Byml)> {
                    let hash = obj.as_map()?;
                    let id = hash
                        .get("HashId")
                        .ok_or(UKError::MissingBymlKey("Map unit rail missing hash ID"))?
                        .as_int()?;
                    Ok((id, obj.clone()))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<MapUnit> for Byml {
    fn from(val: MapUnit) -> Self {
        [
            (
                "Objs",
                val.objects.into_iter().map(|(_, obj)| obj).collect(),
            ),
            ("Rails", val.rails.into_iter().map(|(_, obj)| obj).collect()),
        ]
        .into_iter()
        .chain(
            [
                ("LocationPosX", val.pos_x),
                ("LocationPosZ", val.pos_z),
                ("LocationSize", val.size),
            ]
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, Byml::Float(v)))),
        )
        .collect()
    }
}

impl Mergeable for MapUnit {
    fn diff(&self, other: &Self) -> Self {
        Self {
            pos_x:   other.pos_x,
            pos_z:   other.pos_z,
            size:    other.size,
            objects: self.objects.diff(&other.objects),
            rails:   self.rails.diff(&other.rails),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            pos_x:   diff.pos_x,
            pos_z:   diff.pos_z,
            size:    diff.size,
            objects: self.objects.merge(&diff.objects),
            rails:   self.rails.merge(&diff.rails),
        }
    }
}

impl Resource for MapUnit {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                (name.starts_with("CDungeon")
                    || name.contains("Dynamic")
                    || name.contains("_Static"))
                    && name.ends_with("mubin")
            })
            .unwrap_or(false)
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_cdungeon_munt() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/CDungeon/Dungeon044/Dungeon044_Static.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_cdungeon_munt() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/CDungeon/Dungeon044/Dungeon044_Static.mod.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mainfield_munt() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/MainField/D-3/D-3_Dynamic.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_mainfield_munt() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/MainField/D-3/D-3_Dynamic.mod.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde_mainfield() {
        let byml = load_mainfield_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let data = Byml::from(munt.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        assert_eq!(munt, munt2);
    }

    #[test]
    fn serde_cdungeon() {
        let byml = load_cdungeon_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let data = Byml::from(munt.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        assert_eq!(munt, munt2);
    }

    #[test]
    fn diff_mainfield() {
        let byml = load_mainfield_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let byml2 = load_mod_mainfield_munt();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        let _diff = munt.diff(&munt2);
    }

    #[test]
    fn diff_cdungeon() {
        let byml = load_cdungeon_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let byml2 = load_mod_cdungeon_munt();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        let _diff = munt.diff(&munt2);
    }

    #[test]
    fn merge_mainfield() {
        let byml = load_mainfield_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let byml2 = load_mod_mainfield_munt();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        let diff = munt.diff(&munt2);
        let merged = munt.merge(&diff);
        assert_eq!(merged, munt2);
    }

    #[test]
    fn merge_cdungeon() {
        let byml = load_cdungeon_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let byml2 = load_cdungeon_munt();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        let diff = munt.diff(&munt2);
        let merged = munt.merge(&diff);
        assert_eq!(merged, munt2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Map/MainField/F-3/F-3_Dynamic.smubin");
        assert!(super::MapUnit::path_matches(path));
        let path2 =
            std::path::Path::new("aoc/0010/Map/CDungeon/Dungeon044/Dungeon044_Static.mubin");
        assert!(super::MapUnit::path_matches(path2));
    }
}
