use crate::prelude::*;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageParam(pub ParameterIO);

impl From<&ParameterIO> for DamageParam {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for DamageParam {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<DamageParam> for ParameterIO {
    fn from(val: DamageParam) -> Self {
        val.0
    }
}

impl_simple_aamp!(DamageParam, 0);

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DamageParam/Guardian.bdmgparam")
                .unwrap(),
        )
        .unwrap();
        let dmgparam = super::DamageParam::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(dmgparam.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let dmgparam2 = super::DamageParam::try_from(&pio2).unwrap();
        assert_eq!(dmgparam, dmgparam2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DamageParam/Guardian.bdmgparam")
                .unwrap(),
        )
        .unwrap();
        let dmgparam = super::DamageParam::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/DamageParam/Guardian.bdmgparam")
                .unwrap(),
        )
        .unwrap();
        let dmgparam2 = super::DamageParam::try_from(&pio2).unwrap();
        let diff = dmgparam.diff(&dmgparam2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/DamageParam/Guardian.bdmgparam")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let dmgparam = super::DamageParam::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/DamageParam/Guardian.bdmgparam")
                .unwrap(),
        )
        .unwrap();
        let dmgparam2 = super::DamageParam::try_from(&pio2).unwrap();
        let diff = dmgparam.diff(&dmgparam2);
        let merged = dmgparam.merge(&diff);
        assert_eq!(dmgparam2, merged);
    }
}
