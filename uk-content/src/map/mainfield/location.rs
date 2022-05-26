use crate::{
    prelude::Mergeable,
    util::{DeleteVec, SortedDeleteMap},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct LocationEntry {
    pub show_level: usize,
    pub translate: Byml,
    pub ltype: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Location(pub SortedDeleteMap<String, DeleteVec<LocationEntry>>);

impl TryFrom<&Byml> for Location {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(byml.as_array()?.iter().try_fold(
            SortedDeleteMap::new(),
            |mut locs, loc| -> Result<SortedDeleteMap<String, DeleteVec<LocationEntry>>> {
                let loc = loc.as_hash()?;
                let message = loc
                    .get("MessageID")
                    .ok_or(UKError::MissingBymlKey(
                        "Main field location entry missing message ID",
                    ))?
                    .as_string()?
                    .to_owned();
                let pos = LocationEntry {
                    show_level: loc
                        .get("ShowLevel")
                        .ok_or(UKError::MissingBymlKey(
                            "Main field location entry missing ShowLevel",
                        ))?
                        .as_int()? as usize,
                    translate: loc
                        .get("Translate")
                        .ok_or(UKError::MissingBymlKey(
                            "Main field location entry missing Translate",
                        ))?
                        .clone(),
                    ltype: loc
                        .get("Type")
                        .ok_or(UKError::MissingBymlKey(
                            "Main field location entry missing Type",
                        ))?
                        .as_int()? as usize,
                };
                if let Some(message_locs) = locs.get_mut(&message) {
                    message_locs.push(pos);
                } else {
                    locs.insert(message, [pos].into_iter().collect());
                }
                Ok(locs)
            },
        )?))
    }
}

impl From<Location> for Byml {
    fn from(val: Location) -> Self {
        val.0
            .into_iter()
            .flat_map(|(message, entries)| -> Vec<Byml> {
                entries
                    .into_iter()
                    .map(|pos| -> Byml {
                        [
                            ("MessageID", Byml::String(message.clone())),
                            ("ShowLevel", Byml::Int(pos.show_level as i32)),
                            ("Translate", pos.translate),
                            ("Type", Byml::Int(pos.ltype as i32)),
                        ]
                        .into_iter()
                        .collect()
                    })
                    .collect()
            })
            .collect()
    }
}
