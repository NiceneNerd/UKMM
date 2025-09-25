use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, params, aamp::{ParameterList, Parameter::String32}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlenderBone {
    value: Option<String>,
}

impl TryFrom<&ParameterList> for BlenderBone {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            value: Some(value.objects
                .get("BlenderBone0")
                .ok_or(anyhow!("Missing BlenderBone0"))?
                .get("Value0")
                .ok_or(anyhow!("Missing Value0"))?
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
                    "Value0" => String32(value.value.unwrap().into())
                )
            ),
            lists: Default::default(),
        }
    }
}
