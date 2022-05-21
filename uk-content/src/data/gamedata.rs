use crate::{prelude::Mergeable, util::SortedDeleteMap, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GameData {
    pub data_type: String,
    pub flags: SortedDeleteMap<u32, Byml>,
}

impl TryFrom<&Byml> for GameData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            data_type: hash
                .keys()
                .next()
                .ok_or(UKError::MissingBymlKey("bgdata file missing data type key"))?
                .to_owned(),
            flags: hash
                .values()
                .next()
                .ok_or(UKError::MissingBymlKey("bgdata file missing data"))?
                .as_array()?
                .iter()
                .map(|item| -> Result<(u32, Byml)> {
                    Ok((
                        item.as_hash()?
                            .get("HashValue")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing HashValue",
                            ))?
                            .as_int()? as u32,
                        item.clone(),
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<GameData> for Byml {
    fn from(val: GameData) -> Self {
        [(val.data_type, val.flags.values().cloned().collect())]
            .into_iter()
            .collect()
    }
}

impl Mergeable<Byml> for GameData {
    fn diff(&self, other: &Self) -> Self {
        assert_eq!(
            self.data_type, other.data_type,
            "Attempted to diff different gamedata types: {} and {}",
            self.data_type, other.data_type
        );
        Self {
            data_type: self.data_type.clone(),
            flags: self.flags.diff(&other.flags),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        assert_eq!(
            self.data_type, diff.data_type,
            "Attempted to merge different gamedata types: {} and {}",
            self.data_type, diff.data_type
        );
        Self {
            data_type: self.data_type.clone(),
            flags: self.flags.merge(&diff.flags),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_gamedata() -> Byml {
        Byml::from_binary(&std::fs::read("test/GameData/revival_s32_data_0.bgdata").unwrap())
            .unwrap()
    }

    fn load_mod_gamedata() -> Byml {
        Byml::from_binary(&std::fs::read("test/GameData/revival_s32_data_0.mod.bgdata").unwrap())
            .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let data = Byml::from(gamedata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        assert_eq!(gamedata, gamedata2);
    }

    #[test]
    fn diff() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let byml2 = load_mod_gamedata();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        let diff = gamedata.diff(&gamedata2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let byml2 = load_mod_gamedata();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        let diff = gamedata.diff(&gamedata2);
        let merged = gamedata.merge(&diff);
        assert_eq!(merged, gamedata2);
    }
}
