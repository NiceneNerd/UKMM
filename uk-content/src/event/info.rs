use crate::prelude::*;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct EventInfo(pub Byml);

impl From<Byml> for EventInfo {
    fn from(byml: Byml) -> Self {
        Self(byml)
    }
}

impl From<&Byml> for EventInfo {
    fn from(byml: &Byml) -> Self {
        Self(byml.clone())
    }
}

impl From<EventInfo> for Byml {
    fn from(val: EventInfo) -> Self {
        val.0
    }
}

impl_simple_byml!(EventInfo, 0);

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_eventinfo() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Event/EventInfo.product.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_eventinfo() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Event/EventInfo.product.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_eventinfo();
        let eventinfo = super::EventInfo::try_from(&byml).unwrap();
        let data = Byml::from(eventinfo.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let eventinfo2 = super::EventInfo::try_from(&byml2).unwrap();
        assert_eq!(eventinfo, eventinfo2);
    }

    #[test]
    fn diff() {
        let byml = load_eventinfo();
        let eventinfo = super::EventInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_eventinfo();
        let eventinfo2 = super::EventInfo::try_from(&byml2).unwrap();
        let _diff = eventinfo.diff(&eventinfo2);
    }

    #[test]
    fn merge() {
        let byml = load_eventinfo();
        let eventinfo = super::EventInfo::try_from(&byml).unwrap();
        let byml2 = load_mod_eventinfo();
        let eventinfo2 = super::EventInfo::try_from(&byml2).unwrap();
        let diff = eventinfo.diff(&eventinfo2);
        let merged = eventinfo.merge(&diff);
        assert_eq!(merged, eventinfo2);
    }
}
