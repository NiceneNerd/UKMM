use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

use uk_util::OptionResultExt;

use crate::{actor::ParameterResource, prelude::*};

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
    fn path(name: &str) -> std::string::String {
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
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"bumii")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii = super::UMii::from(&pio);
        let data = roead::aamp::ParameterIO::from(umii.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let umii2 = super::UMii::from(&pio2);
        assert_eq!(umii, umii2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii = super::UMii::from(&pio);
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii2 = super::UMii::from(&pio2);
        let _diff = umii.diff(&umii2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let umii = super::UMii::from(&pio);
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii2 = super::UMii::from(&pio2);
        let diff = umii.diff(&umii2);
        let merged = umii.merge(&diff);
        assert_eq!(umii2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/UMii/Npc_TripMaster_00.bumii",
        );
        assert!(super::UMii::path_matches(path));
    }
}
