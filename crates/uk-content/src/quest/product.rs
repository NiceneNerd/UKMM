use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::DeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]

pub struct QuestProduct(pub DeleteMap<String, Byml>);

impl TryFrom<&Byml> for QuestProduct {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .iter()
                .map(|quest| -> Result<(String, Byml)> {
                    Ok((
                        quest
                            .as_map()?
                            .get("Name")
                            .ok_or(UKError::MissingBymlKey("Quest entry missing name"))?
                            .as_string()?
                            .clone(),
                        quest.clone(),
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<QuestProduct> for Byml {
    fn from(val: QuestProduct) -> Self {
        Self::Array(val.0.into_iter().map(|(_, v)| v).collect())
    }
}

impl Mergeable for QuestProduct {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.merge(&diff.0))
    }
}

impl Resource for QuestProduct {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("QuestProduct")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_quests() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Quest/QuestProduct.sbquestpack").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_quests() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Quest/QuestProduct.mod.sbquestpack").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_quests();
        let quests = super::QuestProduct::try_from(&byml).unwrap();
        let data = Byml::from(quests.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let quests2 = super::QuestProduct::try_from(&byml2).unwrap();
        assert_eq!(quests, quests2);
    }

    #[test]
    fn diff() {
        let byml = load_quests();
        let quests = super::QuestProduct::try_from(&byml).unwrap();
        let byml2 = load_mod_quests();
        let quests2 = super::QuestProduct::try_from(&byml2).unwrap();
        let diff = quests.diff(&quests2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_quests();
        let quests = super::QuestProduct::try_from(&byml).unwrap();
        let byml2 = load_mod_quests();
        let quests2 = super::QuestProduct::try_from(&byml2).unwrap();
        let diff = quests.diff(&quests2);
        let merged = quests.merge(&diff);
        assert_eq!(merged, quests2);
    }

    #[test]
    fn identify() {
        let path =
            std::path::Path::new("content/Pack/TitleBG.pack//Quest/QuestProduct.sbquestpack");
        assert!(super::QuestProduct::path_matches(path));
    }
}
