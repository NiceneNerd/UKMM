use anyhow::Context;
use itertools::Itertools;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RestartPos {
    pub scale:          DeleteMap<char, f32>,
    pub translate:      DeleteMap<char, f32>,
    pub unique_name:    Option<String>,
}

impl RestartPos {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.unique_name.clone().unwrap_or_default()
            )
        )
        .to_string()
        .into()
    }
}

impl TryFrom<&Byml> for RestartPos {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            scale: try_get_vecf(map.get("Scale")
                .context("RestartPos must have Scale")?)
                .context("Invalid RestartPos Scale")?,
            translate: try_get_vecf(map.get("Translate")
                .context("RestartPos must have Translate")?)
                .context("Invalid RestartPos Translate")?,
            unique_name: Some(map.get("UniqueName")
                .context("RestartPos must have UniqueName")?
                .as_string()
                .context("RestartPos UniqueName must be String")?
                .clone()),
        })
    }
}

impl From<RestartPos> for Byml {
    fn from(val: RestartPos) -> Self {
        map!{
            "Scale" => Byml::Map(val.scale
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "UniqueName" => val.unique_name.expect("UniqueName should have been read on diff").into(),
        }
    }
}

impl Mergeable for RestartPos {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            scale: self.scale.diff(&other.scale),
            translate: self.translate.diff(&other.translate),
            unique_name: other.unique_name
                .ne(&self.unique_name)
                .then(|| other.unique_name.clone())
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            scale: self.scale.merge(&diff.scale),
            translate: self.translate.merge(&diff.translate),
            unique_name: diff.unique_name
                .eq(&self.unique_name)
                .then(|| self.unique_name.clone())
                .or_else(|| Some(diff.unique_name.clone()))
                .expect("UniqueName should be in at least one of these files"),
        }
    }
}
