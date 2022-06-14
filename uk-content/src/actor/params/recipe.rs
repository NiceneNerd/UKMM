use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    Result, UKError,
};
use indexmap::IndexMap;
use join_str::jstr;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Recipe(pub IndexMap<String, u8>);

impl TryFrom<&ParameterIO> for Recipe {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let table = pio
            .object("Normal0")
            .ok_or(UKError::MissingAampKey("Recipe missing table Normal0"))?;
        Ok(Self(
            (1..=table
                .param("ColumnNum")
                .ok_or(UKError::MissingAampKey("Recipe table missing column num"))?
                .as_int()?)
                .map(|i| -> Result<(String, u8)> {
                    Ok((
                        table
                            .param(&format!("ItemName{:02}", i))
                            .ok_or(UKError::MissingAampKey("Recipe missing item name"))?
                            .as_string()?
                            .to_owned(),
                        table
                            .param(&format!("ItemNum{:02}", i))
                            .ok_or(UKError::MissingAampKey("Recipe missing item count"))?
                            .as_int()? as u8,
                    ))
                })
                .collect::<Result<IndexMap<_, _>>>()?,
        ))
    }
}

impl From<Recipe> for ParameterIO {
    fn from(val: Recipe) -> Self {
        Self {
            objects: [
                (
                    "Header",
                    [
                        ("TableNum", Parameter::Int(1)),
                        ("Table01", Parameter::String64("Normal0".to_owned())),
                    ]
                    .into_iter()
                    .collect(),
                ),
                (
                    "Normal0",
                    [("ColumnNum".to_owned(), Parameter::Int(val.0.len() as i32))]
                        .into_iter()
                        .chain(
                            val.0
                                .into_iter()
                                .filter(|(_, count)| *count > 0)
                                .enumerate()
                                .flat_map(|(i, (name, count))| {
                                    [
                                        (
                                            format!("ItemName{:02}", i + 1),
                                            Parameter::String64(name),
                                        ),
                                        (
                                            format!("ItemNum{:02}", i + 1),
                                            Parameter::Int(count as i32),
                                        ),
                                    ]
                                }),
                        )
                        .collect(),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        }
    }
}

impl Mergeable for Recipe {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(name, count)| {
                    if self.0.get(name.as_str()) != Some(count) {
                        Some((name.clone(), *count))
                    } else {
                        None
                    }
                })
                .chain(self.0.iter().filter_map(|(name, _)| {
                    if other.0.contains_key(name.as_str()) {
                        None
                    } else {
                        Some((name.clone(), 0))
                    }
                }))
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(
            self.0
                .iter()
                .chain(diff.0.iter())
                .collect::<IndexMap<_, _>>()
                .into_iter()
                .filter_map(|(name, count)| (*count > 0).then(|| (name.clone(), *count)))
                .collect(),
        )
    }
}

impl InfoSource for Recipe {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        info.insert("normal0StuffNum".to_owned(), Byml::Int(self.0.len() as i32));
        for (i, (name, num)) in self.0.iter().enumerate() {
            info.insert(
                format!("normal0ItemName{:02}", i + 1),
                Byml::String(name.clone()),
            );
            info.insert(
                format!("normal0ItemNum{:02}", i + 1),
                Byml::Int(*num as i32),
            );
        }
        Ok(())
    }
}

impl ParameterResource for Recipe {
    fn path(name: &str) -> String {
        jstr!("Actor/Recipe/{name}.brecipe")
    }
}

impl Resource for Recipe {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }
}

#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Armor_151_Upper");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(recipe.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let recipe2 = super::Recipe::try_from(&pio2).unwrap();
        assert_eq!(recipe, recipe2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Armor_151_Upper");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Armor_151_Upper");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe2 = super::Recipe::try_from(&pio2).unwrap();
        let diff = recipe.diff(&recipe2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Armor_151_Upper");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Armor_151_Upper");
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe2 = super::Recipe::try_from(&pio2).unwrap();
        let diff = recipe.diff(&recipe2);
        let merged = recipe.merge(&diff);
        assert_eq!(recipe2, merged);
    }

    #[test]
    fn info() {
        let actor = crate::tests::test_mod_actorpack("Armor_151_Upper");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::new();
        recipe.update_info(&mut info).unwrap();
        assert_eq!(
            info["normal0StuffNum"].as_int().unwrap(),
            recipe.0.len() as i32
        );
        for (i, (name, num)) in recipe.0.iter().enumerate() {
            assert_eq!(
                info[&format!("normal0ItemName{:02}", i + 1)]
                    .as_string()
                    .unwrap(),
                name
            );
            assert_eq!(
                info[&format!("normal0ItemNum{:02}", i + 1)]
                    .as_int()
                    .unwrap(),
                *num as i32
            );
        }
    }
}
