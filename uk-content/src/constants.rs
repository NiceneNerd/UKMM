use crate::UKError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uk_ui_derive::Editable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Editable)]
pub enum Weather {
    Bluesky,
    Cloudy,
    Rain,
    HeavyRain,
    Snow,
    HeavySnow,
    ThunderStorm,
    ThunderRain,
    BlueskyRain,
}

impl TryFrom<&str> for Weather {
    type Error = UKError;

    fn try_from(val: &str) -> Result<Weather, UKError> {
        match val {
            "Bluesky" => Ok(Weather::Bluesky),
            "Cloudy" => Ok(Weather::Cloudy),
            "Rain" => Ok(Weather::Rain),
            "HeavyRain" => Ok(Weather::HeavyRain),
            "Snow" => Ok(Weather::Snow),
            "HeavySnow" => Ok(Weather::HeavySnow),
            "ThunderStorm" => Ok(Weather::ThunderStorm),
            "ThunderRain" => Ok(Weather::ThunderRain),
            "BlueskyRain" => Ok(Weather::BlueskyRain),
            _ => Err(UKError::InvalidWeatherOrTime(val.into())),
        }
    }
}

impl<const N: usize> From<Weather> for roead::types::FixedSafeString<N> {
    fn from(w: Weather) -> Self {
        match w {
            Weather::Bluesky => "Bluesky".into(),
            Weather::Cloudy => "Cloudy".into(),
            Weather::Rain => "Rain".into(),
            Weather::HeavyRain => "HeavyRain".into(),
            Weather::Snow => "Snow".into(),
            Weather::HeavySnow => "HeavySnow".into(),
            Weather::ThunderStorm => "ThunderStorm".into(),
            Weather::ThunderRain => "ThunderRain".into(),
            Weather::BlueskyRain => "BlueskyRain".into(),
        }
    }
}

impl From<&Weather> for smartstring::alias::String {
    fn from(w: &Weather) -> Self {
        match *w {
            Weather::Bluesky => "Bluesky".into(),
            Weather::Cloudy => "Cloudy".into(),
            Weather::Rain => "Rain".into(),
            Weather::HeavyRain => "HeavyRain".into(),
            Weather::Snow => "Snow".into(),
            Weather::HeavySnow => "HeavySnow".into(),
            Weather::ThunderStorm => "ThunderStorm".into(),
            Weather::ThunderRain => "ThunderRain".into(),
            Weather::BlueskyRain => "BlueskyRain".into(),
        }
    }
}

impl fmt::Display for Weather {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl Default for Weather {
    fn default() -> Self {
        Weather::Bluesky
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Editable)]
pub enum Time {
    Morning_A,
    Morning_B,
    Noon_A,
    Noon_B,
    Evening_A,
    Evening_B,
    Night_A,
    Night_B,
    Morning_A1,
    Morning_A2,
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::Morning_A
    }
}

impl TryFrom<&str> for Time {
    type Error = UKError;

    fn try_from(val: &str) -> Result<Time, UKError> {
        match val {
            "Morning_A" => Ok(Time::Morning_A),
            "Morning_B" => Ok(Time::Morning_B),
            "Noon_A" => Ok(Time::Noon_A),
            "Noon_B" => Ok(Time::Noon_B),
            "Evening_A" => Ok(Time::Evening_A),
            "Evening_B" => Ok(Time::Evening_B),
            "Night_A" => Ok(Time::Night_A),
            "Night_B" => Ok(Time::Night_B),
            "Morning_A1" => Ok(Time::Morning_A1),
            "Morning_A2" => Ok(Time::Morning_A2),
            _ => Err(UKError::InvalidWeatherOrTime(val.into())),
        }
    }
}

impl<const N: usize> From<Time> for roead::types::FixedSafeString<N> {
    fn from(t: Time) -> Self {
        match t {
            Time::Morning_A => "Morning_A".into(),
            Time::Morning_B => "Morning_B".into(),
            Time::Noon_A => "Noon_A".into(),
            Time::Noon_B => "Noon_B".into(),
            Time::Evening_A => "Evening_A".into(),
            Time::Evening_B => "Evening_B".into(),
            Time::Night_A => "Night_A".into(),
            Time::Night_B => "Night_B".into(),
            Time::Morning_A1 => "Morning_A1".into(),
            Time::Morning_A2 => "Morning_A2".into(),
        }
    }
}

impl From<&Time> for smartstring::alias::String {
    fn from(t: &Time) -> Self {
        match *t {
            Time::Morning_A => "Morning_A".into(),
            Time::Morning_B => "Morning_B".into(),
            Time::Noon_A => "Noon_A".into(),
            Time::Noon_B => "Noon_B".into(),
            Time::Evening_A => "Evening_A".into(),
            Time::Evening_B => "Evening_B".into(),
            Time::Night_A => "Night_A".into(),
            Time::Night_B => "Night_B".into(),
            Time::Morning_A1 => "Morning_A1".into(),
            Time::Morning_A2 => "Morning_A2".into(),
        }
    }
}

pub static TITLE_ACTORS: &[&str] = &[
    "AncientArrow",
    "Animal_Insect_A",
    "Animal_Insect_B",
    "Animal_Insect_F",
    "Animal_Insect_H",
    "Animal_Insect_M",
    "Animal_Insect_S",
    "Animal_Insect_X",
    "Armor_Default_Extra_00",
    "Armor_Default_Extra_01",
    "BombArrow_A",
    "BrightArrow",
    "BrightArrowTP",
    "CarryBox",
    "DemoXLinkActor",
    "Dm_Npc_Gerudo_HeroSoul_Kago",
    "Dm_Npc_Goron_HeroSoul_Kago",
    "Dm_Npc_RevivalFairy",
    "Dm_Npc_Rito_HeroSoul_Kago",
    "Dm_Npc_Zora_HeroSoul_Kago",
    "ElectricArrow",
    "ElectricWaterBall",
    "EventCameraRumble",
    "EventControllerRumble",
    "EventMessageTransmitter1",
    "EventSystemActor",
    "Explode",
    "Fader",
    "FireArrow",
    "FireRodLv1Fire",
    "FireRodLv2Fire",
    "FireRodLv2FireChild",
    "GameROMPlayer",
    "IceArrow",
    "IceRodLv1Ice",
    "IceRodLv2Ice",
    "Item_Conductor",
    "Item_Magnetglove",
    "Item_Material_01",
    "Item_Material_03",
    "Item_Material_07",
    "Item_Ore_F",
    "NormalArrow",
    "Obj_IceMakerBlock",
    "Obj_SupportApp_Wind",
    "PlayerShockWave",
    "PlayerStole2",
    "RemoteBomb",
    "RemoteBomb2",
    "RemoteBombCube",
    "RemoteBombCube2",
    "SceneSoundCtrlTag",
    "SoundTriggerTag",
    "TerrainCalcCenterTag",
    "ThunderRodLv1Thunder",
    "ThunderRodLv2Thunder",
    "ThunderRodLv2ThunderChild",
    "WakeBoardRope",
];
