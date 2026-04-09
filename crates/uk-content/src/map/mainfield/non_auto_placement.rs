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

    pub fn is_complete(&self) -> bool {
        // Some of these are optional
        self.non_auto_placement_animal.is_some() &&
            self.non_auto_placement_bird.is_some() &&
            self.non_auto_placement_enemy.is_some() &&
            self.non_auto_placement_fish.is_some() &&
            self.non_auto_placement_insect.is_some() &&
            self.non_auto_placement_material.is_some() &&
            self.non_enemy_search_player.is_some() &&
            //self.not_use_for_stats.is_some() &&
            self.rotate_y.is_some() &&
            self.shape.is_some() &&
            self.scale.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z') &&
            self.translate.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z')
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
        map.insert("NonAutoPlacementAnimal".into(), value.non_auto_placement_animal.expect("NonAutoPlacementAnimal should have been read on diff").into());
        map.insert("NonAutoPlacementBird".into(), value.non_auto_placement_bird.expect("NonAutoPlacementBird should have been read on diff").into());
        map.insert("NonAutoPlacementEnemy".into(), value.non_auto_placement_enemy.expect("NonAutoPlacementEnemy should have been read on diff").into());
        map.insert("NonAutoPlacementFish".into(), value.non_auto_placement_fish.expect("NonAutoPlacementFish should have been read on diff").into());
        map.insert("NonAutoPlacementInsect".into(), value.non_auto_placement_insect.expect("NonAutoPlacementInsect should have been read on diff").into());
        map.insert("NonAutoPlacementMaterial".into(), value.non_auto_placement_material.expect("NonAutoPlacementMaterial should have been read on diff").into());
        map.insert("NonEnemySearchPlayer".into(), value.non_enemy_search_player.expect("NonEnemySearchPlayer should have been read on diff").into());
        match value.not_use_for_stats {
            Some(p) => map.insert("NotUseForStats".into(), p.into()),
            None => None,
        };
        map.insert("RotateY".into(), value.rotate_y.expect("RotateY should have been read on diff").into());
        map.insert("Scale".into(), Byml::Map(value.scale
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<HashMap<String, Byml>>()));
        map.insert("Shape".into(), (&value.shape.expect("Shape should have been read on diff")).into());
        map.insert("Translate".into(), Byml::Map(value.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for NonAutoPlacement {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            non_auto_placement_animal: other.non_auto_placement_animal
                .ne(&self.non_auto_placement_animal)
                .then_some(other.non_auto_placement_animal)
                .unwrap_or_default(),
            non_auto_placement_bird: other.non_auto_placement_bird
                .ne(&self.non_auto_placement_bird)
                .then_some(other.non_auto_placement_bird)
                .unwrap_or_default(),
            non_auto_placement_enemy: other.non_auto_placement_enemy
                .ne(&self.non_auto_placement_enemy)
                .then_some(other.non_auto_placement_enemy)
                .unwrap_or_default(),
            non_auto_placement_fish: other.non_auto_placement_fish
                .ne(&self.non_auto_placement_fish)
                .then_some(other.non_auto_placement_fish)
                .unwrap_or_default(),
            non_auto_placement_insect: other.non_auto_placement_insect
                .ne(&self.non_auto_placement_insect)
                .then_some(other.non_auto_placement_insect)
                .unwrap_or_default(),
            non_auto_placement_material: other.non_auto_placement_material
                .ne(&self.non_auto_placement_material)
                .then_some(other.non_auto_placement_material)
                .unwrap_or_default(),
            non_enemy_search_player: other.non_enemy_search_player
                .ne(&self.non_enemy_search_player)
                .then_some(other.non_enemy_search_player)
                .unwrap_or_default(),
            not_use_for_stats: other.not_use_for_stats
                .ne(&self.not_use_for_stats)
                .then_some(other.not_use_for_stats)
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
            scale: self.scale.merge(&diff.scale),
            shape: diff.shape
                .or(self.shape),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
