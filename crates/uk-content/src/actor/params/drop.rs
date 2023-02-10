use join_str::jstr;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util::{IndexMap, ParameterExt},
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Editable)]
pub struct DropTable(pub IndexMap<String64, ParameterObject>);

impl From<DropTable> for ParameterIO {
    fn from(drop: DropTable) -> Self {
        Self {
            param_root: ParameterList {
                objects: {
                    let mut objs = ParameterObjectMap::default();
                    objs.insert(
                        hash_name("Header"),
                        [("TableNum".into(), Parameter::I32(drop.0.len() as i32))]
                            .into_iter()
                            .chain(drop.0.keys().enumerate().map(|(i, name)| {
                                (
                                    format!("Table{:02}", i + 1),
                                    Parameter::String64(Box::new(*name)),
                                )
                            }))
                            .collect(),
                    );
                    objs.extend(
                        drop.0
                            .into_iter()
                            .map(|(name, table)| (hash_name(&name).into(), table)),
                    );
                    objs
                },
                lists:   Default::default(),
            },
            version:    0,
            data_type:  "xml".into(),
        }
    }
}

impl TryFrom<&ParameterIO> for DropTable {
    type Error = UKError;

    fn try_from(list: &ParameterIO) -> Result<Self> {
        let header = list
            .object("Header")
            .ok_or(UKError::MissingAampKey("Drop table missing header", None))?;
        Ok(Self(
            header
                .iter()
                .filter_map(|(_, name)| {
                    name.as_safe_string().ok().and_then(|name| {
                        list.object(name.as_str())
                            .map(|table| (name, table.clone()))
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
                            Some((*name, table.clone()))
                        } else {
                            None
                        }
                    } else {
                        Some((*name, table.clone()))
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
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        )
    }
}

impl InfoSource for DropTable {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        info.insert(
            "drops".into(),
            self.0
                .iter()
                .map(|(name, table)| -> Result<(std::string::String, Byml)> {
                    Ok((name.to_string(), {
                        let count = table
                            .get("ColumnNum")
                            .ok_or(UKError::MissingAampKey(
                                "Drop table missing column count",
                                None,
                            ))?
                            .as_int()?;
                        (1..=count)
                            .map(|i| -> Result<Byml> {
                                Ok(Byml::String(
                                    table
                                        .get(&format!("ItemName{:02}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Drop table missing item name",
                                            None,
                                        ))?
                                        .as_str()?
                                        .into(),
                                ))
                            })
                            .collect::<Result<_>>()
                    }?))
                })
                .collect::<Result<_>>()?,
        );
        Ok(())
    }
}

impl ParameterResource for DropTable {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/DropTable/{name}.bdrop")
    }
}

impl Resource for DropTable {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bdrop")
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
                .get_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop = super::DropTable::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(drop.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let drop2 = super::DropTable::try_from(&pio2).unwrap();
        assert_eq!(drop, drop2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop = super::DropTable::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
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
                .get_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let drop = super::DropTable::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
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
                .get_data("Actor/DropTable/Enemy_Guardian_A.bdrop")
                .unwrap(),
        )
        .unwrap();
        let drop = super::DropTable::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::default();
        drop.update_info(&mut info).unwrap();
        assert_eq!(
            info["drops"].as_hash().unwrap()["Normal"]
                .as_array()
                .unwrap(),
            vec![
                roead::byml::Byml::String("Item_Enemy_27".into()),
                roead::byml::Byml::String("Item_Enemy_28".into()),
                roead::byml::Byml::String("Item_Enemy_26".into()),
                roead::byml::Byml::String("Item_Enemy_29".into()),
                roead::byml::Byml::String("Item_Enemy_30".into()),
                roead::byml::Byml::String("Item_Enemy_31".into()),
            ]
            .as_slice()
        );
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/DropTable/Enemy_Guardian_A.\
             bdrop",
        );
        assert!(super::DropTable::path_matches(path));
    }
}
