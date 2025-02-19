use anyhow::Context;
use itertools::Itertools;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNpcRestStation {
    pub rest_horse_left:    Option<bool>,
    pub rest_only_npc:      Option<bool>,
    pub rest_with_horse:    Option<bool>,
    pub rotate_y:           Option<f32>,
    pub translate:          DeleteMap<char, f32>,
}

impl RoadNpcRestStation {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
            )
        )
        .to_string()
        .into()
    }
}

impl TryFrom<&Byml> for RoadNpcRestStation {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("RoadNpcRestStation node must be HashMap")?;
        Ok(Self {
            rest_horse_left: Some(map.get("RestHorseLeft")
                .context("RoadNpcRestStation must have RestHorseLeft")?
                .as_bool()
                .context("RoadNpcRestStation RestHorseLeft must be Bool")?),
            rest_only_npc: Some(map.get("RestOnlyNpc")
                .context("RoadNpcRestStation must have RestOnlyNpc")?
                .as_bool()
                .context("RoadNpcRestStation RestOnlyNpc must be Bool")?),
            rest_with_horse: Some(map.get("RestWithHorse")
                .context("RoadNpcRestStation must have PosName")?
                .as_bool()
                .context("RoadNpcRestStation RestWithHorse must be Bool")?),
            rotate_y: Some(map.get("RotateY")
                .context("RoadNpcRestStation must have RotateY")?
                .as_float()
                .context("RoadNpcRestStation RotateY must be Float")?),
            translate: try_get_vecf(map.get("Translate")
                .context("RoadNpcRestStation must have Translate")?)
                .context("Invalid RoadNpcRestStation Translate")?,
        })
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
                .eq(&self.rest_horse_left)
                .then(|| self.rest_horse_left)
                .or_else(|| Some(diff.rest_horse_left))
                .unwrap(),
            rest_only_npc: diff.rest_only_npc
                .eq(&self.rest_only_npc)
                .then(|| self.rest_only_npc)
                .or_else(|| Some(diff.rest_only_npc))
                .unwrap(),
            rest_with_horse: diff.rest_with_horse
                .eq(&self.rest_with_horse)
                .then(|| self.rest_with_horse)
                .or_else(|| Some(diff.rest_with_horse))
                .unwrap(),
            rotate_y: diff.rotate_y
                .eq(&self.rotate_y)
                .then(|| self.rotate_y)
                .or_else(|| Some(diff.rotate_y))
                .unwrap(),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
