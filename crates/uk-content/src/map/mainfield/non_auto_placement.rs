use anyhow::Context;
use itertools::Itertools;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};

use super::AreaShape;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct NonAutoPlacement {
    pub non_auto_placement_animal:      Option<bool>,
    pub non_auto_placement_bird:        Option<bool>,
    pub non_auto_placement_enemy:       Option<bool>,
    pub non_auto_placement_fish:        Option<bool>,
    pub non_auto_placement_insect:      Option<bool>,
    pub non_auto_placement_material:    Option<bool>,
    pub non_enemy_search_player:        Option<bool>,
    pub not_use_for_stats:              Option<bool>,
    pub rotate_y:                       Option<f32>,
    pub scale:                          DeleteMap<char, f32>,
    pub shape:                          Option<AreaShape>,
    pub translate:                      DeleteMap<char, f32>,
}

impl NonAutoPlacement {
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
}

impl TryFrom<&Byml> for NonAutoPlacement {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            non_auto_placement_animal: Some(map.get("NonAutoPlacementAnimal")
                .context("NonAutoPlacement must have NonAutoPlacementAnimal")?
                .as_bool()
                .context("NonAutoPlacement NonAutoPlacementAnimal must be Bool")?),
            non_auto_placement_bird: Some(map.get("NonAutoPlacementBird")
                .context("NonAutoPlacement must have NonAutoPlacementBird")?
                .as_bool()
                .context("NonAutoPlacement NonAutoPlacementBird must be Bool")?),
            non_auto_placement_enemy: Some(map.get("NonAutoPlacementEnemy")
                .context("NonAutoPlacement must have NonAutoPlacementEnemy")?
                .as_bool()
                .context("NonAutoPlacement NonAutoPlacementEnemy must be Bool")?),
            non_auto_placement_fish: Some(map.get("NonAutoPlacementFish")
                .context("NonAutoPlacement must have NonAutoPlacementFish")?
                .as_bool()
                .context("NonAutoPlacement NonAutoPlacementFish must be Bool")?),
            non_auto_placement_insect: Some(map.get("NonAutoPlacementInsect")
                .context("NonAutoPlacement must have NonAutoPlacementInsect")?
                .as_bool()
                .context("NonAutoPlacement NonAutoPlacementInsect must be Bool")?),
            non_auto_placement_material: Some(map.get("NonAutoPlacementMaterial")
                .context("NonAutoPlacement must have NonAutoPlacementMaterial")?
                .as_bool()
                .context("NonAutoPlacement NonAutoPlacementMaterial must be Bool")?),
            non_enemy_search_player: Some(map.get("NonEnemySearchPlayer")
                .context("NonAutoPlacement must have NonEnemySearchPlayer")?
                .as_bool()
                .context("NonAutoPlacement NonEnemySearchPlayer must be Bool")?),
            not_use_for_stats: map.get("NotUseForStats")
                .map(|b| b.as_bool()
                    .context("NonAutoPlacement NotUseForStats must be Bool")
                )
                .transpose()?,
            rotate_y: Some(map.get("RotateY")
                .context("NonAutoPlacement must have RotateY")?
                .as_float()
                .context("NonAutoPlacement RotateY must be Float")?),
            scale: try_get_vecf(map.get("Scale")
                .context("NonAutoPlacement must have Scale")?)
                .context("Invalid NonAutoPlacement Scale")?,
            shape: Some(map.get("Shape")
                .context("NonAutoPlacement must have Shape")?
                .try_into()
                .context("NonAutoPlacement has invalid Shape")?),
            translate: try_get_vecf(map.get("Translate")
                .context("NonAutoPlacement must have Translate")?)
                .context("Invalid NonAutoPlacement Translate")?,
        })
    }
}

