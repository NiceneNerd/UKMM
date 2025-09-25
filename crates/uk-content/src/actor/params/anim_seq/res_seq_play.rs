use anyhow::{anyhow, Context, Error, Result};
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use super::res_children::ResourceWithChildren;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SequencePlayContainerResource {
    base: Option<ResourceWithChildren>,
    sequence_loop: Option<bool>,
}

impl TryFrom<&ParameterList> for SequencePlayContainerResource {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            base: Some(value.try_into()?),
            sequence_loop: Some(value.objects
                .get("Parameters")
                .ok_or(anyhow!("Missing Parameters"))?
                .get("SequenceLoop")
                .ok_or(anyhow!("Missing SequenceLoop"))?
                .as_bool()
                .context("Invalid SequenceLoop")?),
        })
    }
}

impl From<SequencePlayContainerResource> for ParameterList {
    fn from(value: SequencePlayContainerResource) -> Self {
        let mut base: Self = value.base.unwrap().into();
        base.objects
            .get_mut("Parameters")
            .unwrap()
            .insert("SequenceLoop", value.sequence_loop.unwrap().into());
        base
    }
}

impl SequencePlayContainerResource {
    pub fn children(&self) -> Box<dyn Iterator<Item = &i32> + '_> {
        if let Some(base) = &self.base {
            Box::new(base.children.values())
        } else {
            Box::new(std::iter::empty::<&i32>())
        }
    }
}
