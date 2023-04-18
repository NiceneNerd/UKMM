use anyhow::Context;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use smartstring::{SmartString, LazyCompact};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;

use crate::{
    util::{DeleteVec, HashMap},
    Result, UKError
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct SingleRecipe {
    pub actors: Option<DeleteVec<i32>>,
    pub hb: Option<i32>,
    pub num: i32,
    pub recipe: i32,
    pub tags: Option<DeleteVec<i32>>,
}

impl TryFrom<&Byml> for SingleRecipe {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            actors: hash
                .get("Actors")
                .map_or(
                    None,
                    |arr| {
                        Some(arr.as_array()
                            .map_err(|_e| UKError::WrongBymlType(
                                "not an array".into(),
                                "an array"
                            ))
                            .unwrap()
                            .iter()
                            .map(|i| {
                                i.as_i32()
                                    .map_err(|_e| UKError::WrongBymlType(
                                        "not an integer".into(),
                                        "an integer"
                                    ))
                                    .unwrap()
                            })
                            .collect::<DeleteVec<i32>>()
                        )
                    }
                ),
            hb: hash
                .get("HB")
                .map_or(
                    None,
                    |i| Some(i.as_i32().context("HB not int").unwrap())
                ),
            num: hash
                .get("Num")
                .ok_or(UKError::MissingBymlKey("Recipe missing num"))?
                .as_i32()
                .map_err(|_e| {
                    UKError::WrongBymlType("not an integer".into(), "an integer")
                })
                .unwrap(),
            recipe: hash
                .get("Recipe")
                .ok_or(UKError::MissingBymlKey("Recipe missing recipe actor"))?
                .as_i32()
                .map_err(|_e| {
                    UKError::WrongBymlType("not an integer".into(), "an integer")
                })
                .unwrap(),
            tags: hash
                .get("Tags")
                .map_or(
                    None,
                    |arr| {
                        Some(arr.as_array()
                            .map_err(|_e| UKError::WrongBymlType(
                                "not an array".into(),
                                "an array"
                            ))
                            .unwrap()
                            .iter()
                            .map(|i| {
                                i.as_i32()
                                    .map_err(|_e| UKError::WrongBymlType(
                                        "not an integer".into(),
                                        "an integer"
                                    ))
                                    .unwrap()
                            })
                            .collect::<DeleteVec<i32>>()
                        )
                    }
                ),
        })
    }
}

impl From<&SingleRecipe> for Byml {
    fn from(val: &SingleRecipe) -> Byml {
        let mut hash: HashMap<SmartString<LazyCompact>, Byml> = HashMap::default();
        if let Some(actors) = &val.actors {
            hash.insert(
                "Actors".into(),
                actors.iter()
                    .map(|v| {
                        if *v < 0 {
                            Byml::U32(*v as u32)
                        }
                        else {
                            Byml::I32(*v)
                        }
                    })
                    .collect::<Vec<Byml>>()
                    .into()
            );
        }
        if let Some(hb) = val.hb {
            hash.insert("HB".into(), hb.into());
        };
        hash.insert("Recipe".into(), val.recipe.into());
        if let Some(tags) = &val.tags {
            hash.insert(
                "Tags".into(),
                tags.iter()
                    .map(|v| {
                        if *v < 0 {
                            Byml::U32(*v as u32)
                        }
                        else {
                            Byml::I32(*v)
                        }
                    })
                    .collect::<Vec<Byml>>()
                    .into()
            );
        }
        hash.into()
    }
}
