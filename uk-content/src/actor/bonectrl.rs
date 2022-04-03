use crate::{prelude::*, util::DeleteList, Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoneControl {
    pub objects: ParameterObjectMap,
    pub bone_groups: IndexMap<String, DeleteList<String>>,
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
                .map(|plist| -> Result<(String, DeleteList<String>)> {
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
            lists: [(
                "BoneGroups",
                ParameterList {
                    lists: val
                        .bone_groups
                        .into_iter()
                        .enumerate()
                        .map(|(i, (group, bones))| {
                            (
                                format!("Bone_{}", i),
                                ParameterList {
                                    objects: [
                                        (
                                            "Param",
                                            [("GroupName", Parameter::String64(group))]
                                                .into_iter()
                                                .collect(),
                                        ),
                                        (
                                            "Bones",
                                            bones
                                                .into_iter()
                                                .enumerate()
                                                .map(|(i, bone)| {
                                                    (
                                                        format!("Bone_{}", i),
                                                        Parameter::String64(bone),
                                                    )
                                                })
                                                .collect(),
                                        ),
                                    ]
                                    .into_iter()
                                    .collect(),
                                    ..Default::default()
                                },
                            )
                        })
                        .collect(),
                    ..Default::default()
                },
            )]
            .into_iter()
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
                                    .iter()
                                    .filter(|b| !self_bones.contains(*b))
                                    .map(|b| (b.clone(), false))
                                    .chain(self_bones.iter().filter_map(|b| {
                                        (!other_bones.contains(b)).then(|| (b.clone(), true))
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
            bone_groups: base
                .bone_groups
                .keys()
                .chain(diff.bone_groups.keys())
                .collect::<HashSet<&String>>()
                .into_iter()
                .map(|group| {
                    (
                        group.clone(),
                        diff.bone_groups
                            .get(group)
                            .map(|diff_bones| {
                                base.bone_groups
                                    .get(group)
                                    .map(|self_bones| self_bones.merge(diff_bones).and_delete())
                                    .unwrap_or_else(|| diff_bones.clone())
                            })
                            .unwrap_or_else(|| base.bone_groups.get(group).cloned().unwrap()),
                    )
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/BoneControl/Npc_TripMaster_00.bbonectrl")
                .unwrap(),
        )
        .unwrap();
        let bonectrl = super::BoneControl::try_from(&pio).unwrap();
        let data = bonectrl.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let bonectrl2 = super::BoneControl::try_from(&pio2).unwrap();
        assert_eq!(bonectrl, bonectrl2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/BoneControl/Npc_TripMaster_00.bbonectrl")
                .unwrap(),
        )
        .unwrap();
        let bonectrl = super::BoneControl::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/BoneControl/Npc_TripMaster_00.bbonectrl")
                .unwrap(),
        )
        .unwrap();
        let bonectrl2 = super::BoneControl::try_from(&pio2).unwrap();
        let diff = bonectrl.diff(&bonectrl2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/BoneControl/Npc_TripMaster_00.bbonectrl")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let bonectrl = super::BoneControl::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/BoneControl/Npc_TripMaster_00.bbonectrl")
                .unwrap(),
        )
        .unwrap();
        let bonectrl2 = super::BoneControl::try_from(&pio2).unwrap();
        let diff = bonectrl.diff(&bonectrl2);
        let merged = super::BoneControl::merge(&bonectrl, &diff);
        assert_eq!(bonectrl2, merged);
    }
}