impl From<NonAutoPlacement> for Byml {
    fn from(value: NonAutoPlacement) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("NonAutoPlacementAnimal".into(), value.non_auto_placement_animal.unwrap().into());
        map.insert("NonAutoPlacementBird".into(), value.non_auto_placement_bird.unwrap().into());
        map.insert("NonAutoPlacementEnemy".into(), value.non_auto_placement_enemy.unwrap().into());
        map.insert("NonAutoPlacementFish".into(), value.non_auto_placement_fish.unwrap().into());
        map.insert("NonAutoPlacementInsect".into(), value.non_auto_placement_insect.unwrap().into());
        map.insert("NonAutoPlacementMaterial".into(), value.non_auto_placement_material.unwrap().into());
        map.insert("NonEnemySearchPlayer".into(), value.non_enemy_search_player.unwrap().into());
        match value.not_use_for_stats {
            Some(p) => map.insert("NotUseForStats".into(), p.into()),
            None => None,
        };
        map.insert("RotateY".into(), value.rotate_y.unwrap().into());
        map.insert("Scale".into(), Byml::Map(value.scale
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        map.insert("Shape".into(), (&value.shape.unwrap()).into());
        map.insert("Translate".into(), Byml::Map(value.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for NonAutoPlacement {
    fn diff(&self, other: &Self) -> Self {
        Self {
            non_auto_placement_animal: other.non_auto_placement_animal
                .ne(&self.non_auto_placement_animal)
                .then(|| other.non_auto_placement_animal)
                .unwrap(),
            non_auto_placement_bird: other.non_auto_placement_bird
                .ne(&self.non_auto_placement_bird)
                .then(|| other.non_auto_placement_bird)
                .unwrap(),
            non_auto_placement_enemy: other.non_auto_placement_enemy
                .ne(&self.non_auto_placement_enemy)
                .then(|| other.non_auto_placement_enemy)
                .unwrap(),
            non_auto_placement_fish: other.non_auto_placement_fish
                .ne(&self.non_auto_placement_fish)
                .then(|| other.non_auto_placement_fish)
                .unwrap(),
            non_auto_placement_insect: other.non_auto_placement_insect
                .ne(&self.non_auto_placement_insect)
                .then(|| other.non_auto_placement_insect)
                .unwrap(),
            non_auto_placement_material: other.non_auto_placement_material
                .ne(&self.non_auto_placement_material)
                .then(|| other.non_auto_placement_material)
                .unwrap(),
            non_enemy_search_player: other.non_enemy_search_player
                .ne(&self.non_enemy_search_player)
                .then(|| other.non_enemy_search_player)
                .unwrap(),
            not_use_for_stats: other.not_use_for_stats
                .ne(&self.not_use_for_stats)
                .then(|| other.not_use_for_stats)
                .unwrap_or_default(),
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
            non_auto_placement_animal: diff.non_auto_placement_animal
                .eq(&self.non_auto_placement_animal)
                .then(|| self.non_auto_placement_animal)
                .or_else(|| Some(diff.non_auto_placement_animal))
                .unwrap(),
            non_auto_placement_bird: diff.non_auto_placement_bird
                .eq(&self.non_auto_placement_bird)
                .then(|| self.non_auto_placement_bird)
                .or_else(|| Some(diff.non_auto_placement_bird))
                .unwrap(),
            non_auto_placement_enemy: diff.non_auto_placement_enemy
                .eq(&self.non_auto_placement_enemy)
                .then(|| self.non_auto_placement_enemy)
                .or_else(|| Some(diff.non_auto_placement_enemy))
                .unwrap(),
            non_auto_placement_fish: diff.non_auto_placement_fish
                .eq(&self.non_auto_placement_fish)
                .then(|| self.non_auto_placement_fish)
                .or_else(|| Some(diff.non_auto_placement_fish))
                .unwrap(),
            non_auto_placement_insect: diff.non_auto_placement_insect
                .eq(&self.non_auto_placement_insect)
                .then(|| self.non_auto_placement_insect)
                .or_else(|| Some(diff.non_auto_placement_insect))
                .unwrap(),
            non_auto_placement_material: diff.non_auto_placement_material
                .eq(&self.non_auto_placement_material)
                .then(|| self.non_auto_placement_material)
                .or_else(|| Some(diff.non_auto_placement_material))
                .unwrap(),
            non_enemy_search_player: diff.non_enemy_search_player
                .eq(&self.non_enemy_search_player)
                .then(|| self.non_enemy_search_player)
                .or_else(|| Some(diff.non_enemy_search_player))
                .unwrap(),
            not_use_for_stats: diff.not_use_for_stats
                .eq(&self.not_use_for_stats)
                .then(|| self.not_use_for_stats)
                .or_else(|| Some(diff.not_use_for_stats))
                .unwrap(),
            rotate_y: diff.rotate_y
                .eq(&self.rotate_y)
                .then(|| self.rotate_y)
                .or_else(|| Some(diff.rotate_y))
                .unwrap(),
            scale: self.scale.merge(&diff.scale),
            shape: diff.shape
                .eq(&self.shape)
                .then(|| self.shape)
                .or_else(|| Some(diff.shape))
                .unwrap(),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
