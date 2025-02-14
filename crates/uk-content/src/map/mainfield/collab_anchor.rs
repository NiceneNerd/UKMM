use anyhow::Context;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::DeleteVec};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CollabAnchor {
    pub collabo_shooting_star_direction:    Option<i32>,
    pub collabo_shooting_star_end_hour:     Option<i32>,
    pub collabo_shooting_star_start_hour:   Option<i32>,
    pub translate:                          DeleteVec<(char, f32)>,
    pub collabo_ssfallout_flag_name:        Option<String>,
    pub collabo_ssopen_flag_name:           Option<String>,
    pub collabo_ssquest_flag:               Option<String>,
}

impl TryFrom<&Byml> for CollabAnchor {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .expect("TargetPosMarker node must be HashMap");
        Ok(Self {
            collabo_shooting_star_direction: Some(map.get("CollaboShootingStarDirection")
                .context("CollabAnchor must have CollaboShootingStarDirection")?
                .as_i32()
                .context("CollabAnchor CollaboShootingStarDirection must be Int")?),
            collabo_shooting_star_end_hour: Some(map.get("CollaboShootingStarEndHour")
                .context("CollabAnchor must have CollaboShootingStarEndHour")?
                .as_i32()
                .context("CollabAnchor CollaboShootingStarEndHour must be Int")?),
            collabo_shooting_star_start_hour: Some(map.get("CollaboShootingStarStartHour")
                .context("CollabAnchor must have CollaboShootingStarStartHour")?
                .as_i32()
                .context("CollabAnchor CollaboShootingStarStartHour must be Int")?),
            translate: map.get("Translate")
                .context("CollabAnchor must have Translate")?
                .as_map()
                .context("Invalid CollabAnchor Translate")?
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().context("Invalid Float").unwrap()
                ))
                .collect::<DeleteVec<_>>(),
            collabo_ssfallout_flag_name: Some(map.get("collaboSSFalloutFlagName")
                .context("CollabAnchor must have collaboSSFalloutFlagName")?
                .as_string()
                .context("CollabAnchor collaboSSFalloutFlagName must be String")?
                .clone()),
            collabo_ssopen_flag_name: Some(map.get("collaboSSOpenFlagName")
                .context("CollabAnchor must have collaboSSOpenFlagName")?
                .as_string()
                .context("CollabAnchor collaboSSOpenFlagName must be String")?
                .clone()),
            collabo_ssquest_flag: Some(map.get("collaboSSQuestFlag")
                .context("CollabAnchor must have collaboSSQuestFlag")?
                .as_string()
                .context("CollabAnchor collaboSSQuestFlag must be String")?
                .clone()),
        })
    }
}

impl From<CollabAnchor> for Byml {
    fn from(val: CollabAnchor) -> Self {
        map!(
            "CollaboShootingStarDirection" => val.collabo_shooting_star_direction
                .unwrap()
                .into(),
            "CollaboShootingStarEndHour" => val.collabo_shooting_star_end_hour
                .unwrap()
                .into(),
            "CollaboShootingStarStartHour" => val.collabo_shooting_star_start_hour
                .unwrap()
                .into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "collaboSSFalloutFlagName" => val.collabo_ssfallout_flag_name
                .unwrap()
                .into(),
            "collaboSSOpenFlagName" => val.collabo_ssopen_flag_name
                .unwrap()
                .into(),
            "collaboSSQuestFlag" => val.collabo_ssquest_flag
                .unwrap()
                .into(),
        )
    }
}

impl Mergeable for CollabAnchor {
    fn diff(&self, other: &Self) -> Self {
        Self {
            collabo_shooting_star_direction: other.collabo_shooting_star_direction
                .ne(&self.collabo_shooting_star_direction)
                .then(|| other.collabo_shooting_star_direction)
                .unwrap(),
            collabo_shooting_star_end_hour: other.collabo_shooting_star_end_hour
                .ne(&self.collabo_shooting_star_end_hour)
                .then(|| other.collabo_shooting_star_end_hour)
                .unwrap(),
            collabo_shooting_star_start_hour: other.collabo_shooting_star_start_hour
                .ne(&self.collabo_shooting_star_start_hour)
                .then(|| other.collabo_shooting_star_start_hour)
                .unwrap(),
            translate: self.translate.diff(&other.translate),
            collabo_ssfallout_flag_name: other.collabo_ssfallout_flag_name
                .ne(&self.collabo_ssfallout_flag_name)
                .then(|| other.collabo_ssfallout_flag_name.clone())
                .unwrap(),
            collabo_ssopen_flag_name: other.collabo_ssopen_flag_name
                .ne(&self.collabo_ssopen_flag_name)
                .then(|| other.collabo_ssopen_flag_name.clone())
                .unwrap(),
            collabo_ssquest_flag: other.collabo_ssquest_flag
                .ne(&self.collabo_ssquest_flag)
                .then(|| other.collabo_ssquest_flag.clone())
                .unwrap(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            collabo_shooting_star_direction: diff.collabo_shooting_star_direction
                .eq(&self.collabo_shooting_star_direction)
                .then(|| self.collabo_shooting_star_direction)
                .or_else(|| Some(diff.collabo_shooting_star_direction))
                .unwrap(),
            collabo_shooting_star_end_hour: diff.collabo_shooting_star_end_hour
                .eq(&self.collabo_shooting_star_end_hour)
                .then(|| self.collabo_shooting_star_end_hour)
                .or_else(|| Some(diff.collabo_shooting_star_end_hour))
                .unwrap(),
            collabo_shooting_star_start_hour: diff.collabo_shooting_star_start_hour
                .eq(&self.collabo_shooting_star_start_hour)
                .then(|| self.collabo_shooting_star_start_hour)
                .or_else(|| Some(diff.collabo_shooting_star_start_hour))
                .unwrap(),
            translate: self.translate.merge(&diff.translate),
            collabo_ssfallout_flag_name: diff.collabo_ssfallout_flag_name.clone()
                .eq(&self.collabo_ssfallout_flag_name)
                .then(|| self.collabo_ssfallout_flag_name.clone())
                .or_else(|| Some(diff.collabo_ssfallout_flag_name.clone()))
                .unwrap(),
            collabo_ssopen_flag_name: diff.collabo_ssopen_flag_name.clone()
                .eq(&self.collabo_ssopen_flag_name)
                .then(|| self.collabo_ssopen_flag_name.clone())
                .or_else(|| Some(diff.collabo_ssopen_flag_name.clone()))
                .unwrap(),
            collabo_ssquest_flag: diff.collabo_ssquest_flag.clone()
                .eq(&self.collabo_ssquest_flag)
                .then(|| self.collabo_ssquest_flag.clone())
                .or_else(|| Some(diff.collabo_ssquest_flag.clone()))
                .unwrap(),
        }
    }
}
