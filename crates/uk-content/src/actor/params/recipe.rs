use anyhow::Context;
use join_str::jstr;
#[cfg(feature = "ui")]
use nk_ui_derive::Editable;
use nk_util::OptionResultExt;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};

use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util::{DeleteMap, IteratorExt},
    Result, UKError,
};

type RecipeTable = DeleteMap<String64, u8>;

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct Recipe(pub DeleteMap<String64, RecipeTable>);

impl TryFrom<&ParameterIO> for Recipe {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let header = pio
            .object("Header")
            .ok_or(UKError::MissingAampKey("Recipe missing header", None))?;
        let table_count = header
            .get("TableNum")
            .ok_or(UKError::MissingAampKey(
                "Recipe header missing table count",
                None,
            ))?
            .as_int()?;
        let table_names = (0..table_count)
            .named_enumerate("Table")
            .with_padding::<2>()
            .with_zero_index(false)
            .map(|(index, _)| -> Result<String64> {
                Ok(header
                    .get(&index)
                    .ok_or_else(|| {
                        UKError::MissingAampKeyD(jstr!("Recipe header missing table name {&index}"))
                    })?
                    .as_safe_string()?)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(
            table_names
                .into_iter()
                .map(|name| -> Result<(String64, RecipeTable)> {
                    let table = pio.object(name.as_str()).ok_or_else(|| {
                        UKError::MissingAampKeyD(jstr!("Recipe missing table {&name}"))
                    })?;
                    let items_count = table
                        .get("ColumnNum")
                        .ok_or(UKError::MissingAampKey(
                            "Recipe table missing column count",
                            None,
                        ))?
                        .as_int()?;
                    let process = |count| -> Result<_> {
                        (1..=count)
                            .named_enumerate("ItemNum")
                            .with_padding::<2>()
                            .with_zero_index(false)
                            .named_enumerate("ItemName")
                            .with_padding::<2>()
                            .with_zero_index(false)
                            .map(|(name, (num, _))| -> Result<(String64, u8)> {
                                Ok((
                                    table
                                        .get(&name)
                                        .ok_or(UKError::MissingAampKey(
                                            "Recipe missing item name",
                                            None,
                                        ))?
                                        .as_safe_string()?,
                                    table
                                        .get(&num)
                                        .ok_or(UKError::MissingAampKey(
                                            "Recipe missing item count",
                                            None,
                                        ))?
                                        .as_int()?,
                                ))
                            })
                            .collect::<Result<_>>()
                            .or_else(|_| {
                                (1..=count)
                                    .named_enumerate("ItemNum")
                                    .with_padding::<3>()
                                    .with_zero_index(false)
                                    .named_enumerate("ItemName")
                                    .with_padding::<3>()
                                    .with_zero_index(false)
                                    .map(|(name, (num, _))| -> Result<(String64, u8)> {
                                        Ok((
                                            table
                                                .get(&name)
                                                .ok_or(UKError::MissingAampKey(
                                                    "Recipe missing item name",
                                                    None,
                                                ))?
                                                .as_safe_string()?,
                                            table
                                                .get(&num)
                                                .ok_or(UKError::MissingAampKey(
                                                    "Recipe missing item count",
                                                    None,
                                                ))?
                                                .as_int()?,
                                        ))
                                    })
                                    .collect::<Result<_>>()
                            })
                    };
                    Ok((
                        name,
                        process(items_count).or_else(|e| {
                            let items_count = (table.0.len() - 1) / 2;
                            process(items_count).context(e)
                        })?,
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
                [("TableNum".into(), Parameter::I32(val.0.len() as i32))]
                    .into_iter()
                    .chain(
                        val.0
                            .keys()
                            .named_enumerate("Table")
                            .with_padding::<2>()
                            .with_zero_index(false)
                            .map(|(index, n)| (index, Parameter::String64(Box::new(*n)))),
                    )
                    .collect(),
            )
            .with_objects(val.0.into_iter().map(|(name, table)| {
                (
                    name,
                    [("ColumnNum".into(), Parameter::I32(table.len() as i32))]
                        .into_iter()
                        .chain(
                            table
                                .into_iter()
                                .filter(|(_, count)| *count > 0)
                                .named_enumerate("ItemNum")
                                .with_padding::<2>()
                                .with_zero_index(false)
                                .named_enumerate("ItemName")
                                .with_padding::<2>()
                                .with_zero_index(false)
                                .flat_map(|(name_idx, (num_idx, (name, count)))| {
                                    [
                                        (name_idx, Parameter::String64(Box::new(name))),
                                        (num_idx, Parameter::I32(count as i32)),
                                    ]
                                }),
                        )
                        .collect(),
                )
            }))
    }
}

impl MergeableImpl for Recipe {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.deep_diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.deep_merge(&diff.0))
    }
}

impl InfoSource for Recipe {
    fn update_info(&self, info: &mut roead::byml::Map) -> crate::Result<()> {
        if let Some(table) = self.0.get(String64::from("Normal0")) {
            info.insert("normal0StuffNum".into(), Byml::I32(table.len() as i32));
            for (name_idx, (num_idx, (name, num))) in table
                .iter()
                .named_enumerate("normal0ItemNum")
                .with_padding::<2>()
                .with_zero_index(false)
                .named_enumerate("normal0ItemName")
                .with_padding::<2>()
                .with_zero_index(false)
            {
                info.insert(name_idx.into(), Byml::String(name.as_str().into()));
                info.insert(num_idx.into(), Byml::I32(*num as i32));
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

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"brecipe")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Armor_151_Upper");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(recipe.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let recipe2 = super::Recipe::try_from(&pio2).unwrap();
        assert_eq!(recipe, recipe2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Armor_151_Upper");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Armor_151_Upper");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/Recipe/Armor_151_Upper.brecipe")
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
                .get_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Armor_151_Upper");
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/Recipe/Armor_151_Upper.brecipe")
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
                .get_data("Actor/Recipe/Armor_151_Upper.brecipe")
                .unwrap(),
        )
        .unwrap();
        let recipe = super::Recipe::try_from(&pio).unwrap();
        let mut info = roead::byml::Map::default();
        recipe.update_info(&mut info).unwrap();
        let table = recipe.0.get(String64::from("Normal0")).unwrap();
        assert_eq!(
            info["normal0StuffNum"].as_i32().unwrap(),
            table.len() as i32
        );
        for (i, (name, num)) in table.iter().enumerate() {
            assert_eq!(
                info[format!("normal0ItemName{:02}", i + 1).as_str()]
                    .as_string()
                    .unwrap(),
                name.as_str()
            );
            assert_eq!(
                info[format!("normal0ItemNum{:02}", i + 1).as_str()]
                    .as_i32()
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
