use std::collections::HashMap;

use crate::{
    prelude::{Convertible, Mergeable},
    Result, UKError,
};
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChemicalBody {
    pub rigid_c: ParameterObject,
    pub shape: ParameterObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Chemical {
    pub unknown: Option<usize>,
    pub body: HashMap<usize, ChemicalBody>,
}

impl TryFrom<ParameterIO> for Chemical {
    type Error = UKError;
    fn try_from(value: ParameterIO) -> Result<Self> {
        value.try_into()
    }
}

impl TryFrom<&ParameterIO> for Chemical {
    type Error = UKError;
    fn try_from(value: &ParameterIO) -> Result<Self> {
        let shape_num = value
            .list("chemical_root")
            .ok_or_else(|| UKError::MissingAampKey("Chemical missing chemical_root".to_owned()))?
            .object("chemical_header")
            .ok_or_else(|| UKError::MissingAampKey("Chemical missing chemical_header".to_owned()))?
            .param("res_shape_num")
            .ok_or_else(|| UKError::MissingAampKey("Chemical missing shape count".to_owned()))?
            .as_u32()? as usize;
        let chemical_body = value
            .list("chemical_root")
            .unwrap()
            .list("chemical_body")
            .ok_or_else(|| UKError::MissingAampKey("Chemical missing chemical_body".to_owned()))?;
        Ok(Self {
            unknown: Some(
                value
                    .list("chemical_root")
                    .ok_or_else(|| {
                        UKError::MissingAampKey("Chemical missing chemical_root".to_owned())
                    })?
                    .object("chemical_header")
                    .ok_or_else(|| {
                        UKError::MissingAampKey("Chemical missing chemical_header".to_owned())
                    })?
                    .0
                    .get(&3635073347)
                    .ok_or_else(|| {
                        UKError::MissingAampKey("Chemical missing 3635073347".to_owned())
                    })?
                    .as_u32()? as usize,
            ),
            body: (0..shape_num)
                .map(|i| -> Result<(usize, ChemicalBody)> {
                    Ok((
                        i,
                        ChemicalBody {
                            rigid_c: chemical_body
                                .object(&format!("rigid_c_{:02}", i))
                                .ok_or_else(|| {
                                    UKError::MissingAampKey(
                                        "Chemical missing rigid_c entry".to_owned(),
                                    )
                                })
                                .cloned()?,
                            shape: chemical_body
                                .object(&format!("shape_{:02}", i))
                                .ok_or_else(|| {
                                    UKError::MissingAampKey(
                                        "Chemical missing shape entry".to_owned(),
                                    )
                                })
                                .cloned()?,
                        },
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<Chemical> for ParameterIO {
    fn from(val: Chemical) -> Self {
        ParameterIO {
            lists: [(
                "chemical_body",
                ParameterList {
                    objects: [(
                        "chemical_header",
                        [
                            (
                                hash_name("res_shape_num"),
                                Parameter::U32(val.body.len() as u32),
                            ),
                            (635073347, Parameter::U32(val.unknown.unwrap() as u32)),
                        ]
                        .into_iter()
                        .collect(),
                    )]
                    .into_iter()
                    .collect(),
                    lists: [(
                        "chemical_body",
                        ParameterList {
                            lists: Default::default(),
                            objects: val
                                .body
                                .into_iter()
                                .flat_map(|(i, body)| {
                                    [
                                        (format!("rigid_c_{:02}", i), body.rigid_c),
                                        (format!("shape_{:02}", i), body.shape),
                                    ]
                                })
                                .collect(),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        }
    }
}

impl Convertible<ParameterIO> for Chemical {}

impl Mergeable<ParameterIO> for Chemical {
    fn diff(&self, other: &Self) -> Self {
        Self {
            unknown: if other.unknown != self.unknown {
                other.unknown
            } else {
                None
            },
            body: other
                .body
                .iter()
                .filter_map(|(i, other_body)| {
                    (self.body.get(i) != Some(other_body)).then(|| (*i, other_body.clone()))
                })
                .collect(),
        }
    }

    fn merge(base: &Self, diff: &Self) -> Self {
        todo!()
    }
}
