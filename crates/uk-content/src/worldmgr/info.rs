#[cfg(feature = "ui")]
use nk_ui_derive::Editable;
use nk_util::OptionResultExt;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct WorldInfo(pub ParameterIO);

impl From<&ParameterIO> for WorldInfo {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for WorldInfo {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<WorldInfo> for ParameterIO {
    fn from(val: WorldInfo) -> Self {
        val.0
    }
}

impl_simple_aamp!(WorldInfo, 0);

impl Resource for WorldInfo {
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
            .contains(&"bwinfo")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::aamp::ParameterIO;

    use crate::prelude::*;

    fn load_winfo() -> ParameterIO {
        ParameterIO::from_binary(std::fs::read("test/WorldMgr/normal.bwinfo").unwrap()).unwrap()
    }

    fn load_mod_winfo() -> ParameterIO {
        ParameterIO::from_binary(std::fs::read("test/WorldMgr/normal.mod.bwinfo").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let pio = load_winfo();
        let winfo = super::WorldInfo::try_from(&pio).unwrap();
        let data = ParameterIO::from(winfo.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(data).unwrap();
        let winfo2 = super::WorldInfo::try_from(&pio2).unwrap();
        assert_eq!(winfo, winfo2);
    }

    #[test]
    fn diff() {
        let pio = load_winfo();
        let winfo = super::WorldInfo::try_from(&pio).unwrap();
        let pio2 = load_mod_winfo();
        let winfo2 = super::WorldInfo::try_from(&pio2).unwrap();
        let _diff = winfo.diff(&winfo2);
    }

    #[test]
    fn merge() {
        let pio = load_winfo();
        let winfo = super::WorldInfo::try_from(&pio).unwrap();
        let pio2 = load_mod_winfo();
        let winfo2 = super::WorldInfo::try_from(&pio2).unwrap();
        let diff = winfo.diff(&winfo2);
        let merged = winfo.merge(&diff);
        assert_eq!(merged, winfo2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/WorldMgr/normal.bwinfo");
        assert!(super::WorldInfo::path_matches(path));
    }
}
