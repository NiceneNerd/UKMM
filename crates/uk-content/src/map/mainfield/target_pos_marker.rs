use anyhow::Context;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{DeleteVec, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TargetPosMarker {
    pub rotate:         DeleteVec<(char, f32)>,
    pub translate:      DeleteVec<(char, f32)>,
    pub unique_name:    Option<String>,
}

impl TryFrom<&Byml> for TargetPosMarker {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            rotate: map.get("Rotate")
                .context("TargetPosMarker must have Rotate")?
                .as_map()
                .context("Invalid TargetPosMarker Rotate")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(c), Ok(f)) => Ok((c, f)),
                        (None, Ok(f)) => Err(anyhow::anyhow!("Invalid TargetPosMarker Rotate with value {f}")),
                        (Some(c), Err(e)) => Err(anyhow::anyhow!("Invalid TargetPosMarker Rotate {c}: {e}")),
                        (None, Err(e)) => Err(anyhow::anyhow!("Invalid TargetPosMarker Rotate index {i}: {e}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
            translate: map.get("Translate")
                .context("TargetPosMarker must have Translate")?
                .as_map()
                .context("Invalid TargetPosMarker Translate")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(c), Ok(f)) => Ok((c, f)),
                        (None, Ok(f)) => Err(anyhow::anyhow!("Invalid TargetPosMarker Translate with value {f}")),
                        (Some(c), Err(e)) => Err(anyhow::anyhow!("Invalid TargetPosMarker Translate {c}: {e}")),
                        (None, Err(e)) => Err(anyhow::anyhow!("Invalid TargetPosMarker Translate index {i}: {e}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
            unique_name: map.get("UniqueName")
                .map(|b| b.as_string()
                    .context("TargetPosMarker UniqueName must be String")
                )
                .transpose()?
                .map(|s| s.clone()),
        })
    }
}

impl From<TargetPosMarker> for Byml {
    fn from(val: TargetPosMarker) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("Rotate".into(), Byml::Map(val.rotate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        map.insert("Translate".into(), Byml::Map(val.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        match &val.unique_name {
            Some(p) => map.insert("UniqueName".into(), p.into()),
            None => None,
        };
        Byml::Map(map)
    }
}

impl Mergeable for TargetPosMarker {
    fn diff(&self, other: &Self) -> Self {
        Self {
            rotate: self.rotate.diff(&other.rotate),
            translate: self.translate.diff(&other.translate),
            unique_name: other.unique_name
                .ne(&self.unique_name)
                .then(|| other.unique_name.clone())
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            rotate: self.rotate.merge(&diff.rotate),
            translate: self.translate.merge(&diff.translate),
            unique_name: diff.unique_name
                .eq(&self.unique_name)
                .then(|| self.unique_name.clone())
                .or_else(|| Some(diff.unique_name.clone()))
                .unwrap(),
        }
    }
}
