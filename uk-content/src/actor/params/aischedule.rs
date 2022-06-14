use crate::{actor::ParameterResource, prelude::*};
use join_str::jstr;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct AISchedule(pub Byml);

impl From<Byml> for AISchedule {
    fn from(byml: Byml) -> Self {
        Self(byml)
    }
}

impl From<&Byml> for AISchedule {
    fn from(byml: &Byml) -> Self {
        Self(byml.clone())
    }
}

impl From<AISchedule> for Byml {
    fn from(anim: AISchedule) -> Self {
        anim.0
    }
}

impl_simple_byml!(AISchedule, 0);

impl ParameterResource for AISchedule {
    fn path(name: &str) -> String {
        jstr!("Actor/AISchedule/{name}.baischedule")
    }
}

impl Resource for AISchedule {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Ok((&Byml::from_binary(data.as_ref())?).into())
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
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
                .get_file_data("Actor/AISchedule/Npc_TripMaster_00.baischedule")
                .unwrap(),
        )
        .unwrap();
        let aischedule = super::AISchedule::from(&byml);
        let data = roead::byml::Byml::from(aischedule.clone()).to_binary(roead::Endian::Big);
        let byml2 = roead::byml::Byml::from_binary(&data).unwrap();
        let aischedule2 = super::AISchedule::from(&byml2);
        assert_eq!(aischedule, aischedule2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let byml = roead::byml::Byml::from_binary(
            actor
                .get_file_data("Actor/AISchedule/Npc_TripMaster_00.baischedule")
                .unwrap(),
        )
        .unwrap();
        let aischedule = super::AISchedule::from(&byml);
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let byml2 = roead::byml::Byml::from_binary(
            actor2
                .get_file_data("Actor/AISchedule/Npc_TripMaster_00.baischedule")
                .unwrap(),
        )
        .unwrap();
        let aischedule2 = super::AISchedule::from(&byml2);
        let diff = aischedule.diff(&aischedule2);
        assert_eq!(diff.0.as_hash().unwrap().len(), 2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let byml = roead::byml::Byml::from_binary(
            actor
                .get_file_data("Actor/AISchedule/Npc_TripMaster_00.baischedule")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let aischedule = super::AISchedule::from(&byml);
        let byml2 = roead::byml::Byml::from_binary(
            actor2
                .get_file_data("Actor/AISchedule/Npc_TripMaster_00.baischedule")
                .unwrap(),
        )
        .unwrap();
        let aischedule2 = super::AISchedule::from(&byml2);
        let diff = aischedule.diff(&aischedule2);
        let merged = aischedule.merge(&diff);
        assert_eq!(aischedule2, merged);
    }
}
