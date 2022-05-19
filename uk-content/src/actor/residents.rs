use crate::{
    prelude::Mergeable,
    util::{self, DeleteMap},
    Result, UKError,
};
use roead::byml::{Byml, Hash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ResidentActorData {
    pub only_res: bool,
    pub scale: Option<Byml>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ResidentActors(pub DeleteMap<String, ResidentActorData>);

impl TryFrom<&Byml> for ResidentActors {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let actors = byml.as_array()?;
        Ok(Self(
            actors
                .iter()
                .map(|actor| -> Result<(String, ResidentActorData)> {
                    actor.as_hash().map_err(UKError::from).and_then(
                        |actor| -> Result<(String, ResidentActorData)> {
                            Ok((
                                actor
                                    .get("name")
                                    .ok_or(UKError::MissingBymlKey(
                                        "Resident actors entry missing name",
                                    ))?
                                    .as_string()?
                                    .to_owned(),
                                ResidentActorData {
                                    only_res: actor
                                        .get("only_res")
                                        .ok_or(UKError::MissingBymlKey(
                                            "Resident actors entry missing only_res",
                                        ))?
                                        .as_bool()?
                                        .to_owned(),
                                    scale: actor.get("scale").cloned(),
                                },
                            ))
                        },
                    )
                })
                .collect::<Result<_>>()?,
        ))
    }
}
