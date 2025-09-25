use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, aamp::{ParameterList, Parameter::String64}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StringArray {
    values: DeleteMap<i32, String>,
}

impl TryFrom<&ParameterList> for StringArray {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            values: value.objects
                .get("StringArray0")
                .ok_or(anyhow!("Missing StringArray0"))?
                .iter()
                .map(|(n, v)| -> Result<(i32, String)> {
                    Ok((
                        super::get_value_index(n.hash())?,
                        v.as_str().context("StringArray contains non-string")?.into()
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
