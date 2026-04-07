use anyhow::Context;
use roead::{params, aamp::{ParameterList, ParameterObject, Parameter::String32}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HoldEvent {
    type_index: Option<i32>,
    start_frame: Option<f32>,
    end_frame: Option<f32>,
    value: Option<String>,
}

impl TryFrom<&ParameterObject> for HoldEvent {
    type Error = UKError;

    fn try_from(value: &ParameterObject) -> Result<Self> {
        Ok(Self {
            type_index: Some(value
                .get("TypeIndex")
                .ok_or(UKError::Other("AnimSeq Element HoldEvent missing TypeIndex"))?
                .as_i32()
                .context("Invalid TypeIndex")?),
            start_frame: Some(value
                .get("StartFrame")
                .ok_or(UKError::Other("AnimSeq Element HoldEvent missing StartFrame"))?
                .as_f32()
                .context("Invalid StartFrame")?),
            end_frame: Some(value
                .get("EndFrame")
                .ok_or(UKError::Other("AnimSeq Element HoldEvent missing EndFrame"))?
                .as_f32()
                .context("Invalid EndFrame")?),
            value: Some(value
                .get("Value")
                .ok_or(UKError::Other("AnimSeq Element HoldEvent missing Value"))?
                .as_str()
                .context("Invalid Value")?
                .into()),
        })
    }
}

impl From<HoldEvent> for ParameterObject {
    fn from(value: HoldEvent) -> Self {
        params!(
            "TypeIndex" => value.type_index
                .expect("TypeIndex should have been read on import")
                .into(),
            "StartFrame" => value.start_frame
                .expect("StartFrame should have been read on import")
                .into(),
            "EndFrame" => value.end_frame
                .expect("EndFrame should have been read on import")
                .into(),
            "Value" => String32(value.value
                .expect("Value should have been read on import")
                .into()),
        )
    }
}

impl Mergeable for HoldEvent {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            type_index: other.type_index
                .ne(&self.type_index)
                .then_some(other.type_index)
                .unwrap_or_default(),
            start_frame: other.start_frame
                .ne(&self.start_frame)
                .then_some(other.start_frame)
                .unwrap_or_default(),
            end_frame: other.end_frame
                .ne(&self.end_frame)
                .then_some(other.end_frame)
                .unwrap_or_default(),
            value: other.value
                .ne(&self.value)
                .then(|| other.value.clone())
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            type_index: diff.type_index
                .or(self.type_index),
            start_frame: diff.start_frame
                .or(self.start_frame),
            end_frame: diff.end_frame
                .or(self.end_frame),
            value: diff.value.clone()
                .or_else(|| self.value.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HoldEvents {
    events: DeleteMap<i32, HoldEvent>,
}

impl TryFrom<&ParameterList> for HoldEvents {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            events: value.objects
                .iter()
                .map(|(n, v)| -> Result<(i32, HoldEvent)> {
                    Ok((
                        super::get_event_index(n.hash())?,
                        v.try_into().context("Invalid HoldEvent")?
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<HoldEvents> for ParameterList {
    fn from(value: HoldEvents) -> Self {
        Self {
            objects: value.events
                .into_iter()
                .map(|(i, h)| (format!("Event{}", i), h.into()))
                .collect(),
            lists: Default::default(),
        }
    }
}

impl Mergeable for HoldEvents {
    fn diff(&self, other: &Self) -> Self {
        Self {
            events: self.events.diff(&other.events),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            events: self.events.merge(&diff.events),
        }
    }
}
