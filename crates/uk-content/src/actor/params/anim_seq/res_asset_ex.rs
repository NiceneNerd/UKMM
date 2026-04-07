use anyhow::{Context, Error, Result};
use itertools::Itertools;
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use super::res_asset::AssetResource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetExResource {
    base:   Option<AssetResource>,
}

impl TryFrom<&ParameterList> for AssetExResource {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            base: Some(value.try_into().context("Invalid AssetResource")?),
        })
    }
}

impl From<AssetExResource> for ParameterList {
    fn from(value: AssetExResource) -> Self {
        value.base
            .expect("AssetExResource should have base AssetResource")
            .into()
    }
}

impl Mergeable for AssetExResource {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            base: self.base.as_ref()
                .expect("AssetExResource should contain base AssetResource")
                .ne(other.base.as_ref().expect("AssetExResource should contain base AssetResource"))
                .then(|| self.base.as_ref().expect("").diff(other.base.as_ref().expect(""))),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            base: diff.base.as_ref()
                .map(|b|
                    self.base.as_ref()
                        .expect("AssetExResource should contain base AssetResource")
                        .merge(b)
                )
                .or(self.base.clone()),
        }
    }
}
