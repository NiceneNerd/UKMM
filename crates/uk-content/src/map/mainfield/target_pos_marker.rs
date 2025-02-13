use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TargetPosMarker {
    pub rotate:         DeleteVec<(char, f32)>,
    pub translate:      DeleteVec<(char, f32)>,
    pub unique_name:    Option<String>,
}

impl From<&Byml> for TargetPosMarker {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("TargetPosMarker node must be HashMap");
        Self {
            rotate: map.get("Rotate")
                .expect("TargetPosMarker must have Rotate")
                .as_map()
                .expect("Invalid TargetPosMarker Rotate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
            translate: map.get("Translate")
                .expect("TargetPosMarker must have Translate")
                .as_map()
                .expect("Invalid TargetPosMarker Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
            unique_name: Some(map.get("UniqueName")
                .expect("TargetPosMarker must have UniqueName")
                .as_string()
                .expect("TargetPosMarker UniqueName must be String")
                .clone()),
        }
    }
}

impl From<TargetPosMarker> for Byml {
    fn from(val: TargetPosMarker) -> Self {
        map!(
            "Rotate" => Byml::Map(val.rotate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "UniqueName" => val.unique_name.unwrap().into(),
        )
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
                .unwrap(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            rotate: self.rotate.diff(&diff.rotate),
            translate: self.translate.diff(&diff.translate),
            unique_name: diff.unique_name.clone()
                .or(self.unique_name.clone()),
        }
    }
}
