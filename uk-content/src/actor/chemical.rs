use crate::{
    prelude::{Convertible, Mergeable},
    util, Result, UKError,
};
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChemicalBody {
    pub rigid_c: ParameterObject,
    pub shape: ParameterObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Chemical {
    pub unknown: Option<usize>,
    pub body: BTreeMap<usize, ChemicalBody>,
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
            .ok_or(UKError::MissingAampKey("Chemical missing chemical_root"))?
            .object("chemical_header")
            .ok_or(UKError::MissingAampKey("Chemical missing chemical_header"))?
            .param("res_shape_num")
            .ok_or(UKError::MissingAampKey("Chemical missing shape count"))?
            .as_u32()? as usize;
        let chemical_body = value
            .list("chemical_root")
            .unwrap()
            .list("chemical_body")
            .ok_or(UKError::MissingAampKey("Chemical missing chemical_body"))?;
        Ok(Self {
            unknown: Some(
                value
                    .list("chemical_root")
                    .ok_or(UKError::MissingAampKey("Chemical missing chemical_root"))?
                    .object("chemical_header")
                    .ok_or(UKError::MissingAampKey("Chemical missing chemical_header"))?
                    .0
                    .get(&3635073347)
                    .ok_or(UKError::MissingAampKey("Chemical missing 3635073347"))?
                    .as_u32()? as usize,
            ),
            body: (0..shape_num)
                .map(|i| -> Result<(usize, ChemicalBody)> {
                    Ok((
                        i,
                        ChemicalBody {
                            rigid_c: chemical_body
                                .object(&format!("rigid_c_{:02}", i))
                                .ok_or(UKError::MissingAampKey("Chemical missing rigid_c entry"))
                                .cloned()?,
                            shape: chemical_body
                                .object(&format!("shape_{:02}", i))
                                .ok_or(UKError::MissingAampKey("Chemical missing shape entry"))
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
                "chemical_root",
                ParameterList {
                    objects: [(
                        "chemical_header",
                        [
                            (
                                hash_name("res_shape_num"),
                                Parameter::U32(val.body.len() as u32),
                            ),
                            (3635073347, Parameter::U32(val.unknown.unwrap() as u32)),
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
            body: util::simple_index_diff(&self.body, &other.body),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            unknown: diff.unknown.or(self.unknown),
            body: util::simple_index_merge(&self.body, &diff.body),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_file_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let data = chemical.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let chemical2 = super::Chemical::try_from(&pio2).unwrap();
        assert_eq!(chemical, chemical2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_file_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/Chemical/NPC.bchemical")
                .unwrap(),
        )
        .unwrap();
        let chemical2 = super::Chemical::try_from(&pio2).unwrap();
        let diff = chemical.diff(&chemical2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_file_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/Chemical/NPC.bchemical")
                .unwrap(),
        )
        .unwrap();
        let chemical2 = super::Chemical::try_from(&pio2).unwrap();
        let diff = chemical.diff(&chemical2);
        let merged = super::Chemical::merge(&chemical, &diff);
        assert_eq!(chemical2, merged);
    }
}
