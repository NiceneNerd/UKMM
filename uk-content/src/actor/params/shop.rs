use crate::{actor::ParameterResource, prelude::*, Result, UKError};
use indexmap::IndexMap;
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
pub struct ShopItem {
    pub sort: u8,
    pub num: u8,
    pub adjust_price: u8,
    pub look_get_flag: bool,
    pub amount: u8,
    pub delete: bool,
}

impl ShopItem {
    fn with_delete(mut self) -> Self {
        self.delete = true;
        self
    }
}

pub type ShopTable = IndexMap<String, ShopItem>;

fn merge_table(base: &ShopTable, diff: &ShopTable) -> ShopTable {
    base.iter()
        .chain(diff.iter())
        .map(|(name, item)| (name.clone(), *item))
        .collect::<IndexMap<_, _>>()
        .into_iter()
        .filter(|(_, item)| !item.delete)
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ShopData(pub IndexMap<String, Option<ShopTable>>);

impl TryFrom<ParameterIO> for ShopData {
    type Error = UKError;

    fn try_from(pio: ParameterIO) -> Result<Self> {
        pio.try_into()
    }
}

impl TryFrom<&ParameterIO> for ShopData {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let header = pio
            .object("Header")
            .ok_or(UKError::MissingAampKey("Shop data missing header"))?;
        let table_count = header
            .param("TableNum")
            .ok_or(UKError::MissingAampKey("Shop data missing table count"))?
            .as_int()? as usize;
        let tables: Vec<String> = (1..=table_count)
            .filter_map(|i| {
                header
                    .param(&format!("Table{:02}", i))
                    .and_then(|p| p.as_string().ok().map(|s| s.to_owned()))
            })
            .collect();
        let mut shop_tables = IndexMap::with_capacity(table_count);
        for table_name in tables {
            let table_obj = pio.object(&table_name).ok_or_else(|| {
                UKError::MissingAampKeyD(jstr!("Table {&table_name} in shop data missing"))
            })?;
            let column_num = table_obj
                .param("ColumnNum")
                .ok_or(UKError::MissingAampKey(
                    "Shop data table missing column count",
                ))?
                .as_int()? as usize;
            shop_tables.insert(
                table_name,
                Some(
                    (1..=column_num)
                        .map(|i| -> Result<(String, ShopItem)> {
                            let item_name = table_obj
                                .param(&format!("ItemName{:03}", i))
                                .ok_or(UKError::MissingAampKey("Shop table missing item name"))?
                                .as_string()?;
                            Ok((
                                item_name.to_owned(),
                                ShopItem {
                                    sort: table_obj
                                        .param(&format!("ItemSort{:03}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Shop table missing item name",
                                        ))?
                                        .as_int()? as u8,
                                    num: table_obj
                                        .param(&format!("ItemNum{:03}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Shop table missing item num",
                                        ))?
                                        .as_int()? as u8,
                                    adjust_price: table_obj
                                        .param(&format!("ItemAdjustPrice{:03}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Shop table missing adjust price",
                                        ))?
                                        .as_int()?
                                        as u8,
                                    look_get_flag: table_obj
                                        .param(&format!("ItemLookGetFlg{:03}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Shop table missing look get flag",
                                        ))?
                                        .as_bool()?,
                                    amount: table_obj
                                        .param(&format!("ItemAdjustPrice{:03}", i))
                                        .ok_or(UKError::MissingAampKey(
                                            "Shop table missing item amount",
                                        ))?
                                        .as_int()?
                                        as u8,
                                    delete: false,
                                },
                            ))
                        })
                        .collect::<Result<ShopTable>>()?,
                ),
            );
        }
        Ok(Self(shop_tables))
    }
}

