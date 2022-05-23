use crate::{
    prelude::{Convertible, Mergeable},
    util::*,
    Result, UKError,
};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelList {
    pub controller_info: ParameterObject,
    pub attention: ParameterObject,
    pub model_data: ParameterList,
    pub anm_target: BTreeMap<usize, ParameterList>,
}

impl TryFrom<&ParameterIO> for ModelList {
    type Error = UKError;
    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            controller_info: pio
                .object("ControllerInfo")
                .ok_or(UKError::MissingAampKey(
                    "Model list missing controller info",
                ))?
                .clone(),
            attention: pio
                .object("Attention")
                .ok_or(UKError::MissingAampKey("Model list missing attention"))?
                .clone(),
            model_data: pio
                .list("ModelData")
                .ok_or(UKError::MissingAampKey("Model list missing model data"))?
                .list("ModelData_0")
                .ok_or(UKError::MissingAampKey("Model list missing model data"))?
                .clone(),
            anm_target: pio
                .list("AnmTarget")
                .ok_or(UKError::MissingAampKey(
                    "Model list missing animation target",
                ))?
                .lists
                .0
                .values()
                .cloned()
                .enumerate()
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
                    ParameterList::new().with_list("ModelData_0", val.model_data),
                ),
                (
                    "AnmTarget",
                    ParameterList::new().with_lists(
                        val.anm_target.into_iter().map(|(i, target)| {
                            (jstr!("AnmTarget_{&lexical::to_string(i)}"), target)
                        }),
                    ),
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
            anm_target: simple_index_diff(&self.anm_target, &other.anm_target),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            controller_info: merge_pobj(&self.controller_info, &diff.controller_info),
            attention: merge_pobj(&self.attention, &diff.attention),
            model_data: merge_plist(&self.model_data, &diff.model_data),
            anm_target: simple_index_merge(&self.anm_target, &diff.anm_target),
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
        let merged = modellist.merge(&diff);
        assert_eq!(modellist2, merged);
    }
}
