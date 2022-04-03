use crate::{prelude::*, Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
struct DropTable(IndexMap<String, ParameterObject>);

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

    fn try_from(plist: &ParameterIO) -> Result<Self> {
        let header = plist
            .object("Header")
            .ok_or_else(|| UKError::MissingAampKey("Drop table missing header".to_owned()))?;
        let table_count = header
            .param("TableNum")
            .ok_or_else(|| {
                UKError::MissingAampKey("Drop table header missing table count".to_owned())
            })?
            .as_int()? as usize;
        Ok(Self(
            (1..=table_count)
                .into_iter()
                .filter_map(|i| {
                    header
                        .param(&format!("Table{:02}", i))
                        .and_then(|name| name.as_string().ok())
                        .and_then(|name| {
                            plist
                                .object(name)
                                .map(|table| (name.to_owned(), table.clone()))
                        })
                })
                .collect(),
        ))
    }
}

impl Mergeable<ParameterIO> for DropTable {
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

    fn merge(base: &Self, diff: &Self) -> Self {
        Self(
            base.0
                .iter()
                .chain(diff.0.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

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
        let data = drop.clone().into_pio().to_binary();
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
        let diff = drop.diff(&drop2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
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
        let merged = super::DropTable::merge(&drop, &diff);
        assert_eq!(drop2, merged);
    }
}
