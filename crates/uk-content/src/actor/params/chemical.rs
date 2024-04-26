use std::collections::BTreeMap;

use join_str::jstr;
use roead::{
    aamp::*,
    byml::{Byml, Map},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;
use uk_util::OptionResultExt;

use crate::{
    actor::{InfoSource, ParameterResource},
    prelude::*,
    util, Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct ChemicalBody {
    pub shape:   ParameterObject,
    pub rigid_c: ParameterObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct Chemical {
    pub unknown: Option<usize>,
    pub body:    BTreeMap<usize, ChemicalBody>,
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
        let chem_root = pio.list("chemical_root").ok_or(UKError::MissingAampKey(
            "Chemical missing chemical_root",
            None,
        ))?;
        let shape_num = chem_root
            .object("chemical_header")
            .ok_or(UKError::MissingAampKey(
                "Chemical missing chemical_header",
                None,
            ))?
            .get("res_shape_num")
            .ok_or(UKError::MissingAampKey(
                "Chemical missing shape count",
                None,
            ))?
            .as_int()?;
        let chemical_body = chem_root
            .list("chemical_body")
            .ok_or(UKError::MissingAampKey(
                "Chemical missing chemical_body",
                None,
            ))?;
        Ok(Self {
            unknown: chem_root
                .object("chemical_header")
                .ok_or(UKError::MissingAampKey(
                    "Chemical missing chemical_header",
                    None,
                ))?
                .0
                .get(&3635073347)
                .map(|x| x.as_int())
                .transpose()?,
            body:    (0..shape_num)
                .map(|i| -> Result<(usize, ChemicalBody)> {
                    Ok((i, ChemicalBody {
                        rigid_c: chemical_body
                            .object(&format!("rigid_c_{:02}", i))
                            .ok_or(UKError::MissingAampKey(
                                "Chemical missing rigid_c entry",
                                None,
                            ))
                            .cloned()?,
                        shape:   chemical_body
                            .object(&format!("shape_{:02}", i))
                            .ok_or(UKError::MissingAampKey(
                                "Chemical missing shape entry",
                                None,
                            ))
                            .cloned()?,
                    }))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<Chemical> for ParameterIO {
    fn from(val: Chemical) -> Self {
        ParameterIO {
            param_root: ParameterList {
                lists: lists!(
                    "chemical_root" => ParameterList {
                        objects: objs!(
                            "chemical_header" =>
                            [(
                                hash_name("res_shape_num"),
                                Parameter::U32(val.body.len() as u32),
                            )]
                            .into_iter()
                            .chain(
                                val.unknown
                                    .into_iter()
                                    .map(|v| (3635073347, Parameter::U32(v as u32))),
                            )
                            .collect()
                        ),
                        lists: lists!(
                            "chemical_body" => ParameterList {
                                lists: Default::default(),
                                objects: val
                                    .body
                                    .into_iter()
                                    .flat_map(|(i, body)| {
                                        [
                                            (format!("shape_{:02}", i), body.shape),
                                            (format!("rigid_c_{:02}", i), body.rigid_c),
                                        ]
                                    })
                                    .collect(),
                            }
                        ),
                    }
                ),
                ..Default::default()
            },
            data_type:  "xml".into(),
            version:    0,
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
            body:    util::simple_index_diff(&self.body, &other.body),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            unknown: diff.unknown.or(self.unknown),
            body:    util::simple_index_merge(&self.body, &diff.body),
        }
    }
}

impl InfoSource for Chemical {
    fn update_info(&self, info: &mut Map) -> crate::Result<()> {
        let mut chem_info = Map::default();
        if self
            .body
            .values()
            .next()
            .and_then(|body| body.rigid_c.get("attribute").and_then(|a| a.as_u32().ok()))
            .map(|att| att == 650)
            .unwrap_or(false)
        {
            chem_info.insert("Capaciter".into(), Byml::I32(1));
        }
        if self
            .body
            .values()
            .next()
            .and_then(|body| body.shape.get("name").and_then(|n| n.as_str().ok()))
            .map(|n| n == "WeaponFire")
            .unwrap_or(false)
        {
            chem_info.insert("Burnable".into(), Byml::I32(1));
        }
        if !chem_info.is_empty() {
            info.insert("Chemical".into(), Byml::Map(chem_info));
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

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"bchemical")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(chemical.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let chemical2 = super::Chemical::try_from(&pio2).unwrap();
        assert_eq!(chemical, chemical2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2.get_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical2 = super::Chemical::try_from(&pio2).unwrap();
        let _diff = chemical.diff(&chemical2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor.get_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2.get_data("Actor/Chemical/NPC.bchemical").unwrap(),
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
            actor.get_data("Actor/Chemical/NPC.bchemical").unwrap(),
        )
        .unwrap();
        let chemical = super::Chemical::try_from(&pio).unwrap();
        let mut info = roead::byml::Map::default();
        chemical.update_info(&mut info).unwrap();
        assert_eq!(info["Chemical"]["Capaciter"], roead::byml::Byml::I32(1));
        assert_eq!(info["Chemical"]["Burnable"], roead::byml::Byml::I32(1));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/Chemical/NPC.bchemical",
        );
        assert!(super::Chemical::path_matches(path));
    }
}
