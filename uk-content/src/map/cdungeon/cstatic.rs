use crate::{map::EntryPos, prelude::Mergeable, util::DeleteMap, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Static(pub DeleteMap<String, DeleteMap<String, EntryPos>>);

impl TryFrom<&Byml> for Static {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_hash()?
                .get("StartPos")
                .ok_or(UKError::MissingBymlKey("CDungeon static missing StartPos"))?
                .as_array()?
                .iter()
                .try_fold(
                    DeleteMap::new(),
                    |mut entry_map,
                     entry|
                     -> Result<DeleteMap<String, DeleteMap<String, EntryPos>>> {
                        let entry = entry.as_hash()?;
                        let map = entry
                            .get("Map")
                            .ok_or(UKError::MissingBymlKey(
                                "CDungeon static entry missing Map name",
                            ))?
                            .as_string()?
                            .to_owned();
                        let pos_name = entry
                            .get("PosName")
                            .ok_or(UKError::MissingBymlKey(
                                "CDungeon static entry missing PosName",
                            ))?
                            .as_string()?
                            .to_owned();
                        let rotate = entry
                            .get("Rotate")
                            .ok_or(UKError::MissingBymlKey(
                                "CDungeon static entry missing Rotate",
                            ))?
                            .clone();
                        let translate = entry
                            .get("Translate")
                            .ok_or(UKError::MissingBymlKey(
                                "CDungeon static entry missing Translate",
                            ))?
                            .clone();
                        if let Some(map_entries) = entry_map.get_mut(&map) {
                            map_entries.insert(
                                pos_name,
                                EntryPos {
                                    rotate,
                                    translate,
                                    player_state: None,
                                },
                            );
                        } else {
                            entry_map.insert(
                                map,
                                [(
                                    pos_name,
                                    EntryPos {
                                        rotate,
                                        translate,
                                        player_state: None,
                                    },
                                )]
                                .into_iter()
                                .collect(),
                            );
                        };
                        Ok(entry_map)
                    },
                )?,
        ))
    }
}

impl From<Static> for Byml {
    fn from(val: Static) -> Self {
        [(
            "StartPos",
            val.0
                .into_iter()
                .flat_map(|(map, entries): (String, DeleteMap<String, EntryPos>)| {
                    entries
                        .into_iter()
                        .map(|(pos_name, pos)| {
                            [
                                ("Map", Byml::String(map.clone())),
                                ("PosName", Byml::String(pos_name)),
                                ("Rotate", pos.rotate),
                                ("Translate", pos.translate),
                            ]
                            .into_iter()
                            .collect()
                        })
                        .collect::<Vec<Byml>>()
                })
                .collect(),
        )]
        .into_iter()
        .collect()
    }
}

impl Mergeable<Byml> for Static {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_static() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Map/CDungeon/Static.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_static() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Map/CDungeon/Static.mod.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_static();
        let cstatic = super::Static::try_from(&byml).unwrap();
        let data = Byml::from(cstatic.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let static2 = super::Static::try_from(&byml2).unwrap();
        assert_eq!(cstatic, static2);
    }

    #[test]
    fn diff() {
        let byml = load_static();
        let cstatic = super::Static::try_from(&byml).unwrap();
        let byml2 = load_mod_static();
        let static2 = super::Static::try_from(&byml2).unwrap();
        let _diff = cstatic.diff(&static2);
    }

    #[test]
    fn merge() {
        let byml = load_static();
        let cstatic = super::Static::try_from(&byml).unwrap();
        let byml2 = load_mod_static();
        let static2 = super::Static::try_from(&byml2).unwrap();
        let diff = cstatic.diff(&static2);
        let merged = cstatic.merge(&diff);
        assert_eq!(merged, static2);
    }
}
