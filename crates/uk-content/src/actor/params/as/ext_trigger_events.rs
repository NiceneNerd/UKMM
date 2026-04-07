use anyhow::{anyhow, Context, Error, Result};
use roead::{params, aamp::{ParameterList, ParameterObject, Parameter::String32}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::prelude::Mergeable;
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TriggerEvent {
    type_index: Option<i32>,
    frame: Option<f32>,
    value: Option<String>,
}

impl TryFrom<&ParameterObject> for TriggerEvent {
    type Error = Error;

    fn try_from(value: &ParameterObject) -> Result<Self> {
        Ok(Self {
            type_index: Some(value
                .get("TypeIndex")
                .ok_or(anyhow!("Missing TypeIndex"))?
                .as_i32()
                .context("Invalid TypeIndex")?),
            frame: Some(value
                .get("Frame")
                .ok_or(anyhow!("Missing Frame"))?
                .as_f32()
                .context("Invalid Frame")?),
            value: Some(value
                .get("Value")
                .ok_or(anyhow!("Missing Value"))?
                .as_str()
                .context("Invalid Value")?
                .into()),
        })
    }
}

impl From<TriggerEvent> for ParameterObject {
    fn from(value: TriggerEvent) -> Self {
        params!(
            "TypeIndex" => value.type_index
                .expect("TriggerEvent should have TypeIndex")
                .into(),
            "Frame" => value.frame
                .expect("TriggerEvent should have Frame")
                .into(),
            "Value" => String32(value.value
                .expect("TriggerEvent should have Value")
                .into())
        )
    }
}

impl Mergeable for TriggerEvent {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            type_index: other.type_index
                .ne(&self.type_index)
                .then_some(other.type_index)
                .unwrap_or_default(),
            frame: other.frame
                .ne(&self.frame)
                .then_some(other.frame)
                .unwrap_or_default(),
            value: other.value.clone()
                .ne(&self.value)
                .then(|| other.value.clone())
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            type_index: diff.type_index
                .or(self.type_index),
            frame: diff.frame
                .or(self.frame),
            value: diff.value.clone()
                .or_else(|| self.value.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TriggerEvents {
    events: DeleteMap<i32, TriggerEvent>,
}

impl TryFrom<&ParameterList> for TriggerEvents {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            events: value.objects
                .iter()
                .map(|(n, v)| -> Result<(i32, TriggerEvent)> {
                    Ok((
                        super::get_event_index(n.hash())?,
                        v.try_into().context("Invalid TriggerEvent")?
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<TriggerEvents> for ParameterList {
    fn from(value: TriggerEvents) -> Self {
        Self {
            objects: value.events
                .into_iter()
                .map(|(i, t)| (format!("Event{}", i), t.into()))
                .collect(),
            lists: Default::default(),
        }
    }
}

impl Mergeable for TriggerEvents {
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
