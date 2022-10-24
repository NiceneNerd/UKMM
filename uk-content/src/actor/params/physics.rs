use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util, Result, UKError,
};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uk_ui_derive::Editable;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Editable)]
pub struct ContactInfoItem {
    pub name: String32,
    pub info_type: String32,
    pub num: Option<i32>,
}

impl TryFrom<&ParameterObject> for ContactInfoItem {
    type Error = UKError;
    fn try_from(obj: &ParameterObject) -> Result<Self> {
        Ok(Self {
            name: *obj
                .get("name")
                .ok_or(UKError::MissingAampKey("Contact info item missing name"))?
                .as_string32()?,
            info_type: *obj
                .get("type")
                .ok_or(UKError::MissingAampKey("Contact info item missing type"))?
                .as_string32()?,
            num: obj.get("num").map(|p| p.as_int()).transpose()?,
        })
    }
}

impl From<ContactInfoItem> for ParameterObject {
    fn from(val: ContactInfoItem) -> Self {
        [
            ("name", Parameter::String32(val.name)),
            ("type", Parameter::String32(val.info_type)),
        ]
        .into_iter()
        .chain(
            [val.num]
                .into_iter()
                .filter_map(|num| num.map(|v| ("num", Parameter::Int(v)))),
        )
        .collect()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Editable)]
pub struct ContactInfo {
    pub contact_point_info: Option<Vec<ContactInfoItem>>,
    pub collision_info: Option<Vec<ContactInfoItem>>,
}

impl TryFrom<&ParameterList> for ContactInfo {
    type Error = UKError;
    fn try_from(list: &ParameterList) -> Result<Self> {
        let point_count = list
            .objects
            .0
            .get(&3387849585)
            .ok_or(UKError::MissingAampKey(
                "Physics rigid contact info missing header",
            ))?
            .get("contact_point_info_num")
            .ok_or(UKError::MissingAampKey(
                "Physics rigid contact info header missing contact point info count",
            ))?
            .as_int()? as usize;
        let collision_count = list.objects.0[&3387849585]
            .get("collision_info_num")
            .ok_or(UKError::MissingAampKey(
                "Physics rigid contact info header missing collision info count",
            ))?
            .as_int()? as usize;
        Ok(Self {
            contact_point_info: Some(
                (0..point_count)
                    .map(|i| -> Result<ContactInfoItem> {
                        list.object(&jstr!("ContactPointInfo_{&lexical::to_string(i)}"))
                            .ok_or(UKError::MissingAampKey(
                                "Physics rigid contact info missing entry",
                            ))?
                            .try_into()
                    })
                    .collect::<Result<_>>()?,
            ),
            collision_info: Some(
                (0..collision_count)
                    .map(|i| -> Result<ContactInfoItem> {
                        list.object(&jstr!("CollisionInfo_{&lexical::to_string(i)}"))
                            .ok_or(UKError::MissingAampKey(
                                "Physics rigid collision info missing entry",
                            ))?
                            .try_into()
                    })
                    .collect::<Result<_>>()?,
            ),
        })
    }
}

impl ParameterResource for Physics {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/Physics/{name}.bphysics")
    }
}

impl Resource for Physics {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bphysics")
    }
}

