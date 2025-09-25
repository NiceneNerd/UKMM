use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, aamp::ParameterList};
use serde::{Deserialize, Serialize};
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntArray {
    values: DeleteMap<i32, i32>,
}

impl TryFrom<&ParameterList> for IntArray {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            values: value.objects
                .get("IntArray0")
                .ok_or(anyhow!("Missing IntArray0"))?
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
