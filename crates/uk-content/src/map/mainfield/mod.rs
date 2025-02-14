use anyhow::Context;
use itertools::Itertools;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

pub mod collab_anchor;
pub mod korok_location;
pub mod location;
pub mod location_marker;
pub mod location_pointer;
pub mod non_auto_gen_area;
pub mod non_auto_placement;
pub mod restart_pos;
pub mod road_npc_rest_station;
pub mod start_pos;
pub mod static_grudge_location;
pub mod target_pos_marker;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MapUnit {
    pub row: String,
    pub col: u32,
}

impl<'a> FromIterator<&'a str> for MapUnit {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut it = iter.into_iter();
        let row = it.next()
            .expect("iter must have 2 elements");
        let col = it.next()
            .expect("iter must have 2 elements")
            .parse::<u32>()
            .expect("MapUnit third character must be number");
        Self {
            row: row.into(),
            col,
        }
    }
}

impl From<&str> for MapUnit {
    fn from(value: &str) -> Self {
        match value.len() {
            3 => value.split('-').collect(),
            _ => Self {
                row: value.into(),
                col: 0xFFFFFFFF,
            },
        }
    }
}

impl TryFrom<&Byml> for MapUnit {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => Ok(s.as_str().into()),
            Err(_) => Err(anyhow::anyhow!("MapUnit must be String")),
        }
    }
}

impl From<&MapUnit> for String {
    fn from(value: &MapUnit) -> Self {
        match value.col {
            0xFFFFFFFF => value.row.clone(),
            _ => format!("{}-{}", value.row, value.col).into(),
        }
    }
}

impl From<&MapUnit> for Byml {
    fn from(value: &MapUnit) -> Self {
        Byml::String(value.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Map {
    AocField,
    CDungeon,
    #[default]
    MainField,
    MainFieldDungeon,
}

impl std::fmt::Display for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&str> for Map {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "AocField" => Ok(Map::AocField),
            "CDungeon" => Ok(Map::CDungeon),
            "MainField" => Ok(Map::MainField),
            "MainFieldDungeon" => Ok(Map::MainFieldDungeon),
            _ => Err(anyhow::anyhow!("{} not valid Map", value)),
        }
    }
}

impl TryFrom<&Byml> for Map {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => s.as_str().try_into(),
            Err(_) => Err(anyhow::anyhow!("Map must be String")),
        }
    }
}

impl<'a> From<&Map> for &'a str {
    fn from(value: &Map) -> Self {
        match value {
            Map::AocField => "AocField",
            Map::CDungeon => "CDungeon",
            Map::MainField => "MainField",
            Map::MainFieldDungeon => "MainFieldDungeon",
        }
    }
}

impl From<&Map> for String {
    fn from(value: &Map) -> Self {
        value.to_string().into()
    }
}

impl From<&Map> for Byml {
    fn from(value: &Map) -> Self {
        Byml::String(value.into())
    }
}


#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MapAndUnit {
    pub map:    Map,
    pub unit:   MapUnit,
}

impl<'a> FromIterator<&'a str> for MapAndUnit {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut it = iter.into_iter();
        let map = it.next()
            .expect("iter must have 2 elements")
            .try_into()
            .expect("MapAndUnit first part must be Map");
        let unit = it.next()
            .expect("iter must have 2 elements")
            .try_into()
            .expect("MapAndUnit second part must be MapUnit");
        Self {
            map,
            unit,
        }
    }
}

impl TryFrom<&str> for MapAndUnit {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        let parts = value.split('/');
        match parts.try_len().unwrap() {
            2 => Ok(parts.collect()),
            _ => Err(anyhow::anyhow!("MapAndUnit must contain 2 parts: {}", parts.try_len().unwrap())),
        }
    }
}

impl TryFrom<&Byml> for MapAndUnit {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => Ok(s.split('/').collect()),
            Err(_) => Err(anyhow::anyhow!("MapAndUnit must be String")),
        }
    }
}

impl From<&MapAndUnit> for String {
    fn from(value: &MapAndUnit) -> Self {
        format!("{}/{}", String::from(&value.map), String::from(&value.unit)).into()
    }
}

impl From<&MapAndUnit> for Byml {
    fn from(value: &MapAndUnit) -> Self {
        Byml::String(value.into())
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum AreaShape {
    Box,
    Capsule,
    Cylinder,
    #[default]
    Sphere,
}

impl std::fmt::Display for AreaShape {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&Byml> for AreaShape {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => match s.as_str() {
                "Box" => Ok(AreaShape::Box),
                "Capsule" => Ok(AreaShape::Capsule),
                "Cylinder" => Ok(AreaShape::Cylinder),
                "Sphere" => Ok(AreaShape::Sphere),
                _ => Err(anyhow::anyhow!("{} not valid AreaShape", s)),
            },
            Err(_) => Err(anyhow::anyhow!("AreaShape must be String")),
        }
    }
}

impl<'a> From<&AreaShape> for &'a str {
    fn from(value: &AreaShape) -> Self {
        match value {
            AreaShape::Box => "Box",
            AreaShape::Capsule => "Capsule",
            AreaShape::Cylinder => "Cylinder",
            AreaShape::Sphere => "Sphere",
        }
    }
}

impl From<&AreaShape> for String {
    fn from(value: &AreaShape) -> Self {
        value.to_string().into()
    }
}

impl From<&AreaShape> for Byml {
    fn from(value: &AreaShape) -> Self {
        Byml::String(value.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ScaleTranslate {
    pub scale:      DeleteVec<(char, f32)>,
    pub translate:  DeleteVec<(char, f32)>,
}

impl TryFrom<&Byml> for ScaleTranslate {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map().context("ScaleTranslate node must be HashMap")?;
        Ok(Self {
            scale: map.get("Scale")
                .context("ScaleTranslate must have Scale")?
                .as_map()
                .context("Invalid ScaleTranslate Scale")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(c), Ok(f)) => Ok((c, f)),
                        (None, Ok(f)) => Err(anyhow::anyhow!("Invalid ScaleTranslate Scale with value {f}")),
                        (Some(c), Err(e)) => Err(anyhow::anyhow!("Invalid ScaleTranslate Scale {c}: {e}")),
                        (None, Err(e)) => Err(anyhow::anyhow!("Invalid ScaleTranslate Scale index {i}: {e}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
            translate: map.get("Translate")
                .context("ScaleTranslate must have Translate")?
                .as_map()
                .context("Invalid ScaleTranslate Translate")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(c), Ok(f)) => Ok((c, f)),
                        (None, Ok(f)) => Err(anyhow::anyhow!("Invalid ScaleTranslate Translate with value {f}")),
                        (Some(c), Err(e)) => Err(anyhow::anyhow!("Invalid ScaleTranslate Translate {c}: {e}")),
                        (None, Err(e)) => Err(anyhow::anyhow!("Invalid ScaleTranslate Translate index {i}: {e}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
        })
    }
}

impl From<ScaleTranslate> for Byml {
    fn from(val: ScaleTranslate) -> Self {
        map!(
            "Scale" => Byml::Map(val.scale
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
        )
    }
}

impl Mergeable for ScaleTranslate {
    fn diff(&self, other: &Self) -> Self {
        Self {
            scale: self.scale.diff(&other.scale),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            scale: self.scale.merge(&diff.scale),
            translate: self.scale.merge(&diff.translate),
        }
    }
}
