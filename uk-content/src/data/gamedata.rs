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

macro_rules! parse_data {
    ($data_type:ident) => {{
        {
            let data_type: &str = "$data_type";
            Default::default()
        }
    }};
}

impl TryFrom<&Sarc<'_>> for GameData {
    type Error = UKError;

    fn try_from(value: &Sarc) -> Result<Self> {
        Ok(Self {
            s32_data: parse_data!(s32_data),
            ..Default::default()
        })
    }
}
