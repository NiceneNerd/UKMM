use std::collections::BTreeMap;

use anyhow::Context;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    util::{DeleteMap, DeleteVec},
    Result, UKError,
};

use super::mainfield::{
    ScaleTranslate,
    collab_anchor::CollabAnchor,
    korok_location::KorokLocation,
    location_marker::LocationMarker,
    location_pointer::LocationPointer,
    non_auto_gen_area::NonAutoGenArea,
    non_auto_placement::NonAutoPlacement,
    restart_pos::RestartPos,
    road_npc_rest_station::RoadNpcRestStation,
    start_pos::StartPos,
    static_grudge_location::StaticGrudgeLocation,
    target_pos_marker::TargetPosMarker,
};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]

pub struct EntryPos {
    pub rotate: Byml,
    pub translate: Byml,
    pub player_state: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct Static {
    pub general:   BTreeMap<String, DeleteVec<Byml>>,
    pub start_pos: DeleteMap<String, DeleteMap<String, EntryPos>>,
}

impl TryFrom<&Byml> for Static {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self {
            start_pos: byml
                .as_map()?
                .get("StartPos")
                .ok_or(UKError::MissingBymlKey("CDungeon static missing StartPos"))?
                .as_array()?
                .iter()
                .try_fold(
                    DeleteMap::new(),
                    |mut entry_map,
                     entry|
                     -> Result<DeleteMap<String, DeleteMap<String, EntryPos>>> {
                        let entry = entry.as_map()?;
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
                .as_map()?
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

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct MainStatic {
    pub dlc_restart_pos:            Option<DeleteMap<String, RestartPos>>,
    pub collab_anchor:              DeleteMap<String, CollabAnchor>,
    pub korok_location:             DeleteMap<String, KorokLocation>,
    pub location_marker:            DeleteMap<String, LocationMarker>,
    pub location_pointer:           DeleteMap<String, LocationPointer>,
    pub non_auto_gen_area:          DeleteMap<String, NonAutoGenArea>,
    pub non_auto_placement:         DeleteMap<String, NonAutoPlacement>,
    pub road_npc_rest_station:      DeleteMap<String, RoadNpcRestStation>,
    pub start_pos:                  DeleteMap<String, StartPos>,
    pub static_grudge_location:     DeleteMap<String, StaticGrudgeLocation>,
    pub target_pos_marker:          DeleteMap<String, TargetPosMarker>,
    pub tera_water_disable:         DeleteMap<String, ScaleTranslate>,
    pub terrain_hide_center_tag:    DeleteMap<String, ScaleTranslate>,
}

impl TryFrom<&Byml> for MainStatic {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let root_map = byml.as_map()?;
        Ok(Self {
            dlc_restart_pos: root_map
                .get("DLCRestartPos")
                .map_or(None, |b| {
                    Some(b.as_array()
                        .expect("Invalid DLCRestartPos")
                        .iter()
                        .enumerate()
                        .map(|(index, entry)| {
                                let entry: RestartPos = entry.try_into()
                                    .with_context(|| format!("Could not read RestartPos {}", index))?;
                                Ok((entry.id(), entry))
                            },
                        )
                        .collect::<Result<DeleteMap<_, _>>>(),
                    )
                })
                .transpose()?,
            collab_anchor: root_map
                .get("FldObj_DLC_ShootingStarCollaborationAnchor")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing FldObj_DLC_ShootingStarCollaborationAnchor"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: CollabAnchor = entry.try_into()
                            .with_context(|| format!("Could not read CollabAnchor {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            korok_location: root_map
                .get("KorokLocation")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing KorokLocation"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: KorokLocation = entry.try_into()
                            .with_context(|| format!("Could not read KorokLocation {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            location_marker: root_map
                .get("LocationMarker")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing LocationMarker"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: LocationMarker = entry.try_into()
                            .with_context(|| format!("Could not read LocationMarker {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            location_pointer: root_map
                .get("LocationPointer")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing LocationPointer"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: LocationPointer = entry.try_into()
                            .with_context(|| format!("Could not read LocationPointer {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            non_auto_gen_area: root_map
                .get("NonAutoGenArea")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing NonAutoGenArea"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: NonAutoGenArea = entry.try_into()
                            .with_context(|| format!("Could not read NonAutoGenArea {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            non_auto_placement: root_map
                .get("NonAutoPlacement")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing NonAutoPlacement"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: NonAutoPlacement = entry.try_into()
                            .with_context(|| format!("Could not read NonAutoPlacement {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            road_npc_rest_station: root_map
                .get("RoadNpcRestStation")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing RoadNpcRestStation"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: RoadNpcRestStation = entry.try_into()
                            .with_context(|| format!("Could not read RoadNpcRestStation {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            start_pos: root_map
                .get("StartPos")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing StartPos"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: StartPos = entry.try_into()
                            .with_context(|| format!("Could not read StartPos {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            static_grudge_location: root_map
                .get("StaticGrudgeLocation")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing StaticGrudgeLocation"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: StaticGrudgeLocation = entry.try_into()
                            .with_context(|| format!("Could not read StaticGrudgeLocation {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            target_pos_marker: root_map
                .get("TargetPosMarker")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing TargetPosMarker"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: TargetPosMarker = entry.try_into()
                            .with_context(|| format!("Could not read TargetPosMarker {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            tera_water_disable: root_map
                .get("TeraWaterDisable")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing TeraWaterDisable"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: ScaleTranslate = entry.try_into()
                            .with_context(|| format!("Could not read ScaleTranslate {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
            terrain_hide_center_tag: root_map
                .get("TerrainHideCenterTag")
                .ok_or(UKError::MissingBymlKey(
                    "MainField static missing TerrainHideCenterTag"
                ))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                        let entry: ScaleTranslate = entry.try_into()
                            .with_context(|| format!("Could not read ScaleTranslate {}", index))?;
                        Ok((entry.id(), entry))
                    },
                )
                .collect::<Result<DeleteMap<_, _>>>()?,
        })
    }
}

impl From<MainStatic> for Byml {
    fn from(val: MainStatic) -> Self {
        val.dlc_restart_pos
            .map_or(Vec::<(String, Byml)>::new(), |d| [(
                String::from("DLCRestartPos"),
                Byml::Array(d.into_iter()
                    .map(|(_, entry)| entry.into())
                    .collect::<Vec<Byml>>())
            )].into())
            .into_iter()
            .chain(
                [(
                    "FldObj_DLC_ShootingStarCollaborationAnchor".into(),
                    val.collab_anchor
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "KorokLocation".into(),
                    val.korok_location
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "LocationMarker".into(),
                    val.location_marker
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "LocationPointer".into(),
                    val.location_pointer
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "NonAutoGenArea".into(),
                    val.non_auto_gen_area
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "NonAutoPlacement".into(),
                    val.non_auto_placement
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "RoadNpcRestStation".into(),
                    val.road_npc_rest_station
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "StartPos".into(),
                    val.start_pos
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "StaticGrudgeLocation".into(),
                    val.static_grudge_location
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "TargetPosMarker".into(),
                    val.target_pos_marker
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "TeraWaterDisable".into(),
                    val.tera_water_disable
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .chain(
                [(
                    "TerrainHideCenterTag".into(),
                    val.terrain_hide_center_tag
                        .into_iter()
                        .map(|(_, entry)| entry.into())
                        .collect::<Vec<_>>()
                        .into(),
                )]
            )
            .collect::<crate::util::HashMap<String, Byml>>()
            .into()
    }
}

impl Mergeable for MainStatic {
    fn diff(&self, other: &Self) -> Self {
        let dlc_restart_pos = match &other.dlc_restart_pos {
            Some(b) => match &self.dlc_restart_pos {
                Some(a) => Some(a.deep_diff(b)),
                None => Some(b.clone()),
            },
            None => self.dlc_restart_pos.clone(),
        };
        Self {
            dlc_restart_pos,
            collab_anchor: self.collab_anchor.deep_diff(&other.collab_anchor),
            korok_location: self.korok_location.deep_diff(&other.korok_location),
            location_marker: self.location_marker.deep_diff(&other.location_marker),
            location_pointer: self.location_pointer.deep_diff(&other.location_pointer),
            non_auto_gen_area: self.non_auto_gen_area.deep_diff(&other.non_auto_gen_area),
            non_auto_placement: self.non_auto_placement.deep_diff(&other.non_auto_placement),
            road_npc_rest_station: self.road_npc_rest_station.deep_diff(&other.road_npc_rest_station),
            start_pos: self.start_pos.deep_diff(&other.start_pos),
            static_grudge_location: self.static_grudge_location.deep_diff(&other.static_grudge_location),
            target_pos_marker: self.target_pos_marker.deep_diff(&other.target_pos_marker),
            tera_water_disable: self.tera_water_disable.deep_diff(&other.tera_water_disable),
            terrain_hide_center_tag: self.terrain_hide_center_tag.deep_diff(&other.terrain_hide_center_tag),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        let dlc_restart_pos = match &diff.dlc_restart_pos {
            Some(b) => match &self.dlc_restart_pos {
                Some(a) => Some(a.deep_merge(b)),
                None => Some(b.clone()),
            },
            None => self.dlc_restart_pos.clone(),
        };
        Self {
            dlc_restart_pos,
            collab_anchor: self.collab_anchor.deep_merge(&diff.collab_anchor),
            korok_location: self.korok_location.deep_merge(&diff.korok_location),
            location_marker: self.location_marker.deep_merge(&diff.location_marker),
            location_pointer: self.location_pointer.deep_merge(&diff.location_pointer),
            non_auto_gen_area: self.non_auto_gen_area.deep_merge(&diff.non_auto_gen_area),
            non_auto_placement: self.non_auto_placement.deep_merge(&diff.non_auto_placement),
            road_npc_rest_station: self.road_npc_rest_station.deep_merge(&diff.road_npc_rest_station),
            start_pos: self.start_pos.deep_merge(&diff.start_pos),
            static_grudge_location: self.static_grudge_location.deep_merge(&diff.static_grudge_location),
            target_pos_marker: self.target_pos_marker.deep_merge(&diff.target_pos_marker),
            tera_water_disable: self.tera_water_disable.deep_merge(&diff.tera_water_disable),
            terrain_hide_center_tag: self.terrain_hide_center_tag.deep_merge(&diff.terrain_hide_center_tag),
        }
    }
}

impl Resource for MainStatic {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .with_extension("")
            .ends_with("MainField/Static")
    }
}

#[allow(clippy::unwrap_used)]
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
        let mstatic = super::MainStatic::try_from(&byml).unwrap();
        let data = Byml::from(mstatic.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let mstatic2 = super::MainStatic::try_from(&byml2).unwrap();
        assert_eq!(mstatic.collab_anchor, mstatic2.collab_anchor);
        assert_eq!(mstatic.korok_location, mstatic2.korok_location);
        assert_eq!(mstatic.location_marker, mstatic2.location_marker);
        assert_eq!(mstatic.location_pointer, mstatic2.location_pointer);
        assert_eq!(mstatic.non_auto_gen_area, mstatic2.non_auto_gen_area);
        assert_eq!(mstatic.non_auto_placement, mstatic2.non_auto_placement);
        assert_eq!(mstatic.road_npc_rest_station, mstatic2.road_npc_rest_station);
        assert_eq!(mstatic.start_pos, mstatic2.start_pos);
        assert_eq!(mstatic.static_grudge_location, mstatic2.static_grudge_location);
        assert_eq!(mstatic.target_pos_marker, mstatic2.target_pos_marker);
        assert_eq!(mstatic.tera_water_disable, mstatic2.tera_water_disable);
        assert_eq!(mstatic.terrain_hide_center_tag, mstatic2.terrain_hide_center_tag);
    }

    #[test]
    fn diff_mainfield() {
        let byml = load_mainfield_static();
        let mstatic = super::MainStatic::try_from(&byml).unwrap();
        let byml2 = load_mod_mainfield_static();
        let mstatic2 = super::MainStatic::try_from(&byml2).unwrap();
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
        let mstatic = super::MainStatic::try_from(&byml).unwrap();
        let byml2 = load_mod_mainfield_static();
        let mstatic2 = super::MainStatic::try_from(&byml2).unwrap();
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
                .contains_key(smartstring::alias::String::from("Dungeon200"))
        );
        assert_eq!(merged, static2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//Map/CDungeon/Static.smubin");
        assert!(super::Static::path_matches(path));
    }
}
