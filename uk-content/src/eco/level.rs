use crate::{prelude::*, util::DeleteMap, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

type Series = DeleteMap<String, f32>;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct WeaponSeries {
    pub actors: DeleteMap<String, (i32, f32)>,
    pub not_rank_up: bool,
}

impl Mergeable for WeaponSeries {
    fn diff(&self, other: &Self) -> Self {
        Self {
            actors: self.actors.diff(&other.actors),
            not_rank_up: other.not_rank_up,
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            actors: self.actors.merge(&diff.actors),
            not_rank_up: diff.not_rank_up,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct LevelSensor {
    pub enemy: DeleteMap<String, Series>,
    pub flag: Series,
    pub setting: Series,
    pub weapon: DeleteMap<(String, String), WeaponSeries>,
}

impl TryFrom<&Byml> for LevelSensor {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            enemy: hash
                .get("enemy")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing enemy section",
                ))?
                .as_array()?
                .iter()
                .map(|enemy| -> Result<(String, Series)> {
                    let enemy = enemy.as_hash()?;
                    Ok((
                        enemy
                            .get("species")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor enemy entry missing species",
                            ))?
                            .as_string()?.into(),
                        enemy
                            .get("actors")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor enemy entry missing actors",
                            ))?
                            .as_array()?
                            .iter()
                            .map(|actor| -> Result<(String, f32)> {
                                let actor = actor.as_hash()?;
                                Ok((
                                    actor
                                        .get("name")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Leven sensor enemy entry actor missing name",
                                        ))?
                                        .as_string()?.into(),
                                    actor
                                        .get("value")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Leven sensor enemy entry actor missing value",
                                        ))?
                                        .as_float()?,
                                ))
                            })
                            .collect::<Result<_>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
            flag: hash
                .get("flag")
                .ok_or(UKError::MissingBymlKey("Level sensor missing flag section"))?
                .as_array()?
                .iter()
                .map(|flag| -> Result<(String, f32)> {
                    let flag = flag.as_hash()?;
                    Ok((
                        flag.get("name")
                            .ok_or(UKError::MissingBymlKey(
                                "Leven sensor flag entry missing name",
                            ))?
                            .as_string()?.into(),
                        flag.get("point")
                            .ok_or(UKError::MissingBymlKey(
                                "Leven sensor flag entry missing point",
                            ))?
                            .as_float()?,
                    ))
                })
                .collect::<Result<_>>()?,
            setting: hash
                .get("setting")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing setting section",
                ))?
                .as_hash()?
                .iter()
                .map(|(k, v)| -> Result<(String, f32)> {
                    Ok((k.into(), v.as_float()?))
                })
                .collect::<Result<_>>()?,
            weapon: hash.get("weapon")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing weapon section",
                ))?
                .as_array()?
                .iter()
                .map(|weapon| -> Result<((String, String), WeaponSeries)> {
                    let weapon = weapon.as_hash()?;
                    Ok((
                        (
                            weapon
                                .get("actorType")
                                .ok_or(UKError::MissingBymlKey(
                                    "Level sensor weapon entry missing actor type",
                                ))?
                                .as_string()?.into(),
                            weapon
                                .get("series")
                                .ok_or(UKError::MissingBymlKey(
                                    "Level sensor weapon entry missing series",
                                ))?
                                .as_string()?.into(),
                        ),
                        WeaponSeries {
                            not_rank_up: weapon
                                .get("not_rank_up")
                                .ok_or(UKError::MissingBymlKey(
                                    "Level sensor weapon entry missing not_rank_up",
                                ))?
                                .as_bool()?,
                            actors: weapon
                                .get("actors")
                                .ok_or(UKError::MissingBymlKey(
                                    "Level sensor weapon entry missing actors list",
                                ))?
                                .as_array()?
                                .iter()
                                .map(|actor| -> Result<(String, (i32, f32))> {
                                    let actor = actor.as_hash()?;
                                    Ok((
                                        actor
                                            .get("name")
                                            .ok_or(UKError::MissingBymlKey(
                                                "Level sensor weapon actor entry missing name",
                                            ))?
                                            .as_string()?.into(),
                                        (
                                            actor
                                                .get("plus")
                                                .ok_or(UKError::MissingBymlKey(
                                                    "Level sensor weapon actor entry missing plus value",
                                                ))?
                                                .as_int()?,
                                            actor
                                                .get("value")
                                                .ok_or(UKError::MissingBymlKey(
                                                    "Level sensor weapon actor entry missing value",
                                                ))?
                                                .as_float()?,
                                        ),
                                    ))
                                })
                                .collect::<Result<_>>()?,
                        },
                    ))
                })
                .collect::<Result<_>>()?
        })
    }
}

