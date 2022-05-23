use crate::{
    prelude::Mergeable,
    util::{self, DeleteVec, SortedDeleteMap},
    Result, UKError,
};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CookData {
    pub recipes: DeleteVec<Byml>,
    pub single_recipes: DeleteVec<Byml>,
    pub system: Byml,
}

impl TryFrom<&Byml> for CookData {
    type Error = UKError;

    fn try_from(value: &Byml) -> Result<Self, Self::Error> {
        todo!()
    }
}
