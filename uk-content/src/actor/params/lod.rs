use crate::{actor::ParameterResource, prelude::*};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lod(pub ParameterIO);

impl From<&ParameterIO> for Lod {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for Lod {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<Lod> for ParameterIO {
    fn from(val: Lod) -> Self {
        val.0
    }
}

impl_simple_aamp!(Lod, 0);

impl ParameterResource for Lod {
    fn path(name: &str) -> String {
        jstr!("Actor/LOD/{name}.blod")
    }
}

impl Resource for Lod {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Ok((&ParameterIO::from_binary(data.as_ref())?).into())
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
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
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod = super::Lod::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(lod.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let lod2 = super::Lod::try_from(&pio2).unwrap();
        assert_eq!(lod, lod2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod = super::Lod::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod2 = super::Lod::try_from(&pio2).unwrap();
        let diff = lod.diff(&lod2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let lod = super::Lod::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/LOD/EnemyNoCalcSkip.blod")
                .unwrap(),
        )
        .unwrap();
        let lod2 = super::Lod::try_from(&pio2).unwrap();
        let diff = lod.diff(&lod2);
        let merged = lod.merge(&diff);
        assert_eq!(lod2, merged);
    }
}
