use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

use super::AreaShape;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct NonAutoGenArea {
    pub enable_auto_flower: Option<bool>,
    pub rotate_y:           Option<f32>,
    pub scale:              DeleteVec<(char, f32)>,
    pub shape:              Option<AreaShape>,
    pub translate:          DeleteVec<(char, f32)>,
}

impl From<&Byml> for NonAutoGenArea {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("TargetPosMarker node must be HashMap");
        Self {
            enable_auto_flower: Some(map.get("EnableAutoFlower")
                .expect("NonAutoGenArea must have EnableAutoFlower")
                .as_bool()
                .expect("NonAutoGenArea EnableAutoFlower must be Bool")),
            rotate_y: Some(map.get("RotateY")
                .expect("NonAutoGenArea must have RotateY")
                .as_float()
                .expect("NonAutoGenArea RotateY must be Float")),
            scale: map.get("Scale")
                .expect("NonAutoGenArea must have Scale")
                .as_map()
                .expect("Invalid NonAutoGenArea Scale")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
            shape: Some(map.get("Shape")
                .expect("NonAutoGenArea must have Shape")
                .try_into()
                .expect("NonAutoGenArea has invalid Shape")),
            translate: map.get("Translate")
                .expect("NonAutoGenArea must have Translate")
                .as_map()
                .expect("Invalid NonAutoGenArea Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
        }
    }
}

impl From<NonAutoGenArea> for Byml {
    fn from(val: NonAutoGenArea) -> Self {
        map!(
            "EnableAutoFlower" => val.enable_auto_flower.unwrap().into(),
            "RotateY" => val.rotate_y.unwrap().into(),
            "Scale" => Byml::Map(val.scale
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "Shape" => (&val.shape.unwrap()).into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
        )
    }
}

impl Mergeable for NonAutoGenArea {
    fn diff(&self, other: &Self) -> Self {
        Self {
            enable_auto_flower: other.enable_auto_flower
                .ne(&self.enable_auto_flower)
                .then(|| other.enable_auto_flower)
                .unwrap(),
            rotate_y: other.rotate_y
                .ne(&self.rotate_y)
                .then(|| other.rotate_y)
                .unwrap(),
            scale: self.scale.diff(&other.scale),
            shape: other.shape
                .ne(&self.shape)
                .then(|| other.shape)
                .unwrap(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            enable_auto_flower: diff.enable_auto_flower
                .or(self.enable_auto_flower),
            rotate_y: diff.rotate_y
                .or(self.rotate_y),
            scale: self.scale.diff(&diff.scale),
            shape: diff.shape
                .or(self.shape),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
