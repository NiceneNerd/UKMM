use anyhow::Context;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use smartstring::{LazyCompact, SmartString};

use crate::{
    util::{DeleteVec, HashMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct SingleRecipe {
    pub actors: Option<DeleteVec<i32>>,
    pub hb:     Option<i32>,
    pub num:    i32,
    pub recipe: i32,
    pub tags:   Option<DeleteVec<i32>>,
}

impl TryFrom<&Byml> for SingleRecipe {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        Ok(Self {
            actors: hash
                .get("Actors")
                .map(|arr| -> Result<_> {
                    arr.as_array()?
                        .iter()
                        .map(|i| Ok(i.as_int::<i32>()?))
                        .collect()
                })
                .transpose()?,
            hb:     hash
                .get("HB")
                .map(|i| i.as_i32().context("HB not int"))
                .transpose()?,
            num:    hash
                .get("Num")
                .ok_or(UKError::MissingBymlKey("SingleRecipe missing num"))?
                .as_i32()?,
            recipe: hash
                .get("Recipe")
                .ok_or(UKError::MissingBymlKey("SingleRecipe missing recipe actor"))?
                .as_int::<i32>()?,
            tags:   hash
                .get("Tags")
                .map(|arr| -> Result<_> {
                    arr.as_array()?
                        .iter()
                        .map(|i| Ok(i.as_int::<i32>()?))
                        .collect()
                })
                .transpose()?,
        })
    }
}

impl From<&SingleRecipe> for Byml {
    fn from(val: &SingleRecipe) -> Byml {
        let mut hash: HashMap<SmartString<LazyCompact>, Byml> = HashMap::default();
        if let Some(actors) = &val.actors {
            hash.insert(
                "Actors".into(),
                actors
                    .iter()
                    .map(|v| {
                        if *v < 0 {
                            Byml::U32(*v as u32)
                        } else {
                            Byml::I32(*v)
                        }
                    })
                    .collect::<Vec<Byml>>()
                    .into(),
            );
        }
        if let Some(hb) = val.hb {
            hash.insert("HB".into(), hb.into());
        };
        hash.insert("Num".into(), val.num.into());
        let recipe: Byml = if val.recipe < 0 {
            Byml::U32(val.recipe as u32)
        } else {
            Byml::I32(val.recipe)
        };
        hash.insert("Recipe".into(), recipe);
        if let Some(tags) = &val.tags {
            hash.insert(
                "Tags".into(),
                tags.iter()
                    .map(|v| {
                        if *v < 0 {
                            Byml::U32(*v as u32)
                        } else {
                            Byml::I32(*v)
                        }
                    })
                    .collect::<Vec<Byml>>()
                    .into(),
            );
        }
        hash.into()
    }
}
