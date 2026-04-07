use anyhow::Context;
use roead::{objs, params, aamp::ParameterList};
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::{UKError, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BitIndex {
    type_index: Option<i32>,
}

impl TryFrom<&ParameterList> for BitIndex {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            type_index: Some(value.objects
                .get("BitIndex0")
                .ok_or(UKError::Other("AnimSeq Element BitIndex missing BitIndex0"))?
                .get("TypeIndex")
                .ok_or(UKError::Other("AnimSeq Element BitIndex0 missing TypeIndex"))?
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
                    "TypeIndex" => value.type_index
                        .expect("BitIndex0 should have been read on import")
                        .into()
                )
            ),
            lists: Default::default(),
        }
    }
}

impl Mergeable for BitIndex {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            type_index: other.type_index
                .ne(&self.type_index)
                .then_some(other.type_index)
                .unwrap_or_default(),
        }
    }
    
    fn merge(&self, diff: &Self) -> Self {
        Self {
            type_index: diff.type_index
                .or(self.type_index),
        }
    }
}
