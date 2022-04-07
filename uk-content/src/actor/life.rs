use crate::{
    constants::{Time, Weather},
    prelude::Mergeable,
    util::DeleteSet,
    Result, UKError,
};
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LifeCondition {
    pub invalid_weathers: Option<DeleteSet<Weather>>,
    pub invalid_times: Option<DeleteSet<Time>>,
    pub display_dist: Option<f32>,
    pub auto_disp_dist_algo: Option<String>,
    pub y_limit_algo: Option<String>,
    pub delete_weathers: Option<DeleteSet<Weather>>,
    pub delete_times: Option<DeleteSet<Time>>,
}

impl TryFrom<&ParameterIO> for LifeCondition {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            invalid_weathers: pio
                .object("InvalidWeathers")
                .map(|weathers| -> Result<DeleteSet<Weather>> {
                    weathers
                        .params()
                        .values()
                        .map(|w| -> Result<(Weather, bool)> {
                            Ok((w.as_string()?.try_into()?, false))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            invalid_times: pio
                .object("InvalidTimes")
                .map(|times| -> Result<DeleteSet<Time>> {
                    times
                        .params()
                        .values()
                        .map(|w| -> Result<(Time, bool)> {
                            Ok((w.as_string()?.try_into()?, false))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            display_dist: Some(
                pio.object("DisplayDistance")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition missing display distance",
                    ))?
                    .param("Item")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition display distance missing item",
                    ))?
                    .as_f32()?,
            ),
            auto_disp_dist_algo: Some(
                pio.object("AutoDisplayDistanceAlgorithm")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition missing display distance",
                    ))?
                    .param("Item")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition display distance missing item",
                    ))?
                    .as_string()?
                    .to_owned(),
            ),
            y_limit_algo: Some(
                pio.object("YLimitAlgorithm")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition missing display distance",
                    ))?
                    .param("Item")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition display distance missing item",
                    ))?
                    .as_string()?
                    .to_owned(),
            ),
            delete_weathers: pio
                .object("DeleteWeathers")
                .map(|weathers| -> Result<DeleteSet<Weather>> {
                    weathers
                        .params()
                        .values()
                        .map(|w| -> Result<(Weather, bool)> {
                            Ok((w.as_string()?.try_into()?, false))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            delete_times: pio
                .object("DeleteTimes")
                .map(|times| -> Result<DeleteSet<Time>> {
                    times
                        .params()
                        .values()
                        .map(|w| -> Result<(Time, bool)> {
                            Ok((w.as_string()?.try_into()?, false))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
        })
    }
}

impl TryFrom<ParameterIO> for LifeCondition {
    type Error = UKError;

    fn try_from(pio: ParameterIO) -> Result<Self> {
        pio.try_into()
    }
}

impl From<LifeCondition> for ParameterIO {
    fn from(val: LifeCondition) -> ParameterIO {
        let mut pio = ParameterIO::new();
        if let Some(weathers) = val.invalid_weathers {
            pio.set_object(
                "InvalidWeathers",
                weathers
                    .into_iter()
                    .enumerate()
                    .map(|(i, weather)| {
                        (
                            format!("Item{:03}", i),
                            Parameter::String64(weather.to_string()),
                        )
                    })
                    .collect(),
            );
        }
        if let Some(times) = val.invalid_times {
            pio.set_object(
                "InvalidTimes",
                times
                    .into_iter()
                    .enumerate()
                    .map(|(i, time)| {
                        (
                            format!("Item{:03}", i),
                            Parameter::String64(time.to_string()),
                        )
                    })
                    .collect(),
            );
        }
        if let Some(display_dist) = val.display_dist {
            pio.set_object(
                "DisplayDistance",
                [("Item", Parameter::F32(display_dist))]
                    .into_iter()
                    .collect(),
            )
        }
        if let Some(auto_display_dist_algo) = val.auto_disp_dist_algo {
            pio.set_object(
                "AutoDisplayDistanceAlgorithm",
                [("Item", Parameter::StringRef(auto_display_dist_algo))]
                    .into_iter()
                    .collect(),
            );
        }
        if let Some(y_limit_algo) = val.y_limit_algo {
            pio.set_object(
                "YLimitAlgorithm",
                [("Item", Parameter::StringRef(y_limit_algo))]
                    .into_iter()
                    .collect(),
            );
        }
        if let Some(weathers) = val.delete_weathers {
            pio.set_object(
                "DeleteWeathers",
                weathers
                    .into_iter()
                    .enumerate()
                    .map(|(i, weather)| {
                        (
                            format!("Item{:03}", i),
                            Parameter::String64(weather.to_string()),
                        )
                    })
                    .collect(),
            );
        }
        if let Some(times) = val.delete_times {
            pio.set_object(
                "DeleteTimes",
                times
                    .into_iter()
                    .enumerate()
                    .map(|(i, time)| {
                        (
                            format!("Item{:03}", i),
                            Parameter::String64(time.to_string()),
                        )
                    })
                    .collect(),
            );
        }
        pio
    }
}

impl Mergeable<ParameterIO> for LifeCondition {
    fn diff(&self, other: &Self) -> Self {
        Self {
            invalid_weathers: self
                .invalid_weathers
                .as_ref()
                .and_then(|self_weathers| {
                    other.invalid_weathers.as_ref().map(|other_weathers| {
                        other_weathers
                            .iter()
                            .filter_map(|weather| {
                                (!self_weathers.contains(weather)).then(|| (*weather, false))
                            })
                            .chain(self_weathers.iter().filter_map(|weather| {
                                (!other_weathers.contains(weather)).then(|| (*weather, true))
                            }))
                            .collect()
                    })
                })
                .or_else(|| other.invalid_weathers.clone()),
            invalid_times: self
                .invalid_times
                .as_ref()
                .and_then(|self_times| {
                    other.invalid_times.as_ref().map(|other_times| {
                        other_times
                            .iter()
                            .filter_map(|time| (!self_times.contains(time)).then(|| (*time, false)))
                            .chain(self_times.iter().filter_map(|time| {
                                (!other_times.contains(time)).then(|| (*time, true))
                            }))
                            .collect()
                    })
                })
                .or_else(|| other.invalid_times.clone()),
            display_dist: (self.display_dist != other.display_dist)
                .then(|| other.display_dist)
                .flatten(),
            auto_disp_dist_algo: (self.auto_disp_dist_algo != other.auto_disp_dist_algo)
                .then(|| other.auto_disp_dist_algo.clone())
                .flatten(),
            y_limit_algo: (self.y_limit_algo != other.y_limit_algo)
                .then(|| other.y_limit_algo.clone())
                .flatten(),
            delete_weathers: self
                .delete_weathers
                .as_ref()
                .and_then(|self_weathers| {
                    other.delete_weathers.as_ref().map(|other_weathers| {
                        other_weathers
                            .iter()
                            .filter_map(|weather| {
                                (!self_weathers.contains(weather)).then(|| (*weather, false))
                            })
                            .chain(self_weathers.iter().filter_map(|weather| {
                                (!other_weathers.contains(weather)).then(|| (*weather, true))
                            }))
                            .collect()
                    })
                })
                .or_else(|| other.delete_weathers.clone()),
            delete_times: self
                .delete_times
                .as_ref()
                .and_then(|self_times| {
                    other.delete_times.as_ref().map(|other_times| {
                        other_times
                            .iter()
                            .filter_map(|time| (!self_times.contains(time)).then(|| (*time, false)))
                            .chain(self_times.iter().filter_map(|time| {
                                (!other_times.contains(time)).then(|| (*time, true))
                            }))
                            .collect()
                    })
                })
                .or_else(|| other.delete_times.clone()),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            invalid_weathers: {
                self.invalid_weathers
                    .as_ref()
                    .and_then(|base_weathers| {
                        diff.invalid_weathers
                            .as_ref()
                            .map(|diff_weathers| base_weathers.merge(diff_weathers))
                            .or_else(|| self.invalid_weathers.clone())
                    })
                    .or_else(|| diff.invalid_weathers.clone())
            },
            invalid_times: {
                self.invalid_times
                    .as_ref()
                    .and_then(|base_times| {
                        diff.invalid_times
                            .as_ref()
                            .map(|diff_times| base_times.merge(diff_times))
                            .or_else(|| self.invalid_times.clone())
                    })
                    .or_else(|| diff.invalid_times.clone())
            },
            display_dist: diff.display_dist.or(self.display_dist),
            auto_disp_dist_algo: diff
                .auto_disp_dist_algo
                .clone()
                .or_else(|| self.auto_disp_dist_algo.clone()),
            y_limit_algo: diff
                .y_limit_algo
                .clone()
                .or_else(|| self.y_limit_algo.clone()),
            delete_weathers: {
                self.delete_weathers
                    .as_ref()
                    .and_then(|base_weathers| {
                        diff.delete_weathers
                            .as_ref()
                            .map(|diff_weathers| base_weathers.merge(diff_weathers))
                            .or_else(|| self.delete_weathers.clone())
                    })
                    .or_else(|| diff.delete_weathers.clone())
            },
            delete_times: {
                self.delete_times
                    .as_ref()
                    .and_then(|base_times| {
                        diff.delete_times
                            .as_ref()
                            .map(|diff_times| base_times.merge(diff_times))
                            .or_else(|| self.delete_times.clone())
                    })
                    .or_else(|| diff.delete_times.clone())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let data = lifecondition.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let lifecondition2 = super::LifeCondition::try_from(&pio2).unwrap();
        assert_eq!(lifecondition, lifecondition2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition2 = super::LifeCondition::try_from(&pio2).unwrap();
        let diff = lifecondition.diff(&lifecondition2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition2 = super::LifeCondition::try_from(&pio2).unwrap();
        let diff = lifecondition.diff(&lifecondition2);
        let merged = lifecondition.merge(&diff);
        println!("{}", serde_json::to_string_pretty(&merged).unwrap());
        assert_eq!(lifecondition2, merged);
    }
}
