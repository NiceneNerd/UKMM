use anyhow::{anyhow, Context, Error, Result};
use roead::aamp::{ParameterList, Parameter::String64};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use super::res::Resource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetResource {
    base:       Option<Resource>,
    file_name:  Option<String>,
}

impl TryFrom<&ParameterList> for AssetResource {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            base: Some(value.try_into()?),
            file_name: Some(value.objects
                .get("Parameters")
                .ok_or(anyhow!("Missing Parameters"))?
                .get("FileName")
                .ok_or(anyhow!("Missing FileName"))?
                .as_str()
                .context("Invalid FileName")?
                .into()),
        })
    }
}

impl From<AssetResource> for ParameterList {
    fn from(resource: AssetResource) -> Self {
        let mut base: Self = resource.base.unwrap().into();
        base.objects
            .get_mut("Parameters")
            .unwrap()
            .insert("FileName", String64(Box::new(resource.file_name.unwrap().into())));
        base
    }
}
