use crate::UKError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            _ => Err(UKError::InvalidWeatherOrTime(val.to_owned())),
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
            _ => Err(UKError::InvalidWeatherOrTime(val.to_owned())),
        }
    }
}
