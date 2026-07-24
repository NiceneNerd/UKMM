use itertools::Itertools;
use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::DeleteMap, Result, UKError};

type Series = DeleteMap<String, (f32, usize)>;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct WeaponSeries {
    pub actors: DeleteMap<(String, i32), f32>,
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
    pub enemy:   DeleteMap<String, Series>,
    pub flag:    Series,
    pub setting: Series,
    pub weapon:  DeleteMap<String, DeleteMap<String, WeaponSeries>>,
}

impl TryFrom<&Byml> for LevelSensor {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        Ok(Self {
            enemy:   hash
                .get("enemy")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing enemy section",
                ))?
                .as_array()?
                .iter()
                .map(|enemy| -> Result<(String, Series)> {
                    let enemy = enemy.as_map()?;
                    Ok((
                        enemy
                            .get("species")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor enemy entry missing species",
                            ))?
                            .as_string()?
                            .clone(),
                        enemy
                            .get("actors")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor enemy entry missing actors",
                            ))?
                            .as_array()?
                            .iter()
                            .enumerate()
                            .map(|(idx, actor)| -> Result<(String, (f32, usize))> {
                                let actor = actor.as_map()?;
                                Ok((
                                    actor
                                        .get("name")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Level sensor enemy entry actor missing name",
                                        ))?
                                        .as_string()?
                                        .clone(),
                                    (
                                        actor
                                            .get("value")
                                            .ok_or(UKError::MissingBymlKey(
                                                "Level sensor enemy entry actor missing value",
                                            ))?
                                            .as_float()?,
                                        idx,
                                    )
                                ))
                            })
                            .collect::<Result<_>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
            flag:    hash
                .get("flag")
                .ok_or(UKError::MissingBymlKey("Level sensor missing flag section"))?
                .as_array()?
                .iter()
                .enumerate()
                .map(|(idx, flag)| -> Result<(String, (f32, usize))> {
                    let flag = flag.as_map()?;
                    Ok((
                        flag.get("name")
                            .ok_or(UKError::MissingBymlKey(
                                "Leven sensor flag entry missing name",
                            ))?
                            .as_string()?
                            .clone(),
                        (
                            flag.get("point")
                                .ok_or(UKError::MissingBymlKey(
                                    "Leven sensor flag entry missing point",
                                ))?
                                .as_float()?,
                            idx,
                        )
                    ))
                })
                .collect::<Result<_>>()?,
            setting: hash
                .get("setting")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing setting section",
                ))?
                .as_map()?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| -> Result<(String, (f32, usize))> {
                    Ok((
                        k.clone(),
                        (
                            v.as_float()?,
                            i,
                        )
                    ))
                })
                .collect::<Result<_>>()?,
            weapon:  hash
                .get("weapon")
                .ok_or(UKError::MissingBymlKey(
                    "Level sensor missing weapon section",
                ))?
                .as_array()?
                .iter()
                .try_fold(
                    DeleteMap::default(),
                    |mut weapons, weapon| -> Result<DeleteMap<_, _>> {
                        let weapon = weapon.as_map()?;
                        let series = weapon
                            .get("series")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor weapons entry missing series name",
                            ))?
                            .as_string()?
                            .clone();
                        let series_map: &mut DeleteMap<String, WeaponSeries> =
                            weapons.get_or_insert_default(series);
                        let actor_type = weapon
                            .get("actorType")
                            .ok_or(UKError::MissingBymlKey(
                                "Level sensor weapons entry missing actor type",
                            ))?
                            .as_string()?
                            .clone();
                        series_map.insert(actor_type, WeaponSeries {
                            actors: weapon
                                .get("actors")
                                .ok_or(UKError::MissingBymlKey(
                                    "Level sensor weapon entry missing actor list",
                                ))?
                                .as_array()?
                                .iter()
                                .map(|actor| -> Result<((String, i32), f32)> {
                                    let actor = actor.as_map()?;
                                    Ok((
                                        (
                                            actor
                                                .get("name")
                                                .ok_or(UKError::MissingBymlKey(
                                                    "Level sensor weapons actor entry missing name",
                                                ))?
                                                .as_string()?
                                                .clone(),
                                            actor
                                                .get("plus")
                                                .ok_or(UKError::MissingBymlKey(
                                                    "Level sensor weapons actor entry missing \
                                                     plus value",
                                                ))?
                                                .as_int()?,
                                        ),
                                        actor
                                            .get("value")
                                            .ok_or(UKError::MissingBymlKey(
                                                "Level sensor weapons actor entry missing value",
                                            ))?
                                            .as_float()?,
                                    ))
                                })
                                .collect::<Result<_>>()?,
                            not_rank_up: weapon
                                .get("not_rank_up")
                                .and_then(|v| v.as_bool().ok())
                                .unwrap_or_default(),
                        });
                        Ok(weapons)
                    },
                )?,
        })
    }
}

impl From<LevelSensor> for Byml {
    fn from(val: LevelSensor) -> Self {
        map!(
            "enemy" => val.enemy
                .into_iter()
                .map(|(species, actors): (String, Series)| -> Byml {
                    map!(
                        "actors" =>  actors
                            .into_iter()
                            .sorted_by_key(|x| x.1.1)
                            .map(|(actor, value)| -> Byml {
                                map!(
                                    "name" => Byml::String(actor),
                                    "value" => Byml::Float(value.0),
                                )
                            })
                            .collect(),
                        "species" => Byml::String(species)
                    )
                })
                .collect(),
            "flag" => val.flag
                .into_iter()
                .sorted_by_key(|x| x.1.1)
                .map(|(flag, point)| -> Byml {
                    map!(
                        "name" => Byml::String(flag),
                        "point" => Byml::Float(point.0)
                    )
                })
                .collect(),
            "setting" => val.setting
                .into_iter()
                .sorted_by_key(|x| x.1.1)
                .map(|(setting, value)| (setting.to_string(), Byml::Float(value.0)))
                .collect(),
            "weapon" => val.weapon
                .into_iter()
                .flat_map(|(series, type_map)| -> Vec<Byml> {
                    type_map
                        .into_iter()
                        .map(|(actor_type, weapons)| {
                            map!(
                                "actorType" => actor_type.into(),
                                "actors" => weapons
                                    .actors
                                    .into_iter()
                                    .map(|((name, plus), value)| {
                                        map!(
                                            "name" => name.into(),
                                            "plus" => plus.into(),
                                            "value" => value.into()
                                        )
                                    })
                                    .collect::<Byml>(),
                                "not_rank_up" => weapons.not_rank_up.into(),
                                "series" => series.clone().into(),
                            )
                        })
                        .collect::<Vec<Byml>>()
                })
                .collect()
        )
    }
}

impl Mergeable for LevelSensor {
    fn diff(&self, other: &Self) -> Self {
        Self {
            enemy:   self.enemy.deep_diff(&other.enemy),
            flag:    self.flag.diff(&other.flag),
            setting: self.setting.diff(&other.setting),
            weapon:  self.weapon.deep_diff(&other.weapon),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            enemy:   self.enemy.deep_merge(&diff.enemy),
            flag:    self.flag.merge(&diff.flag),
            setting: self.setting.merge(&diff.setting),
            weapon:  self.weapon.deep_merge(&diff.weapon),
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

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_sensor() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Ecosystem/LevelSensor.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_sensor() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Ecosystem/LevelSensor.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_sensor();
        let sensor = super::LevelSensor::try_from(&byml).unwrap();
        let data = Byml::from(sensor.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
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
