use anyhow::Context;
use itertools::Itertools;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap}};

use super::AreaShape;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct NonAutoGenArea {
    pub enable_auto_flower: Option<bool>,
    pub rotate_y:           Option<f32>,
    pub scale:              DeleteMap<char, f32>,
    pub shape:              Option<AreaShape>,
    pub translate:          DeleteMap<char, f32>,
}

impl NonAutoGenArea {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.scale.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.rotate_y.unwrap_or_default(),
                self.shape.unwrap_or_default(),
            )
        )
        .to_string()
        .into()
    }

    pub fn is_complete(&self) -> bool {
        self.enable_auto_flower.is_some() &&
            self.rotate_y.is_some() &&
            self.shape.is_some() &&
            self.scale.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z') &&
            self.translate.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z')
    }
}

impl TryFrom<&Byml> for NonAutoGenArea {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            enable_auto_flower: Some(map.get("EnableAutoFlower")
                .context("NonAutoGenArea must have EnableAutoFlower")?
                .as_bool()
                .context("NonAutoGenArea EnableAutoFlower must be Bool")?),
            rotate_y: Some(map.get("RotateY")
                .context("NonAutoGenArea must have RotateY")?
                .as_float()
                .context("NonAutoGenArea RotateY must be Float")?),
            scale: try_get_vecf(map.get("Scale")
                .context("NonAutoGenArea must have Scale")?)
                .context("Invalid NonAutoGenArea Scale")?,
            shape: Some(map.get("Shape")
                .context("NonAutoGenArea must have Shape")?
                .try_into()
                .context("NonAutoGenArea has invalid Shape")?),
            translate: try_get_vecf(map.get("Translate")
                .context("NonAutoGenArea must have Translate")?)
                .context("Invalid NonAutoGenArea Translate")?
        })
    }
}

impl From<NonAutoGenArea> for Byml {
    fn from(val: NonAutoGenArea) -> Self {
        map!(
            "EnableAutoFlower" => val.enable_auto_flower.expect("EnableAutoFlower should have been read on diff").into(),
            "RotateY" => val.rotate_y.expect("RotateY should have been read on diff").into(),
            "Scale" => Byml::Map(val.scale
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "Shape" => (&val.shape.expect("Shape should have been read on diff")).into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
        )
    }
}

impl Mergeable for NonAutoGenArea {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            enable_auto_flower: other.enable_auto_flower
                .ne(&self.enable_auto_flower)
                .then_some(other.enable_auto_flower)
                .unwrap_or_default(),
            rotate_y: other.rotate_y
                .ne(&self.rotate_y)
                .then_some(other.rotate_y)
                .unwrap_or_default(),
            scale: self.scale.diff(&other.scale),
            shape: other.shape
                .ne(&self.shape)
                .then_some(other.shape)
                .unwrap_or_default(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            enable_auto_flower: diff.enable_auto_flower
                .or(self.enable_auto_flower),
            rotate_y: diff.rotate_y
                .or(self.rotate_y),
            scale: self.scale.merge(&diff.scale),
            shape: diff.shape
                .or(self.shape),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
