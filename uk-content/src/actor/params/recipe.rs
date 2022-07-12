use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util::DeleteMap,
    Result, UKError,
};
use join_str::jstr;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};

type RecipeTable = DeleteMap<String, u8>;

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Recipe(pub DeleteMap<String, RecipeTable>);

impl TryFrom<&ParameterIO> for Recipe {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let header = pio
            .object("Header")
            .ok_or(UKError::MissingAampKey("Recipe missing header"))?;
        let table_count = header
            .param("TableNum")
            .ok_or(UKError::MissingAampKey("Recipe header missing table count"))?
            .as_int()?;
        let table_names = (0..table_count)
            .map(|i| -> Result<&str> {
                Ok(header
                    .param(format!("Table{:02}", i + 1).as_str())
                    .ok_or_else(|| {
                        UKError::MissingAampKeyD(jstr!(
                            "Recipe header missing table name {&lexical::to_string(i + 1)}"
                        ))
                    })?
                    .as_string()?)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(
            table_names
                .into_iter()
                .map(|name| -> Result<(String, RecipeTable)> {
                    let table = pio.object(name).ok_or_else(|| {
                        UKError::MissingAampKeyD(jstr!("Recipe missing table {&name}"))
                    })?;
                    Ok((
                        name.into(),
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
                                        .into(),
                                    table
                                        .param(&format!("ItemNum{:02}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Recipe missing item count",
                                        ))?
                                        .as_int()? as u8,
                                ))
                            })
                            .collect::<Result<_>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<Recipe> for ParameterIO {
    fn from(val: Recipe) -> Self {
        Self::new()
            .with_object(
                "Header",
                [("TableNum".to_owned(), Parameter::Int(val.0.len() as i32))]
                    .into_iter()
                    .chain(val.0.keys().enumerate().map(|(i, n)| {
                        (
                            format!("Table{:02}", i + 1),
                            Parameter::String64(n.to_owned().into()),
                        )
                    }))
                    .collect(),
            )
            .with_objects(val.0.into_iter().map(|(name, table)| {
                (
                    name,
                    [("ColumnNum".to_owned(), Parameter::Int(table.len() as i32))]
                        .into_iter()
                        .chain(
                            table
                                .into_iter()
                                .filter(|(_, count)| *count > 0)
                                .enumerate()
                                .flat_map(|(i, (name, count))| {
                                    [
                                        (
                                            format!("ItemName{:02}", i + 1),
                                            Parameter::String64(name.into()),
                                        ),
                                        (
                                            format!("ItemNum{:02}", i + 1),
                                            Parameter::Int(count as i32),
                                        ),
                                    ]
                                }),
                        )
                        .collect(),
                )
            }))
    }
}

impl Mergeable for Recipe {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl InfoSource for Recipe {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        if let Some(table) = self.0.get(&String::from("Normal0")) {
            info.insert("normal0StuffNum".to_owned(), Byml::Int(table.len() as i32));
            for (i, (name, num)) in table.iter().enumerate() {
                info.insert(
                    format!("normal0ItemName{:02}", i + 1),
                    Byml::String(name.to_string()),
                );
                info.insert(
                    format!("normal0ItemNum{:02}", i + 1),
                    Byml::Int(*num as i32),
                );
            }
        }
        Ok(())
    }
}

impl ParameterResource for Recipe {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/Recipe/{name}.brecipe")
    }
}

impl Resource for Recipe {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> roead::Bytes {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("brecipe")
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
        let _diff = recipe.diff(&recipe2);
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
        let table = recipe.0.get(&String::from("Normal0")).unwrap();
        assert_eq!(
            info["normal0StuffNum"].as_int().unwrap(),
            table.len() as i32
        );
        for (i, (name, num)) in table.iter().enumerate() {
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

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Armor_151_Upper.sbactorpack//Actor/Recipe/Armor_151_Upper.brecipe",
        );
        assert!(super::Recipe::path_matches(path));
    }
}
