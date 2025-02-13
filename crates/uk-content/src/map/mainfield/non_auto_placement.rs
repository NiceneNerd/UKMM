use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

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
    pub rotate_y:                       Option<f32>,
    pub scale:                          DeleteVec<(char, f32)>,
    pub shape:                          Option<AreaShape>,
    pub translate:                      DeleteVec<(char, f32)>,
}

impl From<&Byml> for NonAutoPlacement {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("TargetPosMarker node must be HashMap");
        Self {
            non_auto_placement_animal: Some(map.get("NonAutoPlacementAnimal")
                .expect("NonAutoPlacement must have NonAutoPlacementAnimal")
                .as_bool()
                .expect("NonAutoPlacement NonAutoPlacementAnimal must be Bool")),
            non_auto_placement_bird: Some(map.get("NonAutoPlacementBird")
                .expect("NonAutoPlacement must have NonAutoPlacementBird")
                .as_bool()
                .expect("NonAutoPlacement NonAutoPlacementBird must be Bool")),
            non_auto_placement_enemy: Some(map.get("NonAutoPlacementEnemy")
                .expect("NonAutoPlacement must have NonAutoPlacementEnemy")
                .as_bool()
                .expect("NonAutoPlacement NonAutoPlacementEnemy must be Bool")),
            non_auto_placement_fish: Some(map.get("NonAutoPlacementFish")
                .expect("NonAutoPlacement must have NonAutoPlacementFish")
                .as_bool()
                .expect("NonAutoPlacement NonAutoPlacementFish must be Bool")),
            non_auto_placement_insect: Some(map.get("NonAutoPlacementInsect")
                .expect("NonAutoPlacement must have NonAutoPlacementInsect")
                .as_bool()
                .expect("NonAutoPlacement NonAutoPlacementInsect must be Bool")),
            non_auto_placement_material: Some(map.get("NonAutoPlacementMaterial")
                .expect("NonAutoPlacement must have NonAutoPlacementMaterial")
                .as_bool()
                .expect("NonAutoPlacement NonAutoPlacementMaterial must be Bool")),
            non_enemy_search_player: Some(map.get("NonEnemySearchPlayer")
                .expect("NonAutoPlacement must have NonEnemySearchPlayer")
                .as_bool()
                .expect("NonAutoPlacement NonEnemySearchPlayer must be Bool")),
            rotate_y: Some(map.get("RotateY")
                .expect("NonAutoPlacement must have RotateY")
                .as_float()
                .expect("NonAutoPlacement RotateY must be Float")),
            scale: map.get("Scale")
                .expect("NonAutoPlacement must have Scale")
                .as_map()
                .expect("Invalid NonAutoPlacement Scale")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
            shape: Some(map.get("Shape")
                .expect("NonAutoPlacement must have Shape")
                .try_into()
                .expect("NonAutoPlacement has invalid Shape")),
            translate: map.get("Translate")
                .expect("NonAutoPlacement must have Translate")
                .as_map()
                .expect("Invalid NonAutoPlacement Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
        }
    }
}

impl From<NonAutoPlacement> for Byml {
    fn from(val: NonAutoPlacement) -> Self {
        map!{
            "NonAutoPlacementAnimal" => val.non_auto_placement_animal.unwrap().into(),
            "NonAutoPlacementBird" => val.non_auto_placement_bird.unwrap().into(),
            "NonAutoPlacementEnemy" => val.non_auto_placement_enemy.unwrap().into(),
            "NonAutoPlacementFish" => val.non_auto_placement_fish.unwrap().into(),
            "NonAutoPlacementInsect" => val.non_auto_placement_insect.unwrap().into(),
            "NonAutoPlacementMaterial" => val.non_auto_placement_material.unwrap().into(),
            "NonEnemySearchPlayer" => val.non_enemy_search_player.unwrap().into(),
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
        }
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
            rotate_y: diff.rotate_y
                .or(self.rotate_y),
            scale: self.scale.diff(&diff.scale),
            shape: diff.shape
                .or(self.shape),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
