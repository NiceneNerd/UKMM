use crate::{
    map::EntryPos,
    prelude::Mergeable,
    util::{DeleteMap, DeleteVec, SortedDeleteMap},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Static {
    pub general: BTreeMap<String, DeleteVec<Byml>>,
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
                            map_entries.insert(pos_name, EntryPos { rotate, translate });
                        } else {
                            entry_map.insert(
                                map,
                                [(pos_name, EntryPos { rotate, translate })]
                                    .into_iter()
                                    .collect(),
                            );
                        };
                        Ok(entry_map)
                    },
                )?,
            general: byml
                .as_hash()?
                .iter()
                .map(|(key, array)| -> Result<(String, DeleteVec<Byml>)> {
                    Ok((key.to_owned(), array.as_array()?.iter().cloned().collect()))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<Static> for Byml {
    fn from(val: Static) -> Self {
        [(
            "StartPos".to_owned(),
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
