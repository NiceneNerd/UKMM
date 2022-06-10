use crate::prelude::*;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Awareness(pub ParameterIO);

impl From<&ParameterIO> for Awareness {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for Awareness {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<Awareness> for ParameterIO {
    fn from(val: Awareness) -> Self {
        val.0
    }
}

impl_simple_aamp!(Awareness, 0);

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness = super::Awareness::try_from(&pio).unwrap();
        let data = awareness.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let awareness2 = super::Awareness::try_from(&pio2).unwrap();
        assert_eq!(awareness, awareness2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness = super::Awareness::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness2 = super::Awareness::try_from(&pio2).unwrap();
        let diff = awareness.diff(&awareness2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let awareness = super::Awareness::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness2 = super::Awareness::try_from(&pio2).unwrap();
        let diff = awareness.diff(&awareness2);
        let merged = awareness.merge(&diff);
        assert_eq!(awareness2, merged);
    }
}
