use anyhow::Context;
use itertools::Itertools;
use roead::byml::{map, Byml};
use smartstring::alias::String;

use crate::{util::parsers::try_get_vecf, prelude::Mergeable, util::DeleteMap};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CollabAnchor {
    pub collabo_shooting_star_direction:    Option<i32>,
    pub collabo_shooting_star_end_hour:     Option<i32>,
    pub collabo_shooting_star_start_hour:   Option<i32>,
    pub translate:                          DeleteMap<char, f32>,
    pub collabo_ssfallout_flag_name:        Option<String>,
    pub collabo_ssopen_flag_name:           Option<String>,
    pub collabo_ssquest_flag:               Option<String>,
}

impl CollabAnchor {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.collabo_ssopen_flag_name.clone().expect("collaboSSOpenFlagName should have been read on diff")
            )
        )
        .to_string()
        .into()
    }
    
    pub fn is_complete(&self) -> bool {
        self.collabo_shooting_star_direction.is_some() &&
            self.collabo_shooting_star_end_hour.is_some() &&
            self.collabo_shooting_star_start_hour.is_some() &&
            self.collabo_ssfallout_flag_name.is_some() &&
            self.collabo_ssopen_flag_name.is_some() &&
            self.collabo_ssquest_flag.is_some() &&
            self.translate.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z')
    }
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
            translate: try_get_vecf(map.get("Translate")
                .context("CollabAnchor must have Translate")?)
                .context("Invalid CollabAnchor Translate")?,
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
                .expect("CollaboShootingStarDirection should have been read on diff")
                .into(),
            "CollaboShootingStarEndHour" => val.collabo_shooting_star_end_hour
                .expect("CollaboShootingStarEndHour should have been read on diff")
                .into(),
            "CollaboShootingStarStartHour" => val.collabo_shooting_star_start_hour
                .expect("CollaboShootingStarStartHour should have been read on diff")
                .into(),
            "Translate" => Byml::Map(val.translate
                .iter()
                .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
                .collect::<crate::util::HashMap<String, Byml>>()),
            "collaboSSFalloutFlagName" => val.collabo_ssfallout_flag_name
                .expect("collaboSSFalloutFlagName should have been read on diff")
                .into(),
            "collaboSSOpenFlagName" => val.collabo_ssopen_flag_name
                .expect("collaboSSOpenFlagName should have been read on diff")
                .into(),
            "collaboSSQuestFlag" => val.collabo_ssquest_flag
                .expect("collaboSSQuestFlag should have been read on diff")
                .into(),
        )
    }
}

impl Mergeable for CollabAnchor {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            collabo_shooting_star_direction: other.collabo_shooting_star_direction
                .ne(&self.collabo_shooting_star_direction)
                .then_some(other.collabo_shooting_star_direction)
                .unwrap_or_default(),
            collabo_shooting_star_end_hour: other.collabo_shooting_star_end_hour
                .ne(&self.collabo_shooting_star_end_hour)
                .then_some(other.collabo_shooting_star_end_hour)
                .unwrap_or_default(),
            collabo_shooting_star_start_hour: other.collabo_shooting_star_start_hour
                .ne(&self.collabo_shooting_star_start_hour)
                .then_some(other.collabo_shooting_star_start_hour)
                .unwrap_or_default(),
            translate: self.translate.diff(&other.translate),
            collabo_ssfallout_flag_name: other.collabo_ssfallout_flag_name
                .ne(&self.collabo_ssfallout_flag_name)
                .then(|| other.collabo_ssfallout_flag_name.clone())
                .unwrap_or_default(),
            collabo_ssopen_flag_name: other.collabo_ssopen_flag_name
                .ne(&self.collabo_ssopen_flag_name)
                .then(|| other.collabo_ssopen_flag_name.clone())
                .unwrap_or_default(),
            collabo_ssquest_flag: other.collabo_ssquest_flag
                .ne(&self.collabo_ssquest_flag)
                .then(|| other.collabo_ssquest_flag.clone())
                .unwrap_or_default(),
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
            translate: self.translate.merge(&diff.translate),
            collabo_ssfallout_flag_name: diff.collabo_ssfallout_flag_name.clone()
                .or_else(|| self.collabo_ssfallout_flag_name.clone()),
            collabo_ssopen_flag_name: diff.collabo_ssopen_flag_name.clone()
                .or_else(|| self.collabo_ssopen_flag_name.clone()),
            collabo_ssquest_flag: diff.collabo_ssquest_flag.clone()
                .or_else(|| self.collabo_ssquest_flag.clone()),
        }
    }
}
