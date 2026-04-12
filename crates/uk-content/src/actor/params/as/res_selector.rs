use anyhow::Context;
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use super::res_children::ResourceWithChildren;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SelectorResource {
    base: Option<ResourceWithChildren>,
    no_sync: Option<bool>,
    judge_once: Option<bool>,
}

impl TryFrom<&ParameterList> for SelectorResource {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let parameters = value.objects
            .get("Parameters")
            .ok_or(UKError::MissingAampKey("SelectorResource missing Parameters", Box::from(None)))?;
        Ok(Self {
            base: Some(value.try_into()?),
            no_sync: parameters
                .get("NoSync")
                .map(|p| p.as_bool().context("SelectorResource has invalid NoSync"))
                .transpose()?,
            judge_once: parameters
                .get("JudgeOnce")
                .map(|p| p.as_bool().context("SelectorResource has invalid JudgeOnce"))
                .transpose()?,
        })
    }
}

impl From<SelectorResource> for ParameterList {
    fn from(value: SelectorResource) -> Self {
        let mut base: Self = value.base
            .expect("SelectorResource should have base ResourceWithChildren")
            .into();
        let params = base.objects
            .get_mut("Parameters")
            .expect("SelectorResource should have Parameters");
        value.no_sync.into_iter().for_each(|p| { params.insert("NoSync", p.into()) });
        value.judge_once.into_iter().for_each(|p| { params.insert("JudgeOnce", p.into()) });
        base
    }
}

impl SelectorResource {
    pub fn children(&self) -> Box<dyn Iterator<Item = &i32> + '_> {
        if let Some(base) = &self.base {
            Box::new(base.children.values())
        } else {
            Box::new(std::iter::empty::<&i32>())
        }
    }
}

impl Mergeable for SelectorResource {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            base: self.base.as_ref()
                .expect("SelectorResource should contain base ResourceWithChildren")
                .ne(other.base.as_ref().expect("SelectorResource should contain base ResourceWithChildren"))
                .then(|| self.base.as_ref().expect("").diff(other.base.as_ref().expect(""))),
            no_sync: other.no_sync
                .ne(&self.no_sync)
                .then_some(other.no_sync)
                .unwrap_or_default(),
            judge_once: other.judge_once
                .ne(&self.judge_once)
                .then_some(other.judge_once)
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            base: diff.base.as_ref()
                .map(|b|
                    self.base.as_ref()
                        .expect("SelectorResource should contain base ResourceWithChildren")
                        .merge(b)
                )
                .or(self.base.clone()),
            no_sync: diff.no_sync
                .or(self.no_sync),
            judge_once: diff.judge_once
                .or(self.judge_once),
        }
    }
}
