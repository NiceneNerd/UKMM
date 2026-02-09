use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    util::{DeleteVec, SortedDeleteMap},
    Result, UKError,
};

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
                let loc = loc.as_map()?;
                let message = loc
                    .get("MessageID")
                    .ok_or(UKError::MissingBymlKey(
                        "Main field location entry missing message ID",
                    ))?
                    .as_string()?
                    .clone();
                let pos = LocationEntry {
                    show_level: loc
                        .get("ShowLevel")
                        .ok_or(UKError::MissingBymlKey(
                            "Main field location entry missing ShowLevel",
                        ))?
                        .as_int()?,
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
                        .as_int()?,
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
                        map!(
                            "MessageID" => Byml::String(message.clone()),
                            "ShowLevel" => Byml::I32(pos.show_level as i32),
                            "Translate" => pos.translate,
                            "Type" => Byml::I32(pos.ltype as i32),
                        )
                    })
                    .collect()
            })
            .collect()
    }
}

impl Mergeable for Location {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl Resource for Location {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("Location")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_location() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Map/MainField/Location.smubin").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_location() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Map/MainField/Location.mod.smubin").unwrap(),
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
        let byml2 = Byml::from_binary(data).unwrap();
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

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Map/MainField/Location.smubin");
        assert!(super::Location::path_matches(path));
    }
}
