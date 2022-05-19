use crate::{util::SortedDeleteMap, Result, UKError};
use roead::{
    byml::Byml,
    sarc::{Sarc, SarcWriter},
};
use serde::{Deserialize, Serialize};

type GameDataGroup = SortedDeleteMap<String, Byml>;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GameData {
    pub bool_array_data: GameDataGroup,
    pub bool_data: GameDataGroup,
    pub f32_array_data: GameDataGroup,
    pub f32_data: GameDataGroup,
    pub revival_bool_data: GameDataGroup,
    pub revival_s32_data: GameDataGroup,
    pub s32_array_data: GameDataGroup,
    pub s32_data: GameDataGroup,
    pub string256_array_data: GameDataGroup,
    pub string256_data: GameDataGroup,
    pub string32_data: GameDataGroup,
    pub string64_array_data: GameDataGroup,
    pub string64_data: GameDataGroup,
    pub vector2f_array_data: GameDataGroup,
    pub vector2f_data: GameDataGroup,
    pub vector3f_array_data: GameDataGroup,
    pub vector3f_data: GameDataGroup,
    pub vector4f_data: GameDataGroup,
}

fn parse_data(sarc: &Sarc, data_type: &str) -> Result<GameDataGroup> {
    Ok(sarc
        .files()
        .filter_map(|f| -> Option<Result<GameDataGroup>> {
            f.name()
                .unwrap()
                .starts_with(data_type)
                .then(|| -> Result<GameDataGroup> {
                    match Byml::from_binary(f.data()) {
                        Ok(Byml::Hash(hash)) => {
                            let type_data = hash
                                .get(data_type)
                                .ok_or_else(|| {
                                    UKError::MissingBymlKeyD(format!(
                                        "bgdata file missing {}",
                                        data_type
                                    ))
                                })?
                                .as_array()?;
                            type_data
                                .iter()
                                .map(|item| -> Result<(String, Byml)> {
                                    Ok((
                                        item.as_hash()?
                                            .get("DataName")
                                            .ok_or(UKError::MissingBymlKey(
                                                "bgdata file entry missing DataName",
                                            ))?
                                            .as_string()?
                                            .to_owned(),
                                        item.clone(),
                                    ))
                                })
                                .collect::<Result<_>>()
                        }
                        _ => Err(UKError::Other("Invalid bgdata file")),
                    }
                })
        })
        .collect::<Result<Vec<GameDataGroup>>>()?
        .into_iter()
        .flatten()
        .collect())
}

impl TryFrom<&Sarc<'_>> for GameData {
    type Error = UKError;

    fn try_from(value: &Sarc) -> Result<Self> {
        Ok(Self {
            bool_array_data: parse_data(value, "bool_array_data")?,
            bool_data: parse_data(value, "bool_data")?,
            f32_array_data: parse_data(value, "f32_array_data")?,
            f32_data: parse_data(value, "f32_data")?,
            revival_bool_data: parse_data(value, "revival_bool_data")?,
            revival_s32_data: parse_data(value, "revival_s32_data")?,
            s32_array_data: parse_data(value, "s32_array_data")?,
            s32_data: parse_data(value, "s32_data")?,
            string256_array_data: parse_data(value, "string256_array_data")?,
            string256_data: parse_data(value, "string256_data")?,
            string32_data: parse_data(value, "string32_data")?,
            string64_array_data: parse_data(value, "string64_array_data")?,
            string64_data: parse_data(value, "string64_data")?,
            vector2f_array_data: parse_data(value, "vector2f_array_data")?,
            vector2f_data: parse_data(value, "vector2f_data")?,
            vector3f_array_data: parse_data(value, "vector3f_array_data")?,
            vector3f_data: parse_data(value, "vector3f_data")?,
            vector4f_data: parse_data(value, "vector4f_data")?,
        })
    }
}
