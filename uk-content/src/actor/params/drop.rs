use crate::{actor::InfoSource, prelude::*, Result, UKError};
use indexmap::IndexMap;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
struct DropTable(pub IndexMap<String, ParameterObject>);

impl From<DropTable> for ParameterIO {
    fn from(drop: DropTable) -> Self {
        Self {
            objects: ParameterObjectMap({
                let mut objs: IndexMap<u32, ParameterObject> = IndexMap::new();
                objs.insert(
                    hash_name("Header"),
                    [("TableNum".to_owned(), Parameter::Int(drop.0.len() as i32))]
                        .into_iter()
                        .chain(drop.0.keys().enumerate().map(|(i, name)| {
                            (
                                format!("Table{:02}", i + 1),
                                Parameter::StringRef(name.to_owned()),
                            )
                        }))
                        .collect(),
                );
                objs.extend(
                    drop.0
                        .into_iter()
                        .map(|(name, table)| (hash_name(&name), table)),
                );
                objs
            }),
            ..Default::default()
        }
    }
}

impl TryFrom<&ParameterIO> for DropTable {
    type Error = UKError;

    fn try_from(list: &ParameterIO) -> Result<Self> {
        let header = list
            .object("Header")
            .ok_or(UKError::MissingAampKey("Drop table missing header"))?;
        let table_count = header
            .param("TableNum")
            .ok_or(UKError::MissingAampKey(
                "Drop table header missing table count",
            ))?
            .as_int()? as usize;
        Ok(Self(
            (1..=table_count)
                .into_iter()
                .filter_map(|i| {
                    header
                        .param(&format!("Table{:02}", i))
                        .and_then(|name| name.as_string().ok())
                        .and_then(|name| {
                            list.object(name)
                                .map(|table| (name.to_owned(), table.clone()))
                        })
                })
                .collect(),
        ))
    }
}

impl Mergeable for DropTable {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(name, table)| {
                    if let Some(self_table) = self.0.get(name) {
                        if self_table != table {
                            Some((name.clone(), table.clone()))
                        } else {
                            None
                        }
                    } else {
                        Some((name.clone(), table.clone()))
                    }
                })
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(
            self.0
                .iter()
                .chain(diff.0.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    }
}

impl InfoSource for DropTable {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        info.insert(
            "drops".to_owned(),
            self.0
                .iter()
                .map(|(name, table)| -> Result<(String, Byml)> {
                    Ok((name.clone(), {
                        let count = table
                            .param("ColumnNum")
                            .ok_or(UKError::MissingAampKey("Drop table missing column count"))?
                            .as_int()?;
                        (1..=count)
                            .map(|i| -> Result<Byml> {
                                Ok(table
                                    .param(&format!("ItemName{:02}", i))
                                    .ok_or(UKError::MissingAampKey("Drop table missing item name"))?
                                    .as_string()?
                                    .to_owned()
                                    .into())
                            })
                            .collect::<Result<_>>()
                    }?))
                })
                .collect::<Result<_>>()?,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop = super::DropTable::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(drop.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let drop2 = super::DropTable::try_from(&pio2).unwrap();
        assert_eq!(drop, drop2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop = super::DropTable::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop2 = super::DropTable::try_from(&pio2).unwrap();
        let _diff = drop.diff(&drop2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let drop = super::DropTable::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop2 = super::DropTable::try_from(&pio2).unwrap();
        let diff = drop.diff(&drop2);
        let merged = drop.merge(&diff);
        assert_eq!(drop2, merged);
    }

    #[test]
    fn info() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop = super::DropTable::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::new();
        drop.update_info(&mut info).unwrap();
        assert_eq!(
            info["drops"].as_hash().unwrap()["Normal"]
                .as_array()
                .unwrap(),
            vec![
                roead::byml::Byml::String("Item_Enemy_27".to_owned()),
                roead::byml::Byml::String("Item_Enemy_28".to_owned()),
                roead::byml::Byml::String("Item_Enemy_26".to_owned()),
                roead::byml::Byml::String("Item_Enemy_29".to_owned()),
                roead::byml::Byml::String("Item_Enemy_30".to_owned()),
                roead::byml::Byml::String("Item_Enemy_31".to_owned()),
            ]
            .as_slice()
        );
    }
}
