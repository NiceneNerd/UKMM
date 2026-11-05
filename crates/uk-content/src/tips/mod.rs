use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use uk_content_derive::BymlData;

use crate::{prelude::*, util::SortedDeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, BymlData)]

pub struct TipData {
    #[name = "ConditionEntry"]
    condition_entry: String,
    #[name = "ConditionFile"]
    condition_file: String,
    #[name = "Interval"]
    interval: String,
    #[name = "IntervalOption"]
    interval_option: i32,
    #[name = "MessageId"]
    message_id: String,
    #[name = "Priority"]
    priority: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct Tips(pub SortedDeleteMap<String, TipData>);

impl TryFrom<&Byml> for Tips {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .iter()
                .map(|entry| -> Result<(String, TipData)> {
                    let hash = entry.as_map()?;
                    Ok((
                        hash.get("MessageId")
                            .ok_or(UKError::MissingBymlKey("Tips file entry missing MessageId"))?
                            .as_string()?
                            .clone(),
                        entry.try_into()?,
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<Tips> for Byml {
    fn from(val: Tips) -> Self {
        val.0
            .into_iter()
            .map(|(_, v)| -> Byml { v.into() })
            .collect()
    }
}

impl Mergeable for Tips {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.merge(&diff.0))
    }
}

impl Resource for Tips {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("Tips") && name.ends_with("byml"))
            .unwrap_or(false)
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_tips() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Tips/TipsWorld.sbyml").unwrap()).unwrap(),
        )
        .unwrap()
    }

    fn load_mod_tips() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Tips/TipsWorld.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_tips();
        let tips = super::Tips::try_from(&byml).unwrap();
        let data = Byml::from(tips.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let tips2 = super::Tips::try_from(&byml2).unwrap();
        assert_eq!(tips, tips2);
    }

    #[test]
    fn diff() {
        let byml = load_tips();
        let tips = super::Tips::try_from(&byml).unwrap();
        let byml2 = load_mod_tips();
        let tips2 = super::Tips::try_from(&byml2).unwrap();
        let _diff = tips.diff(&tips2);
    }

    #[test]
    fn merge() {
        let byml = load_tips();
        let tips = super::Tips::try_from(&byml).unwrap();
        let byml2 = load_mod_tips();
        let tips2 = super::Tips::try_from(&byml2).unwrap();
        let diff = tips.diff(&tips2);
        let merged = tips.merge(&diff);
        assert_eq!(merged, tips2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//Tips/TipsWorld.sbyml");
        assert!(super::Tips::path_matches(path));
    }
}
