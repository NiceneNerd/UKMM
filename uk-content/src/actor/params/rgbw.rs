use std::{collections::HashSet, sync::Arc};

use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
use uk_ui::{editor::EditableValue, egui::mutex::RwLock, icons::IconButtonExt};

use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{params, plists, pobjs, DeleteMap, IndexMap},
    Result, UKError,
};

#[derive(
    Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Serialize, Deserialize, ParamData,
)]
pub struct Key {
    #[name = "StateKey"]
    pub state_key:  String32,
    #[name = "SystemKey"]
    pub system_key: String32,
}

impl ToString for Key {
    fn to_string(&self) -> std::string::String {
        jstr!("StateKey: {&self.state_key} :: SystemKey: {&self.system_key}")
    }
}

impl uk_ui::editor::EditableValue for Key {
    const DISPLAY: uk_ui::editor::EditableDisplay = uk_ui::editor::EditableDisplay::Inline;

    fn edit_ui(&mut self, ui: &mut uk_ui::egui::Ui) -> uk_ui::egui::Response {
        self.edit_ui_with_id(ui, "rgbw_key")
    }

    fn edit_ui_with_id(
        &mut self,
        ui: &mut uk_ui::egui::Ui,
        id: impl std::hash::Hash,
    ) -> uk_ui::egui::Response {
        let id = uk_ui::egui::Id::new(id);
        let mut changed = false;
        let mut res = ui
            .horizontal(|ui| {
                ui.label("State Key: ");
                changed = changed
                    || self
                        .state_key
                        .edit_ui_with_id(ui, id.with("state_key"))
                        .changed();
                ui.label("System Key: ");
                changed = changed
                    || self
                        .system_key
                        .edit_ui_with_id(ui, id.with("system_key"))
                        .changed();
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RagdollBlendWeight(IndexMap<Key, DeleteMap<String32, f32>>);

impl TryFrom<&ParameterIO> for RagdollBlendWeight {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self(
            pio.lists()
                .0
                .values()
                .map(|list| -> Result<(Key, DeleteMap<String32, f32>)> {
                    Ok((
                        list.object("Setting")
                            .ok_or(UKError::MissingAampKey(
                                "Ragdoll blend weight state missing header",
                                None,
                            ))?
                            .try_into()?,
                        list.list("InputWeightList")
                            .ok_or(UKError::MissingAampKey(
                                "Ragdoll blend weight state missing input weight list",
                                None,
                            ))?
                            .objects
                            .0
                            .values()
                            .map(|obj| -> Result<(String32, f32)> {
                                Ok((
                                    *obj.get("RigidName")
                                        .ok_or(UKError::MissingAampKey(
                                            "Ragdoll blend weight state input missing rigid name",
                                            None,
                                        ))?
                                        .as_string32()?,
                                    obj.get("BlendRate")
                                        .ok_or(UKError::MissingAampKey(
                                            "Ragdoll blend weight state input missing blend rate",
                                            None,
                                        ))?
                                        .as_f32()?,
                                ))
                            })
                            .collect::<Result<_>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<RagdollBlendWeight> for ParameterIO {
    fn from(val: RagdollBlendWeight) -> Self {
        Self::new().with_lists(val.0.into_iter().enumerate().map(|(idx, (key, state))| {
            (
                jstr!("State_{&lexical::to_string(idx + 1)}"),
                ParameterList {
                    objects: pobjs!("Setting" => key.into()),
                    lists:   plists!(
                        "InputWeightList" => ParameterList::new()
                            .with_objects(state.into_iter().enumerate().map(
                                |(i, (name, rate))| {
                                    (
                                        jstr!("InputWeight_{&lexical::to_string(i + 1)}"),
                                        params!(
                                            "RigidName" => Parameter::String32(name),
                                            "BlendRate" => Parameter::F32(rate),
                                        ),
                                    )
                                },
                            ))
                    ),
                },
            )
        }))
    }
}

impl Mergeable for RagdollBlendWeight {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(key, other_list)| {
                    let self_list = self.0.get(key);
                    if let Some(self_list) = self_list && other_list != self_list {
                Some((
                    key.clone(),
                    self_list.diff(other_list)
                ))
            } else if self_list.is_none() {
                Some(( key.clone(), other_list.clone() ))
            } else {
                None
            }
                })
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        let all_keys: HashSet<Key> = self.0.keys().chain(diff.0.keys()).cloned().collect();
        Self(
            all_keys
                .into_iter()
                .map(|key| {
                    (
                        key.clone(),
                        if let Some(self_list) = self.0.get(&key) && let Some(other_list) = diff.0.get(&key) {
                            self_list.merge(other_list)
                        } else {
                            diff.0
                                .get(&key)
                                .or_else(|| self.0.get(&key))
                                .cloned()
                                .unwrap()
                        },
                    )
                })
                .collect(),
        )
    }
}

impl ParameterResource for RagdollBlendWeight {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/RagdollBlendWeight/{name}.brgbw")
    }
}

impl Resource for RagdollBlendWeight {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("brgbw")
    }
}

impl EditableValue for RagdollBlendWeight {
    const DISPLAY: uk_ui::editor::EditableDisplay = uk_ui::editor::EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut uk_ui::egui::Ui) -> uk_ui::egui::Response {
        self.edit_ui_with_id(ui, "rgbw")
    }

    fn edit_ui_with_id(
        &mut self,
        ui: &mut uk_ui::egui::Ui,
        id: impl std::hash::Hash,
    ) -> uk_ui::egui::Response {
        use uk_ui::egui;
        let mut changed = false;
        let id = egui::Id::new(id);
        let res = egui::CollapsingHeader::new("RagdollBlendWeight")
            .id_source(id)
            .show(ui, |ui| {
                for (k, v) in self.0.iter_mut() {
                    egui::CollapsingHeader::new(k.to_string())
                        .id_source(id.with(k))
                        .show(ui, |ui| {
                            changed = changed
                                || v.edit_ui_with_id(ui, id.with(k).with("inner")).changed();
                        });
                }
            });
        let tmp_id = id.with("new_key");
        let mut add_new = false;
        let new_key = ui.data().get_temp::<Arc<RwLock<Key>>>(tmp_id);
        if let Some(new_key) = new_key {
            ui.horizontal(|ui| {
                new_key.write().edit_ui_with_id(ui, tmp_id.with("value"));
                if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                    add_new = true;
                }
            });
        } else if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
            ui.data()
                .insert_temp(tmp_id, Arc::new(RwLock::new(Key::default())));
        }
        if add_new {
            self.0.insert(
                ui.data()
                    .get_temp::<Arc<RwLock<Key>>>(tmp_id)
                    .expect("key should exist")
                    .read()
                    .clone(),
                Default::default(),
            );
            ui.data().remove::<Arc<RwLock<Key>>>(tmp_id);
        }
        let mut res = res.body_response.unwrap_or(res.header_response);
        if changed {
            res.mark_changed();
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(rgbw.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(data).unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        assert_eq!(rgbw, rgbw2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        let diff = rgbw.diff(&rgbw2);
        dbg!("{}", diff);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let rgbw = super::RagdollBlendWeight::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_data("Actor/RagdollBlendWeight/Moriblin.brgbw")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let rgbw2 = super::RagdollBlendWeight::try_from(&pio2).unwrap();
        let diff = rgbw.diff(&rgbw2);
        let merged = rgbw.merge(&diff);
        assert_eq!(rgbw2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Moriblin_Junior.sbactorpack//Actor/RagdollBlendWeight/\
             Moriblin.brgbw",
        );
        assert!(super::RagdollBlendWeight::path_matches(path));
    }
}
