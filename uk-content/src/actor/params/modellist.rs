use crate::{
    actor::{info_params_filtered, InfoSource, ParameterResource},
    prelude::*,
    util::*,
    Result, UKError,
};
use join_str::jstr;
use roead::{aamp::*, byml::Byml};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelList {
    pub controller_info: ParameterObject,
    pub attention: ParameterObject,
    pub model_data: DeleteVec<ParameterList>,
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
                .lists
                .0
                .values()
                .cloned()
                .collect(),
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
                    ParameterList::new().with_lists(
                        val.model_data
                            .into_iter()
                            .enumerate()
                            .map(|(i, list)| (jstr!("ModelData_{&lexical::to_string(i)}"), list)),
                    ),
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

impl Mergeable for ModelList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            controller_info: diff_pobj(&self.controller_info, &other.controller_info),
            attention: diff_pobj(&self.attention, &other.attention),
            model_data: self.model_data.diff(&other.model_data),
            anm_target: simple_index_diff(&self.anm_target, &other.anm_target),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            controller_info: merge_pobj(&self.controller_info, &diff.controller_info),
            attention: merge_pobj(&self.attention, &diff.attention),
            model_data: self.model_data.merge(&diff.model_data),
            anm_target: simple_index_merge(&self.anm_target, &diff.anm_target),
        }
    }
}

impl InfoSource for ModelList {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()> {
        info_params_filtered!(&self.attention, info, {
            ("cursorOffsetY", "CursorOffsetY", f32)
        });
        info_params_filtered!(&self.controller_info, info, {
            ("variationMatAnim", "VariationMatAnim", String),
            ("variationMatAnimFrame", "VariationMatAnimFrame", i32)
        });
        if let Some(Parameter::Vec3(lookat)) = self.attention.param("LookAtOffset") {
            info.insert("lookAtOffsetY".to_string(), lookat.y.into());
        }
        if let Some(Parameter::Color(add_color)) = self.controller_info.param("AddColor") && add_color.a + add_color.r + add_color.g + add_color.b > 0.0 {
            info.insert("addColorR".to_string(), add_color.r.into());
            info.insert("addColorG".to_string(), add_color.g.into());
            info.insert("addColorB".to_string(), add_color.b.into());
            info.insert("addColorA".to_string(), add_color.a.into());
        }
        if let Some(Parameter::Vec3(base_scale)) = self.controller_info.param("BaseScale") {
            info.insert("baseScaleX".to_string(), base_scale.x.into());
            info.insert("baseScaleY".to_string(), base_scale.y.into());
            info.insert("baseScaleZ".to_string(), base_scale.z.into());
        }
        if let Some(Parameter::Vec3(fm_center)) = self.controller_info.param("FarModelCullingCenter")
            && let Some(Parameter::F32(fm_height)) = self.controller_info.param("FarModelCullingHeight")
            && let Some(Parameter::F32(fm_radius)) = self.controller_info.param("FarModelCullingRadius")
            && fm_center.x + fm_center.y + fm_center.z + fm_height + fm_radius > 0.0
        {
            info.insert(
                "farModelCulling".to_owned(),
                [
                    (
                        "center",
                        [
                            ("X", fm_center.x.into()),
                            ("Y", fm_center.y.into()),
                            ("Z", fm_center.z.into()),
                        ]
                        .into_iter()
                        .collect::<Byml>(),
                    ),
                    ("height", (*fm_height).into()),
                    ("radius", (*fm_radius).into()),
                ]
                .into_iter()
                .collect::<Byml>(),
            );
        }
        if let Some(Parameter::String64(bfres)) = self
            .model_data
            .get(0)
            .and_then(|list| list.object("Base").and_then(|o| o.param("Folder")))
        {
            info.insert("bfres".to_owned(), bfres.clone().into());
        }
        if let Some(Parameter::String64(model)) = self.model_data.get(0).and_then(|list| {
            list.list("Unit")
                .and_then(|list| list.object("Unit_0").and_then(|obj| obj.param("UnitName")))
        }) {
            info.insert("mainModel".to_owned(), model.clone().into());
        }
        Ok(())
    }
}

impl ParameterResource for ModelList {
    fn path(name: &str) -> String {
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
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(modellist.clone()).to_binary();
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

    #[test]
    fn info() {
        use roead::byml::Byml;
        let actor = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let mut info = roead::byml::Hash::new();
        modellist.update_info(&mut info).unwrap();
        assert_eq!(info["cursorOffsetY"], Byml::Float(0.7));
        assert_eq!(info["baseScaleY"], Byml::Float(1.0));
        assert_eq!(
            info["mainModel"],
            Byml::String("Npc_Hylia_Jonathan".to_owned())
        );
        assert!(!info.contains_key("variationMatAnim"));
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Npc_TripMaster_00.sbactorpack//Actor/ModelList/Npc_TripMaster_00.bmodellist",
        );
        assert!(super::ModelList::path_matches(path));
    }
}
