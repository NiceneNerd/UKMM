use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, DeleteMap},
    Result, UKError,
};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BodyParam {
    pub name: String64,
    pub friction_scale: f32,
    pub buoyancy_scale: f32,
}

impl BodyParam {
    pub(crate) fn try_from(obj: &ParameterObject) -> Result<Self> {
        Ok(Self {
            name: *unsafe {
                // This is sound because this function is never
                // called until the name has already been checked.
                obj.param("RigidName").unwrap_unchecked()
            }
            .as_string64()?,
            friction_scale: obj
                .param("FrictionScale")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll config list body param entry missing friction scale",
                ))?
                .as_f32()?,
            buoyancy_scale: obj
                .param("BuoyancyScale")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll config list body param entry missing buoyancy scale",
                ))?
                .as_f32()?,
        })
    }
}

impl From<BodyParam> for ParameterObject {
    fn from(val: BodyParam) -> Self {
        [
            ("RigidName", Parameter::String64(val.name)),
            ("FrictionScale", Parameter::F32(val.friction_scale)),
            ("BuoyancyScale", Parameter::F32(val.buoyancy_scale)),
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RagdollConfigList {
    pub common_data: ParameterObject,
    pub impulse_params: ParameterList,
    pub body_param_list: DeleteMap<String64, BodyParam>,
}

impl TryFrom<&ParameterIO> for RagdollConfigList {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            common_data: pio
                .object("CommonData")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll config list missing common data",
                ))?
                .clone(),
            impulse_params: pio
                .list("ImpulseParamList")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll config list missing impulse param list",
                ))?
                .clone(),
            body_param_list: pio
                .list("BodyParamList")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll config list missing body param list",
                ))?
                .objects
                .0
                .values()
                .map(|body_param| -> Result<(String64, BodyParam)> {
                    Ok((
                        *body_param
                            .param("RigidName")
                            .ok_or(UKError::MissingAampKey(
                                "Ragdoll config list missing body param name",
                            ))?
                            .as_string64()?,
                        BodyParam::try_from(body_param)?,
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<RagdollConfigList> for ParameterIO {
    fn from(val: RagdollConfigList) -> Self {
        Self::new()
            .with_object("CommonData", val.common_data)
            .with_list("ImpulseParamList", val.impulse_params)
            .with_list(
                "BodyParamList",
                ParameterList::new().with_objects(val.body_param_list.into_iter().enumerate().map(
                    |(i, (_, body_param))| {
                        (
                            jstr!("BodyParam_{&lexical::to_string(i)}"),
                            body_param.into(),
                        )
                    },
                )),
            )
    }
}

impl Mergeable for RagdollConfigList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            common_data: util::diff_pobj(&self.common_data, &other.common_data),
            impulse_params: util::diff_plist(&self.impulse_params, &other.impulse_params),
            body_param_list: self.body_param_list.diff(&other.body_param_list),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            common_data: util::merge_pobj(&self.common_data, &diff.common_data),
            impulse_params: util::merge_plist(&self.impulse_params, &diff.impulse_params),
            body_param_list: self.body_param_list.merge(&diff.body_param_list),
        }
    }
}

impl ParameterResource for RagdollConfigList {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/RagdollConfigList/{name}.brgconfiglist")
    }
}

impl Resource for RagdollConfigList {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> roead::Bytes {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("brgconfiglist")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollConfigList/Moriblin_Blue.brgconfiglist")
                .unwrap(),
        )
        .unwrap();
        let rgconfiglist = super::RagdollConfigList::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(rgconfiglist.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let rgconfiglist2 = super::RagdollConfigList::try_from(&pio2).unwrap();
        assert_eq!(rgconfiglist, rgconfiglist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollConfigList/Moriblin_Blue.brgconfiglist")
                .unwrap(),
        )
        .unwrap();
        let rgconfiglist = super::RagdollConfigList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/RagdollConfigList/Moriblin_Blue.brgconfiglist")
                .unwrap(),
        )
        .unwrap();
        let rgconfiglist2 = super::RagdollConfigList::try_from(&pio2).unwrap();
        let _diff = rgconfiglist.diff(&rgconfiglist2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollConfigList/Moriblin_Blue.brgconfiglist")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let rgconfiglist = super::RagdollConfigList::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/RagdollConfigList/Moriblin_Blue.brgconfiglist")
                .unwrap(),
        )
        .unwrap();
        let rgconfiglist2 = super::RagdollConfigList::try_from(&pio2).unwrap();
        let diff = rgconfiglist.diff(&rgconfiglist2);
        let merged = rgconfiglist.merge(&diff);
        assert_eq!(rgconfiglist2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Moriblin_Junior.sbactorpack//Actor/RagdollConfigList/Moriblin_Blue.brgconfiglist",
        );
        assert!(super::RagdollConfigList::path_matches(path));
    }
}
