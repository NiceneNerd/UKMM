use crate::{actor::ParameterResource, prelude::*};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UMii(pub ParameterIO);

impl From<&ParameterIO> for UMii {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for UMii {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<UMii> for ParameterIO {
    fn from(val: UMii) -> Self {
        val.0
    }
}

impl_simple_aamp!(UMii, 0);

impl ParameterResource for UMii {
    fn path(name: &str) -> String {
        jstr!("Actor/UMii/{name}.bumii")
    }
}

impl Resource for UMii {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Ok((&ParameterIO::from_binary(data.as_ref())?).into())
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bumii")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii = super::UMii::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(umii.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let umii2 = super::UMii::try_from(&pio2).unwrap();
        assert_eq!(umii, umii2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii = super::UMii::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii2 = super::UMii::try_from(&pio2).unwrap();
        let diff = umii.diff(&umii2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let umii = super::UMii::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii2 = super::UMii::try_from(&pio2).unwrap();
        let diff = umii.diff(&umii2);
        let merged = umii.merge(&diff);
        assert_eq!(umii2, merged);
    }
}
