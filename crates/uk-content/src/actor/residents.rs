use roead::byml::{Byml, Map};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, resource::SortedDeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct ResidentActorData {
    pub only_res: bool,
    pub scale:    Option<Byml>,
}

impl TryFrom<&Map> for ResidentActorData {
    type Error = UKError;

    fn try_from(val: &Map) -> std::result::Result<Self, Self::Error> {
        Ok(ResidentActorData {
            only_res: val
                .get("only_res")
                .ok_or(UKError::MissingBymlKey(
                    "Resident actors entry missing only_res",
                ))?
                .as_bool()?,
            scale:    val.get("scale").cloned(),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct ResidentActors(pub SortedDeleteMap<String, ResidentActorData>);

impl TryFrom<&Byml> for ResidentActors {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let actors = byml.as_array()?;
        Ok(Self(
            actors
                .iter()
                .map(|actor| -> Result<(String, ResidentActorData)> {
                    actor.as_map().map_err(UKError::from).and_then(
                        |actor| -> Result<(String, ResidentActorData)> {
                            Ok((
                                actor
                                    .get("name")
                                    .ok_or(UKError::MissingBymlKey(
                                        "Resident actors entry missing name",
                                    ))?
                                    .as_string()?
                                    .clone(),
                                actor.try_into()?,
                            ))
                        },
                    )
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<ResidentActors> for Byml {
    fn from(val: ResidentActors) -> Self {
        Byml::Array(
            val.0
                .into_iter()
                .map(|(name, data)| {
                    Byml::Map(
                        [
                            ("name", Some(Byml::String(name))),
                            ("only_res", Some(Byml::Bool(data.only_res))),
                            ("scale", data.scale),
                        ]
                        .into_iter()
                        .filter_map(|(k, v)| v.map(|v| (k.into(), v)))
                        .collect(),
                    )
                })
                .collect(),
        )
    }
}

impl Mergeable for ResidentActors {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.merge(&diff.0))
    }
}

impl Resource for ResidentActors {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_name().and_then(|name| name.to_str()) == Some("ResidentActors.byml")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_residents() -> Byml {
        Byml::from_binary(std::fs::read("test/Actor/ResidentActors.byml").unwrap()).unwrap()
    }

    fn load_mod_residents() -> Byml {
        Byml::from_binary(std::fs::read("test/Actor/ResidentActors.mod.byml").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_residents();
        let residents = super::ResidentActors::try_from(&byml).unwrap();
        let data = Byml::from(residents.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let residents2 = super::ResidentActors::try_from(&byml2).unwrap();
        assert_eq!(residents, residents2);
    }

    #[test]
    fn diff() {
        let byml = load_residents();
        let residents = super::ResidentActors::try_from(&byml).unwrap();
        let byml2 = load_mod_residents();
        let residents2 = super::ResidentActors::try_from(&byml2).unwrap();
        let diff = residents.diff(&residents2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_residents();
        let residents = super::ResidentActors::try_from(&byml).unwrap();
        let byml2 = load_mod_residents();
        let residents2 = super::ResidentActors::try_from(&byml2).unwrap();
        let diff = residents.diff(&residents2);
        let merged = residents.merge(&diff);
        assert_eq!(merged, residents2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/TitleBG.pack//Actor/ResidentActors.byml");
        assert!(super::ResidentActors::path_matches(path));
    }
}
