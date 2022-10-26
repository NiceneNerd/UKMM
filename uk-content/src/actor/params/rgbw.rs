use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{DeleteMap, IndexMap},
    Result, UKError,
};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uk_ui_derive::Editable;

#[derive(
    Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Serialize, Deserialize, Editable,
)]
pub struct Key {
    pub state_key: String32,
    pub system_key: String32,
}

impl TryFrom<&ParameterObject> for Key {
    type Error = UKError;

    fn try_from(obj: &ParameterObject) -> Result<Self> {
        Ok(Self {
            state_key: *obj
                .get("StateKey")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll blend weight state header missing state key",
                ))?
                .as_string32()?,
            system_key: *obj
                .get("SystemKey")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll blend weight state header missing system key",
                ))?
                .as_string32()?,
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

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RagdollBlendWeight(IndexMap<Key, DeleteMap<String32, f32>>);

impl TryFrom<&ParameterIO> for RagdollBlendWeight {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self(
            pio.lists()
                .0
                .values()
                .map(|list| -> Result<(Key, DeleteMap<String32, f32>)> {
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
                            .map(|obj| -> Result<(String32, f32)> {
                                Ok((
                                    *obj.get("RigidName")
                                        .ok_or(UKError::MissingAampKey(
                                            "Ragdoll blend weight state input missing rigid name",
                                        ))?
                                        .as_string32()?,
                                    obj.get("BlendRate")
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
        Self::new().with_lists(val.0.into_iter().enumerate().map(|(idx, (key, state))| {
            (
                jstr!("State_{&lexical::to_string(idx + 1)}"),
                ParameterList {
                    objects: [("Setting", key.into())].into_iter().collect(),
                    lists: [(
                        "InputWeightList",
                        ParameterList::new().with_objects(state.into_iter().enumerate().map(
                            |(i, (name, rate))| {
                                (
                                    jstr!("InputWeight_{&lexical::to_string(i + 1)}"),
                                    [
                                        ("RigidName", Parameter::String32(name)),
                                        ("BlendRate", Parameter::F32(rate)),
                                    ]
                                    .into_iter()
                                    .collect(),
                                )
                            },
                        )),
                    )]
                    .into_iter()
                    .collect(),
                },
            )
        }))
    }
}

impl Mergeable for RagdollBlendWeight {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(key, other_list)| {
                    let self_list = self.0.get(key);
                    if let Some(self_list) = self_list && other_list != self_list {
                Some((
                    key.clone(),
                    self_list.diff(other_list)
                ))
            } else if self_list.is_none() {
                Some(( key.clone(), other_list.clone() ))
            } else {
                None
            }
                })
                .collect(),
        )
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
                            self_list.merge(other_list)
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

impl ParameterResource for RagdollBlendWeight {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/RagdollBlendWeight/{name}.brgbw")
    }
}

impl Resource for RagdollBlendWeight {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("brgbw")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(rgbw.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        assert_eq!(rgbw, rgbw2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        let diff = rgbw.diff(&rgbw2);
        dbg!("{}", diff);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        let diff = rgbw.diff(&rgbw2);
        let merged = rgbw.merge(&diff);
        assert_eq!(rgbw2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Moriblin_Junior.sbactorpack//Actor/RagdollBlendWeight/Moriblin.brgbw",
        );
        assert!(super::RagdollBlendWeight::path_matches(path));
    }
}
