use anyhow::{anyhow, Context, Error, Result};
use roead::{params, aamp::{ParameterList, ParameterObject, Parameter::String32}};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::util::DeleteMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HoldEvent {
    type_index: Option<i32>,
    start_frame: Option<f32>,
    end_frame: Option<f32>,
    value: Option<String>,
}

impl TryFrom<&ParameterObject> for HoldEvent {
    type Error = Error;

    fn try_from(value: &ParameterObject) -> Result<Self> {
        Ok(Self {
            type_index: Some(value
                .get("TypeIndex")
                .ok_or(anyhow!("Missing TypeIndex"))?
                .as_i32()
                .context("Invalid TypeIndex")?),
            start_frame: Some(value
                .get("StartFrame")
                .ok_or(anyhow!("Missing StartFrame"))?
                .as_f32()
                .context("Invalid StartFrame")?),
            end_frame: Some(value
                .get("EndFrame")
                .ok_or(anyhow!("Missing EndFrame"))?
                .as_f32()
                .context("Invalid EndFrame")?),
            value: Some(value
                .get("Value")
                .ok_or(anyhow!("Missing Value"))?
                .as_str()
                .context("Invalid Value")?
                .into()),
        })
    }
}

impl From<HoldEvent> for ParameterObject {
    fn from(value: HoldEvent) -> Self {
        params!(
            "TypeIndex" => value.type_index.unwrap().into(),
            "StartFrame" => value.start_frame.unwrap().into(),
            "EndFrame" => value.end_frame.unwrap().into(),
            "Value" => String32(value.value.unwrap().into()),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HoldEvents {
    events: DeleteMap<i32, HoldEvent>,
}

impl TryFrom<&ParameterList> for HoldEvents {
    type Error = Error;

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
