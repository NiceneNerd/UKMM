use crate::prelude::*;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldInfo(pub ParameterIO);

impl Convertible<ParameterIO> for WorldInfo {}

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

impl SimpleMergeableAamp for WorldInfo {
    fn inner(&self) -> &ParameterIO {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::aamp::ParameterIO;

    fn load_winfo() -> ParameterIO {
        ParameterIO::from_binary(&std::fs::read("test/WorldMgr/normal.bwinfo").unwrap()).unwrap()
    }

    fn load_mod_winfo() -> ParameterIO {
        ParameterIO::from_binary(&std::fs::read("test/WorldMgr/normal.mod.bwinfo").unwrap())
            .unwrap()
    }

    #[test]
    fn serde() {
        let pio = load_winfo();
        let winfo = super::WorldInfo::try_from(&pio).unwrap();
        let data = ParameterIO::from(winfo.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(&data).unwrap();
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
}
