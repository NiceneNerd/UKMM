use anyhow::Context;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};

use super::MapAndUnit;

#[derive(Debug, Copy, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum LocationIcon {
    Castle,
    CheckPoint,
    Dungeon,
    Hatago,
    Labo,
    RemainsElectric,
    RemainsFire,
    RemainsWater,
    RemainsWind,
    ShopBougu,
    ShopColor,
    ShopJewel,
    ShopYadoya,
    ShopYorozu,
    StartPoint,
    Tower,
    Village,
}

impl std::fmt::Display for LocationIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&Byml> for LocationIcon {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => match s.as_str() {
                "Castle" => Ok(LocationIcon::Castle),
                "CheckPoint" => Ok(LocationIcon::CheckPoint),
                "Dungeon" => Ok(LocationIcon::Dungeon),
                "Hatago" => Ok(LocationIcon::Hatago),
                "Labo" => Ok(LocationIcon::Labo),
                "RemainsElectric" => Ok(LocationIcon::RemainsElectric),
                "RemainsFire" => Ok(LocationIcon::RemainsFire),
                "RemainsWater" => Ok(LocationIcon::RemainsWater),
                "RemainsWind" => Ok(LocationIcon::RemainsWind),
                "ShopBougu" => Ok(LocationIcon::ShopBougu),
                "ShopColor" => Ok(LocationIcon::ShopColor),
                "ShopJewel" => Ok(LocationIcon::ShopJewel),
                "ShopYadoya" => Ok(LocationIcon::ShopYadoya),
                "ShopYorozu" => Ok(LocationIcon::ShopYorozu),
                "StartPoint" => Ok(LocationIcon::StartPoint),
                "Tower" => Ok(LocationIcon::Tower),
                "Village" => Ok(LocationIcon::Village),
                _ => Err(anyhow::anyhow!("{} not valid LocationIcon", s)),
            },
            Err(_) => Err(anyhow::anyhow!("LocationIcon must be String")),
        }
    }
}

impl<'a> From<&LocationIcon> for &'a str {
    fn from(value: &LocationIcon) -> Self {
        match value {
            LocationIcon::Castle => "Castle",
            LocationIcon::CheckPoint => "CheckPoint",
            LocationIcon::Dungeon => "Dungeon",
            LocationIcon::Hatago => "Hatago",
            LocationIcon::Labo => "Labo",
            LocationIcon::RemainsElectric => "RemainsElectric",
            LocationIcon::RemainsFire => "RemainsFire",
            LocationIcon::RemainsWater => "RemainsWater",
            LocationIcon::RemainsWind => "RemainsWind",
            LocationIcon::ShopBougu => "ShopBougu",
            LocationIcon::ShopColor => "ShopColor",
            LocationIcon::ShopJewel => "ShopJewel",
            LocationIcon::ShopYadoya => "ShopYadoya",
            LocationIcon::ShopYorozu => "ShopYorozu",
            LocationIcon::StartPoint => "StartPoint",
            LocationIcon::Tower => "Tower",
            LocationIcon::Village => "Village",
        }
    }
}

impl From<&LocationIcon> for String {
    fn from(value: &LocationIcon) -> Self {
        value.to_string().into()
    }
}

