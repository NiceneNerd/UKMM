use std::{collections::HashMap, str::FromStr};

use crate::{
    constants::{Time, Weather},
    prelude::Mergeable,
    Result, UKError,
};
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LifeCondition {
    pub invalid_weathers: Option<HashMap<Weather, bool>>,
    pub invalid_times: Option<HashMap<Time, bool>>,
    pub display_dist: Option<f32>,
    pub auto_disp_dist_algo: Option<String>,
    pub y_limit_algo: Option<String>,
    pub delete_weathers: Option<HashMap<Weather, bool>>,
    pub delete_times: Option<HashMap<Time, bool>>,
}

impl TryFrom<&ParameterIO> for LifeCondition {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            invalid_weathers: pio
                .object("InvalidWeathers")
                .map(|weathers| -> Result<HashMap<Weather, bool>> {
                    weathers
                        .params()
                        .values()
                        .map(|w| -> Result<(Weather, bool)> {
                            Ok((Weather::from_str(w.as_string()?)?, false))
                        })
                        .collect::<Result<HashMap<_, _>>>()
                })
                .transpose()?,
            invalid_times: pio
                .object("InvalidTimes")
                .map(|times| -> Result<HashMap<Time, bool>> {
                    times
                        .params()
                        .values()
                        .map(|w| -> Result<(Time, bool)> {
                            Ok((Time::from_str(w.as_string()?)?, false))
                        })
                        .collect::<Result<HashMap<_, _>>>()
                })
                .transpose()?,
            display_dist: Some(
                pio.object("DisplayDistance")
                    .ok_or_else(|| {
                        UKError::MissingAampKey(
                            "Life condition missing display distance".to_owned(),
                        )
                    })?
                    .param("Item")
                    .ok_or_else(|| {
                        UKError::MissingAampKey(
                            "Life condition display distance missing item".to_owned(),
                        )
                    })?
                    .as_f32()?,
            ),
            auto_disp_dist_algo: Some(
                pio.object("AutoDisplayDistanceAlgorithm")
                    .ok_or_else(|| {
                        UKError::MissingAampKey(
                            "Life condition missing display distance".to_owned(),
                        )
                    })?
                    .param("Item")
                    .ok_or_else(|| {
                        UKError::MissingAampKey(
                            "Life condition display distance missing item".to_owned(),
                        )
                    })?
                    .as_string()?
                    .to_owned(),
            ),
            y_limit_algo: Some(
                pio.object("YLimitAlgorithm")
                    .ok_or_else(|| {
                        UKError::MissingAampKey(
                            "Life condition missing display distance".to_owned(),
                        )
                    })?
                    .param("Item")
                    .ok_or_else(|| {
                        UKError::MissingAampKey(
                            "Life condition display distance missing item".to_owned(),
                        )
                    })?
                    .as_string()?
                    .to_owned(),
            ),
            delete_weathers: pio
                .object("DeleteWeathers")
                .map(|weathers| -> Result<HashMap<Weather, bool>> {
                    weathers
                        .params()
                        .values()
                        .map(|w| -> Result<(Weather, bool)> {
                            Ok((Weather::from_str(w.as_string()?)?, false))
                        })
                        .collect::<Result<HashMap<_, _>>>()
                })
                .transpose()?,
            delete_times: pio
                .object("DeleteTimes")
                .map(|times| -> Result<HashMap<Time, bool>> {
                    times
                        .params()
                        .values()
                        .map(|w| -> Result<(Time, bool)> {
                            Ok((Time::from_str(w.as_string()?)?, false))
                        })
                        .collect::<Result<HashMap<_, _>>>()
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
                ParameterObject(
                    weathers
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, (weather, delete))| {
                            if delete {
                                None
                            } else {
                                Some((
                                    hash_name(&format!("Item{:03}", i)),
                                    Parameter::String64(weather.to_string()),
                                ))
                            }
                        })
                        .collect(),
                ),
            );
        }
        if let Some(times) = val.invalid_times {
            pio.set_object(
                "InvalidTimes",
                ParameterObject(
                    times
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, (time, delete))| {
                            if delete {
                                None
                            } else {
                                Some((
                                    hash_name(&format!("Item{:03}", i)),
                                    Parameter::String64(time.to_string()),
                                ))
                            }
                        })
                        .collect(),
                ),
            );
        }
        if let Some(display_dist) = val.display_dist {
            pio.set_object(
                "DisplayDistance",
                ParameterObject(
                    [(hash_name("Item"), Parameter::F32(display_dist))]
                        .into_iter()
                        .collect(),
                ),
            )
        }
        if let Some(auto_display_dist_algo) = val.auto_disp_dist_algo {
            pio.set_object(
                "AutoDisplayDistanceAlgorithm",
                ParameterObject(
                    [(
                        hash_name("Item"),
                        Parameter::StringRef(auto_display_dist_algo),
                    )]
                    .into_iter()
                    .collect(),
                ),
            );
        }
        if let Some(y_limit_algo) = val.y_limit_algo {
            pio.set_object(
                "YLimitAlgorithm",
                ParameterObject(
                    [(hash_name("Item"), Parameter::StringRef(y_limit_algo))]
                        .into_iter()
                        .collect(),
                ),
            );
        }
        if let Some(weathers) = val.delete_weathers {
            pio.set_object(
                "DeleteWeathers",
                ParameterObject(
                    weathers
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, (weather, delete))| {
                            if delete {
                                None
                            } else {
                                Some((
                                    hash_name(&format!("Item{:03}", i)),
                                    Parameter::String64(weather.to_string()),
                                ))
                            }
                        })
                        .collect(),
                ),
            );
        }
        if let Some(times) = val.delete_times {
            pio.set_object(
                "DeleteTimes",
                ParameterObject(
                    times
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, (time, delete))| {
                            if delete {
                                None
                            } else {
                                Some((
                                    hash_name(&format!("Item{:03}", i)),
                                    Parameter::String64(time.to_string()),
                                ))
                            }
                        })
                        .collect(),
                ),
            );
        }
        pio
    }
}

