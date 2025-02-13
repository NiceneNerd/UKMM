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

impl From<&Byml> for CollabAnchor {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("TargetPosMarker node must be HashMap");
        Self {
            collabo_shooting_star_direction: Some(map.get("CollaboShootingStarDirection")
                .expect("CollabAnchor must have CollaboShootingStarDirection")
                .as_i32()
                .expect("CollabAnchor CollaboShootingStarDirection must be Int")),
            collabo_shooting_star_end_hour: Some(map.get("CollaboShootingStarEndHour")
                .expect("CollabAnchor must have CollaboShootingStarEndHour")
                .as_i32()
                .expect("CollabAnchor CollaboShootingStarEndHour must be Int")),
            collabo_shooting_star_start_hour: Some(map.get("CollaboShootingStarStartHour")
                .expect("CollabAnchor must have CollaboShootingStarStartHour")
                .as_i32()
                .expect("CollabAnchor CollaboShootingStarStartHour must be Int")),
            translate: map.get("Translate")
                .expect("CollabAnchor must have Translate")
                .as_map()
                .expect("Invalid CollabAnchor Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
            collabo_ssfallout_flag_name: Some(map.get("collaboSSFalloutFlagName")
                .expect("CollabAnchor must have collaboSSFalloutFlagName")
                .as_string()
                .expect("CollabAnchor collaboSSFalloutFlagName must be String")
                .clone()),
            collabo_ssopen_flag_name: Some(map.get("collaboSSOpenFlagName")
                .expect("CollabAnchor must have collaboSSOpenFlagName")
                .as_string()
                .expect("CollabAnchor collaboSSOpenFlagName must be String")
                .clone()),
            collabo_ssquest_flag: Some(map.get("collaboSSQuestFlag")
                .expect("CollabAnchor must have collaboSSQuestFlag")
                .as_string()
                .expect("CollabAnchor collaboSSQuestFlag must be String")
                .clone()),
        }
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
                .or(self.collabo_shooting_star_direction),
            collabo_shooting_star_end_hour: diff.collabo_shooting_star_end_hour
                .or(self.collabo_shooting_star_end_hour),
            collabo_shooting_star_start_hour: diff.collabo_shooting_star_start_hour
                .or(self.collabo_shooting_star_start_hour),
            translate: self.translate.diff(&diff.translate),
            collabo_ssfallout_flag_name: diff.collabo_ssfallout_flag_name.clone()
                .or(self.collabo_ssfallout_flag_name.clone()),
            collabo_ssopen_flag_name: diff.collabo_ssopen_flag_name.clone()
                .or(self.collabo_ssopen_flag_name.clone()),
            collabo_ssquest_flag: diff.collabo_ssquest_flag.clone()
                .or(self.collabo_ssquest_flag.clone()),
        }
    }
}
