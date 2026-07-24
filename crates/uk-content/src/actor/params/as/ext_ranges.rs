use anyhow::Context;
use roead::{params, aamp::{ParameterList, ParameterObject}};
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Range {
    start: Option<f32>,
    end: Option<f32>,
}

impl TryFrom<&ParameterObject> for Range {
    type Error = UKError;

    fn try_from(value: &ParameterObject) -> Result<Self> {
        Ok(Self {
            start: Some(value
                .get("Start")
                .ok_or(UKError::MissingAampKey("Range missing Start", Box::from(None)))?
                .as_f32()
                .context("Range has invalid Start")?),
            end: Some(value
                .get("End")
                .ok_or(UKError::MissingAampKey("Range missing End", Box::from(None)))?
                .as_f32()
                .context("Range has invalid End")?),
        })
    }
}

impl From<Range> for ParameterObject {
    fn from(value: Range) -> Self {
        params!(
            "Start" => value.start.expect("Range Start should have been read on import").into(),
            "End" => value.end.expect("Range End should have been read on import").into(),
        )
    }
}

impl Mergeable for Range {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            start: other.start
                .ne(&self.start)
                .then_some(other.start)
                .unwrap_or_default(),
            end: other.end
                .ne(&self.end)
                .then_some(other.end)
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            start: diff.start
                .or(self.start),
            end: diff.end
                .or(self.end),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ranges {
    ranges: DeleteMap<i32, Range>,
}

impl TryFrom<&ParameterList> for Ranges {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            ranges: value.objects.iter()
                .map(|(n, v)| -> Result<(i32, Range)> {
                    Ok((
                        super::get_range_index(n.hash())?,
                        v.try_into().context("Ranges has invalid Range")?
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

impl Mergeable for Ranges {
    fn diff(&self, other: &Self) -> Self {
        Self {
            ranges: self.ranges.diff(&other.ranges),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            ranges: self.ranges.merge(&diff.ranges),
        }
    }
}
