use anyhow::Context;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{DeleteVec, HashMap}};

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
    pub scale:                          DeleteVec<(char, f32)>,
    pub shape:                          Option<AreaShape>,
    pub translate:                      DeleteVec<(char, f32)>,
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
            not_use_for_stats: Some(map.get("NotUseForStats")
                .context("NonAutoPlacement must have NotUseForStats")?
                .as_bool()
                .context("NonAutoPlacement NotUseForStats must be Bool")?),
            rotate_y: Some(map.get("RotateY")
                .context("NonAutoPlacement must have RotateY")?
                .as_float()
                .context("NonAutoPlacement RotateY must be Float")?),
            scale: map.get("Scale")
                .context("NonAutoPlacement must have Scale")?
                .as_map()
                .context("Invalid NonAutoPlacement Scale")?
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().context("Invalid Float").unwrap()
                ))
                .collect::<DeleteVec<_>>(),
            shape: Some(map.get("Shape")
                .context("NonAutoPlacement must have Shape")?
                .try_into()
                .context("NonAutoPlacement has invalid Shape")?),
            translate: map.get("Translate")
                .context("NonAutoPlacement must have Translate")?
                .as_map()
                .context("Invalid NonAutoPlacement Translate")?
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().context("Invalid Float").unwrap()
                ))
                .collect::<DeleteVec<_>>(),
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
                .or(self.non_auto_placement_animal),
            non_auto_placement_bird: diff.non_auto_placement_bird
                .or(self.non_auto_placement_bird),
            non_auto_placement_enemy: diff.non_auto_placement_enemy
                .or(self.non_auto_placement_enemy),
            non_auto_placement_fish: diff.non_auto_placement_fish
                .or(self.non_auto_placement_fish),
            non_auto_placement_insect: diff.non_auto_placement_insect
                .or(self.non_auto_placement_insect),
            non_auto_placement_material: diff.non_auto_placement_material
                .or(self.non_auto_placement_material),
            non_enemy_search_player: diff.non_enemy_search_player
                .or(self.non_enemy_search_player),
            not_use_for_stats: diff.not_use_for_stats
                .or(self.not_use_for_stats),
            rotate_y: diff.rotate_y
                .or(self.rotate_y),
            scale: self.scale.diff(&diff.scale),
            shape: diff.shape
                .or(self.shape),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
