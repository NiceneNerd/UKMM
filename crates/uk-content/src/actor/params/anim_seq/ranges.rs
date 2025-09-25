use anyhow::{anyhow, Context, Error, Result};
use roead::{params, aamp::{ParameterList, ParameterObject}};
use serde::{Deserialize, Serialize};
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Range {
    start: Option<f32>,
    end: Option<f32>,
}

impl TryFrom<&ParameterObject> for Range {
    type Error = Error;

    fn try_from(value: &ParameterObject) -> Result<Self> {
        Ok(Self {
            start: Some(value
                .get("Start")
                .ok_or(anyhow!("Missing Start"))?
                .as_f32()
                .context("Invalid Start")?),
            end: Some(value
                .get("End")
                .ok_or(anyhow!("Missing End"))?
                .as_f32()
                .context("Invalid End")?),
        })
    }
}

impl From<Range> for ParameterObject {
    fn from(value: Range) -> Self {
        params!(
            "Start" => value.start.unwrap().into(),
            "End" => value.end.unwrap().into(),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ranges {
    ranges: DeleteMap<i32, Range>,
}

impl TryFrom<&ParameterList> for Ranges {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            ranges: value.objects.iter()
                .map(|(n, v)| -> Result<(i32, Range)> {
                    Ok((
                        super::get_range_index(n.hash())?,
                        v.try_into().context("Invalid Range")?
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<Ranges> for ParameterList {
    fn from(value: Ranges) -> Self {
        Self {
            objects: value.ranges
                .into_iter()
                .map(|(i, r)| (format!("Range{}", i), r.into()))
                .collect(),
            lists: Default::default(),
        }
    }
}
