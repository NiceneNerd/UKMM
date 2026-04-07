use anyhow::Context;
use roead::{objs, aamp::ParameterList};
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntArray {
    values: DeleteMap<i32, i32>,
}

impl TryFrom<&ParameterList> for IntArray {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            values: value.objects
                .get("IntArray0")
                .ok_or(UKError::Other("AnimSeq Element IntArray missing IntArray0"))?
                .iter()
                .map(|(n, v)| -> Result<(i32, i32)> {
                    Ok((
                        super::get_value_index(n.hash())?,
                        v.as_i32().context("IntArray contains non-integer")?
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<IntArray> for ParameterList {
    fn from(value: IntArray) -> Self {
        Self {
            objects: objs!(
                "IntArray0" => value.values
                    .into_iter()
                    .map(|(i, v)| (format!("Value{}", i), v.into()))
                    .collect()
            ),
            lists: Default::default(),
        }
    }
}

impl Mergeable for IntArray {
    fn diff(&self, other: &Self) -> Self {
        Self {
            values: self.values.diff(&other.values),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            values: self.values.merge(&diff.values),
        }
    }
}
