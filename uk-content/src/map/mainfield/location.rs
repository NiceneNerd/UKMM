use crate::{
    prelude::Mergeable,
    util::{DeleteVec, SortedDeleteMap},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

impl Mergeable for Location {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(message, other_entries)| {
                    if let Some(self_entries) = self.0.get(message) {
                        if self_entries != other_entries {
                            Some((message.clone(), self_entries.diff(other_entries), false))
                        } else {
                            None
                        }
                    } else {
                        Some((message.clone(), other_entries.clone(), false))
                    }
                })
                .chain(self.0.keys().filter_map(|message| {
                    (!other.0.contains_key(message))
                        .then(|| (message.clone(), Default::default(), true))
                }))
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        let messages: BTreeSet<&String> = self.0.keys().chain(diff.0.keys()).collect();
        Self(messages.into_iter().map(|message| {
            let (self_entries, diff_entries) = (self.0.get(message), diff.0.get(message));
            if let Some(self_entries) = self_entries && let Some(diff_entries) = diff_entries {
                (message.clone(), self_entries.merge(diff_entries), diff.0.is_delete(message).unwrap_or_default())
            } else {
                (message.clone(), diff_entries.or(self_entries).cloned().unwrap(), diff.0.is_delete(message).unwrap_or_default())
            }
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_location() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Map/MainField/Location.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_location() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Map/MainField/Location.mod.smubin").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_location();
        let location = super::Location::try_from(&byml).unwrap();
        let data = Byml::from(location.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let location2 = super::Location::try_from(&byml2).unwrap();
        assert_eq!(location, location2);
    }

    #[test]
    fn diff() {
        let byml = load_location();
        let location = super::Location::try_from(&byml).unwrap();
        let byml2 = load_mod_location();
        let location2 = super::Location::try_from(&byml2).unwrap();
        let _diff = location.diff(&location2);
    }

    #[test]
    fn merge() {
        let byml = load_location();
        let location = super::Location::try_from(&byml).unwrap();
        let byml2 = load_mod_location();
        let location2 = super::Location::try_from(&byml2).unwrap();
        let diff = location.diff(&location2);
        let merged = location.merge(&diff);
        assert_eq!(merged, location2);
    }
}
