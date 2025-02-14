use anyhow::Context;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RestartPos {
    pub scale:          DeleteVec<(char, f32)>,
    pub translate:      DeleteVec<(char, f32)>,
    pub unique_name:    Option<String>,
}

impl TryFrom<&Byml> for RestartPos {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            scale: map.get("Scale")
                .context("RestartPos must have Scale")?
                .as_map()
                .context("Invalid RestartPos Scale")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(c), Ok(f)) => Ok((c, f)),
                        (None, Ok(f)) => Err(anyhow::anyhow!("Invalid RestartPos Scale with value {f}")),
                        (Some(c), Err(e)) => Err(anyhow::anyhow!("Invalid RestartPos Scale {c}: {e}")),
                        (None, Err(e)) => Err(anyhow::anyhow!("Invalid RestartPos Scale index {i}: {e}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
            translate: map.get("Translate")
                .context("RestartPos must have Translate")?
                .as_map()
                .context("Invalid RestartPos Translate")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(c), Ok(f)) => Ok((c, f)),
                        (None, Ok(f)) => Err(anyhow::anyhow!("Invalid RestartPos Translate with value {f}")),
                        (Some(c), Err(e)) => Err(anyhow::anyhow!("Invalid RestartPos Translate {c}: {e}")),
                        (None, Err(e)) => Err(anyhow::anyhow!("Invalid RestartPos Translate index {i}: {e}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
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
            "UniqueName" => val.unique_name.unwrap().into(),
        }
    }
}

impl Mergeable for RestartPos {
    fn diff(&self, other: &Self) -> Self {
        Self {
            scale: self.scale.diff(&other.scale),
            translate: self.translate.diff(&other.translate),
            unique_name: other.unique_name
                .ne(&self.unique_name)
                .then(|| other.unique_name.clone())
                .unwrap(),
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
                .unwrap(),
        }
    }
}
