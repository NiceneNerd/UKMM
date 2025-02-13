use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNpcRestStation {
    pub rest_horse_left:    Option<bool>,
    pub rest_only_npc:      Option<bool>,
    pub rest_with_horse:    Option<bool>,
    pub rotate_y:           Option<f32>,
    pub translate:          DeleteVec<(char, f32)>,
}

impl From<&Byml> for RoadNpcRestStation {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("RoadNpcRestStation node must be HashMap");
        Self {
            rest_horse_left: Some(map.get("RestHorseLeft")
                .expect("RoadNpcRestStation must have RestHorseLeft")
                .as_bool()
                .expect("RoadNpcRestStation RestHorseLeft must be Bool")),
            rest_only_npc: Some(map.get("RestOnlyNpc")
                .expect("RoadNpcRestStation must have RestOnlyNpc")
                .as_bool()
                .expect("RoadNpcRestStation RestOnlyNpc must be Bool")),
            rest_with_horse: Some(map.get("RestWithHorse")
                .expect("RoadNpcRestStation must have PosName")
                .as_bool()
                .expect("RoadNpcRestStation RestWithHorse must be Bool")),
            rotate_y: Some(map.get("RotateY")
                .expect("RoadNpcRestStation must have RotateY")
                .as_float()
                .expect("RoadNpcRestStation RotateY must be Float")),
            translate: map.get("Translate")
                .expect("RoadNpcRestStation must have Translate")
                .as_map()
                .expect("Invalid RoadNpcRestStation Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
        }
    }
}

impl From<RoadNpcRestStation> for Byml {
    fn from(val: RoadNpcRestStation) -> Self {
        map!{
            "RestHorseLeft" => val.rest_horse_left.unwrap().into(),
            "RestOnlyNpc" => val.rest_only_npc.unwrap().into(),
            "RestWithHorse" => val.rest_with_horse.unwrap().into(),
            "RotateY" => val.rotate_y.unwrap().into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
        }
    }
}

impl Mergeable for RoadNpcRestStation {
    fn diff(&self, other: &Self) -> Self {
        Self {
            rest_horse_left: other.rest_horse_left
                .ne(&self.rest_horse_left)
                .then(|| other.rest_horse_left)
                .unwrap(),
            rest_only_npc: other.rest_only_npc
                .ne(&self.rest_only_npc)
                .then(|| other.rest_only_npc)
                .unwrap(),
            rest_with_horse: other.rest_with_horse
                .ne(&self.rest_with_horse)
                .then(|| other.rest_with_horse)
                .unwrap(),
            rotate_y: other.rotate_y
                .ne(&self.rotate_y)
                .then(|| other.rotate_y)
                .unwrap(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            rest_horse_left: diff.rest_horse_left
                .or(self.rest_horse_left),
            rest_only_npc: diff.rest_only_npc
                .or(self.rest_only_npc),
            rest_with_horse: diff.rest_with_horse
                .or(self.rest_with_horse),
            rotate_y: diff.rotate_y
                .or(self.rotate_y),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
