use std::collections::BTreeMap;

use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_ui_derive::Editable;

use crate::{
    actor::{info_params_filtered, InfoSource, ParameterResource},
    prelude::*,
    util::*,
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Editable)]
pub struct ModelList {
    pub controller_info: ParameterObject,
    pub attention: ParameterObject,
    pub model_data: DeleteVec<ParameterList>,
    pub anm_target: BTreeMap<usize, ParameterList>,
    pub locators: DeleteVec<ParameterObject>,
}

impl TryFrom<&ParameterIO> for ModelList {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            controller_info: pio
                .object("ControllerInfo")
                .ok_or(UKError::MissingAampKey(
                    "Model list missing controller info",
                    None,
                ))?
                .clone(),
            attention: pio
                .object("Attention")
                .ok_or(UKError::MissingAampKey(
                    "Model list missing attention",
                    None,
                ))?
                .clone(),
            model_data: pio
                .list("ModelData")
                .ok_or(UKError::MissingAampKey(
                    "Model list missing model data",
                    None,
                ))?
                .lists
                .0
                .values()
                .cloned()
                .collect(),
            anm_target: pio
                .list("AnmTarget")
                .ok_or(UKError::MissingAampKey(
                    "Model list missing animation target",
                    None,
                ))?
                .lists
                .0
                .values()
                .cloned()
                .enumerate()
                .collect(),
            locators: (0..)
                .map(|i| pio.object(format!("Locator_{}", i)).cloned())
                .fuse()
                .filter_map(|v| v)
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
            param_root: ParameterList {
                objects: pobjs!(
                    "ControllerInfo" => val.controller_info,
                    "Attention" => val.attention,
                ),
                lists:   plists!(
                    "ModelData" => ParameterList::new()
                        .with_lists(
                            val.model_data.into_iter().enumerate().map(|(i, list)| {
                                (jstr!("ModelData_{&lexical::to_string(i)}"), list)
                            }),
                        ),
                    "AnmTarget" => ParameterList::new()
                        .with_lists(val.anm_target.into_iter().map(
                            |(i, target)| (jstr!("AnmTarget_{&lexical::to_string(i)}"), target),
                        )),
                ),
            }
            .with_objects(
                val.locators
                    .into_iter()
                    .enumerate()
                    .map(|(i, obj)| (jstr!("Locator_{&lexical::to_string(i)}"), obj)),
            ),
            version:    0,
            data_type:  "xml".into(),
        }
    }
}

impl Mergeable for ModelList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            controller_info: diff_pobj(&self.controller_info, &other.controller_info),
            attention: diff_pobj(&self.attention, &other.attention),
            model_data: self.model_data.diff(&other.model_data),
            anm_target: simple_index_diff(&self.anm_target, &other.anm_target),
            locators: self.locators.diff(&other.locators),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            controller_info: merge_pobj(&self.controller_info, &diff.controller_info),
            attention: merge_pobj(&self.attention, &diff.attention),
            model_data: self.model_data.merge(&diff.model_data),
            anm_target: simple_index_merge(&self.anm_target, &diff.anm_target),
            locators: self.locators.merge(&diff.locators),
        }
    }
}

impl InfoSource for ModelList {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        info_params_filtered!(&self.attention, info, {
            ("cursorOffsetY", "CursorOffsetY", f32)
        });
        info_params_filtered!(&self.controller_info, info, {
            ("variationMatAnimFrame", "VariationMatAnimFrame", i32)
        });
        if let Some(Parameter::String64(mat_anim)) = self.controller_info.get("VariationMatAnim")
            && !mat_anim.is_empty()
        {
            info.insert("variationMatAnim".into(), mat_anim.as_str().into());
        }
        if let Some(Parameter::Vec3(lookat)) = self.attention.get("LookAtOffset") {
            info.insert("lookAtOffsetY".into(), lookat.y.into());
        }
        if let Some(Parameter::Color(add_color)) = self.controller_info.get("AddColor") && add_color.a + add_color.r + add_color.g + add_color.b > 0.0 {
            info.insert("addColorR".into(), add_color.r.into());
            info.insert("addColorG".into(), add_color.g.into());
            info.insert("addColorB".into(), add_color.b.into());
            info.insert("addColorA".into(), add_color.a.into());
        }
        if let Some(Parameter::Vec3(base_scale)) = self.controller_info.get("BaseScale") {
            info.insert("baseScaleX".into(), base_scale.x.into());
            info.insert("baseScaleY".into(), base_scale.y.into());
            info.insert("baseScaleZ".into(), base_scale.z.into());
        }
        if let Some(Parameter::Vec3(fm_center)) = self.controller_info.get("FarModelCullingCenter")
            && let Some(Parameter::F32(fm_height)) = self.controller_info.get("FarModelCullingHeight")
            && let Some(Parameter::F32(fm_radius)) = self.controller_info.get("FarModelCullingRadius")
            && fm_center.x + fm_center.y + fm_center.z + fm_height + fm_radius > 0.0
        {
            info.insert(
                "farModelCulling".into(),
                bhash!(
                    "center" => bhash!(
                        "X" => fm_center.x.into(),
                        "Y" => fm_center.y.into(),
                        "Z" => fm_center.z.into()
                    ),
                    "height" => (*fm_height).into(),
                    "radius" => (*fm_radius).into(),
                ),
            );
        }
        if let Some(Parameter::String64(bfres)) = self
            .model_data
            .get(0)
            .and_then(|list| list.object("Base").and_then(|o| o.get("Folder")))
        {
            info.insert("bfres".into(), bfres.as_str().into());
        }
        if let Some(Parameter::String64(model)) = self.model_data.get(0).and_then(|list| {
            list.list("Unit")
                .and_then(|list| list.object("Unit_0").and_then(|obj| obj.get("UnitName")))
        }) {
            info.insert("mainModel".into(), model.as_str().into());
        }
        Ok(())
    }
}

impl ParameterResource for ModelList {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/ModelList/{name}.bmodellist")
    }
}

impl Resource for ModelList {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bmodellist")
    }
}

#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(modellist.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let modellist2 = super::ModelList::try_from(&pio2).unwrap();
        assert_eq!(modellist, modellist2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let modellist2 = super::ModelList::try_from(&pio2).unwrap();
        let _diff = modellist.diff(&modellist2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let modellist2 = super::ModelList::try_from(&pio2).unwrap();
        let diff = modellist.diff(&modellist2);
        let merged = modellist.merge(&diff);
        assert_eq!(modellist2, merged);
    }

    #[test]
    fn info() {
        use roead::byml::Byml;
        let actor = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::default();
        modellist.update_info(&mut info).unwrap();
        assert_eq!(info["cursorOffsetY"], Byml::Float(0.7));
        assert_eq!(info["baseScaleY"], Byml::Float(1.0));
        assert_eq!(info["mainModel"], Byml::String("Npc_Hylia_Jonathan".into()));
        assert!(!info.contains_key("variationMatAnim"));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/ModelList/Npc_TripMaster_00.\
             bmodellist",
        );
        assert!(super::ModelList::path_matches(path));
    }
}