impl From<ContactInfo> for ParameterList {
    fn from(val: ContactInfo) -> Self {
        if let Some(contact_point_info) = val.contact_point_info && let Some(collision_info) = val.collision_info {
            Self {
                objects: [(
                    3387849585,
                    [
                        (
                            "contact_point_info_num",
                            Parameter::Int(contact_point_info.len() as i32),
                        ),
                        (
                            "collision_info_num",
                            Parameter::Int(collision_info.len() as i32),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .chain(
                    contact_point_info.into_iter().enumerate().map(|(i, info)| {
                        (hash_name(&jstr!("ContactPointInfo_{&lexical::to_string(i)}")), info.into())
                    }),
                )
                .chain(
                    collision_info
                        .into_iter()
                        .enumerate()
                        .map(|(i, info)| (hash_name(&jstr!("CollisionInfo_{&lexical::to_string(i)}")), info.into())),
                )
                .collect(),
                ..Default::default()
            }
        } else {
            Self::default()
        }
    }
}

impl Mergeable for ContactInfo {
    fn diff(&self, other: &Self) -> Self {
        Self {
            contact_point_info: if other.contact_point_info != self.contact_point_info {
                other.contact_point_info.clone()
            } else {
                None
            },
            collision_info: if other.collision_info != self.collision_info {
                other.collision_info.clone()
            } else {
                None
            },
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            contact_point_info: diff
                .contact_point_info
                .as_ref()
                .or(self.contact_point_info.as_ref())
                .cloned(),
            collision_info: diff
                .collision_info
                .as_ref()
                .or(self.collision_info.as_ref())
                .cloned(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Editable)]
pub struct CharacterController {
    pub header: ParameterObject,
    pub forms: BTreeMap<usize, ParameterList>,
}

impl TryFrom<&ParameterList> for CharacterController {
    type Error = UKError;
    fn try_from(list: &ParameterList) -> Result<Self> {
        Ok(Self {
            header: list
                .objects
                .get(2311816730)
                .ok_or(UKError::MissingAampKey(
                    "Physics character controller missing header",
                ))?
                .clone(),
            forms: list.lists.0.values().cloned().enumerate().collect(),
        })
    }
}

impl From<CharacterController> for ParameterList {
    fn from(val: CharacterController) -> Self {
        Self {
            objects: [(2311816730, val.header)].into_iter().collect(),
            lists: val
                .forms
                .into_iter()
                .map(|(i, form)| (jstr!("Form_{&lexical::to_string(i)}"), form))
                .collect(),
        }
    }
}

impl Mergeable for CharacterController {
    fn diff(&self, other: &Self) -> Self {
        Self {
            header: util::diff_pobj(&self.header, &other.header),
            forms: util::simple_index_diff(&self.forms, &other.forms),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            header: util::merge_pobj(&self.header, &diff.header),
            forms: util::simple_index_merge(&self.forms, &diff.forms),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Editable)]
pub struct Cloth {
    pub setup_file_path: Option<String>,
    pub subwind: ParameterObject,
    pub cloths: BTreeMap<usize, ParameterObject>,
}

impl TryFrom<&ParameterList> for Cloth {
    type Error = UKError;
    fn try_from(list: &ParameterList) -> Result<Self> {
        let header = list
            .object("ClothHeader")
            .ok_or(UKError::MissingAampKey("Physics missing cloth header"))?;
        Ok(Self {
            setup_file_path: Some(
                header
                    .get("cloth_setup_file_path")
                    .ok_or(UKError::MissingAampKey(
                        "Physics cloth header missing setup file path",
                    ))?
                    .as_str()?
                    .into(),
            ),
            subwind: list
                .object("ClothSubWind")
                .ok_or(UKError::MissingAampKey("Physics cloth missing subwind"))?
                .clone(),
            cloths: header
                .get("cloth_num")
                .ok_or(UKError::MissingAampKey(
                    "Physics cloth header missing cloth count",
                ))?
                .as_int()
                .map(|count| -> Result<BTreeMap<usize, ParameterObject>> {
                    (0..count)
                        .map(|i| -> Result<(usize, ParameterObject)> {
                            Ok((
                                i as usize,
                                list.object(&jstr!("Cloth_{&lexical::to_string(i)}"))
                                    .ok_or_else(|| {
                                        UKError::MissingAampKeyD(jstr!(
                                            "Physics cloth missing Cloth_{&lexical::to_string(i)}"
                                        ))
                                    })?
                                    .clone(),
                            ))
                        })
                        .collect::<Result<_>>()
                })??,
        })
    }
}

impl From<Cloth> for ParameterList {
    fn from(val: Cloth) -> Self {
        Self {
            objects: [
                (
                    "ClothHeader".into(),
                    [
                        (
                            "cloth_setup_file_path",
                            Parameter::String256(val.setup_file_path.unwrap_or_default().into()),
                        ),
                        ("cloth_num", Parameter::Int(val.cloths.len() as i32)),
                    ]
                    .into_iter()
                    .collect(),
                ),
                ("ClothSubWind".into(), val.subwind),
            ]
            .into_iter()
            .chain(
                val.cloths
                    .into_iter()
                    .map(|(i, cloth)| (jstr!("Cloth_{&lexical::to_string(i)}"), cloth)),
            )
            .collect(),
            ..Default::default()
        }
    }
}

impl Mergeable for Cloth {
    fn diff(&self, other: &Self) -> Self {
        Self {
            setup_file_path: if other.setup_file_path != self.setup_file_path {
                other.setup_file_path.clone()
            } else {
                None
            },
            subwind: util::diff_pobj(&self.subwind, &other.subwind),
            cloths: util::simple_index_diff(&self.cloths, &other.cloths),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            setup_file_path: if diff.setup_file_path != self.setup_file_path {
                diff.setup_file_path.clone()
            } else {
                None
            },
            subwind: util::merge_pobj(&self.subwind, &diff.subwind),
            cloths: util::simple_index_merge(&self.cloths, &diff.cloths),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Editable)]
pub struct Physics {
    pub ragdoll: Option<String>,
    pub support_bone: Option<String>,
    pub rigid_contact_info: Option<ContactInfo>,
    pub rigid_body_set: Option<BTreeMap<usize, ParameterList>>,
    pub character_controller: Option<CharacterController>,
    pub cloth: Option<Cloth>,
    pub use_system_group_handler: Option<bool>,
}

impl TryFrom<&ParameterIO> for Physics {
    type Error = UKError;
    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let param_set = pio
            .list("ParamSet")
            .ok_or(UKError::MissingAampKey("Physics missing param set"))?;
        let header = param_set
            .objects
            .get(1258832850)
            .ok_or(UKError::MissingAampKey("Physics missing header"))?;
        Ok(Self {
            ragdoll: header
                .get("use_ragdoll")
                .ok_or(UKError::MissingAampKey(
                    "Physics header missing use_ragdoll",
                ))?
                .as_bool()?
                .then(|| -> Result<String> {
                    Ok(param_set
                        .object("Ragdoll")
                        .ok_or(UKError::MissingAampKey("Physics missing ragdoll header"))?
                        .get("ragdoll_setup_file_path")
                        .ok_or(UKError::MissingAampKey(
                            "Physics ragdoll header missing setup file path",
                        ))?
                        .as_str()?
                        .into())
                })
                .transpose()?,
            support_bone: header
                .get("use_support_bone")
                .ok_or(UKError::MissingAampKey(
                    "Physics header missing use_support_bone",
                ))?
                .as_bool()?
                .then(|| -> Result<String> {
                    Ok(param_set
                        .object("SupportBone")
                        .ok_or(UKError::MissingAampKey(
                            "Physics missing support bone header",
                        ))?
                        .get("support_bone_setup_file_path")
                        .ok_or(UKError::MissingAampKey(
                            "Physics support bone header missing setup file path",
                        ))?
                        .as_str()?
                        .into())
                })
                .transpose()?,
            rigid_contact_info: header
                .get("use_contact_info")
                .ok_or(UKError::MissingAampKey(
                    "Physics header missing use_contact_info",
                ))?
                .as_bool()?
                .then(|| -> Result<ContactInfo> {
                    param_set
                        .list("RigidContactInfo")
                        .ok_or(UKError::MissingAampKey(
                            "Physics missing rigid contact info",
                        ))?
                        .try_into()
                })
                .transpose()?,
            rigid_body_set: header
                .get("use_rigid_body_set_num")
                .ok_or(UKError::MissingAampKey(
                    "Physics header missing use_rigid_body_set_num",
                ))?
                .as_int()
                .map(|count| {
                    (count > 0).then(|| -> Result<BTreeMap<usize, ParameterList>> {
                        let rigid_body_set =
                            param_set
                                .list("RigidBodySet")
                                .ok_or(UKError::MissingAampKey(
                                    "Physics missing rigid body set list",
                                ))?;
                        (0..count)
                            .map(|i| -> Result<(usize, ParameterList)> {
                                Ok((
                                    i as usize,
                                    rigid_body_set
                                        .list(&jstr!("RigidBodySet_{&lexical::to_string(i)}"))
                                        .ok_or(UKError::MissingAampKey(
                                            "Physics missing rigid body set entry",
                                        ))?
                                        .clone(),
                                ))
                            })
                            .collect::<Result<_>>()
                    })
                })?
                .transpose()?,
            character_controller: header
                .get("use_character_controller")
                .ok_or(UKError::MissingAampKey(
                    "Physics header missing use_character_controller",
                ))?
                .as_bool()?
                .then(|| {
                    param_set
                        .list("CharacterController")
                        .ok_or(UKError::MissingAampKey(
                            "Physics missing character controller",
                        ))?
                        .try_into()
                })
                .transpose()?,
            cloth: header
                .get("use_cloth")
                .ok_or(UKError::MissingAampKey("Physics header missing use_cloth"))?
                .as_bool()?
                .then(|| -> Result<Cloth> {
                    param_set
                        .list("Cloth")
                        .ok_or(UKError::MissingAampKey("Physics missing cloth section"))?
                        .try_into()
                })
                .transpose()?,
            use_system_group_handler: Some(
                header
                    .get("use_system_group_handler")
                    .ok_or(UKError::MissingAampKey(
                        "Physics missing use_system_group_handler",
                    ))?
                    .as_bool()?,
            ),
        })
    }
}

impl From<Physics> for ParameterIO {
    fn from(val: Physics) -> Self {
        Self::new().with_lists(
            [(
                "ParamSet",
                ParameterList {
                    objects: [(
                        1258832850,
                        [
                            (
                                "use_rigid_body_set_num",
                                Parameter::Int(
                                    val.rigid_body_set
                                        .as_ref()
                                        .map(|s| s.len() as i32)
                                        .unwrap_or_default(),
                                ),
                            ),
                            ("use_ragdoll", Parameter::Bool(val.ragdoll.is_some())),
                            ("use_cloth", Parameter::Bool(val.cloth.is_some())),
                            (
                                "use_support_bone",
                                Parameter::Bool(val.support_bone.is_some()),
                            ),
                            (
                                "use_character_controller",
                                Parameter::Bool(val.character_controller.is_some()),
                            ),
                            (
                                "use_contact_info",
                                Parameter::Bool(val.rigid_contact_info.is_some()),
                            ),
                            ("use_edge_rigid_body_num", Parameter::Int(0)),
                            (
                                "use_system_group_handler",
                                Parameter::Bool(val.use_system_group_handler.unwrap_or_default()),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    )]
                    .into_iter()
                    .chain(
                        [
                            (hash_name("Ragdoll"), "ragdoll_setup_file_path", val.ragdoll),
                            (
                                hash_name("SupportBone"),
                                "support_bone_setup_file_path",
                                val.support_bone,
                            ),
                        ]
                        .into_iter()
                        .filter_map(|(k, p, v)| {
                            v.map(|s| {
                                (
                                    k,
                                    [(p, Parameter::String256(s.into()))].into_iter().collect(),
                                )
                            })
                        }),
                    )
                    .collect(),
                    lists: [
                        (
                            "RigidContactInfo",
                            val.rigid_contact_info.map(|info| info.into()),
                        ),
                        (
                            "RigidBodySet",
                            val.rigid_body_set.map(|rigid_body_set| ParameterList {
                                lists: rigid_body_set
                                    .into_iter()
                                    .map(|(i, list)| {
                                        (jstr!("RigidBodySet_{&lexical::to_string(i)}"), list)
                                    })
                                    .collect(),
                                ..Default::default()
                            }),
                        ),
                        (
                            "CharacterController",
                            val.character_controller.map(|controller| controller.into()),
                        ),
                        ("Cloth", val.cloth.map(|cloth| cloth.into())),
                    ]
                    .into_iter()
                    .filter_map(|(k, list)| list.map(|list| (k, list)))
                    .collect(),
                },
            )]
            .into_iter(),
        )
    }
}

impl Mergeable for Physics {
    fn diff(&self, other: &Self) -> Self {
        Self {
            ragdoll: if other.ragdoll != self.ragdoll {
                other.ragdoll.clone()
            } else {
                None
            },
            support_bone: if other.support_bone != self.support_bone {
                other.support_bone.clone()
            } else {
                None
            },
            rigid_contact_info: if let Some(self_info) = &self.rigid_contact_info && let Some(other_info) =
                &other.rigid_contact_info && self_info != other_info
            {
                Some(self_info.diff(other_info))
            } else if self.rigid_contact_info == other.rigid_contact_info {
                None
            } else {
                other.rigid_contact_info.clone()
            },
            rigid_body_set: if let Some(self_body) = &self.rigid_body_set && let Some(other_body) =
                &other.rigid_body_set && self_body != other_body
            {
                Some(util::simple_index_diff(self_body, other_body))
            } else if self.rigid_body_set == other.rigid_body_set {
                None
            } else {
                other.rigid_body_set.clone()
            },
            character_controller: if let Some(self_controller) = &self.character_controller
                && let Some(other_controller) =
                &other.character_controller && self_controller != other_controller
            {
                Some(self_controller.diff(other_controller))
            } else if self.character_controller == other.character_controller {
                None
            } else {
                other.character_controller.clone()
            },
            cloth: if let Some(self_cloth) =
                &self.cloth && let Some(other_cloth) = &other.cloth && self_cloth != other_cloth
            {
                Some(self_cloth.diff(other_cloth))
            } else if self.cloth == other.cloth {
                None
            } else {
                other.cloth.clone()
            },
            use_system_group_handler: if other.use_system_group_handler != self.use_system_group_handler
            {
                other.use_system_group_handler
            } else {
                None
            },
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            ragdoll: diff.ragdoll.clone().or_else(|| self.ragdoll.clone()),
            support_bone: diff
                .support_bone
                .clone()
                .or_else(|| self.support_bone.clone()),
            rigid_contact_info: diff
                .rigid_contact_info
                .as_ref()
                .map(|diff_info| {
                    self.rigid_contact_info
                        .as_ref()
                        .map(|base_info| base_info.merge(diff_info))
                        .unwrap_or_else(|| diff_info.clone())
                })
                .or_else(|| self.rigid_contact_info.clone()),
            rigid_body_set: diff
                .rigid_body_set
                .as_ref()
                .map(|diff_body| {
                    self.rigid_body_set
                        .as_ref()
                        .map(|base_body| util::simple_index_merge(base_body, diff_body))
                        .unwrap_or_else(|| diff_body.clone())
                })
                .or_else(|| self.rigid_body_set.clone()),
            character_controller: diff
                .character_controller
                .as_ref()
                .map(|diff_controller| {
                    self.character_controller
                        .as_ref()
                        .map(|base_controller| base_controller.merge(diff_controller))
                        .unwrap_or_else(|| diff_controller.clone())
                })
                .or_else(|| self.character_controller.clone()),
            cloth: diff
                .cloth
                .as_ref()
                .map(|diff_cloth| {
                    self.cloth
                        .as_ref()
                        .map(|base_cloth| base_cloth.merge(diff_cloth))
                        .unwrap_or_else(|| diff_cloth.clone())
                })
                .or_else(|| self.cloth.clone()),
            use_system_group_handler: diff
                .use_system_group_handler
                .or(self.use_system_group_handler),
        }
    }
}

impl InfoSource for Physics {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        if let Some(Parameter::Vec3(center)) = self.rigid_body_set.as_ref().and_then(|body_set| {
            body_set.values().next().and_then(|list| {
                list.lists.0.values().next().and_then(|list| {
                    list.objects()
                        .get(948250248)
                        .and_then(|obj| obj.get("center_of_mass"))
                })
            })
        }) {
            info.insert("rigidBodyCenterY".into(), center.y.into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Physics/Npc_TripMaster_00.bphysics")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let physics = super::Physics::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(physics.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let physics2 = super::Physics::try_from(&pio2).unwrap();
        assert_eq!(physics, physics2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Physics/Npc_TripMaster_00.bphysics")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let physics = super::Physics::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/Physics/Npc_TripMaster_00.bphysics")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let physics2 = super::Physics::try_from(&pio2).unwrap();
        let _diff = physics.diff(&physics2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Physics/Npc_TripMaster_00.bphysics")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let physics = super::Physics::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/Physics/Npc_TripMaster_00.bphysics")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let physics2 = super::Physics::try_from(&pio2).unwrap();
        let diff = physics.diff(&physics2);
        let merged = physics.merge(&diff);
        assert_eq!(physics2, merged);
    }

    #[test]
    fn info() {
        let actor = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Physics/Npc_TripMaster_00.bphysics")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let physics = super::Physics::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::default();
        physics.update_info(&mut info).unwrap();
        assert_eq!(info["rigidBodyCenterY"], roead::byml::Byml::Float(0.15));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/Physics/Npc_TripMaster_00.bphysics",
        );
        assert!(super::Physics::path_matches(path));
    }
}
