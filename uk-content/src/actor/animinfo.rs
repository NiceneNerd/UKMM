use roead::byml::Byml;
use serde::{Deserialize, Serialize};

use crate::prelude::{Convertible, ShallowMergeableByml};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
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

impl Convertible<Byml> for AnimationInfo {}

impl ShallowMergeableByml for AnimationInfo {
    fn inner(&self) -> &roead::byml::Byml {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let byml = roead::byml::Byml::from_binary(
            actor
                .get_file_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
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
                .get_file_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap(),
        )
        .unwrap();
        let animinfo = super::AnimationInfo::from(&byml);
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let byml2 = roead::byml::Byml::from_binary(
            actor2
                .get_file_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
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
                .get_file_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let animinfo = super::AnimationInfo::from(&byml);
        let byml2 = roead::byml::Byml::from_binary(
            actor2
                .get_file_data("Actor/AnimationInfo/Npc_TripMaster_00.baniminfo")
                .unwrap(),
        )
        .unwrap();
        let animinfo2 = super::AnimationInfo::from(&byml2);
        let diff = animinfo.diff(&animinfo2);
        let merged = super::AnimationInfo::merge(&animinfo, &diff);
        assert_eq!(animinfo2, merged);
    }
}

