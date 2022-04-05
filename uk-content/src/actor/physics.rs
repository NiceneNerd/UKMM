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
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactInfo {
    pub contact_point_info: DeleteVec<ContactInfo>,
    pub collision_info: DeleteVec<ContactInfo>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterController {
    pub params: ParameterObject,
    pub forms: BTreeMap<usize, ParameterList>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cloth {
    pub setup_file_path: String,
    pub subwind: ParameterObject,
    pub cloths: BTreeMap<usize, ParameterObject>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Physics {
    pub ragdoll: Option<String>,
    pub support_bone: Option<String>,
    pub rigid_contact_info: Option<ContactInfo>,
    pub rigid_body_set: Option<BTreeMap<usize, ParameterList>>,
    pub character_controller: Option<CharacterController>,
    pub cloth: Option<Cloth>,
}
