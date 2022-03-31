use crate::{prelude::*, Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoneControl {
    pub objects: ParameterObjectMap,
    pub bone_groups: IndexMap<String, IndexMap<String, bool>>,
}

impl TryFrom<ParameterIO> for BoneControl {
    type Error = UKError;

    fn try_from(value: ParameterIO) -> Result<Self> {
        value.try_into()
    }
}

impl TryFrom<&ParameterIO> for BoneControl {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            objects: pio.objects.clone(),
            bone_groups: pio
                .list("BoneGroups")
                .ok_or_else(|| {
                    UKError::MissingAampKey("Bone control missing BoneGroups".to_owned())
                })?
                .lists
                .0
                .values()
                .map(|plist| -> Result<(String, IndexMap<String, bool>)> {
                    Ok((
                        plist
                            .object("Param")
                            .ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "Bone control group missing param".to_owned(),
                                )
                            })?
                            .param("GroupName")
                            .ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "Bone control group missing group name".to_owned(),
                                )
                            })?
                            .as_string()?
                            .to_owned(),
                        plist
                            .object("Bones")
                            .ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "Bone control group missing bone list".to_owned(),
                                )
                            })?
                            .params()
                            .values()
                            .filter_map(|v| v.as_string().ok().map(|s| (s.to_owned(), false)))
                            .collect(),
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<BoneControl> for ParameterIO {
    fn from(val: BoneControl) -> Self {
        Self {
            objects: val.objects,
            lists: val
                .bone_groups
                .into_iter()
                .enumerate()
                .map(|(i, (group, bones))| {
                    (
                        hash_name(&format!("Bone_{}", i)),
                        ParameterList {
                            objects: ParameterObjectMap(
                                [
                                    (
                                        hash_name("Param"),
                                        ParameterObject(
                                            [(hash_name("GroupName"), Parameter::String64(group))]
                                                .into_iter()
                                                .collect(),
                                        ),
                                    ),
                                    (
                                        hash_name("Bones"),
                                        ParameterObject(
                                            bones
                                                .into_iter()
                                                .filter(|(_, del)| !del)
                                                .enumerate()
                                                .map(|(i, (bone, _))| {
                                                    (
                                                        hash_name(&format!("Bone_{}", i)),
                                                        Parameter::String64(bone),
                                                    )
                                                })
                                                .collect(),
                                        ),
                                    ),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }
    }
}

impl Convertible<ParameterIO> for BoneControl {}

impl Mergeable<ParameterIO> for BoneControl {
    fn diff(&self, other: &Self) -> Self {
        Self {
            objects: crate::util::diff_plist(
                &ParameterList {
                    objects: self.objects.clone(),
                    ..Default::default()
                },
                &ParameterList {
                    objects: other.objects.clone(),
                    ..Default::default()
                },
            )
            .objects,
            bone_groups: other
                .bone_groups
                .iter()
                .filter_map(|(group, other_bones)| {
                    if let Some(self_bones) = self.bone_groups.get(group.as_str()) {
                        if self_bones == other_bones {
                            None
                        } else {
                            Some((
                                group.clone(),
                                other_bones
                                    .keys()
                                    .filter(|b| !self_bones.contains_key(b.as_str()))
                                    .map(|b| (b.clone(), false))
                                    .chain(self_bones.keys().filter_map(|b| {
                                        (!other_bones.contains_key(b.as_str()))
                                            .then(|| (b.clone(), true))
                                    }))
                                    .collect(),
                            ))
                        }
                    } else {
                        Some((group.clone(), other_bones.clone()))
                    }
                })
                .collect(),
        }
    }

    fn merge(base: &Self, diff: &Self) -> Self {
        Self {
            objects: crate::util::merge_plist(
                &ParameterList {
                    objects: base.objects.clone(),
                    ..Default::default()
                },
                &ParameterList {
                    objects: diff.objects.clone(),
                    ..Default::default()
                },
            )
            .objects,
            ..Default::default()
        }
    }
}
