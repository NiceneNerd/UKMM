use anyhow::Context;
use itertools::Itertools;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};

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

impl From<&PlayerState> for &str {
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
    pub rotate:         DeleteMap<char, f32>,
    pub translate:      DeleteMap<char, f32>,
}

impl StartPos {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.pos_name.clone().unwrap_or_default()
            )
        )
        .to_string()
        .into()
    }
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
                .map(|b| b.try_into().context("Invalid PlayerState"))
                .transpose()?,
            pos_name: map.get("PosName")
                .map(|b| b.as_string()
                    .context("StartPos PosName must be String")
                )
                .transpose()?
                .cloned(),
            rotate: try_get_vecf(map.get("Rotate")
                .context("StartPos must have Rotate")?)
                .context("Invalid StartPos Rotate")?,
            translate: try_get_vecf(map.get("Translate")
                .context("StartPos must have Translate")?)
                .context("Invalid StartPos Translate")?,
        })
    }
}

impl From<StartPos> for Byml {
    fn from(value: StartPos) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("Map".into(), (&value.map.expect("Map should have been read on diff")).into());
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
            .collect::<HashMap<String, Byml>>()));
        map.insert("Translate".into(), Byml::Map(value.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for StartPos {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            map: other.map
                .ne(&self.map)
                .then(|| other.map.clone())
                .unwrap_or_default(),
            player_state: other.player_state
                .ne(&self.player_state)
                .then_some(other.player_state)
                .unwrap_or_default(),
            pos_name: other.pos_name
                .ne(&self.pos_name)
                .then(|| other.pos_name.clone())
                .unwrap_or_default(),
            rotate: self.rotate.diff(&other.rotate),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            map: diff.map
                .eq(&self.map)
                .then(|| self.map.clone())
                .or_else(|| Some(diff.map.clone()))
                .expect("Map should be in at least one of these files"),
            player_state: diff.player_state
                .eq(&self.player_state)
                .then_some(self.player_state)
                .or(Some(diff.player_state))
                .expect("PlayerState should be in at least one of these files"),
            pos_name: diff.pos_name
                .eq(&self.pos_name)
                .then(|| self.pos_name.clone())
                .or_else(|| Some(diff.pos_name.clone()))
                .expect("PosName should be in at least one of these files"),
            rotate: self.rotate.merge(&diff.rotate),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
