use crate::{prelude::*, util::DeleteMap, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, Editable)]
pub struct ResidentEvents(pub DeleteMap<String, String>);

impl TryFrom<&Byml> for ResidentEvents {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .iter()
                .map(|event| -> Result<(String, String)> {
                    let event = event.as_hash()?;
                    Ok((
                        event
                            .get("entry")
                            .ok_or(UKError::MissingBymlKey(
                                "Resident events entry missing entry name",
                            ))?
                            .as_string()?
                            .clone(),
                        event
                            .get("file")
                            .ok_or(UKError::MissingBymlKey(
                                "Resident events entry missing file name",
                            ))?
                            .as_string()?
                            .clone(),
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<ResidentEvents> for Byml {
    fn from(val: ResidentEvents) -> Self {
        val.0
            .into_iter()
            .map(|(entry, file)| -> Byml {
                [("entry", Byml::String(entry)), ("file", Byml::String(file))]
                    .into_iter()
                    .collect()
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
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("ResidentEvent")
    }
}

single_path!(
    ResidentEvents,
    "Pack/TitleBG.pack//Event/ResidentEvent.byml"
);

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_residents() -> Byml {
        Byml::from_binary(&std::fs::read("test/Event/ResidentEvent.byml").unwrap()).unwrap()
    }

    fn load_mod_residents() -> Byml {
        Byml::from_binary(&std::fs::read("test/Event/ResidentEvent.mod.byml").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_residents();
        let residents = super::ResidentEvents::try_from(&byml).unwrap();
        let data = Byml::from(residents.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
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
