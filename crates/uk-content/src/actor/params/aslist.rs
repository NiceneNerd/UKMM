use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;

use crate::{actor::ParameterResource, prelude::*, util::DeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ParamData)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct AddRes {
    #[name = "Anim"]
    pub anim: String64,
    #[name = "RetargetModel"]
    pub retarget_model: Option<String64>,
    #[name = "RetargetNoCorrect"]
    pub retarget_nocorrect: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct ASList {
    pub common:     Option<ParameterObject>,
    pub add_reses:  DeleteMap<String, AddRes>,
    pub as_defines: DeleteMap<String64, String64>,
    pub cf_defines: Option<DeleteMap<String, ParameterList>>,
}

impl TryFrom<&ParameterIO> for ASList {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            common:     pio.object("Common").cloned(),
            add_reses:  pio
                .list("AddReses")
                .ok_or(UKError::MissingAampKey("AS list missing add reses", None))?
                .objects
                .0
                .values()
                .map(|obj| -> Result<(String, AddRes)> {
                    let res: AddRes = obj.try_into()?;
                    Ok((res.anim.into(), res))
                })
                .collect::<Result<_>>()?,
            as_defines: pio
                .list("ASDefines")
                .ok_or(UKError::MissingAampKey("AS list missing AS defines", None))?
                .objects
                .0
                .values()
                .map(|obj| -> Result<(String64, String64)> {
                    Ok((
                        obj.get("Name")
                            .ok_or(UKError::MissingAampKey(
                                "AS list AS define missing name",
                                None,
                            ))?
                            .as_safe_string()?,
                        obj.get("Filename")
                            .ok_or(UKError::MissingAampKey(
                                "AS list AS define missing filename",
                                None,
                            ))?
                            .as_safe_string()?,
                    ))
                })
                .collect::<Result<_>>()?,
            cf_defines: pio
                .list("CFDefines")
                .map(|defines| -> Result<DeleteMap<String, ParameterList>> {
                    defines
                        .lists
                        .0
                        .values()
                        .map(|list| -> Result<(String, ParameterList)> {
                            let pre_name = list
                                .object("CFPre")
                                .ok_or(UKError::MissingAampKey(
                                    "AS list CF define missing CFPre",
                                    None,
                                ))?
                                .get("Name")
                                .ok_or(UKError::MissingAampKey(
                                    "AS list CF define missing CFPre name",
                                    None,
                                ))?
                                .as_str()?
                                .into();
                            Ok((pre_name, list.clone()))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
        })
    }
}

impl From<ASList> for ParameterIO {
    fn from(val: ASList) -> Self {
        Self::new()
            .with_objects(val.common.into_iter().map(|c| ("Common", c)))
            .with_lists(
                [
                    (
                        "AddReses",
                        Some(ParameterList::new().with_objects(
                            val.add_reses.into_iter().enumerate().map(|(i, (_, res))| {
                                (jstr!("AddRes_{&lexical::to_string(i)}"), res.into())
                            }),
                        )),
                    ),
                    (
                        "ASDefines",
                        Some(
                            ParameterList::new().with_objects(
                                val.as_defines.into_iter().enumerate().map(
                                    |(i, (name, filename))| {
                                        (
                                            jstr!("ASDefine_{&lexical::to_string(i)}"),
                                            ParameterObject::new()
                                                .with_parameter(
                                                    "Name",
                                                    Parameter::String64(Box::new(name)),
                                                )
                                                .with_parameter(
                                                    "Filename",
                                                    Parameter::String64(Box::new(filename)),
                                                ),
                                        )
                                    },
                                ),
                            ),
                        ),
                    ),
                    (
                        "CFDefines",
                        val.cf_defines.map(|defines| {
                            ParameterList::new().with_lists(defines.into_iter().enumerate().map(
                                |(i, (_, list))| (jstr!("CFDefine_{&lexical::to_string(i)}"), list),
                            ))
                        }),
                    ),
                ]
                .into_iter()
                .filter_map(|(k, v)| v.map(|v| (k, v))),
            )
    }
}

impl Mergeable for ASList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            common:     (other.common != self.common)
                .then(|| other.common.clone())
                .flatten(),
            add_reses:  self.add_reses.diff(&other.add_reses),
            as_defines: self.as_defines.diff(&other.as_defines),
            cf_defines: self
                .cf_defines
                .as_ref()
                .map(|self_defines| {
                    other
                        .cf_defines
                        .as_ref()
                        .map(|other_defines| self_defines.diff(other_defines))
                        .unwrap_or_default()
                })
                .or_else(|| other.cf_defines.clone()),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            common:     diff.common.clone().or_else(|| self.common.clone()),
            add_reses:  self.add_reses.merge(&diff.add_reses),
            as_defines: self.as_defines.merge(&diff.as_defines),
            cf_defines: diff
                .cf_defines
                .as_ref()
                .map(|diff_defines| {
                    self.cf_defines
                        .as_ref()
                        .map(|self_defines| self_defines.merge(diff_defines))
                        .unwrap_or_else(|| diff_defines.clone())
                })
                .or_else(|| self.cf_defines.clone()),
        }
    }
}

impl ParameterResource for ASList {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/ASList/{name}.baslist")
    }
}

impl Resource for ASList {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("baslist")
    }
}

#[cfg(test)]
mod tests {
    use roead::aamp::*;

    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist = super::ASList::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(aslist.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(data).unwrap();
        let aslist2 = super::ASList::try_from(&pio2).unwrap();
        assert_eq!(aslist, aslist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist = super::ASList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist2 = super::ASList::try_from(&pio2).unwrap();
        let _diff = aslist.diff(&aslist2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let aslist = super::ASList::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist2 = super::ASList::try_from(&pio2).unwrap();
        let diff = aslist.diff(&aslist2);
        let merged = aslist.merge(&diff);
        assert_eq!(aslist2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npn_TripMaster_00.sbactorpack//Actor/ASList/Npc_TripMaster_00.\
             baslist",
        );
        assert!(super::ASList::path_matches(path));
    }
}
