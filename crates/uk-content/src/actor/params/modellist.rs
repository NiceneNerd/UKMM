use std::collections::BTreeMap;

use itertools::Itertools;
use join_str::jstr;
#[cfg(feature = "ui")]
use nk_ui_derive::Editable;
use nk_util::OptionResultExt;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

use crate::{
    actor::{info_params_filtered, InfoSource, ParameterResource},
    prelude::*,
    util::*,
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct ModelData {
    pub folder: String64,
    pub units:  DeleteMap<String64, String64>,
}

impl TryFrom<&ParameterList> for ModelData {
    type Error = UKError;

    fn try_from(list: &ParameterList) -> std::result::Result<Self, Self::Error> {
        let folder = list
            .object("Base")
            .ok_or_else(|| UKError::MissingAampKey("Model data missing Base object", None))?
            .get("Folder")
            .ok_or_else(|| UKError::MissingAampKey("Model data missing Folder", None))?
            .as_safe_string()?;
        let units = list
            .list("Unit")
            .map(|unit| -> Result<_> {
                unit.objects
                    .0
                    .values()
                    .map(|obj| -> Result<(String64, String64)> {
                        let name = obj
                            .get("UnitName")
                            .ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "Model data unit missing name",
                                    Some(obj.into()),
                                )
                            })?
                            .as_safe_string()?;
                        let bone = obj
                            .get("BindBone")
                            .ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "Model data unit missing bind bone",
                                    Some(obj.into()),
                                )
                            })?
                            .as_safe_string()?;
                        Ok((name, bone))
                    })
                    .collect::<Result<_>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Self { folder, units })
    }
}

impl From<ModelData> for ParameterList {
    fn from(val: ModelData) -> Self {
        ParameterList::new()
            .with_object(
                "Base",
                ParameterObject::new().with_parameter("Folder", val.folder.into()),
            )
            .with_list(
                "Unit",
                ParameterList::new().with_objects(val.units.into_iter().enumerate().map(
                    |(i, (name, bone))| {
                        (
                            format!("Unit_{}", i),
                            ParameterObject::new()
                                .with_parameter("UnitName", name.into())
                                .with_parameter("BindBone", bone.into()),
                        )
                    },
                )),
            )
    }
}

impl MergeableImpl for ModelData {
    fn diff(&self, other: &Self) -> Self {
        Self {
            folder: other.folder,
            units:  self.units.diff(&other.units),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            folder: diff.folder,
            units:  self.units.merge(&diff.units),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct ModelList {
    pub controller_info: ParameterObject,
    pub attention: ParameterObject,
    pub model_data: BTreeMap<usize, ModelData>,
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
                .enumerate()
                .map(|(i, list)| Ok((i, ModelData::try_from(list)?)))
                .collect::<Result<_>>()?,
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
                .while_some()
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
                            val.model_data.into_iter().map(|(i, list)| {
                                (jstr!("ModelData_{&lexical::to_string(i)}"), list.into())
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

impl MergeableImpl for ModelList {
    fn diff(&self, other: &Self) -> Self {
        Self {
            controller_info: diff_pobj(&self.controller_info, &other.controller_info),
            attention: diff_pobj(&self.attention, &other.attention),
            model_data: other
                .model_data
                .iter()
                .filter_map(|(i, data)| {
                    match self.model_data.get(i) {
                        Some(v) if v == data => None,
                        Some(v) if v != data => Some((*i, v.diff(data))),
                        _ => Some((*i, data.clone())),
                    }
                })
                .collect(),
            anm_target: simple_index_diff(&self.anm_target, &other.anm_target),
            locators: self.locators.diff(&other.locators),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            controller_info: merge_pobj(&self.controller_info, &diff.controller_info),
            attention: merge_pobj(&self.attention, &diff.attention),
            model_data: {
                let keys: HashSet<usize> = self
                    .model_data
                    .keys()
                    .chain(diff.model_data.keys())
                    .copied()
                    .collect();
                keys.into_iter()
                    .map(|i| {
                        match (self.model_data.get(&i), diff.model_data.get(&i)) {
                            (Some(data), Some(diff_data)) => (i, data.merge(diff_data)),
                            (Some(data), None) => (i, data.clone()),
                            (None, Some(diff_data)) => (i, diff_data.clone()),
                            _ => unreachable!(),
                        }
                    })
                    .collect()
            },
            anm_target: simple_index_merge(&self.anm_target, &diff.anm_target),
            locators: self.locators.merge(&diff.locators),
        }
    }
}

impl InfoSource for ModelList {
    fn update_info(&self, info: &mut roead::byml::Map) -> crate::Result<()> {
        info_params_filtered!(&self.attention, info, {
            ("cursorOffsetY", "CursorOffsetY", f32)
        });
        info_params_filtered!(&self.controller_info, info, {
            ("variationMatAnimFrame", "VariationMatAnimFrame", i32)
        });
        if let Some(Parameter::String64(mat_anim)) = self
            .controller_info
            .get("VariationMatAnim")
            .filter(|m| m.as_string64().map(|s| !s.is_empty()).unwrap_or(false))
        {
            info.insert("variationMatAnim".into(), mat_anim.as_str().into());
        }
        if let Some(Parameter::Vec3(lookat)) = self.attention.get("LookAtOffset") {
            info.insert("lookAtOffsetY".into(), lookat.y.into());
        }
        if let Some(add_color) = self
            .controller_info
            .get("AddColor")
            .and_then(|c| c.as_color().ok())
            .filter(|c| c.a + c.r + c.g + c.b > 0.0)
        {
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
        if let (
            Some(Parameter::Vec3(fm_center)),
            Some(Parameter::F32(fm_height)),
            Some(Parameter::F32(fm_radius)),
        ) = (
            self.controller_info.get("FarModelCullingCenter"),
            self.controller_info.get("FarModelCullingHeight"),
            self.controller_info.get("FarModelCullingRadius"),
        ) {
            if fm_center.x + fm_center.y + fm_center.z + fm_height + fm_radius > 0.0 {
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
        }
        if let Some(bfres) = self.model_data.values().next().map(|data| data.folder) {
            info.insert("bfres".into(), bfres.as_str().into());
        }
        if let Some(model) = self
            .model_data
            .values()
            .next()
            .and_then(|data| data.units.iter().next())
            .map(|d| d.0)
        {
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
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"bmodellist")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::{actor::InfoSource, prelude::*};

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
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
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
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
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/ModelList/Npc_TripMaster_00.bmodellist")
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
                .unwrap(),
        )
        .unwrap();
        let modellist = super::ModelList::try_from(&pio).unwrap();
        let mut info = roead::byml::Map::default();
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
