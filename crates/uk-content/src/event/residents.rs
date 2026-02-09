use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    util::{DeleteMap, DeleteSet},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct ResidentEvents(pub DeleteMap<String, DeleteSet<String>>);

impl TryFrom<&Byml> for ResidentEvents {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let arr = byml.as_array()?;
        Ok(Self(arr.iter().try_fold(
            DeleteMap::<String, DeleteSet<String>>::with_capacity(arr.len()),
            |mut events, event| -> Result<_> {
                let event = event.as_map()?;
                let entry_name = event
                    .get("entry")
                    .ok_or(UKError::MissingBymlKey(
                        "Resident events entry missing entry name",
                    ))?
                    .as_string()?
                    .clone();
                let file_name = event
                    .get("file")
                    .ok_or(UKError::MissingBymlKey(
                        "Resident events entry missing file name",
                    ))?
                    .as_string()?
                    .clone();
                events.get_or_insert_default(entry_name).insert(file_name);
                Ok(events)
            },
        )?))
    }
}

impl From<ResidentEvents> for Byml {
    fn from(val: ResidentEvents) -> Self {
        val.0
            .iter()
            .flat_map(|(entry, files)| {
                files.iter().map(|file| {
                    map!(
                        "entry" => Byml::String(entry.clone()),
                        "file" => Byml::String(file.clone())
                    )
                })
            })
            .collect()
    }
}

impl Mergeable for ResidentEvents {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.merge(&diff.0))
    }
}

impl Resource for ResidentEvents {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("ResidentEvent")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_residents() -> Byml {
        Byml::from_binary(std::fs::read("test/Event/ResidentEvent.byml").unwrap()).unwrap()
    }

    fn load_mod_residents() -> Byml {
        Byml::from_binary(std::fs::read("test/Event/ResidentEvent.mod.byml").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_residents();
        let residents = super::ResidentEvents::try_from(&byml).unwrap();
        let data = Byml::from(residents.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let residents2 = super::ResidentEvents::try_from(&byml2).unwrap();
        assert_eq!(residents, residents2);
    }

    #[test]
    fn diff() {
        let byml = load_residents();
        let residents = super::ResidentEvents::try_from(&byml).unwrap();
        let byml2 = load_mod_residents();
        let residents2 = super::ResidentEvents::try_from(&byml2).unwrap();
        let diff = residents.diff(&residents2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_residents();
        let residents = super::ResidentEvents::try_from(&byml).unwrap();
        let byml2 = load_mod_residents();
        let residents2 = super::ResidentEvents::try_from(&byml2).unwrap();
        let diff = residents.diff(&residents2);
        let merged = residents.merge(&diff);
        assert_eq!(merged, residents2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/TitleBG.pack//Event/ResidentEvent.byml");
        assert!(super::ResidentEvents::path_matches(path));
    }
}
