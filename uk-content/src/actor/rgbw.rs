use std::collections::HashSet;

use crate::{prelude::Mergeable, Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct Key {
    pub state_key: String,
    pub system_key: String,
}

impl TryFrom<&ParameterObject> for Key {
    type Error = UKError;

    fn try_from(obj: &ParameterObject) -> Result<Self> {
        Ok(Self {
            state_key: obj
                .param("StateKey")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll blend weight state header missing state key",
                ))?
                .as_string()?
                .to_owned(),
            system_key: obj
                .param("SystemKey")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll blend weight state header missing system key",
                ))?
                .as_string()?
                .to_owned(),
        })
    }
}

impl From<Key> for ParameterObject {
    fn from(key: Key) -> Self {
        [
            ("StateKey", Parameter::String32(key.state_key)),
            ("SystemKey", Parameter::String32(key.system_key)),
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RagdollBlendWeight(IndexMap<Key, IndexMap<String, f32>>);

impl TryFrom<&ParameterIO> for RagdollBlendWeight {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self(
            pio.lists
                .0
                .values()
                .map(|list| -> Result<(Key, IndexMap<String, f32>)> {
                    Ok((
                        list.object("Setting")
                            .ok_or(UKError::MissingAampKey(
                                "Ragdoll blend weight state missing header",
                            ))?
                            .try_into()?,
                        list.list("InputWeightList")
                            .ok_or(UKError::MissingAampKey(
                                "Ragdoll blend weight state missing input weight list",
                            ))?
                            .objects
                            .0
                            .values()
                            .map(|obj| -> Result<(String, f32)> {
                                Ok((
                                    obj.param("RigidName")
                                        .ok_or(UKError::MissingAampKey(
                                            "Ragdoll blend weight state input missing rigid name",
                                        ))?
                                        .as_string()?
                                        .to_owned(),
                                    obj.param("BlendRate")
                                        .ok_or(UKError::MissingAampKey(
                                            "Ragdoll blend weight state input missing blend rate",
                                        ))?
                                        .as_f32()?,
                                ))
                            })
                            .collect::<Result<_>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<RagdollBlendWeight> for ParameterIO {
    fn from(val: RagdollBlendWeight) -> Self {
        Self {
            lists: val
                .0
                .into_iter()
                .enumerate()
                .map(|(idx, (key, state))| {
                    (
                        format!("State_{}", idx + 1),
                        ParameterList {
                            objects: [("Setting", key.into())].into_iter().collect(),
                            lists: [(
                                "InputWeightList",
                                ParameterList {
                                    objects: state
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, (name, rate))| {
                                            (
                                                format!("InputWeight_{}", i + 1),
                                                [
                                                    ("RigidName", Parameter::String32(name)),
                                                    ("BlendRate", Parameter::F32(rate)),
                                                ]
                                                .into_iter()
                                                .collect(),
                                            )
                                        })
                                        .collect(),
                                    ..Default::default()
                                },
                            )]
                            .into_iter()
                            .collect(),
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }
    }
}

impl Mergeable<ParameterIO> for RagdollBlendWeight {
    fn diff(&self, other: &Self) -> Self {
        Self(other
        .0
        .iter()
        .filter_map(|(key, other_list)| {
            let self_list = self.0.get(key);
            if let Some(self_list) = self_list && other_list != self_list {
                Some((
                    key.clone(),
                    other_list.iter().filter_map(|(name, other_rate)| {
                        (self_list.get(name) != Some(other_rate)).then(|| (name.clone(), *other_rate))
                    }).collect()
                ))
            } else if self_list == None {
                Some(( key.clone(), other_list.clone() ))
            } else {
                None
            }
        })
        .collect())
    }

    fn merge(&self, diff: &Self) -> Self {
        let all_keys: HashSet<Key> = self.0.keys().chain(diff.0.keys()).cloned().collect();
        Self(
            all_keys
                .into_iter()
                .map(|key| {
                    (
                        key.clone(),
                        if let Some(self_list) = self.0.get(&key) && let Some(other_list) = diff.0.get(&key) {
                            self_list
                                .iter()
                                .chain(other_list.iter())
                                .map(|(n, r)| (n.clone(), *r))
                                .collect()
                        } else {
                            diff.0
                                .get(&key)
                                .or_else(|| self.0.get(&key))
                                .cloned()
                                .unwrap()
                        },
                    )
                })
                .collect(),
        )
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
                .get_file_data("Actor/RagdollBlendWeight/Enemy_Guardian_A.brgbw")
                .unwrap(),
        )
        .unwrap();
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let data = rgbw.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        assert_eq!(rgbw, rgbw2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollBlendWeight/Enemy_Guardian_A.brgbw")
                .unwrap(),
        )
        .unwrap();
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/RagdollBlendWeight/Enemy_Guardian_A.brgbw")
                .unwrap(),
        )
        .unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        let diff = rgbw.diff(&rgbw2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollBlendWeight/Enemy_Guardian_A.brgbw")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/RagdollBlendWeight/Enemy_Guardian_A.brgbw")
                .unwrap(),
        )
        .unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        let diff = rgbw.diff(&rgbw2);
        let merged = rgbw.merge(&diff);
        assert_eq!(rgbw2, merged);
    }
}