impl From<ShopData> for ParameterIO {
    fn from(val: ShopData) -> ParameterIO {
        let mut pio = ParameterIO::new();
        pio.set_object(
            "Header",
            [("TableNum".to_owned(), Parameter::Int(val.0.len() as i32))]
                .into_iter()
                .chain(val.0.keys().enumerate().map(|(i, name)| {
                    (
                        format!("Table{:02}", i + 1),
                        Parameter::String64(name.to_owned()),
                    )
                }))
                .collect(),
        );
        val.0
            .into_iter()
            .filter_map(|(name, table)| table.map(|t| (name, t)))
            .for_each(|(name, table)| {
                pio.set_object(
                    &name,
                    [("ColumnNum".to_owned(), Parameter::Int(table.len() as i32))]
                        .into_iter()
                        .chain(
                            table
                                .into_iter()
                                .filter(|(_, data)| !data.delete)
                                .enumerate()
                                .flat_map(|(i, (name, data))| {
                                    let i = i + 1;
                                    [
                                        (
                                            format!("ItemSort{:03}", i),
                                            Parameter::Int(data.sort as i32),
                                        ),
                                        (format!("ItemName{:03}", i), Parameter::String64(name)),
                                        (
                                            format!("ItemNum{:03}", i),
                                            Parameter::Int(data.num as i32),
                                        ),
                                        (
                                            format!("ItemAdjustPrice{:03}", i),
                                            Parameter::Int(data.adjust_price as i32),
                                        ),
                                        (
                                            format!("ItemLookGetFlg{:03}", i),
                                            Parameter::Bool(data.look_get_flag),
                                        ),
                                        (
                                            format!("ItemAmount{:03}", i),
                                            Parameter::Int(data.amount as i32),
                                        ),
                                    ]
                                }),
                        )
                        .collect(),
                );
            });
        pio
    }
}

impl Mergeable for ShopData {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(name, table)| {
                    if let Some(Some(self_table)) = self.0.get(name.as_str()) {
                        if let Some(other_table) = table {
                            if self_table != other_table {
                                Some((
                                    name.clone(),
                                    Some(
                                        other_table
                                            .iter()
                                            .filter_map(|(item, data)| {
                                                if let Some(self_data) =
                                                    self_table.get(item.as_str()) && self_data == data
                                                {
                                                    None
                                                } else {
                                                    Some((item.clone(), *data))
                                                }
                                            })
                                            .chain(self_table.iter().filter_map(|(item, data)| {
                                                if other_table.contains_key(item.as_str()) {
                                                    None
                                                } else {
                                                    Some((item.clone(), (*data).with_delete()))
                                                }
                                            }))
                                            .collect(),
                                    ),
                                ))
                            } else {
                                None
                            }
                        } else {
                            Some((name.clone(), None))
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
                .filter_map(|(base_name, base_table)| {
                    if let Some(base_table) = base_table {
                        if let Some(Some(diff_table)) = diff.0.get(base_name.as_str()) {
                            Some((base_name.clone(), Some(merge_table(base_table, diff_table))))
                        } else {
                            None
                        }
                    } else {
                        Some((
                            base_name.clone(),
                            diff.0.get(base_name.as_str()).cloned().flatten(),
                        ))
                    }
                })
                .chain(diff.0.iter().filter_map(|(diff_name, diff_table)| {
                    (!self.0.contains_key(diff_name.as_str()))
                        .then(|| (diff_name.clone(), diff_table.clone()))
                }))
                .collect(),
        )
    }
}

impl ParameterResource for ShopData {
    fn path(name: &str) -> String {
        jstr!("Actor/ShopData/{name}.bshop")
    }
}

impl Resource for ShopData {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bshop")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ShopData/Npc_TripMaster_00.bshop")
                .unwrap(),
        )
        .unwrap();
        let shop = super::ShopData::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(shop.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let shop2 = super::ShopData::try_from(&pio2).unwrap();
        assert_eq!(shop, shop2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ShopData/Npc_TripMaster_00.bshop")
                .unwrap(),
        )
        .unwrap();
        let shop = super::ShopData::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ShopData/Npc_TripMaster_00.bshop")
                .unwrap(),
        )
        .unwrap();
        let shop2 = super::ShopData::try_from(&pio2).unwrap();
        let diff = shop.diff(&shop2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ShopData/Npc_TripMaster_00.bshop")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let shop = super::ShopData::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ShopData/Npc_TripMaster_00.bshop")
                .unwrap(),
        )
        .unwrap();
        let shop2 = super::ShopData::try_from(&pio2).unwrap();
        let diff = shop.diff(&shop2);
        let merged = shop.merge(&diff);
        assert_eq!(shop2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/ShopData/Npc_TripMaster_00.bshop",
        );
        assert!(super::ShopData::path_matches(path));
    }
}
