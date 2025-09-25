use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, params, aamp::ParameterList};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BitIndex {
    type_index: Option<i32>,
}

impl TryFrom<&ParameterList> for BitIndex {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            type_index: Some(value.objects
                .get("BitIndex0")
                .ok_or(anyhow!("Missing BitIndex0"))?
                .get("TypeIndex")
                .ok_or(anyhow!("Missing TypeIndex"))?
                .as_i32()
                .context("Invalid TypeIndex")?),
        })
    }
}

impl From<BitIndex> for ParameterList {
    fn from(value: BitIndex) -> Self {
        Self {
            objects: objs!(
                "BitIndex0" => params!(
                    "TypeIndex" => value.type_index.unwrap().into()
                )
            ),
            lists: Default::default(),
        }
    }
}
