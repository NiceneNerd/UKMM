use crate::{actor::ParameterResource, prelude::*, util::DeleteSet, Result, UKError};
use indexmap::IndexMap;
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoneControl {
    pub objects: ParameterObjectMap,
    pub bone_groups: IndexMap<String, DeleteSet<String>>,
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
                .ok_or(UKError::MissingAampKey("Bone control missing BoneGroups"))?
                .lists
                .0
                .values()
                .map(|list| -> Result<(String, DeleteSet<String>)> {
                    Ok((
                        list.object("Param")
                            .ok_or(UKError::MissingAampKey("Bone control group missing param"))?
                            .param("GroupName")
                            .ok_or(UKError::MissingAampKey(
                                "Bone control group missing group name",
                            ))?
                            .as_string()?
                            .to_owned(),
                        list.object("Bones")
                            .ok_or(UKError::MissingAampKey(
                                "Bone control group missing bone list",
                            ))?
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
                                jstr!("Bone_{&lexical::to_string(i)}"),
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
                                                        jstr!("Bone_{&lexical::to_string(i)}"),
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

impl Mergeable for BoneControl {
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
                            Some((group.clone(), self_bones.diff(other_bones)))
                        }
                    } else {
                        Some((group.clone(), other_bones.clone()))
                    }
                })
                .collect(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            objects: crate::util::merge_plist(
                &ParameterList {
                    objects: self.objects.clone(),
                    ..Default::default()
                },
                &ParameterList {
                    objects: diff.objects.clone(),
                    ..Default::default()
                },
            )
            .objects,
            bone_groups: self
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
                                self.bone_groups
                                    .get(group)
                                    .map(|self_bones| self_bones.merge(diff_bones).and_delete())
                                    .unwrap_or_else(|| diff_bones.clone())
                            })
                            .unwrap_or_else(|| self.bone_groups.get(group).cloned().unwrap()),
                    )
                })
                .collect(),
        }
    }
}

impl ParameterResource for BoneControl {
    fn path(name: &str) -> String {
        jstr!("Actor/BoneControl/{name}.bbonectrl")
    }
}

impl Resource for BoneControl {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bbonectrl")
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
        let data = roead::aamp::ParameterIO::from(bonectrl.clone()).to_binary();
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
        let merged = bonectrl.merge(&diff);
        assert_eq!(bonectrl2, merged);
    }
}