impl From<LevelSensor> for Byml {
    fn from(val: LevelSensor) -> Self {
        [
            (
                "enemy",
                val.enemy
                    .into_iter()
                    .map(|(species, actors): (String, Series)| -> Byml {
                        [
                            (
                                "actors",
                                actors
                                    .into_iter()
                                    .map(|(actor, value)| -> Byml {
                                        [
                                            ("name", Byml::String(actor.into())),
                                            ("value", Byml::Float(value)),
                                        ]
                                        .into_iter()
                                        .collect()
                                    })
                                    .collect(),
                            ),
                            ("species", Byml::String(species.into())),
                        ]
                        .into_iter()
                        .collect()
                    })
                    .collect(),
            ),
            (
                "flag",
                val.flag
                    .into_iter()
                    .map(|(flag, point)| -> Byml {
                        [
                            ("name", Byml::String(flag.into())),
                            ("point", Byml::Float(point)),
                        ]
                        .into_iter()
                        .collect()
                    })
                    .collect(),
            ),
            (
                "setting",
                val.setting
                    .into_iter()
                    .map(|(setting, value)| (setting.to_string(), Byml::Float(value)))
                    .collect(),
            ),
            (
                "weapon",
                val.weapon
                    .into_iter()
                    .map(|((actor_type, series), data)| -> Byml {
                        [
                            ("actorType", Byml::String(actor_type.into())),
                            (
                                "actors",
                                data.actors
                                    .into_iter()
                                    .map(|(actor, (plus, value))| -> Byml {
                                        [
                                            ("name", Byml::String(actor.into())),
                                            ("plus", Byml::Int(plus)),
                                            ("value", Byml::Float(value)),
                                        ]
                                        .into_iter()
                                        .collect()
                                    })
                                    .collect(),
                            ),
                            ("not_rank_up", Byml::Bool(data.not_rank_up)),
                            ("series", Byml::String(series.into())),
                        ]
                        .into_iter()
                        .collect()
                    })
                    .collect(),
            ),
        ]
        .into_iter()
        .collect()
    }
}

impl Mergeable for LevelSensor {
    fn diff(&self, other: &Self) -> Self {
        Self {
            enemy: self.enemy.deep_diff(&other.enemy),
            flag: self.flag.diff(&other.flag),
            setting: self.setting.diff(&other.setting),
            weapon: self.weapon.deep_diff(&other.weapon),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            enemy: self.enemy.deep_merge(&diff.enemy),
            flag: self.flag.merge(&diff.flag),
            setting: self.setting.merge(&diff.setting),
            weapon: self.weapon.deep_merge(&diff.weapon),
        }
    }
}

impl Resource for LevelSensor {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> roead::Bytes {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("LevelSensor")
    }
}

single_path!(LevelSensor, "Pack/Bootup.pack//Ecosystem/LevelSensor.sbyml");

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_sensor() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Ecosystem/LevelSensor.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_sensor() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Ecosystem/LevelSensor.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_sensor();
        let sensor = super::LevelSensor::try_from(&byml).unwrap();
        let data = Byml::from(sensor.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let sensor2 = super::LevelSensor::try_from(&byml2).unwrap();
        assert_eq!(sensor, sensor2);
    }

    #[test]
    fn diff() {
        let byml = load_sensor();
        let sensor = super::LevelSensor::try_from(&byml).unwrap();
        let byml2 = load_mod_sensor();
        let sensor2 = super::LevelSensor::try_from(&byml2).unwrap();
        let _diff = sensor.diff(&sensor2);
    }

    #[test]
    fn merge() {
        let byml = load_sensor();
        let sensor = super::LevelSensor::try_from(&byml).unwrap();
        let byml2 = load_mod_sensor();
        let sensor2 = super::LevelSensor::try_from(&byml2).unwrap();
        let diff = sensor.diff(&sensor2);
        let merged = sensor.merge(&diff);
        assert_eq!(merged, sensor2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//Ecosystem/LevelSensor.sbyml");
        assert!(super::LevelSensor::path_matches(path));
    }
}
