use anyhow::{anyhow, Context, Error, Result};
use roead::aamp::ParameterList;
use serde::{Deserialize, Serialize};
use super::res_asset::AssetResource;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkeletalAssetResource {
    base: Option<AssetResource>,
    init_anm_driven: Option<i32>,
    morph: Option<f32>,
    reset_morph: Option<f32>,
}

impl TryFrom<&ParameterList> for SkeletalAssetResource {
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let parameters = value.objects
            .get("Parameters")
            .ok_or(anyhow!("Missing Parameters"))?;
        Ok(Self {
            base: Some(value.try_into()?),
            init_anm_driven: parameters
                .get("InitAnmDriven")
                .map(|p| p.as_i32().context("Invalid InitAnmDriven"))
                .transpose()?,
            morph: parameters
                .get("Morph")
                .map(|p| p.as_f32().context("Invalid Morph"))
                .transpose()?,
            reset_morph: parameters
                .get("ResetMorph")
                .map(|p| p.as_f32().context("Invalid ResetMorph"))
                .transpose()?,
        })
    }
}

impl From<SkeletalAssetResource> for ParameterList {
    fn from(value: SkeletalAssetResource) -> Self {
        let mut base: Self = value.base.unwrap().into();
        let params = base.objects.get_mut("Parameters").unwrap();
        value.init_anm_driven.into_iter()
            .for_each(|f| params.insert("InitAnmDriven", f.into()));
        value.morph.into_iter()
            .for_each(|f| params.insert("Morph", f.into()));
        value.reset_morph.into_iter()
            .for_each(|f| params.insert("ResetMorph", f.into()));
        base
    }
}
