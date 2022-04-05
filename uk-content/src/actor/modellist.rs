use crate::{
    prelude::{Convertible, Mergeable},
    util::*,
    Result, UKError,
};
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelList {
    pub controller_info: ParameterObject,
    pub attention: ParameterObject,
    pub model_data: ParameterList,
    pub anm_target: DeleteVec<ParameterList>,
}

impl TryFrom<&ParameterIO> for ModelList {
    type Error = UKError;
    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            controller_info: pio
                .object("ControllerInfo")
                .ok_or_else(|| {
                    UKError::MissingAampKey("Model list missing controller info".to_owned())
                })?
                .clone(),
            attention: pio
                .object("Attention")
                .ok_or_else(|| UKError::MissingAampKey("Model list missing attention".to_owned()))?
                .clone(),
            model_data: pio
                .list("ModelData")
                .ok_or_else(|| UKError::MissingAampKey("Model list missing model data".to_owned()))?
                .list("ModelData_0")
                .ok_or_else(|| UKError::MissingAampKey("Model list missing model data".to_owned()))?
                .clone(),
            anm_target: pio
                .list("AnmTarget")
                .ok_or_else(|| {
                    UKError::MissingAampKey("Model list missing animation target".to_owned())
                })?
                .lists
                .0
                .values()
                .cloned()
                .collect(),
        })
    }
}

impl TryFrom<ParameterIO> for ModelList {
    type Error = UKError;
    fn try_from(pio: ParameterIO) -> Result<Self> {
        pio.try_into()
    }
}

impl From<ModelList> for ParameterIO {
    fn from(val: ModelList) -> Self {
        Self {
            objects: [
                ("ControllerInfo", val.controller_info),
                ("Attention", val.attention),
            ]
            .into_iter()
            .collect(),
            lists: [
                (
                    "ModelData",
                    ParameterList {
                        lists: [("ModelData_0", val.model_data)].into_iter().collect(),
                        ..Default::default()
                    },
                ),
                (
                    "AnmTarget",
                    ParameterList {
                        lists: val
                            .anm_target
                            .into_iter()
                            .enumerate()
                            .map(|(i, target)| (format!("AnmTarget_{}", i), target))
                            .collect(),
                        ..Default::default()
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        }
    }
}

impl Convertible<ParameterIO> for ModelList {}

impl Mergeable<ParameterIO> for ModelList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            controller_info: diff_pobj(&self.controller_info, &other.controller_info),
            attention: diff_pobj(&self.attention, &other.attention),
            model_data: diff_plist(&self.model_data, &other.model_data),
            anm_target: self.anm_target.diff(&other.anm_target),
        }
    }

    fn merge(base: &Self, diff: &Self) -> Self {
        Self {
            controller_info: merge_pobj(&base.controller_info, &diff.controller_info),
            attention: merge_pobj(&base.attention, &diff.attention),
            model_data: merge_plist(&base.model_data, &diff.model_data),
            anm_target: base.anm_target.merge(&diff.anm_target),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let data = modellist.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let modellist2 = super::ModelList::try_from(&pio2).unwrap();
        assert_eq!(modellist, modellist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let modellist2 = super::ModelList::try_from(&pio2).unwrap();
        let diff = modellist.diff(&modellist2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let modellist2 = super::ModelList::try_from(&pio2).unwrap();
        let diff = modellist.diff(&modellist2);
        let merged = super::ModelList::merge(&modellist, &diff);
        assert_eq!(modellist2, merged);
    }
}
