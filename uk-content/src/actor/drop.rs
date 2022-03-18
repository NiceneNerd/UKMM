use crate::{Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
struct DropTable(pub IndexMap<String, ParameterObject>);

impl TryFrom<ParameterIO> for DropTable {
    type Error = UKError;

    fn try_from(plist: ParameterIO) -> Result<Self> {
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
            (0..table_count)
                .into_iter()
                .filter_map(|i| {
                    header
                        .param(&format!("Table{:2}", i))
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
