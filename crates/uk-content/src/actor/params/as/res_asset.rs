use anyhow::Context;
use roead::aamp::{ParameterList, Parameter::String64};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use super::res::Resource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetResource {
    base:       Option<Resource>,
    file_name:  Option<String>,
}

impl TryFrom<&ParameterList> for AssetResource {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            base: Some(value.try_into()?),
            file_name: Some(value.objects
                .get("Parameters")
                .ok_or(UKError::MissingAampKey("Element missing Parameters", Box::from(None)))?
                .get("FileName")
                .ok_or(UKError::MissingAampKey("Element missing FileName", Box::from(None)))?
                .as_str()
                .context("Element has invalid FileName")?
                .into()),
        })
    }
}

impl From<AssetResource> for ParameterList {
    fn from(resource: AssetResource) -> Self {
        let mut base: Self = resource.base
            .expect("AssetResource should contain base Resource")
            .into();
        base.objects
            .get_mut("Parameters")
            .expect("Resource should contain parameters")
            .insert(
                "FileName",
                String64(Box::new(
                    resource.file_name.expect("AssetResource should have FileName").into()
                ))
            );
        base
    }
}

impl Mergeable for AssetResource {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            base: self.base.as_ref()
                .expect("AssetResource should contain base Resource")
                .ne(other.base.as_ref().expect("AssetResource should contain base Resource"))
                .then(|| self.base.as_ref().expect("").diff(other.base.as_ref().expect(""))),
            file_name: other.file_name.clone()
                .ne(&self.file_name)
                .then(|| other.file_name.clone())
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            base: diff.base.as_ref()
                .map(|b|
                    self.base.as_ref()
                        .expect("AssetResource should contain base Resource")
                        .merge(b)
                )
                .or(self.base.clone()),
            file_name: diff.file_name.clone()
                .or_else(|| self.file_name.clone()),
        }
    }
}
