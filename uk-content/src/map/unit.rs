use crate::{prelude::Mergeable, util::SortedDeleteMap, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct MapUnit {
    pub pos_x: f32,
    pub pos_y: f32,
    pub size: f32,
    pub objects: SortedDeleteMap<u32, Byml>,
    pub rails: SortedDeleteMap<u32, Byml>,
}

impl TryFrom<&Byml> for MapUnit {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            pos_x: hash
                .get("LocationPosX")
                .ok_or(UKError::MissingBymlKey("Map unit missing location pos x"))?
                .as_float()?,
            pos_y: hash
                .get("LocationPosY")
                .ok_or(UKError::MissingBymlKey("Map unit missing location pos y"))?
                .as_float()?,
            size: hash
                .get("LocationSize")
                .ok_or(UKError::MissingBymlKey("Map unit missing location size"))?
                .as_float()?,
            objects: hash
                .get("Objs")
                .ok_or(UKError::MissingBymlKey("Map unit missing objs"))?
                .as_array()?
                .iter()
                .map(|obj| -> Result<(u32, Byml)> {
                    let hash = obj.as_hash()?;
                    let id = hash
                        .get("HashId")
                        .ok_or(UKError::MissingBymlKey("Map unit object missing hash ID"))?
                        .as_uint()?;
                    Ok((id, obj.clone()))
                })
                .collect::<Result<_>>()?,
            rails: hash
                .get("Rails")
                .ok_or(UKError::MissingBymlKey("Map unit missing rails"))?
                .as_array()?
                .iter()
                .map(|obj| -> Result<(u32, Byml)> {
                    let hash = obj.as_hash()?;
                    let id = hash
                        .get("HashId")
                        .ok_or(UKError::MissingBymlKey("Map unit rail missing hash ID"))?
                        .as_uint()?;
                    Ok((id, obj.clone()))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<MapUnit> for Byml {
    fn from(val: MapUnit) -> Self {
        [
            ("LocationPosX", Byml::Float(val.pos_x)),
            ("LocationPosY", Byml::Float(val.pos_y)),
            ("LocationSize", Byml::Float(val.size)),
            (
                "Objs",
                val.objects.into_iter().map(|(_, obj)| obj).collect(),
            ),
            ("Rails", val.rails.into_iter().map(|(_, obj)| obj).collect()),
        ]
        .into_iter()
        .collect()
    }
}

impl Mergeable<Byml> for MapUnit {
    fn diff(&self, other: &Self) -> Self {
        Self {
            pos_x: other.pos_x,
            pos_y: other.pos_y,
            size: other.size,
            objects: self.objects.diff(&other.objects),
            rails: self.rails.diff(&other.rails),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            pos_x: diff.pos_x,
            pos_y: diff.pos_y,
            size: diff.size,
            objects: self.objects.merge(&diff.objects),
            rails: self.rails.merge(&diff.rails),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_munt() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Map/MainField/E-.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_munt() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Map/MainField/MapUnit.mod.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let data = Byml::from(munt.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        assert_eq!(munt, munt2);
    }

    #[test]
    fn diff() {
        let byml = load_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let byml2 = load_mod_munt();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        let _diff = munt.diff(&munt2);
    }

    #[test]
    fn merge() {
        let byml = load_munt();
        let munt = super::MapUnit::try_from(&byml).unwrap();
        let byml2 = load_mod_munt();
        let munt2 = super::MapUnit::try_from(&byml2).unwrap();
        let diff = munt.diff(&munt2);
        let merged = munt.merge(&diff);
        assert_eq!(merged, munt2);
    }
}
