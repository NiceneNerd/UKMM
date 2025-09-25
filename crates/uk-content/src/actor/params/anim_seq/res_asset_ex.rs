use anyhow::{Context, Error, Result};
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
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
        value.base.unwrap().into()
    }
}
