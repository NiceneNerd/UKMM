use crate::{prelude::*, util::DeleteMap, Result, UKError};
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AddRes {
    anim: String,
    retarget_model: Option<String>,
    retarget_nocorrect: Option<bool>,
}

impl TryFrom<&ParameterObject> for AddRes {
    type Error = UKError;

    fn try_from(value: &ParameterObject) -> Result<Self> {
        Ok(Self {
            anim: value
                .param("Anim")
                .ok_or(UKError::MissingAampKey("AS list add res missing anim"))?
                .as_string64()?
                .to_owned(),
            retarget_model: value
                .param("RetargetModel")
                .map(|v| v.as_string64().map(|s| s.to_owned()))
                .transpose()?,
            retarget_nocorrect: value
                .param("RetargetNoCorrect")
                .map(|v| v.as_bool())
                .transpose()?,
        })
    }
}

impl From<AddRes> for ParameterObject {
    fn from(value: AddRes) -> Self {
        [
            ("Anim", Some(Parameter::String64(value.anim))),
            (
                "RetargetModel",
                value.retarget_model.map(Parameter::String64),
            ),
            (
                "RetargetNoCorrect",
                value.retarget_nocorrect.map(Parameter::Bool),
            ),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ASList {
    common: Option<ParameterObject>,
    add_reses: DeleteMap<String, AddRes>,
    as_defines: DeleteMap<String, String>,
    cf_defines: Option<DeleteMap<String, ParameterList>>,
}

impl TryFrom<&ParameterIO> for ASList {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            common: pio.object("Common").cloned(),
            add_reses: pio
                .list("AddReses")
                .ok_or(UKError::MissingAampKey("AS list missing add reses"))?
                .objects
                .0
                .values()
                .map(|obj| -> Result<(String, AddRes)> {
                    let res: AddRes = obj.try_into()?;
                    Ok((res.anim.clone(), res))
                })
                .collect::<Result<_>>()?,
            as_defines: pio
                .list("ASDefines")
                .ok_or(UKError::MissingAampKey("AS list missing AS defines"))?
                .objects
                .0
                .values()
                .map(|obj| -> Result<(String, String)> {
                    Ok((
                        obj.param("Name")
                            .ok_or(UKError::MissingAampKey("AS list AS define missing name"))?
                            .as_string64()?
                            .to_owned(),
                        obj.param("Filename")
                            .ok_or(UKError::MissingAampKey(
                                "AS list AS define missing filename",
                            ))?
                            .as_string64()?
                            .to_owned(),
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
                                .ok_or(UKError::MissingAampKey("AS list CF define missing CFPre"))?
                                .param("Name")
                                .ok_or(UKError::MissingAampKey(
                                    "AS list CF define missing CFPre name",
                                ))?
                                .as_string()?
                                .to_owned();
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
        Self {
            objects: val.common.into_iter().map(|c| ("Common", c)).collect(),
            lists: [
                (
                    "AddReses",
                    Some(
                        ParameterList::new().with_objects(
                            val.add_reses
                                .into_iter()
                                .enumerate()
                                .map(|(i, (_, res))| (format!("AddRes_{}", i), res.into())),
                        ),
                    ),
                ),
                (
                    "ASDefines",
                    Some(
                        ParameterList::new().with_objects(
                            val.as_defines
                                .into_iter()
                                .enumerate()
                                .map(|(i, (name, filename))| {
                                    (
                                        format!("ASDefine_{}", i),
                                        ParameterObject::new()
                                            .with_param("Name", Parameter::String64(name))
                                            .with_param("Filename", Parameter::String64(filename)),
                                    )
                                }),
                        ),
                    ),
                ),
                (
                    "CFDefines",
                    val.cf_defines.map(|defines| {
                        ParameterList::new().with_lists(
                            defines
                                .into_iter()
                                .enumerate()
                                .map(|(i, (_, list))| (format!("CFDefine_{}", i), list)),
                        )
                    }),
                ),
            ]
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect(),
            ..Default::default()
        }
    }
}

impl Mergeable<ParameterIO> for ASList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            common: (other.common != self.common)
                .then(|| other.common.clone())
                .flatten(),
            add_reses: self.add_reses.diff(&other.add_reses),
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
            common: diff.common.clone().or_else(|| self.common.clone()),
            add_reses: self.add_reses.merge(&diff.add_reses),
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

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use pretty_assertions::assert_eq;
    use roead::aamp::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist = super::ASList::try_from(&pio).unwrap();
        let data = aslist.clone().into_pio().to_binary();
        let pio2 = ParameterIO::from_binary(&data).unwrap();
        let aslist2 = super::ASList::try_from(&pio2).unwrap();
        assert_eq!(aslist, aslist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist = super::ASList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist2 = super::ASList::try_from(&pio2).unwrap();
        let diff = aslist.diff(&aslist2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let aslist = super::ASList::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap(),
        )
        .unwrap();
        let aslist2 = super::ASList::try_from(&pio2).unwrap();
        let diff = aslist.diff(&aslist2);
        let merged = aslist.merge(&diff);
        assert_eq!(aslist2, merged);
    }
}
