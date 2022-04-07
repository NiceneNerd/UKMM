use crate::prelude::*;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralParamList(pub ParameterIO);

impl Convertible<ParameterIO> for GeneralParamList {}

impl From<&ParameterIO> for GeneralParamList {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for GeneralParamList {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<GeneralParamList> for ParameterIO {
    fn from(val: GeneralParamList) -> Self {
        val.0
    }
}

impl SimpleMergeableAamp for GeneralParamList {
    fn inner(&self) -> &ParameterIO {
        &self.0
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
                .get_file_data("Actor/GeneralParamList/Enemy_Guardian_A.bgparamlist")
                .unwrap(),
        )
        .unwrap();
        let gparamlist = super::GeneralParamList::try_from(&pio).unwrap();
        let data = gparamlist.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let gparamlist2 = super::GeneralParamList::try_from(&pio2).unwrap();
        assert_eq!(gparamlist, gparamlist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/GeneralParamList/Enemy_Guardian_A.bgparamlist")
                .unwrap(),
        )
        .unwrap();
        let gparamlist = super::GeneralParamList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/GeneralParamList/Enemy_Guardian_A.bgparamlist")
                .unwrap(),
        )
        .unwrap();
        let gparamlist2 = super::GeneralParamList::try_from(&pio2).unwrap();
        let diff = gparamlist.diff(&gparamlist2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/GeneralParamList/Enemy_Guardian_A.bgparamlist")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let gparamlist = super::GeneralParamList::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/GeneralParamList/Enemy_Guardian_A.bgparamlist")
                .unwrap(),
        )
        .unwrap();
        let gparamlist2 = super::GeneralParamList::try_from(&pio2).unwrap();
        let diff = gparamlist.diff(&gparamlist2);
        let merged = gparamlist.merge(&diff);
        assert_eq!(gparamlist2, merged);
    }
}
