use anyhow::{anyhow, Context, Error, Result};
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use super::res_children::ResourceWithChildren;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SelectorResource {
    base: Option<ResourceWithChildren>,
    no_sync: Option<bool>,
    judge_once: Option<bool>,
}

impl TryFrom<&ParameterList> for SelectorResource {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let parameters = value.objects
            .get("Parameters")
            .ok_or(anyhow!("Missing Parameters"))?;
        Ok(Self {
            base: Some(value.try_into()?),
            no_sync: parameters
                .get("NoSync")
                .map(|p| p.as_bool().context("Invalid NoSync"))
                .transpose()?,
            judge_once: parameters
                .get("JudgeOnce")
                .map(|p| p.as_bool().context("Invalid JudgeOnce"))
                .transpose()?,
        })
    }
}

impl From<SelectorResource> for ParameterList {
    fn from(value: SelectorResource) -> Self {
        let mut base: Self = value.base.unwrap().into();
        let params = base.objects.get_mut("Parameters").unwrap();
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
