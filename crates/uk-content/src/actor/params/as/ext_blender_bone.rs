use anyhow::Context;
use roead::{objs, params, aamp::{ParameterList, Parameter::String32}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::prelude::Mergeable;
use crate::{UKError, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlenderBone {
    value: Option<String>,
}

impl TryFrom<&ParameterList> for BlenderBone {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            value: Some(value.objects
                .get("BlenderBone0")
                .ok_or(UKError::Other("AnimSeq Element BlenderBone missing BlenderBone0"))?
                .get("Value0")
                .ok_or(UKError::Other("AnimSeq Element BlenderBone0 missing Value0"))?
                .as_str()
                .context("Invalid Value0")?
                .into()),
        })
    }
}

impl From<BlenderBone> for ParameterList {
    fn from(value: BlenderBone) -> Self {
        Self {
            objects: objs!(
                "BlenderBone0" => params!(
                    "Value0" => String32(value.value
                        .expect("BlenderBone Value should have been read on import")
                        .into())
                )
            ),
            lists: Default::default(),
        }
    }
}

impl Mergeable for BlenderBone {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            value: other.value
                .ne(&self.value)
                .then(|| other.value.clone())
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            value: diff.value.clone()
                .or_else(|| self.value.clone()),
        }
    }
}
