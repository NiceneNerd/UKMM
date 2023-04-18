use anyhow::Context;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;

use crate::{
    cooking::{
        recipe::Recipe,
        single_recipe::SingleRecipe,
        system::System,
    },
    prelude::*,
    util::{bhash, DeleteVec},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct CookData {
    pub recipes: DeleteVec<Recipe>,
    pub single_recipes: DeleteVec<SingleRecipe>,
    pub system: System,
}

impl TryFrom<&Byml> for CookData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            recipes: hash
                .get("Recipes")
                .ok_or(UKError::MissingBymlKey("Cook data missing Recipes"))?
                .as_array()
                .map_err(|_| UKError::WrongBymlType("not an array".into(), "an array"))?
                .iter()
                .map(|r| {
                    Recipe::try_from(r)
                        .context("Failed to parse Recipe")
                        .unwrap()
                })
                .collect(),
            single_recipes: hash
                .get("SingleRecipes")
                .ok_or(UKError::MissingBymlKey("Cook data missing SingleRecipes"))?
                .as_array()
                .map_err(|_| UKError::WrongBymlType("not an array".into(), "an array"))?
                .iter()
                .map(|sr| {
                    SingleRecipe::try_from(sr)
                    .context("Failed to parse SingleRecipe")
                    .unwrap()
                })
                .collect(),
            system: hash
                .get("System")
                .ok_or(UKError::MissingBymlKey("Cook data missing System"))?
                .try_into()
                .context("Failed to parse System")
                .unwrap(),
        })
    }
}

impl From<CookData> for Byml {
    fn from(val: CookData) -> Self {
        bhash!(
            "Recipes" => val.recipes.iter().map(|r| Byml::from(r)).collect(),
            "SingleRecipes" => val.single_recipes.iter().map(|sr| Byml::from(sr)).collect(),
            "System" => val.system.into(),
        )
    }
}

impl Mergeable for CookData {
    fn diff(&self, other: &Self) -> Self {
        Self {
            recipes: self.recipes.diff(&other.recipes),
            single_recipes: self.single_recipes.diff(&other.single_recipes),
            system: self.system.diff(&other.system),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            recipes: self.recipes.merge(&diff.recipes),
            single_recipes: self.single_recipes.merge(&diff.single_recipes),
            system: self.system.merge(&diff.system),
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

#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_cookdata() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Cooking/CookData.sbyml").unwrap()).unwrap(),
        )
        .unwrap()
    }

    fn load_mod_cookdata() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Cooking/CookData.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_cookdata();
        let cookdata = super::CookData::try_from(&byml).unwrap();
        let data = Byml::from(cookdata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
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
