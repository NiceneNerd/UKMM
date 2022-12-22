use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, DeleteMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Editable)]
pub struct AttClientList {
    pub att_pos:     ParameterObject,
    pub att_clients: DeleteMap<String64, String64>,
}

impl TryFrom<&ParameterIO> for AttClientList {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            att_pos:     pio
                .object("AttPos")
                .ok_or(UKError::MissingAampKey(
                    "Attention client list missing AttPos",
                    None,
                ))?
                .clone(),
            att_clients: pio
                .list("AttClients")
                .ok_or(UKError::MissingAampKey(
                    "Attention client list missing attention lists",
                    None,
                ))?
                .objects
                .0
                .values()
                .map(|obj| -> Result<(String64, String64)> {
                    Ok((
                        *obj.get("Name")
                            .ok_or(UKError::MissingAampKey(
                                "Attention client list client missing name",
                                None,
                            ))?
                            .as_string64()?,
                        *obj.get("FileName")
                            .ok_or(UKError::MissingAampKey(
                                "Attention client list client missing filename",
                                None,
                            ))?
                            .as_string64()?,
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<AttClientList> for ParameterIO {
    fn from(val: AttClientList) -> Self {
        Self::new().with_object("AttPos", val.att_pos).with_list(
            "AttClients",
            ParameterList::new().with_objects(val.att_clients.into_iter().enumerate().map(
                |(i, (name, filename))| {
                    (
                        jstr!("AttClient_{&lexical::to_string(i)}"),
                        ParameterObject::new()
                            .with_parameter("Name", Parameter::String64(Box::new(name)))
                            .with_parameter("FileName", Parameter::String64(Box::new(filename))),
                    )
                },
            )),
        )
    }
}

impl Mergeable for AttClientList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            att_pos:     util::diff_pobj(&self.att_pos, &other.att_pos),
            att_clients: self.att_clients.diff(&other.att_clients),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            att_pos:     util::merge_pobj(&self.att_pos, &diff.att_pos),
            att_clients: self.att_clients.merge(&diff.att_clients),
        }
    }
}

impl ParameterResource for AttClientList {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/AttClientList/{name}.batcllist")
    }
}

impl Resource for AttClientList {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("batcllist")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/AttClientList/Guardian_A.batcllist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let atcllist = super::AttClientList::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(atcllist.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let atcllist2 = super::AttClientList::try_from(&pio2).unwrap();
        assert_eq!(atcllist, atcllist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/AttClientList/Guardian_A.batcllist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let atcllist = super::AttClientList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/AttClientList/Guardian_A.batcllist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let atcllist2 = super::AttClientList::try_from(&pio2).unwrap();
        let _diff = atcllist.diff(&atcllist2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/AttClientList/Guardian_A.batcllist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let atcllist = super::AttClientList::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/AttClientList/Guardian_A.batcllist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let atcllist2 = super::AttClientList::try_from(&pio2).unwrap();
        let diff = atcllist.diff(&atcllist2);
        let merged = atcllist.merge(&diff);
        assert_eq!(atcllist2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/AttClientList/Guardian_A.\
             batcllist",
        );
        assert!(super::AttClientList::path_matches(path));
    }
}
