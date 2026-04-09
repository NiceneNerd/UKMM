use anyhow::Context;
use itertools::Itertools;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap}};

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

impl From<PlacementType> for &str {
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
    pub translate:                      DeleteMap<char, f32>,
}

impl KorokLocation {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.flag.clone().unwrap_or_default()
            )
        )
        .to_string()
        .into()
    }

    pub fn is_complete(&self) -> bool {
        self.flag.is_some() &&
            self.hidden_korok_body_color.is_some() &&
            self.hidden_korok_left_plant_type.is_some() &&
            self.hidden_korok_mask_type.is_some() &&
            self.hidden_korok_right_plant_type.is_some() &&
            self.is_appear_check.is_some() &&
            self.is_hidden_korok_lift_appear.is_some() &&
            self.is_invisible_korok.is_some() &&
            self.korok_event_start_wait_frame.is_some() &&
            self.placement_type.is_some() &&
            self.rail_move_speed.is_some() &&
            self.territory_area.is_some() &&
            self.translate.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z')
    }
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
            translate: try_get_vecf(map.get("Translate")
                .context("KorokLocation must have Translate")?)
                .context("Invalid KorokLocation Translate")?,
        })
    }
}

impl From<KorokLocation> for Byml {
    fn from(val: KorokLocation) -> Self {
        map!{
            "Flag" => val.flag.expect("Flag should have been read on diff").into(),
            "HiddenKorokBodyColor" => val.hidden_korok_body_color.expect("HiddenKorokBodyColor should have been read on diff").into(),
            "HiddenKorokLeftPlantType" => val.hidden_korok_left_plant_type.expect("HiddenKorokLeftPlantType should have been read on diff").into(),
            "HiddenKorokMaskType" => val.hidden_korok_mask_type.expect("HiddenKorokMaskType should have been read on diff").into(),
            "HiddenKorokRightPlantType" => val.hidden_korok_right_plant_type.expect("HiddenKorokRightPlantType should have been read on diff").into(),
            "IsAppearCheck" => val.is_appear_check.expect("IsAppearCheck should have been read on diff").into(),
            "IsHiddenKorokLiftAppear" => val.is_hidden_korok_lift_appear.expect("IsHiddenKorokLiftAppear should have been read on diff").into(),
            "IsInvisibleKorok" => val.is_invisible_korok.expect("IsInvisibleKorok should have been read on diff").into(),
            "KorokEventStartWaitFrame" => val.korok_event_start_wait_frame.expect("KorokEventStartWaitFrame should have been read on diff").into(),
            "PlacementType" => val.placement_type.expect("PlacementType should have been read on diff").into(),
            "RailMoveSpeed" => val.rail_move_speed.expect("RailMoveSpeed should have been read on diff").into(),
            "TerritoryArea" => val.territory_area.expect("TerritoryArea should have been read on diff").into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
        }
    }
}

impl Mergeable for KorokLocation {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            flag: other.flag
                .ne(&self.flag)
                .then(|| other.flag.clone())
                .unwrap_or_default(),
            hidden_korok_body_color: other.hidden_korok_body_color
                .ne(&self.hidden_korok_body_color)
                .then_some(other.hidden_korok_body_color)
                .unwrap_or_default(),
            hidden_korok_left_plant_type: other.hidden_korok_left_plant_type
                .ne(&self.hidden_korok_left_plant_type)
                .then_some(other.hidden_korok_left_plant_type)
                .unwrap_or_default(),
            hidden_korok_mask_type: other.hidden_korok_mask_type
                .ne(&self.hidden_korok_mask_type)
                .then_some(other.hidden_korok_mask_type)
                .unwrap_or_default(),
            hidden_korok_right_plant_type: other.hidden_korok_right_plant_type
                .ne(&self.hidden_korok_right_plant_type)
                .then_some(other.hidden_korok_right_plant_type)
                .unwrap_or_default(),
            is_appear_check: other.is_appear_check
                .ne(&self.is_appear_check)
                .then_some(other.is_appear_check)
                .unwrap_or_default(),
            is_hidden_korok_lift_appear: other.is_hidden_korok_lift_appear
                .ne(&self.is_hidden_korok_lift_appear)
                .then_some(other.is_hidden_korok_lift_appear)
                .unwrap_or_default(),
            is_invisible_korok: other.is_invisible_korok
                .ne(&self.is_invisible_korok)
                .then_some(other.is_invisible_korok)
                .unwrap_or_default(),
            korok_event_start_wait_frame: other.korok_event_start_wait_frame
                .ne(&self.korok_event_start_wait_frame)
                .then_some(other.korok_event_start_wait_frame)
                .unwrap_or_default(),
            placement_type: other.placement_type
                .ne(&self.placement_type)
                .then_some(other.placement_type)
                .unwrap_or_default(),
            rail_move_speed: other.rail_move_speed
                .ne(&self.rail_move_speed)
                .then_some(other.rail_move_speed)
                .unwrap_or_default(),
            territory_area: other.territory_area
                .ne(&self.territory_area)
                .then_some(other.territory_area)
                .unwrap_or_default(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            flag: diff.flag.clone()
                .or_else(|| self.flag.clone()),
            hidden_korok_body_color: diff.hidden_korok_body_color
                .or(self.hidden_korok_body_color),
            hidden_korok_left_plant_type: diff.hidden_korok_left_plant_type
                .or(self.hidden_korok_left_plant_type),
            hidden_korok_mask_type: diff.hidden_korok_mask_type
                .or(self.hidden_korok_mask_type),
            hidden_korok_right_plant_type: diff.hidden_korok_right_plant_type
                .or(self.hidden_korok_right_plant_type),
            is_appear_check: diff.is_appear_check
                .or(self.is_appear_check),
            is_hidden_korok_lift_appear: diff.is_hidden_korok_lift_appear
                .or(self.is_hidden_korok_lift_appear),
            is_invisible_korok: diff.is_invisible_korok
                .or(self.is_invisible_korok),
            korok_event_start_wait_frame: diff.korok_event_start_wait_frame
                .or(self.korok_event_start_wait_frame),
            placement_type: diff.placement_type
                .or(self.placement_type),
            rail_move_speed: diff.rail_move_speed
                .or(self.rail_move_speed),
            territory_area: diff.territory_area
                .or(self.territory_area),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
