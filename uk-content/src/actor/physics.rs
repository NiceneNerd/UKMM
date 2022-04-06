use crate::{
    prelude::{Convertible, Mergeable},
    util::{self, DeleteVec},
    Result, UKError,
};
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactInfoItem {
    pub name: String,
    pub info_type: String,
    pub num: Option<i32>,
}

impl TryFrom<&ParameterObject> for ContactInfoItem {
    type Error = UKError;
    fn try_from(obj: &ParameterObject) -> Result<Self> {
        Ok(Self {
            name: obj
                .param("name")
                .ok_or(UKError::MissingAampKey("Contact info item missing name"))?
                .as_string()?
                .to_owned(),
            info_type: obj
                .param("type")
                .ok_or(UKError::MissingAampKey("Contact info item missing type"))?
                .as_string()?
                .to_owned(),
            num: obj.param("num").map(|p| p.as_int()).transpose()?,
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

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactInfo {
    pub contact_point_info: DeleteVec<ContactInfoItem>,
    pub collision_info: DeleteVec<ContactInfoItem>,
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
            .param("contact_point_info_num")
            .ok_or(UKError::MissingAampKey(
                "Physics rigid contact info header missing contact point info count",
            ))?
            .as_int()? as usize;
        let collision_count = list.objects.0[&3387849585]
            .param("collision_info_num")
            .ok_or(UKError::MissingAampKey(
                "Physics rigid contact info header missing collision info count",
            ))?
            .as_int()? as usize;
        Ok(Self {
            contact_point_info: (0..point_count)
                .map(|i| -> Result<ContactInfoItem> {
                    list.object(&format!("ContactPointInfo_{}", i))
                        .ok_or(UKError::MissingAampKey(
                            "Physics rigid contact info missing entry",
                        ))?
                        .try_into()
                })
                .collect::<Result<_>>()?,
            collision_info: (0..collision_count)
                .map(|i| -> Result<ContactInfoItem> {
                    list.object(&format!("CollisionInfo_{}", i))
                        .ok_or(UKError::MissingAampKey(
                            "Physics rigid collision info missing entry",
                        ))?
                        .try_into()
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<ContactInfo> for ParameterList {
    fn from(val: ContactInfo) -> Self {
        Self {
            objects: [(
                3387849585,
                [
                    (
                        "contact_point_info_num",
                        Parameter::Int(val.contact_point_info.len() as i32),
                    ),
                    (
                        "collision_info_num",
                        Parameter::Int(val.collision_info.len() as i32),
                    ),
                ]
                .into_iter()
                .collect(),
            )]
            .into_iter()
            .chain(
                val.contact_point_info
                    .into_iter()
                    .enumerate()
                    .map(|(i, info)| (hash_name(&format!("ContactPointInfo_{}", i)), info.into())),
            )
            .chain(
                val.collision_info
                    .into_iter()
                    .enumerate()
                    .map(|(i, info)| (hash_name(&format!("CollisionInfo_{}", i)), info.into())),
            )
            .collect(),
            ..Default::default()
        }
    }
}

impl Mergeable<()> for ContactInfo {
    fn diff(&self, other: &Self) -> Self {
        Self {
            collision_info: self.collision_info.diff(&other.collision_info),
            contact_point_info: self.contact_point_info.diff(&other.collision_info),
        }
    }

    fn merge(base: &Self, diff: &Self) -> Self {
        Self {
            collision_info: base.collision_info.merge(&diff.collision_info),
            contact_point_info: base.contact_point_info.merge(&diff.collision_info),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
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
                .get(&2311816730)
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
                .map(|(i, form)| (format!("Form_{}", i), form))
                .collect(),
        }
    }
}

impl Mergeable<()> for CharacterController {
    fn diff(&self, other: &Self) -> Self {
        Self {
            header: util::diff_pobj(&self.header, &other.header),
            forms: util::simple_index_diff(&self.forms, &other.forms),
        }
    }

    fn merge(base: &Self, diff: &Self) -> Self {
        Self {
            header: util::merge_pobj(&base.header, &diff.header),
            forms: util::simple_index_merge(&base.forms, &diff.forms),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
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
                    .param("cloth_setup_file_path")
                    .ok_or(UKError::MissingAampKey(
                        "Physics cloth header missing setup file path",
                    ))?
                    .as_string()?
                    .to_owned(),
            ),
            subwind: list
                .object("ClothSubWind")
                .ok_or(UKError::MissingAampKey("Physics cloth missing subwind"))?
                .clone(),
            cloths: header
                .param("cloth_num")
                .ok_or(UKError::MissingAampKey(
                    "Physics cloth header missing cloth count",
                ))?
                .as_int()
                .map(|count| -> Result<BTreeMap<usize, ParameterObject>> {
                    (0..count)
                        .map(|i| -> Result<(usize, ParameterObject)> {
                            Ok((
                                i as usize,
                                list.object(&format!("Cloth_{}", i))
                                    .ok_or_else(|| {
                                        UKError::MissingAampKeyD(format!(
                                            "Physics cloth missing Cloth_{}",
                                            i
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
                    "ClothHeader".to_owned(),
                    [
                        (
                            "cloth_file_setup_path",
                            Parameter::String256(val.setup_file_path.unwrap_or_default()),
                        ),
                        ("cloth_num", Parameter::Int(val.cloths.len() as i32)),
                    ]
                    .into_iter()
                    .collect(),
                ),
                ("ClothSubWind".to_owned(), val.subwind),
            ]
            .into_iter()
            .chain(
                val.cloths
                    .into_iter()
                    .map(|(i, cloth)| (format!("Cloth_{}", i), cloth)),
            )
            .collect(),
            ..Default::default()
        }
    }
}

impl Mergeable<()> for Cloth {
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

    fn merge(base: &Self, diff: &Self) -> Self {
        Self {
            setup_file_path: if diff.setup_file_path != base.setup_file_path {
                diff.setup_file_path.clone()
            } else {
                None
            },
            subwind: util::merge_pobj(&base.subwind, &diff.subwind),
            cloths: util::simple_index_merge(&base.cloths, &diff.cloths),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Physics {
    pub ragdoll: Option<String>,
    pub support_bone: Option<String>,
    pub rigid_contact_info: Option<ContactInfo>,
    pub rigid_body_set: Option<BTreeMap<usize, ParameterList>>,
    pub character_controller: Option<CharacterController>,
    pub cloth: Option<Cloth>,
    pub use_system_group_handler: bool,
}

impl TryFrom<&ParameterIO> for Physics {
    type Error = UKError;
    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let param_set = pio
            .list("ParamSet")
            .ok_or(UKError::MissingAampKey("Physics missing param set"))?;
        let header = param_set
            .objects
            .get(&1258832850)
            .ok_or(UKError::MissingAampKey("Physics missing header"))?;
        Ok(Self {
            ragdoll: header
                .param("use_ragdoll")
                .ok_or(UKError::MissingAampKey(
                    "Physics header missing use_ragdoll",
                ))?
                .as_bool()?
                .then(|| -> Result<String> {
                    Ok(param_set
                        .object("Ragdoll")
                        .ok_or(UKError::MissingAampKey("Physics missing ragdoll header"))?
                        .param("ragdoll_setup_file_path")
                        .ok_or(UKError::MissingAampKey(
                            "Physics ragdoll header missing setup file path",
                        ))?
                        .as_string()?
                        .to_owned())
                })
                .transpose()?,
            support_bone: header
                .param("use_support_bone")
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
                        .param("support_bone_setup_file_path")
                        .ok_or(UKError::MissingAampKey(
                            "Physics support bone header missing setup file path",
                        ))?
                        .as_string()?
                        .to_owned())
                })
                .transpose()?,
            rigid_contact_info: header
                .param("use_contact_info")
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
                .param("use_rigid_body_set_num")
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
                                        .list(&format!("RigidBodySet_{}", i))
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
                .param("use_character_controller")
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
                .param("use_cloth")
                .ok_or(UKError::MissingAampKey("Physics header missing use_cloth"))?
                .as_bool()?
                .then(|| -> Result<Cloth> {
                    param_set
                        .list("Cloth")
                        .ok_or(UKError::MissingAampKey("Physics missing cloth section"))?
                        .try_into()
                })
                .transpose()?,
            ..Default::default()
        })
    }
}
