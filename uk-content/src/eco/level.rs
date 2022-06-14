use crate::{prelude::*, util::DeleteMap, Result, UKError};
use indexmap::IndexSet;
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
        let hash = byml.as_hash().unwrap();
        Ok(Self {
            enemy: hash
                .get("enemy")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing enemy section",
                )).unwrap()
                .as_array().unwrap()
                .iter()
                .map(|enemy| -> Result<(String, Series)> {
                    let enemy = enemy.as_hash().unwrap();
                    Ok((
                        enemy
                            .get("species")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor enemy entry missing species",
                            )).unwrap()
                            .as_string().unwrap()
                            .to_owned(),
                        enemy
                            .get("actors")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor enemy entry missing actors",
                            )).unwrap()
                            .as_array().unwrap()
                            .iter()
                            .map(|actor| -> Result<(String, f32)> {
                                let actor = actor.as_hash().unwrap();
                                Ok((
                                    actor
                                        .get("name")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Leven sensor enemy entry actor missing name",
                                        )).unwrap()
                                        .as_string().unwrap()
                                        .to_owned(),
                                    actor
                                        .get("value")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Leven sensor enemy entry actor missing value",
                                        )).unwrap()
                                        .as_float().unwrap(),
                                ))
                            })
                            .collect::<Result<_>>().unwrap(),
                    ))
                })
                .collect::<Result<_>>().unwrap(),
            flag: hash
                .get("flag")
                .ok_or(UKError::MissingBymlKey("Level sensor missing flag section")).unwrap()
                .as_array().unwrap()
                .iter()
                .map(|flag| -> Result<(String, f32)> {
                    let flag = flag.as_hash().unwrap();
                    Ok((
                        flag.get("name")
                            .ok_or(UKError::MissingBymlKey(
                                "Leven sensor flag entry missing name",
                            )).unwrap()
                            .as_string().unwrap()
                            .to_owned(),
                        flag.get("point")
                            .ok_or(UKError::MissingBymlKey(
                                "Leven sensor flag entry missing point",
                            )).unwrap()
                            .as_float().unwrap(),
                    ))
                })
                .collect::<Result<_>>().unwrap(),
            setting: hash
                .get("setting")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing setting section",
                )).unwrap()
                .as_hash().unwrap()
                .iter()
                .map(|(k, v)| -> Result<(String, f32)> { Ok((k.clone(), v.as_float().unwrap())) })
                .collect::<Result<_>>().unwrap(),
            weapon: hash.get("weapon").ok_or(UKError::MissingBymlKey("Level sensor missing weapon section")).unwrap().as_array().unwrap().iter().map(|weapon| -> Result<((String, String), WeaponSeries)> {
                let weapon = weapon.as_hash().unwrap();
                Ok((
                    (
                        weapon
                            .get("actorType")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor weapon entry missing actor type",
                            ))
                            .unwrap()
                            .as_string()
                            .unwrap()
                            .to_owned(),
                        weapon
                            .get("series")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor weapon entry missing series",
                            ))
                            .unwrap()
                            .as_string()
                            .unwrap()
                            .to_owned(),
                    ),
                    WeaponSeries {
                        not_rank_up: weapon
                            .get("not_rank_up")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor weapon entry missing not_rank_up",
                            ))
                            .unwrap()
                            .as_bool()
                            .unwrap(),
                        actors: weapon
                            .get("actors")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor weapon entry missing actors list",
                            ))
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|actor| -> Result<(String, (i32, f32))> {
                                let actor = actor.as_hash().unwrap();
                                Ok((
                                    actor
                                        .get("name")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Level sensor weapon actor entry missing name",
                                        ))
                                        .unwrap()
                                        .as_string()
                                        .unwrap()
                                        .to_owned(),
                                    (
                                        actor
                                            .get("plus")
                                            .ok_or(UKError::MissingBymlKey(
                                                "Level sensor weapon actor entry missing plus value",
                                            ))
                                            .unwrap()
                                            .as_int()
                                            .unwrap(),
                                        actor
                                            .get("value")
                                            .ok_or(UKError::MissingBymlKey(
                                                "Level sensor weapon actor entry missing value",
                                            ))
                                            .unwrap()
                                            .as_float()
                                            .unwrap(),
                                    ),
                                ))
                            })
                            .collect::<Result<_>>()
                            .unwrap(),
                    },
                ))
            }).collect::<Result<_>>().unwrap()
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
                                            ("name", Byml::String(actor)),
                                            ("value", Byml::Float(value)),
                                        ]
                                        .into_iter()
                                        .collect()
                                    })
                                    .collect(),
                            ),
                            ("species", Byml::String(species)),
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
                        [("name", Byml::String(flag)), ("point", Byml::Float(point))]
                            .into_iter()
                            .collect()
                    })
                    .collect(),
            ),
            (
                "setting",
                val.setting
                    .into_iter()
                    .map(|(setting, value)| (setting, Byml::Float(value)))
                    .collect(),
            ),
            (
                "weapon",
                val.weapon
                    .into_iter()
                    .map(|((actor_type, series), data)| -> Byml {
                        [
                            ("actorType", Byml::String(actor_type)),
                            (
                                "actors",
                                data.actors
                                    .into_iter()
                                    .map(|(actor, (plus, value))| -> Byml {
                                        [
                                            ("name", Byml::String(actor)),
                                            ("plus", Byml::Int(plus)),
                                            ("value", Byml::Float(value)),
                                        ]
                                        .into_iter()
                                        .collect()
                                    })
                                    .collect(),
                            ),
                            ("not_rank_up", Byml::Bool(data.not_rank_up)),
                            ("series", Byml::String(series)),
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
            weapon: other
                .weapon
                .iter()
                .filter_map(|(key, diff_series)| {
                    if let Some(self_series) = self.weapon.get(key) {
                        if self_series != diff_series {
                            Some((key.clone(), self_series.diff(diff_series), false))
                        } else {
                            None
                        }
                    } else {
                        Some((key.clone(), diff_series.clone(), false))
                    }
                })
                .chain(self.weapon.iter().filter_map(|(key, _)| {
                    (!other.weapon.contains_key(key))
                        .then(|| (key.clone(), Default::default(), true))
                }))
                .collect(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            enemy: self.enemy.deep_merge(&diff.enemy),
            flag: self.flag.merge(&diff.flag),
            setting: self.setting.merge(&diff.setting),
            weapon: {
                let key: IndexSet<_> = self
                    .weapon
                    .keys()
                    .chain(diff.weapon.keys())
                    .cloned()
                    .collect();
                key.into_iter().map(|key| {
                    let (self_series, diff_series) = (self.weapon.get(&key), diff.weapon.get(&key));
                    if let Some(self_series) = self_series && let Some(diff_series) = diff_series {
                        (key, self_series.merge(diff_series))
                    } else {
                        (key, diff_series.or(self_series).cloned().unwrap())
                    }
                })
                .collect::<DeleteMap<_, _>>()
            }
            .and_delete(),
        }
    }
}

impl Resource for LevelSensor {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("LevelSensor")
    }
}

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
}
