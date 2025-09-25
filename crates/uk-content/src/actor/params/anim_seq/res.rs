use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, params, aamp::ParameterList};
use roead::aamp::Name;
use serde::{Deserialize, Serialize};
use crate::util::DeleteVec;
use super::Extension;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    type_index: Option<i32>,
    extensions: DeleteVec<Extension>,
}

impl TryFrom<&ParameterList> for Resource {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            // These are fine because AnimSeq already verified these exist
            // in order to call this or one of the TryFrom parents
            type_index: Some(unsafe { value.objects
                .get("Parameters")
                .unwrap_unchecked()
                .get("TypeIndex")
                .unwrap_unchecked()
                .as_i32()
                .unwrap_unchecked() }),
            extensions: value.lists
                .get("Extend")
                .ok_or(anyhow!("Missing Extend"))?
                .lists
                .iter()
                .map(Extension::try_from)
                .collect::<Result<_>>()
                .context("Invalid Extend")?,
        })
    }
}

impl From<Resource> for ParameterList {
    fn from(value: Resource) -> Self {
        Self {
            objects: objs!(
                "Parameters" => params!(
                    "TypeIndex" => value.type_index.unwrap().into(),
                )
            ),
            lists: value.extensions
                .into_iter()
                .map(<(Name, ParameterList)>::from)
                .collect(),
        }
    }
}
