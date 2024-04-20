use roead::aamp::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;
use uk_util::OptionResultExt;

use crate::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct Demo(pub ParameterIO);

impl From<&ParameterIO> for Demo {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for Demo {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<Demo> for ParameterIO {
    fn from(val: Demo) -> Self {
        val.0
    }
}

impl_simple_aamp!(Demo, 0);

impl Resource for Demo {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Ok((&ParameterIO::from_binary(data)?).into())
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"bdemo")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::aamp::ParameterIO;

    use crate::prelude::*;

    fn load_demo() -> ParameterIO {
        ParameterIO::from_binary(std::fs::read("test/Demo/Demo005_0.bdemo").unwrap()).unwrap()
    }

    fn load_mod_demo() -> ParameterIO {
        ParameterIO::from_binary(std::fs::read("test/Demo/Demo005_0.mod.bdemo").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let pio = load_demo();
        let demo = super::Demo::from(&pio);
        let data = ParameterIO::from(demo.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(data).unwrap();
        let demo2 = super::Demo::from(&pio2);
        assert_eq!(demo, demo2);
    }

    #[test]
    fn diff() {
        let pio = load_demo();
        let demo = super::Demo::from(&pio);
        let pio2 = load_mod_demo();
        let demo2 = super::Demo::from(&pio2);
        let _diff = demo.diff(&demo2);
    }

    #[test]
    fn merge() {
        let pio = load_demo();
        let demo = super::Demo::from(&pio);
        let pio2 = load_mod_demo();
        let demo2 = super::Demo::from(&pio2);
        let diff = demo.diff(&demo2);
        let merged = demo.merge(&diff);
        assert_eq!(merged, demo2)
    }

    #[test]
    fn identify() {
        let path =
            std::path::Path::new("content/Event/SomeEvent.sbeventpack//Demo/Demo005_0.bdemo");
        assert!(super::Demo::path_matches(path));
    }
}
