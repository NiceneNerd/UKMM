use std::{fmt, path::Path, str::FromStr};

use join_str::jstr;
use lighter::lighter;
use serde::{Deserialize, Serialize};

use crate::UKError;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[default]
    USen,
    EUen,
    USfr,
    USes,
    EUde,
    EUes,
    EUfr,
    EUit,
    EUnl,
    EUru,
    CNzh,
    JPja,
    KRko,
    TWzh,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl Language {
    pub fn iter() -> std::slice::Iter<'static, Self> {
        [
            Self::USen,
            Self::EUen,
            Self::USfr,
            Self::USes,
            Self::EUde,
            Self::EUes,
            Self::EUfr,
            Self::EUit,
            Self::EUnl,
            Self::EUru,
            Self::CNzh,
            Self::JPja,
            Self::KRko,
            Self::TWzh,
        ]
        .iter()
    }

    #[inline(always)]
    pub fn to_str(self) -> &'static str {
        self.into()
    }

    #[inline(always)]
    pub fn short(&self) -> &'static str {
        &self.to_str()[2..4]
    }

    pub fn nearest<'l>(&self, langs: &'l [Self]) -> &'l Self {
        langs
            .iter()
            .find(|lang| *lang == self)
            .or_else(|| langs.iter().find(|lang| lang.short() == self.short()))
            .or_else(|| langs.iter().find(|lang| lang.short() == "en"))
            .or_else(|| langs.first())
            .unwrap_or(&Language::USen)
    }

    #[inline(always)]
    pub fn from_path(path: &Path) -> Option<Self> {
        path.file_stem()
            .and_then(|n| n.to_str())
            .filter(|n| n.len() >= 4)
            .and_then(|n| Self::from_str(&n[n.len() - 4..]).ok())
    }

    #[inline(always)]
    pub fn from_message_path(path: &Path) -> Option<Self> {
        path.file_stem()
            .and_then(|n| n.to_str())
            .filter(|n| n.len() >= 4)
            .and_then(|n| Self::from_str(&n[n.len() - 12..n.len() - 8]).ok())
    }

    #[inline]
    pub fn bootup_path(&self) -> smartstring::alias::String {
        let mut string = smartstring::alias::String::from("Pack/Bootup_");
        string.push_str(self.to_str());
        string.push_str(".pack");
        string
    }

    #[inline]
    pub fn message_path(&self) -> smartstring::alias::String {
        let mut string = smartstring::alias::String::from("Message/Msg_");
        string.push_str(self.to_str());
        string.push_str(".product.ssarc");
        string
    }
}

impl FromStr for Language {
    type Err = UKError;

    #[allow(clippy::needless_borrow)]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        lighter! {
            match s {
                "USen" => Ok(Language::USen),
                "EUen" => Ok(Language::EUen),
                "USfr" => Ok(Language::USfr),
                "USes" => Ok(Language::USes),
                "EUde" => Ok(Language::EUde),
                "EUes" => Ok(Language::EUes),
                "EUfr" => Ok(Language::EUfr),
                "EUit" => Ok(Language::EUit),
                "EUnl" => Ok(Language::EUnl),
                "EUru" => Ok(Language::EUru),
                "CNzh" => Ok(Language::CNzh),
                "JPja" => Ok(Language::JPja),
                "KRko" => Ok(Language::KRko),
                "TWzh" => Ok(Language::TWzh),
                _ => Err(UKError::OtherD(jstr!("Invalid language: {s}"))),
            }
        }
    }
}

impl From<Language> for &str {
    fn from(lang: Language) -> Self {
        match lang {
            Language::USen => "USen",
            Language::EUen => "EUen",
            Language::USfr => "USfr",
            Language::USes => "USes",
            Language::EUde => "EUde",
            Language::EUes => "EUes",
            Language::EUfr => "EUfr",
            Language::EUit => "EUit",
            Language::EUnl => "EUnl",
            Language::EUru => "EUru",
            Language::CNzh => "CNzh",
            Language::JPja => "JPja",
            Language::KRko => "KRko",
            Language::TWzh => "TWzh",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]

pub enum Weather {
    #[default]
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

    #[allow(clippy::needless_borrow)]
    fn try_from(val: &str) -> Result<Weather, UKError> {
        lighter! {
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

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]

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

    #[allow(clippy::needless_borrow)]
    fn try_from(val: &str) -> Result<Time, UKError> {
        lighter! {
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
