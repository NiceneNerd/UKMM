use anyhow::Context;
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::{UKError, Result};
use super::res_asset::AssetResource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkeletalAssetResource {
    base: Option<AssetResource>,
    init_anm_driven: Option<i32>,
    morph: Option<f32>,
    reset_morph: Option<f32>,
}

impl TryFrom<&ParameterList> for SkeletalAssetResource {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let parameters = value.objects
            .get("Parameters")
            .ok_or(UKError::MissingAampKey("Element missing Parameters", Box::from(None)))?;
        Ok(Self {
            base: Some(value.try_into()?),
            init_anm_driven: parameters
                .get("InitAnmDriven")
                .map(|p| p.as_i32().context("Element has invalid InitAnmDriven"))
                .transpose()?,
            morph: parameters
                .get("Morph")
                .map(|p| p.as_f32().context("Element has invalid Morph"))
                .transpose()?,
            reset_morph: parameters
                .get("ResetMorph")
                .map(|p| p.as_f32().context("Element has invalid ResetMorph"))
                .transpose()?,
        })
    }
}

impl From<SkeletalAssetResource> for ParameterList {
    fn from(value: SkeletalAssetResource) -> Self {
        let mut base: Self = value.base
            .expect("SkeletalAssetResource should have base Resource")
            .into();
        let params = base.objects
            .get_mut("Parameters")
            .expect("SkeletalAssetResource should have Parameters");
        value.init_anm_driven.into_iter()
            .for_each(|f| params.insert("InitAnmDriven", f.into()));
        value.morph.into_iter()
            .for_each(|f| params.insert("Morph", f.into()));
        value.reset_morph.into_iter()
            .for_each(|f| params.insert("ResetMorph", f.into()));
        base
    }
}

impl Mergeable for SkeletalAssetResource {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            base: self.base.as_ref()
                .expect("SkeletalAssetResource should contain base AssetResource")
                .ne(other.base.as_ref().expect("SkeletalAssetResource should contain base AssetResource"))
                .then(|| self.base.as_ref().expect("").diff(other.base.as_ref().expect(""))),
            init_anm_driven: other.init_anm_driven
                .ne(&self.init_anm_driven)
                .then_some(other.init_anm_driven)
                .unwrap_or_default(),
            morph: other.morph
                .ne(&self.morph)
                .then_some(other.morph)
                .unwrap_or_default(),
            reset_morph: other.reset_morph
                .ne(&self.reset_morph)
                .then_some(other.reset_morph)
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            base: diff.base.as_ref()
                .map(|b|
                    self.base.as_ref()
                        .expect("SkeletalAssetResource should contain base AssetResource")
                        .merge(b)
                )
                .or(self.base.clone()),
            init_anm_driven: diff.init_anm_driven
                .or(self.init_anm_driven),
            morph: diff.morph
                .or(self.morph),
            reset_morph: diff.reset_morph
                .or(self.reset_morph),
        }
    }
}