impl From<&LocationIcon> for Byml {
    fn from(value: &LocationIcon) -> Self {
        Byml::String(value.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LocationMarker {
    pub icon:               Option<LocationIcon>,
    pub message_id:         Option<String>,
    pub priority:           Option<i32>,
    pub save_flag:          Option<String>,
    pub translate:          DeleteMap<char, f32>,
    pub warp_dest_map_name: Option<MapAndUnit>,
    pub warp_dest_pos_name: Option<String>,
}

impl TryFrom<&Byml> for LocationMarker {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            icon: map.get("Icon")
                .map(|b| b.try_into()
                    .context("LocationMarker Icon invalid")
                )
                .transpose()?,
            message_id: map.get("MessageID")
                .map(|b| b.as_string()
                    .context("LocationMarker MessageID must be String")
                )
                .transpose()?
                .map(|s| s.clone()),
            priority: Some(map.get("Priority")
                .context("LocationMarker must have Priority")?
                .as_i32()
                .context("LocationMarker Priority must be Int")?),
            save_flag: Some(map.get("SaveFlag")
                .context("LocationMarker must have SaveFlag")?
                .as_string()
                .context("LocationMarker SaveFlag must be String")?
                .clone()),
            translate: try_get_vecf(map.get("Translate")
                .context("LocationMarker must have Translate")?)
                .context("Invalid LocationMarker Translate")?,
            warp_dest_map_name: map.get("WarpDestMapName")
                .map(|b| b.try_into()
                    .context("Invalid LocationMarker WarpDestMapName")
                )
                .transpose()?,
            warp_dest_pos_name: map.get("WarpDestPosName")
                .map(|b| b.as_string()
                    .context("LocationMarker WarpDestPosName must be String")
                )
                .transpose()?
                .map(|s| s.clone()),
        })
    }
}

impl From<LocationMarker> for Byml {
    fn from(value: LocationMarker) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        match &value.icon {
            Some(i) => map.insert("Icon".into(), i.into()),
            None => None,
        };
        match &value.message_id {
            Some(i) => map.insert("MessageID".into(), i.into()),
            None => None,
        };
        map.insert("Priority".into(), value.priority.unwrap().into());
        map.insert("SaveFlag".into(), value.save_flag.unwrap().into());
        map.insert("Translate".into(), Byml::Map(value.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        match &value.warp_dest_map_name {
            Some(i) => map.insert("WarpDestMapName".into(), i.into()),
            None => None,
        };
        match &value.warp_dest_pos_name {
            Some(i) => map.insert("WarpDestPosName".into(), i.into()),
            None => None,
        };
        Byml::Map(map)
    }
}

impl Mergeable for LocationMarker {
    fn diff(&self, other: &Self) -> Self {
        Self {
            icon: other.icon
                .ne(&self.icon)
                .then(|| other.icon)
                .unwrap(),
            message_id: other.message_id
                .ne(&self.message_id)
                .then(|| other.message_id.clone())
                .unwrap(),
            priority: other.priority
                .ne(&self.priority)
                .then(|| other.priority)
                .unwrap(),
            save_flag: other.save_flag
                .ne(&self.save_flag)
                .then(|| other.save_flag.clone())
                .unwrap(),
            translate: self.translate.diff(&other.translate),
            warp_dest_map_name: other.warp_dest_map_name
                .ne(&self.warp_dest_map_name)
                .then(|| other.warp_dest_map_name.clone())
                .unwrap(),
            warp_dest_pos_name: other.warp_dest_pos_name
                .ne(&self.warp_dest_pos_name)
                .then(|| other.warp_dest_pos_name.clone())
                .unwrap(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            icon: diff.icon
                .eq(&self.icon)
                .then(|| self.icon)
                .or_else(|| Some(diff.icon))
                .unwrap(),
            message_id: diff.message_id
                .eq(&self.message_id)
                .then(|| self.message_id.clone())
                .or_else(|| Some(diff.message_id.clone()))
                .unwrap(),
            priority: diff.priority
                .eq(&self.priority)
                .then(|| self.priority)
                .or_else(|| Some(diff.priority))
                .unwrap(),
            save_flag: diff.save_flag
                .eq(&self.save_flag)
                .then(|| self.save_flag.clone())
                .or_else(|| Some(diff.save_flag.clone()))
                .unwrap(),
            translate: self.translate.merge(&diff.translate),
            warp_dest_map_name: diff.warp_dest_map_name
                .eq(&self.warp_dest_map_name)
                .then(|| self.warp_dest_map_name.clone())
                .or_else(|| Some(diff.warp_dest_map_name.clone()))
                .unwrap(),
            warp_dest_pos_name: diff.warp_dest_pos_name
                .eq(&self.warp_dest_pos_name)
                .then(|| self.warp_dest_pos_name.clone())
                .or_else(|| Some(diff.warp_dest_pos_name.clone()))
                .unwrap(),
        }
    }
}
