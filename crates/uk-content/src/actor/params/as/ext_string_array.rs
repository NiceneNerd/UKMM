use anyhow::Context;
use roead::{objs, aamp::{ParameterList, Parameter::String64}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StringArray {
    values: DeleteMap<i32, String>,
}

impl TryFrom<&ParameterList> for StringArray {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            values: value.objects
                .get("StringArray0")
                .ok_or(UKError::MissingAampKey("StringArray missing StringArray0", Box::from(None)))?
                .iter()
                .map(|(n, v)| -> Result<(i32, String)> {
                    let index = super::get_value_index(n.hash())
                        .context(format!("Could not get index of Value with key hash {}", n))?;
                    Ok((
                        index,
                        v.as_str()
                            .context(format!("StringArray has invalid Value{}", index))?
                            .into()
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<StringArray> for ParameterList {
    fn from(value: StringArray) -> Self {
        Self {
            objects: objs!(
                "StringArray0" => value.values
                    .into_iter()
                    .map(|(i, s)|
                        (format!("Value{}", i), String64(Box::new(s.into())))
                    )
                    .collect()
            ),
            lists: Default::default()
        }
    }
}

impl Mergeable for StringArray {
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