impl Mergeable for LifeCondition {
    fn diff(&self, other: &Self) -> Self {
        Self {
            invalid_weathers: self
                .invalid_weathers
                .as_ref()
                .and_then(|self_weathers| {
                    other.invalid_weathers.as_ref().map(|other_weathers| {
                        other_weathers
                            .iter()
                            .filter_map(|(weather, _)| {
                                (!self_weathers.contains_key(weather)).then(|| (*weather, false))
                            })
                            .chain(self_weathers.iter().filter_map(|(weather, _)| {
                                (!other_weathers.contains_key(weather)).then(|| (*weather, true))
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
                            .filter_map(|(time, _)| {
                                (!self_times.contains_key(time)).then(|| (*time, false))
                            })
                            .chain(self_times.iter().filter_map(|(time, _)| {
                                (!other_times.contains_key(time)).then(|| (*time, true))
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
                            .filter_map(|(weather, _)| {
                                (!self_weathers.contains_key(weather)).then(|| (*weather, false))
                            })
                            .chain(self_weathers.iter().filter_map(|(weather, _)| {
                                (!other_weathers.contains_key(weather)).then(|| (*weather, true))
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
                            .filter_map(|(time, _)| {
                                (!self_times.contains_key(time)).then(|| (*time, false))
                            })
                            .chain(self_times.iter().filter_map(|(time, _)| {
                                (!other_times.contains_key(time)).then(|| (*time, true))
                            }))
                            .collect()
                    })
                })
                .or_else(|| other.delete_times.clone()),
        }
    }

    fn merge(base: &Self, diff: &Self) -> Self {
        Self {
            ..Default::default()
        }
    }
}
