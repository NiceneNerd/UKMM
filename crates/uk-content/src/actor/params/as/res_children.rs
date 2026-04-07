use anyhow::{anyhow, Context, Error, Result};
use itertools::Itertools;
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::util::DeleteMap;
use super::{get_child_index};
use super::res::Resource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceWithChildren {
    base: Option<Resource>,
    pub children: DeleteMap<i32, i32>,
}

impl TryFrom<&ParameterList> for ResourceWithChildren {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let children = value.objects
            .get("Children")
            .ok_or(anyhow!("Missing Children"))?;
        Ok(Self {
            base: Some(value.try_into()?),
            children: children
                .iter()
                .map(|(n, p)| -> Result<(i32, i32)> {
                    Ok((
                        get_child_index(n.hash())?,
                        p.as_i32().context("Invalid Child Index")?
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<ResourceWithChildren> for ParameterList {
    fn from(value: ResourceWithChildren) -> Self {
        ParameterList::from(value.base.expect("ResourceWithChildren should have base Resource"))
            .with_object(
                "Children",
                value.children
                    .into_iter()
                    .map(|(i, c)| (format!("Child{}", i), c.into()))
                    .collect()
            )
    }
}

impl Mergeable for ResourceWithChildren {
    fn diff(&self, other: &Self) -> Self {
        Self {
            base: self.base.as_ref()
                .expect("ResourceWithChildren should contain base Resource")
                .ne(other.base.as_ref().expect("ResourceWithChildren should contain base Resource"))
                .then(|| self.base.as_ref().expect("").diff(other.base.as_ref().expect(""))),
            children: self.children.diff(&other.children),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            base: diff.base.as_ref()
                .map(|b|
                    self.base.as_ref()
                        .expect("ResourceWithChildren should contain base Resource")
                        .merge(b)
                )
                .or(self.base.clone()),
            children: self.children.merge(&diff.children),
        }
    }
}
