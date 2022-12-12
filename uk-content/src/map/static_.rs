use std::collections::BTreeMap;

use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

use crate::{
    prelude::*,
    util::{DeleteMap, DeleteVec},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize, Editable)]
pub struct EntryPos {
    pub rotate: roead::byml::Byml,
    pub translate: roead::byml::Byml,
    pub player_state: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, Editable)]
pub struct Static {
    pub general:   BTreeMap<String, DeleteVec<Byml>>,
    pub start_pos: DeleteMap<String, DeleteMap<String, EntryPos>>,
}

impl TryFrom<&Byml> for Static {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self {
            start_pos: byml
                .as_hash()?
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
                            .clone();
                        let pos_name = match entry.get("PosName") {
                            Some(pos_name) => pos_name.as_string()?.clone(),
                            _ => return Ok(entry_map),
                        };
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
                        let player_state = entry
                            .get("PlayerState")
                            .map(|state| -> Result<String> { Ok(state.as_string()?.clone()) })
                            .transpose()?;
                        if let Some(map_entries) = entry_map.get_mut(&map) {
                            map_entries.insert(pos_name, EntryPos {
                                rotate,
                                translate,
                                player_state,
                            });
                        } else {
                            entry_map.insert(
                                map,
                                [(pos_name, EntryPos {
                                    rotate,
                                    translate,
                                    player_state,
                                })]
                                .into_iter()
                                .collect(),
                            );
                        };
                        Ok(entry_map)
                    },
                )?,
            general:   byml
                .as_hash()?
                .iter()
                .filter(|(k, _)| k.as_str() != "StartPos")
                .map(|(key, array)| -> Result<(String, DeleteVec<Byml>)> {
                    Ok((key.clone(), array.as_array()?.iter().cloned().collect()))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<Static> for Byml {
    fn from(val: Static) -> Self {
        [(
            "StartPos".into(),
            val.start_pos
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
                            .chain(
                                pos.player_state
                                    .map(|state| ("PlayerState", Byml::String(state))),
                            )
                            .collect()
                        })
                        .collect::<Vec<Byml>>()
                })
                .collect(),
        )]
        .into_iter()
        .chain(
            val.general
                .into_iter()
                .map(|(key, array)| (key, array.into_iter().collect())),
        )
        .collect()
    }
}

impl Mergeable for Static {
    fn diff(&self, other: &Self) -> Self {
        Self {
            general:   other
                .general
                .iter()
                .filter_map(|(key, diff_entries)| {
                    if let Some(self_entries) = self.general.get(key) {
                        if self_entries == diff_entries {
                            None
                        } else {
                            Some((key.clone(), self_entries.diff(diff_entries)))
                        }
                    } else {
                        Some((key.clone(), diff_entries.clone()))
                    }
                })
                .collect(),
            start_pos: self.start_pos.deep_diff(&other.start_pos),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            general:   self
                .general
                .iter()
                .map(|(key, self_entries)| {
                    if let Some(diff_entries) = diff.general.get(key) {
                        (key.clone(), self_entries.merge(diff_entries))
                    } else {
                        (key.clone(), self_entries.clone())
                    }
                })
                .collect(),
            start_pos: self.start_pos.deep_merge(&diff.start_pos),
        }
    }
}

impl Resource for Static {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .with_extension("")
            .ends_with("CDungeon/Static")
    }
}

#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_cdungeon_static() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Map/CDungeon/Static.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_cdungeon_static() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Map/CDungeon/Static.mod.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mainfield_static() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Map/MainField/Static.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_mainfield_static() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Map/MainField/Static.mod.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_mainfield_static();
        let mstatic = super::Static::try_from(&byml).unwrap();
        let data = Byml::from(mstatic.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let mstatic2 = super::Static::try_from(&byml2).unwrap();
        assert_eq!(mstatic.general, mstatic2.general);
        assert_eq!(mstatic.start_pos, mstatic2.start_pos);
    }

    #[test]
    fn diff_mainfield() {
        let byml = load_mainfield_static();
        let mstatic = super::Static::try_from(&byml).unwrap();
        let byml2 = load_mod_mainfield_static();
        let mstatic2 = super::Static::try_from(&byml2).unwrap();
        let _diff = mstatic.diff(&mstatic2);
    }

    #[test]
    fn diff_cdungeon() {
        let byml = load_cdungeon_static();
        let mstatic = super::Static::try_from(&byml).unwrap();
        let byml2 = load_mod_cdungeon_static();
        let mstatic2 = super::Static::try_from(&byml2).unwrap();
        let _diff = mstatic.diff(&mstatic2);
    }

    #[test]
    fn merge_mainfield() {
        let byml = load_mainfield_static();
        let mstatic = super::Static::try_from(&byml).unwrap();
        let byml2 = load_mod_mainfield_static();
        let mstatic2 = super::Static::try_from(&byml2).unwrap();
        let diff = mstatic.diff(&mstatic2);
        let merged = mstatic.merge(&diff);
        assert_eq!(merged, mstatic2);
    }

    #[test]
    fn merge() {
        let byml = load_cdungeon_static();
        let static_ = super::Static::try_from(&byml).unwrap();
        let byml2 = load_mod_cdungeon_static();
        let static2 = super::Static::try_from(&byml2).unwrap();
        let diff = static_.diff(&static2);
        let merged = static_.merge(&diff);
        assert!(
            merged
                .start_pos
                .contains_key(&smartstring::alias::String::from("Dungeon200"))
        );
        assert_eq!(merged, static2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//Map/CDungeon/Static.smubin");
        assert!(super::Static::path_matches(path));
    }
}
