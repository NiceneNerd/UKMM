use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;
use uk_util::OptionResultExt;

use crate::{
    actor::{InfoSource, ParameterResource},
    constants::{Time, Weather},
    prelude::*,
    util::{params, DeleteSet, IteratorExt},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
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
                        .0
                        .values()
                        .map(|w| -> Result<(Weather, bool)> {
                            Ok((w.as_str()?.try_into()?, false))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            invalid_times: pio
                .object("InvalidTimes")
                .map(|times| -> Result<DeleteSet<Time>> {
                    times
                        .0
                        .values()
                        .map(|w| -> Result<(Time, bool)> { Ok((w.as_str()?.try_into()?, false)) })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            display_dist: Some(
                pio.object("DisplayDistance")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition missing display distance",
                        None,
                    ))?
                    .get("Item")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition display distance missing item",
                        None,
                    ))?
                    .as_f32()?,
            ),
            auto_disp_dist_algo: Some(
                pio.object("AutoDisplayDistanceAlgorithm")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition missing display distance",
                        None,
                    ))?
                    .get("Item")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition display distance missing item",
                        None,
                    ))?
                    .as_str()?
                    .into(),
            ),
            y_limit_algo: Some(
                pio.object("YLimitAlgorithm")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition missing display distance",
                        None,
                    ))?
                    .get("Item")
                    .ok_or(UKError::MissingAampKey(
                        "Life condition display distance missing item",
                        None,
                    ))?
                    .as_str()?
                    .into(),
            ),
            delete_weathers: pio
                .object("DeleteWeathers")
                .map(|weathers| -> Result<DeleteSet<Weather>> {
                    weathers
                        .0
                        .values()
                        .map(|w| -> Result<(Weather, bool)> {
                            Ok((w.as_str()?.try_into()?, false))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            delete_times: pio
                .object("DeleteTimes")
                .map(|times| -> Result<DeleteSet<Time>> {
                    times
                        .0
                        .values()
                        .map(|w| -> Result<(Time, bool)> { Ok((w.as_str()?.try_into()?, false)) })
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
            pio.objects_mut().insert(
                "InvalidWeathers",
                weathers
                    .into_iter()
                    .named_enumerate("Item")
                    .with_padding::<3>()
                    .with_zero_index(false)
                    .map(|(index, weather)| (index, Parameter::String64(Box::new(weather.into()))))
                    .collect(),
            );
        }
        if let Some(times) = val.invalid_times {
            pio.objects_mut().insert(
                "InvalidTimes",
                times
                    .into_iter()
                    .named_enumerate("Item")
                    .with_padding::<3>()
                    .with_zero_index(false)
                    .map(|(index, time)| (index, Parameter::String64(Box::new(time.into()))))
                    .collect(),
            );
        }
        if let Some(display_dist) = val.display_dist {
            pio.objects_mut().insert(
                "DisplayDistance",
                params!("Item" => Parameter::F32(display_dist)),
            )
        }
        if let Some(auto_display_dist_algo) = val.auto_disp_dist_algo {
            pio.objects_mut().insert(
                "AutoDisplayDistanceAlgorithm",
                params!("Item" => Parameter::StringRef(auto_display_dist_algo)),
            );
        }
        if let Some(y_limit_algo) = val.y_limit_algo {
            pio.objects_mut().insert(
                "YLimitAlgorithm",
                params!("Item" => Parameter::StringRef(y_limit_algo)),
            );
        }
        if let Some(weathers) = val.delete_weathers {
            pio.objects_mut().insert(
                "DeleteWeathers",
                weathers
                    .into_iter()
                    .named_enumerate("Item")
                    .with_padding::<3>()
                    .with_zero_index(false)
                    .map(|(index, weather)| (index, Parameter::String64(Box::new(weather.into()))))
                    .collect(),
            );
        }
        if let Some(times) = val.delete_times {
            pio.objects_mut().insert(
                "DeleteTimes",
                times
                    .into_iter()
                    .named_enumerate("Item")
                    .with_padding::<3>()
                    .with_zero_index(false)
                    .map(|(index, time)| (index, Parameter::String64(Box::new(time.into()))))
                    .collect(),
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
                            .filter_map(|weather| {
                                (!self_weathers.contains(weather)).then_some((*weather, false))
                            })
                            .chain(self_weathers.iter().filter_map(|weather| {
                                (!other_weathers.contains(weather)).then_some((*weather, true))
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
                            .filter_map(|time| {
                                (!self_times.contains(time)).then_some((*time, false))
                            })
                            .chain(self_times.iter().filter_map(|time| {
                                (!other_times.contains(time)).then_some((*time, true))
                            }))
                            .collect()
                    })
                })
                .or_else(|| other.invalid_times.clone()),
            display_dist: (self.display_dist != other.display_dist)
                .then_some(other.display_dist)
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
                                (!self_weathers.contains(weather)).then_some((*weather, false))
                            })
                            .chain(self_weathers.iter().filter_map(|weather| {
                                (!other_weathers.contains(weather)).then_some((*weather, true))
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
                            .filter_map(|time| {
                                (!self_times.contains(time)).then_some((*time, false))
                            })
                            .chain(self_times.iter().filter_map(|time| {
                                (!other_times.contains(time)).then_some((*time, true))
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

impl InfoSource for LifeCondition {
    fn update_info(&self, info: &mut roead::byml::Map) -> crate::Result<()> {
        use roead::byml::Byml;
        if let Some(display_dist) = self.display_dist {
            info.insert("traverseDist".into(), display_dist.into());
        }
        if let Some(limit) = &self.y_limit_algo {
            info.insert("yLimitAlgo".into(), limit.into());
        }
        if let Some(invalid_times) = self.invalid_times.as_ref().filter(|t| !t.is_empty()) {
            info.insert(
                "invalidTimes".into(),
                invalid_times
                    .iter()
                    .map(|t| Byml::String(t.into()))
                    .collect(),
            );
        }
        if let Some(invalid_weathers) = self.invalid_weathers.as_ref().filter(|t| !t.is_empty()) {
            info.insert(
                "invalidWeathers".into(),
                invalid_weathers
                    .iter()
                    .map(|w| Byml::String(w.into()))
                    .collect(),
            );
        }
        Ok(())
    }
}

impl ParameterResource for LifeCondition {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/LifeCondition/{name}.blifecondition")
    }
}

impl Resource for LifeCondition {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"blifecondition")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(lifecondition.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let lifecondition2 = super::LifeCondition::try_from(&pio2).unwrap();
        assert_eq!(lifecondition, lifecondition2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition2 = super::LifeCondition::try_from(&pio2).unwrap();
        let _diff = lifecondition.diff(&lifecondition2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition2 = super::LifeCondition::try_from(&pio2).unwrap();
        let diff = lifecondition.diff(&lifecondition2);
        let merged = lifecondition.merge(&diff);
        assert_eq!(lifecondition2, merged);
    }

    #[test]
    fn info() {
        use roead::byml::Byml;
        let actor = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/LifeCondition/Enemy_Guardian_A.blifecondition")
                .unwrap(),
        )
        .unwrap();
        let lifecondition = super::LifeCondition::try_from(&pio).unwrap();
        let mut info = roead::byml::Map::default();
        lifecondition.update_info(&mut info).unwrap();
        assert!(
            info["invalidTimes"]
                .as_array()
                .unwrap()
                .contains(&Byml::String("Morning_B".into()))
        );
        assert!(
            info["invalidWeathers"]
                .as_array()
                .unwrap()
                .contains(&Byml::String("ThunderRain".into()))
        );
        assert_eq!(info["traverseDist"], Byml::Float(0.0));
        assert_eq!(info["yLimitAlgo"], Byml::String("NoLimit".into()));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/LifeCondition/\
             Enemy_Guardian_A.blifecondition",
        );
        assert!(super::LifeCondition::path_matches(path));
    }
}
