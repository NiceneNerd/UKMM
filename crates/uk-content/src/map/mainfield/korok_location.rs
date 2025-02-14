use anyhow::Context;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

#[derive(Debug, Copy, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlacementType {
    #[default]
    Ground,
    Air,
}

impl std::fmt::Display for PlacementType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&Byml> for PlacementType {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => match s.as_str() {
                "Ground" => Ok(PlacementType::Ground),
                "Air" => Ok(PlacementType::Air),
                _ => Err(anyhow::anyhow!("{} not valid PlacementType", s)),
            },
            Err(_) => Err(anyhow::anyhow!("PlacementType must be String")),
        }
    }
}

impl<'a> From<PlacementType> for &'a str {
    fn from(value: PlacementType) -> Self {
        match value {
            PlacementType::Ground => "Ground",
            PlacementType::Air => "Air",
        }
    }
}

impl From<PlacementType> for String {
    fn from(value: PlacementType) -> Self {
        value.to_string().into()
    }
}

impl From<PlacementType> for Byml {
    fn from(value: PlacementType) -> Self {
        Byml::String(value.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct KorokLocation {
    pub flag:                           Option<String>,
    pub hidden_korok_body_color:        Option<i32>,
    pub hidden_korok_left_plant_type:   Option<i32>,
    pub hidden_korok_mask_type:         Option<i32>,
    pub hidden_korok_right_plant_type:  Option<i32>,
    pub is_appear_check:                Option<bool>,
    pub is_hidden_korok_lift_appear:    Option<bool>,
    pub is_invisible_korok:             Option<bool>,
    pub korok_event_start_wait_frame:   Option<i32>,
    pub placement_type:                 Option<PlacementType>,
    pub rail_move_speed:                Option<f32>,
    pub territory_area:                 Option<f32>,
    pub translate:                      DeleteVec<(char, f32)>,
}

impl TryFrom<&Byml> for KorokLocation {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            flag: Some(map.get("Flag")
                .context("KorokLocation must have Flag")?
                .as_string()
                .context("KorokLocation Flag must be String")?
                .clone()),
            hidden_korok_body_color: Some(map.get("HiddenKorokBodyColor")
                .context("KorokLocation must have HiddenKorokBodyColor")?
                .as_i32()
                .context("KorokLocation HiddenKorokBodyColor must be Int")?),
            hidden_korok_left_plant_type: Some(map.get("HiddenKorokLeftPlantType")
                .context("KorokLocation must have HiddenKorokLeftPlantType")?
                .as_i32()
                .context("KorokLocation HiddenKorokLeftPlantType must be Int")?),
            hidden_korok_mask_type: Some(map.get("HiddenKorokMaskType")
                .context("KorokLocation must have HiddenKorokMaskType")?
                .as_i32()
                .context("KorokLocation HiddenKorokMaskType must be Int")?),
            hidden_korok_right_plant_type: Some(map.get("HiddenKorokRightPlantType")
                .context("KorokLocation must have HiddenKorokRightPlantType")?
                .as_i32()
                .context("KorokLocation HiddenKorokRightPlantType must be Int")?),
            is_appear_check: Some(map.get("IsAppearCheck")
                .context("KorokLocation must have IsAppearCheck")?
                .as_bool()
                .context("KorokLocation IsAppearCheck must be Bool")?),
            is_hidden_korok_lift_appear: Some(map.get("IsHiddenKorokLiftAppear")
                .context("KorokLocation must have IsHiddenKorokLiftAppear")?
                .as_bool()
                .context("KorokLocation IsHiddenKorokLiftAppear must be Bool")?),
            is_invisible_korok: Some(map.get("IsInvisibleKorok")
                .context("KorokLocation must have IsInvisibleKorok")?
                .as_bool()
                .context("KorokLocation IsInvisibleKorok must be Bool")?),
            korok_event_start_wait_frame: Some(map.get("KorokEventStartWaitFrame")
                .context("KorokLocation must have KorokEventStartWaitFrame")?
                .as_i32()
                .context("KorokLocation KorokEventStartWaitFrame must be Int")?),
            placement_type: Some(map.get("PlacementType")
                .context("KorokLocation must have PlacementType")?
                .try_into()
                .context("Invalid KorokLocation PlacementType")?),
            rail_move_speed: Some(map.get("RailMoveSpeed")
                .context("KorokLocation must have RailMoveSpeed")?
                .as_float()
                .context("KorokLocation RailMoveSpeed must be Float")?),
            territory_area: Some(map.get("TerritoryArea")
                .context("KorokLocation must have TerritoryArea")?
                .as_float()
                .context("KorokLocation TerritoryArea must be Float")?),
            translate: map.get("Translate")
                .context("KorokLocation must have Translate")?
                .as_map()
                .context("Invalid KorokLocation Translate")?
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().context("Invalid Float").unwrap()
                ))
                .collect::<DeleteVec<_>>(),
        })
    }
}

impl From<KorokLocation> for Byml {
    fn from(val: KorokLocation) -> Self {
        map!{
            "Flag" => val.flag.unwrap().into(),
            "HiddenKorokBodyColor" => val.hidden_korok_body_color.unwrap().into(),
            "HiddenKorokLeftPlantType" => val.hidden_korok_left_plant_type.unwrap().into(),
            "HiddenKorokMaskType" => val.hidden_korok_mask_type.unwrap().into(),
            "HiddenKorokRightPlantType" => val.hidden_korok_right_plant_type.unwrap().into(),
            "IsAppearCheck" => val.is_appear_check.unwrap().into(),
            "IsHiddenKorokLiftAppear" => val.is_hidden_korok_lift_appear.unwrap().into(),
            "IsInvisibleKorok" => val.is_invisible_korok.unwrap().into(),
            "KorokEventStartWaitFrame" => val.korok_event_start_wait_frame.unwrap().into(),
            "PlacementType" => val.placement_type.unwrap().into(),
            "RailMoveSpeed" => val.rail_move_speed.unwrap().into(),
            "TerritoryArea" => val.territory_area.unwrap().into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
        }
    }
}

