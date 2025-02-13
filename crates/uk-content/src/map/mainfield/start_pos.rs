use anyhow::Context;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{DeleteVec, HashMap}};

use super::MapUnit;

#[derive(Debug, Copy, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PlayerState {
    Guard,
    Wait,
    WaitAttentionUpper,
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&str> for PlayerState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Guard" => Ok(PlayerState::Guard),
            "Wait" => Ok(PlayerState::Wait),
            "WaitAttentionUpper" => Ok(PlayerState::WaitAttentionUpper),
            _ => Err(anyhow::anyhow!("{} not valid PlayerState", value)),
        }
    }
}

impl TryFrom<&Byml> for PlayerState {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        match value.as_string() {
            Ok(s) => s.as_str().try_into(),
            Err(_) => Err(anyhow::anyhow!("PlayerState must be String")),
        }
    }
}

impl<'a> From<&PlayerState> for &'a str {
    fn from(value: &PlayerState) -> Self {
        match value {
            PlayerState::Guard => "Guard",
            PlayerState::Wait => "Wait",
            PlayerState::WaitAttentionUpper => "WaitAttentionUpper",
        }
    }
}

impl From<&PlayerState> for String {
    fn from(value: &PlayerState) -> Self {
        value.to_string().into()
    }
}

impl From<&PlayerState> for Byml {
    fn from(value: &PlayerState) -> Self {
        Byml::String(value.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StartPos {
    pub map:            Option<MapUnit>,
    pub player_state:   Option<PlayerState>,
    pub pos_name:       Option<String>,
    pub rotate:         DeleteVec<(char, f32)>,
    pub translate:      DeleteVec<(char, f32)>,
}

impl TryFrom<&Byml> for StartPos {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("StartPos node must be HashMap")?;
        Ok(Self {
            map: Some(map.get("Map")
                .context("StartPos must have Map")?
                .as_string()
                .context("StartPos Map must be String")?
                .as_str()
                .into()),
            player_state: map.get("PlayerState")
                .map(|b| b.try_into().context("Invalid PlayerState").unwrap()),
            pos_name: map.get("PosName")
                .map(|b| b.as_string()
                    .context("StartPos PosName must be String")
                    .unwrap()
                    .clone()
                ),
            rotate: map.get("Rotate")
                .context("StartPos must have Rotate")?
                .as_map()
                .context("Invalid StartPos Rotate")?
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().context("Invalid Float").unwrap()
                ))
                .collect::<DeleteVec<_>>(),
            translate: map.get("Translate")
                .context("StartPos must have Translate")?
                .as_map()
                .context("Invalid StartPos Translate")?
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().context("Invalid Float").unwrap()
                ))
                .collect::<DeleteVec<_>>(),
        })
    }
}

impl From<StartPos> for Byml {
    fn from(value: StartPos) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("Map".into(), (&value.map.unwrap()).into());
        match &value.player_state {
            Some(p) => map.insert("PlayerState".into(), p.into()),
            None => None,
        };
        match &value.pos_name {
            Some(p) => map.insert("PosName".into(), p.into()),
            None => None,
        };
        map.insert("Rotate".into(), Byml::Map(value.rotate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        map.insert("Translate".into(), Byml::Map(value.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for StartPos {
    fn diff(&self, other: &Self) -> Self {
        Self {
            map: other.map
                .ne(&self.map)
                .then(|| other.map.clone())
                .unwrap(),
            player_state: other.player_state
                .ne(&self.player_state)
                .then(|| other.player_state)
                .unwrap(),
            pos_name: other.pos_name
                .ne(&self.pos_name)
                .then(|| other.pos_name.clone())
                .unwrap(),
            rotate: self.rotate.diff(&other.rotate),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            map: diff.map.clone()
                .or(self.map.clone()),
            player_state: diff.player_state
                .or(self.player_state),
            pos_name: diff.pos_name.clone()
                .or(self.pos_name.clone()),
            rotate: self.rotate.diff(&diff.rotate),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
