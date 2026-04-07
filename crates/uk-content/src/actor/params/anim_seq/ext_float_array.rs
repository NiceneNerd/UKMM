use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, aamp::ParameterList};
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatArray {
    values: DeleteMap<i32, f32>,
}

impl TryFrom<&ParameterList> for FloatArray {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            values: value.objects
                .get("FloatArray0")
                .ok_or(anyhow!("Missing FloatArray0"))?
                .iter()
                .map(|(n, v)| -> Result<(i32, f32)> {
                    Ok((
                        super::get_value_index(n.hash())?,
                        v.as_f32().context("FloatArray contains non-float")?
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<FloatArray> for ParameterList {
    fn from(value: FloatArray) -> Self {
        Self {
            objects: objs!(
                "FloatArray0" => value.values
                    .into_iter()
                    .map(|(i, f)| (format!("Value{}", i), f.into()))
                    .collect()
            ),
            lists: Default::default(),
        }
    }
}

impl Mergeable for FloatArray {
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
