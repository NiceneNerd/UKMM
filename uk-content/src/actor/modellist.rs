use std::collections::BTreeMap;

use crate::{Result, UKError};
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelList {
    pub controller_info: ParameterObject,
    pub attention: ParameterObject,
    pub model_data: ParameterList,
    pub anm_target: BTreeMap<usize, ParameterList>,
}
