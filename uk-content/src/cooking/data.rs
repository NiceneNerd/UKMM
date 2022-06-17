use crate::{
    prelude::*,
    util::{self, DeleteVec},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CookData {
    pub recipes: DeleteVec<Byml>,
    pub single_recipes: DeleteVec<Byml>,
    pub system: Byml,
}

impl TryFrom<&Byml> for CookData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            recipes: hash
                .get("Recipes")
                .ok_or(UKError::MissingBymlKey("Cook data missing Recipes"))?
                .as_array()?
                .iter()
                .cloned()
                .collect(),
            single_recipes: hash
                .get("SingleRecipes")
                .ok_or(UKError::MissingBymlKey("Cook data missing SingleRecipes"))?
                .as_array()?
                .iter()
                .cloned()
                .collect(),
            system: hash
                .get("System")
                .ok_or(UKError::MissingBymlKey("Cook data missing System"))?
                .clone(),
        })
    }
}

impl From<CookData> for Byml {
    fn from(val: CookData) -> Self {
        [
            ("Recipes", val.recipes.into_iter().collect()),
            ("SingleRecipes", val.single_recipes.into_iter().collect()),
            ("System", val.system),
        ]
        .into_iter()
        .collect()
    }
}

impl Mergeable for CookData {
    fn diff(&self, other: &Self) -> Self {
        Self {
            recipes: self.recipes.diff(&other.recipes),
            single_recipes: self.single_recipes.diff(&other.single_recipes),
            system: util::diff_byml_shallow(&self.system, &other.system),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            recipes: self.recipes.merge(&diff.recipes),
            single_recipes: self.single_recipes.merge(&diff.single_recipes),
            system: util::merge_byml_shallow(&self.system, &diff.system),
        }
    }
}

impl Resource for CookData {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("CookData")
    }
}

single_path!(CookData, "Pack/Bootuo.pack//Cooking/CookData.sbyml");

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_cookdata() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Cooking/CookData.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_cookdata() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Cooking/CookData.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_cookdata();
        let cookdata = super::CookData::try_from(&byml).unwrap();
        let data = Byml::from(cookdata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let cookdata2 = super::CookData::try_from(&byml2).unwrap();
        assert_eq!(cookdata, cookdata2);
    }

    #[test]
    fn diff() {
        let byml = load_cookdata();
        let cookdata = super::CookData::try_from(&byml).unwrap();
        let byml2 = load_mod_cookdata();
        let cookdata2 = super::CookData::try_from(&byml2).unwrap();
        let _diff = cookdata.diff(&cookdata2);
    }

    #[test]
    fn merge() {
        let byml = load_cookdata();
        let cookdata = super::CookData::try_from(&byml).unwrap();
        let byml2 = load_mod_cookdata();
        let cookdata2 = super::CookData::try_from(&byml2).unwrap();
        let diff = cookdata.diff(&cookdata2);
        let merged = cookdata.merge(&diff);
        assert_eq!(merged, cookdata2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//Cooking/CookData.sbyml");
        assert!(super::CookData::path_matches(path));
    }
}
