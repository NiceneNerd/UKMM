use join_str::jstr;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

use crate::{actor::ParameterResource, prelude::*};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, Editable)]
pub struct AnimationInfo(pub Byml);

impl From<Byml> for AnimationInfo {
    fn from(byml: Byml) -> Self {
        Self(byml)
    }
}

impl From<&Byml> for AnimationInfo {
    fn from(byml: &Byml) -> Self {
        Self(byml.clone())
    }
}

impl From<AnimationInfo> for Byml {
    fn from(anim: AnimationInfo) -> Self {
        anim.0
    }
}

impl ParameterResource for AnimationInfo {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/AnimationInfo/{name}.baniminfo")
    }
}

impl Resource for AnimationInfo {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Ok((&Byml::from_binary(data.as_ref())?).into())
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("baniminfo")
    }
}

impl_simple_byml!(AnimationInfo, 0);

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let byml = roead::byml::Byml::from_binary(
            actor
                .get_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let animinfo = super::AnimationInfo::from(&byml);
        let data = roead::byml::Byml::from(animinfo.clone()).to_binary(roead::Endian::Big);
        let byml2 = roead::byml::Byml::from_binary(&data).unwrap();
        let animinfo2 = super::AnimationInfo::from(&byml2);
        assert_eq!(animinfo, animinfo2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let byml = roead::byml::Byml::from_binary(
            actor
                .get_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let animinfo = super::AnimationInfo::from(&byml);
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let byml2 = roead::byml::Byml::from_binary(
            actor2
                .get_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let animinfo2 = super::AnimationInfo::from(&byml2);
        let diff = animinfo.diff(&animinfo2);
        assert_eq!(diff.0.as_hash().unwrap().len(), 2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let byml = roead::byml::Byml::from_binary(
            actor
                .get_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let animinfo = super::AnimationInfo::from(&byml);
        let byml2 = roead::byml::Byml::from_binary(
            actor2
                .get_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let animinfo2 = super::AnimationInfo::from(&byml2);
        let diff = animinfo.diff(&animinfo2);
        let merged = animinfo.merge(&diff);
        assert_eq!(animinfo2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/AnimationInfo/\
             Npc_TripMaster_00.baniminfo",
        );
        assert!(super::AnimationInfo::path_matches(path));
    }
}
