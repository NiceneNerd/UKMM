use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util, Result, UKError,
};
use join_str::jstr;
use roead::{
    aamp::*,
    byml::{Byml, Hash},
};
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
    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let chem_root = pio
            .list("chemical_root")
            .ok_or(UKError::MissingAampKey("Chemical missing chemical_root"))?;
        let shape_num = chem_root
            .object("chemical_header")
            .ok_or(UKError::MissingAampKey("Chemical missing chemical_header"))?
            .param("res_shape_num")
            .ok_or(UKError::MissingAampKey("Chemical missing shape count"))?
            .as_u32()? as usize;
        let chemical_body = chem_root
            .list("chemical_body")
            .ok_or(UKError::MissingAampKey("Chemical missing chemical_body"))?;
        Ok(Self {
            unknown: Some(
                chem_root
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

impl Mergeable for Chemical {
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

impl InfoSource for Chemical {
    fn update_info(&self, info: &mut Hash) -> crate::Result<()> {
        let mut chem_info = Hash::new();
        if let Some(Parameter::U32(attribute)) = self
            .body
            .values()
            .next()
            .and_then(|body| body.rigid_c.param("attribute"))
            && *attribute == 650
        {
            chem_info.insert("Capaciter".into(), Byml::Int(1));
        }
        if let Some(Parameter::String32(name)) = self
            .body
            .values()
            .next()
            .and_then(|body| body.shape.param("name"))
            && name == "WeaponFire"
        {
            chem_info.insert("Burnable".into(), Byml::Int(1));
        }
        if !chem_info.is_empty() {
            info.insert("Chemical".to_owned(), Byml::Hash(chem_info));
        }
        Ok(())
    }
}

impl ParameterResource for Chemical {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/Chemical/{name}.bchemical")
    }
}

impl Resource for Chemical {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> roead::Bytes {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bchemical")
    }
}

#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_file_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(chemical.clone()).to_binary();
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
        let _diff = chemical.diff(&chemical2);
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
        let merged = chemical.merge(&diff);
        assert_eq!(chemical2, merged);
    }

    #[test]
    fn info() {
        let actor = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_file_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::new();
        chemical.update_info(&mut info).unwrap();
        assert_eq!(info["Chemical"]["Capaciter"], roead::byml::Byml::Int(1));
        assert_eq!(info["Chemical"]["Burnable"], roead::byml::Byml::Int(1));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/Chemical/NPC.bchemical",
        );
        assert!(super::Chemical::path_matches(path));
    }
}
