use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

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

impl Resource for EventInfo {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Ok((&Byml::from_binary(data.as_ref())?).into())
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("EventInfo.product")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_eventinfo() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(std::fs::read("test/Event/EventInfo.product.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_eventinfo() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Event/EventInfo.product.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_eventinfo();
        let eventinfo = super::EventInfo::from(&byml);
        let data = Byml::from(eventinfo.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let eventinfo2 = super::EventInfo::from(&byml2);
        assert_eq!(eventinfo, eventinfo2);
    }

    #[test]
    fn diff() {
        let byml = load_eventinfo();
        let eventinfo = super::EventInfo::from(&byml);
        let byml2 = load_mod_eventinfo();
        let eventinfo2 = super::EventInfo::from(&byml2);
        let _diff = eventinfo.diff(&eventinfo2);
    }

    #[test]
    fn merge() {
        let byml = load_eventinfo();
        let eventinfo = super::EventInfo::from(&byml);
        let byml2 = load_mod_eventinfo();
        let eventinfo2 = super::EventInfo::from(&byml2);
        let diff = eventinfo.diff(&eventinfo2);
        let merged = eventinfo.merge(&diff);
        assert_eq!(merged, eventinfo2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/TitleBG.pack//Event/EventInfo.product.sbyml");
        assert!(super::EventInfo::path_matches(path));
    }
}
