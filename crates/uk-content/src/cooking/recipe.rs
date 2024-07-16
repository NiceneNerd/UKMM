use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use smartstring::{LazyCompact, SmartString};


use crate::{
    util::{DeleteVec, HashMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct Recipe {
    pub actors: Option<DeleteVec<DeleteVec<i32>>>,
    pub hb:     Option<i32>,
    pub recipe: i32,
    pub tags:   Option<DeleteVec<DeleteVec<i32>>>,
}

impl TryFrom<&Byml> for Recipe {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        Ok(Self {
            actors: hash
                .get("Actors")
                .map(|arr| -> Result<DeleteVec<DeleteVec<i32>>> {
                    arr.as_array()?
                        .iter()
                        .map(|arr2| -> Result<DeleteVec<i32>> {
                            arr2.as_array()?
                                .iter()
                                .map(|i| Ok(i.as_int::<i32>()?))
                                .collect()
                        })
                        .collect()
                })
                .transpose()?,
            hb:     hash.get("HB").map(|i| i.as_int()).transpose()?,
            recipe: hash
                .get("Recipe")
                .ok_or(UKError::MissingBymlKey("Recipe missing recipe actor"))?
                .as_int::<i32>()?,
            tags:   hash
                .get("Tags")
                .map(|arr| -> Result<DeleteVec<DeleteVec<i32>>> {
                    arr.as_array()?
                        .iter()
                        .map(|arr2| -> Result<DeleteVec<i32>> {
                            arr2.as_array()?
                                .iter()
                                .map(|i| Ok(i.as_int::<i32>()?))
                                .collect()
                        })
                        .collect()
                })
                .transpose()?,
        })
    }
}

impl From<&Recipe> for Byml {
    fn from(val: &Recipe) -> Byml {
        let mut hash: HashMap<SmartString<LazyCompact>, Byml> = HashMap::default();
        if let Some(actors) = &val.actors {
            hash.insert(
                "Actors".into(),
                actors
                    .iter()
                    .map(|v| {
                        v.iter()
                            .map(|v2| {
                                if *v2 < 0 {
                                    Byml::U32(*v2 as u32)
                                } else {
                                    Byml::I32(*v2)
                                }
                            })
                            .collect::<Vec<Byml>>()
                            .into()
                    })
                    .collect::<Vec<Byml>>()
                    .into(),
            );
        }
        if let Some(hb) = val.hb {
            hash.insert("HB".into(), hb.into());
        };
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
                        v.iter()
                            .map(|v2| {
                                if *v2 < 0 {
                                    Byml::U32(*v2 as u32)
                                } else {
                                    Byml::I32(*v2)
                                }
                            })
                            .collect::<Vec<Byml>>()
                            .into()
                    })
                    .collect::<Vec<Byml>>()
                    .into(),
            );
        }
        hash.into()
    }
}
