use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;
use uk_util::OptionResultExt;

use crate::{actor::ParameterResource, prelude::*};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
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

impl ParameterResource for Awareness {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/Awareness/{name}.bawareness")
    }
}

impl Resource for Awareness {
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
            .contains(&"bawareness")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness = super::Awareness::from(&pio);
        let data = roead::aamp::ParameterIO::from(awareness.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let awareness2 = super::Awareness::from(&pio2);
        assert_eq!(awareness, awareness2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness = super::Awareness::from(&pio);
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness2 = super::Awareness::from(&pio2);
        let _diff = awareness.diff(&awareness2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let awareness = super::Awareness::from(&pio);
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/Awareness/Guardian.bawareness")
                .unwrap(),
        )
        .unwrap();
        let awareness2 = super::Awareness::from(&pio2);
        let diff = awareness.diff(&awareness2);
        let merged = awareness.merge(&diff);
        assert_eq!(awareness2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/Awareness/Guardian.bawareness",
        );
        assert!(super::Awareness::path_matches(path));
    }
}