impl Mergeable for KorokLocation {
    fn diff(&self, other: &Self) -> Self {
        Self {
            flag: other.flag
                .ne(&self.flag)
                .then(|| other.flag.clone())
                .unwrap(),
            hidden_korok_body_color: other.hidden_korok_body_color
                .ne(&self.hidden_korok_body_color)
                .then(|| other.hidden_korok_body_color)
                .unwrap(),
            hidden_korok_left_plant_type: other.hidden_korok_left_plant_type
                .ne(&self.hidden_korok_left_plant_type)
                .then(|| other.hidden_korok_left_plant_type)
                .unwrap(),
            hidden_korok_mask_type: other.hidden_korok_mask_type
                .ne(&self.hidden_korok_mask_type)
                .then(|| other.hidden_korok_mask_type)
                .unwrap(),
            hidden_korok_right_plant_type: other.hidden_korok_right_plant_type
                .ne(&self.hidden_korok_right_plant_type)
                .then(|| other.hidden_korok_right_plant_type)
                .unwrap(),
            is_appear_check: other.is_appear_check
                .ne(&self.is_appear_check)
                .then(|| other.is_appear_check)
                .unwrap(),
            is_hidden_korok_lift_appear: other.is_hidden_korok_lift_appear
                .ne(&self.is_hidden_korok_lift_appear)
                .then(|| other.is_hidden_korok_lift_appear)
                .unwrap(),
            is_invisible_korok: other.is_invisible_korok
                .ne(&self.is_invisible_korok)
                .then(|| other.is_invisible_korok)
                .unwrap(),
            korok_event_start_wait_frame: other.korok_event_start_wait_frame
                .ne(&self.korok_event_start_wait_frame)
                .then(|| other.korok_event_start_wait_frame)
                .unwrap(),
            placement_type: other.placement_type
                .ne(&self.placement_type)
                .then(|| other.placement_type)
                .unwrap(),
            rail_move_speed: other.rail_move_speed
                .ne(&self.rail_move_speed)
                .then(|| other.rail_move_speed)
                .unwrap(),
            territory_area: other.territory_area
                .ne(&self.territory_area)
                .then(|| other.territory_area)
                .unwrap(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            flag: diff.flag
                .eq(&self.flag)
                .then(|| self.flag.clone())
                .or_else(|| Some(diff.flag.clone()))
                .unwrap(),
            hidden_korok_body_color: diff.hidden_korok_body_color
                .eq(&self.hidden_korok_body_color)
                .then(|| self.hidden_korok_body_color)
                .or_else(|| Some(diff.hidden_korok_body_color))
                .unwrap(),
            hidden_korok_left_plant_type: diff.hidden_korok_left_plant_type
                .eq(&self.hidden_korok_left_plant_type)
                .then(|| self.hidden_korok_left_plant_type)
                .or_else(|| Some(diff.hidden_korok_left_plant_type))
                .unwrap(),
            hidden_korok_mask_type: diff.hidden_korok_mask_type
                .eq(&self.hidden_korok_mask_type)
                .then(|| self.hidden_korok_mask_type)
                .or_else(|| Some(diff.hidden_korok_mask_type))
                .unwrap(),
            hidden_korok_right_plant_type: diff.hidden_korok_right_plant_type
                .eq(&self.hidden_korok_right_plant_type)
                .then(|| self.hidden_korok_right_plant_type)
                .or_else(|| Some(diff.hidden_korok_right_plant_type))
                .unwrap(),
            is_appear_check: diff.is_appear_check
                .eq(&self.is_appear_check)
                .then(|| self.is_appear_check)
                .or_else(|| Some(diff.is_appear_check))
                .unwrap(),
            is_hidden_korok_lift_appear: diff.is_hidden_korok_lift_appear
                .eq(&self.is_hidden_korok_lift_appear)
                .then(|| self.is_hidden_korok_lift_appear)
                .or_else(|| Some(diff.is_hidden_korok_lift_appear))
                .unwrap(),
            is_invisible_korok: diff.is_invisible_korok
                .eq(&self.is_invisible_korok)
                .then(|| self.is_invisible_korok)
                .or_else(|| Some(diff.is_invisible_korok))
                .unwrap(),
            korok_event_start_wait_frame: diff.korok_event_start_wait_frame
                .eq(&self.korok_event_start_wait_frame)
                .then(|| self.korok_event_start_wait_frame)
                .or_else(|| Some(diff.korok_event_start_wait_frame))
                .unwrap(),
            placement_type: diff.placement_type
                .eq(&self.placement_type)
                .then(|| self.placement_type)
                .or_else(|| Some(diff.placement_type))
                .unwrap(),
            rail_move_speed: diff.rail_move_speed
                .eq(&self.rail_move_speed)
                .then(|| self.rail_move_speed)
                .or_else(|| Some(diff.rail_move_speed))
                .unwrap(),
            territory_area: diff.territory_area
                .eq(&self.territory_area)
                .then(|| self.territory_area)
                .or_else(|| Some(diff.territory_area))
                .unwrap(),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
