use anyhow::{anyhow, Context, Error, Result};
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
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
        let mut base: Self = value.base
            .expect("SequencePlayContainerResource should have base ResourceWithChildren")
            .into();
        base.objects
            .get_mut("Parameters")
            .expect("SequencePlayContainerResource should have Parameters")
            .insert(
                "SequenceLoop",
                value.sequence_loop
                    .expect("SequencePlayContainerResource should have SequenceLoop")
                    .into()
            );
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

impl Mergeable for SequencePlayContainerResource {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            base: self.base.as_ref()
                .expect("SequencePlayContainerResource should contain base ResourceWithChildren")
                .ne(other.base.as_ref().expect("SequencePlayContainerResource should contain base ResourceWithChildren"))
                .then(|| self.base.as_ref().expect("").diff(other.base.as_ref().expect(""))),
            sequence_loop: other.sequence_loop
                .ne(&self.sequence_loop)
                .then_some(other.sequence_loop)
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            base: diff.base.as_ref()
                .map(|b|
                    self.base.as_ref()
                        .expect("BlenderResource should contain base ResourceWithChildren")
                        .merge(b)
                )
                .or(self.base.clone()),
            sequence_loop: diff.sequence_loop
                .or(self.sequence_loop),
        }
    }
}
