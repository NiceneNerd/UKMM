use crate::{ext::UiExt, icons::IconButtonExt};

use super::{EditableDisplay, EditableValue};
use egui::Layout;
use indexmap::map::IndexMap;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    hash::{BuildHasher, Hash},
    ops::DerefMut,
    str::FromStr,
};

impl<T, U, S> EditableValue for IndexMap<T, U, S>
where
    T: std::fmt::Display + Eq + Hash + for<'a> TryFrom<&'a str>,
    U: PartialEq + Clone + EditableValue + Default,
    S: BuildHasher,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "index_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut max_height = ui.spacing().interact_size.y;
        let mut res = ui
            .scope(|ui| {
                ui.allocate_space([ui.available_width(), 0.0].into());
                for (key, val) in self.iter_mut() {
                    let str_key = format!("{}", &key).trim_matches('"').to_owned();
                    match <U as EditableValue>::DISPLAY {
                        EditableDisplay::Block => {
                            egui::CollapsingHeader::new(&str_key)
                                .id_source(id.with(key))
                                .show(ui, |ui| {
                                    let res =
                                        val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                    changed |= res.changed();
                                    max_height = res.rect.height();
                                });
                        }
                        EditableDisplay::Inline => {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), ui.spacing().interact_size.y].into(),
                                Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let gallery = ui.fonts().layout_no_wrap(
                                        str_key.clone(),
                                        ui.style()
                                            .text_styles
                                            .get(&egui::TextStyle::Body)
                                            .unwrap()
                                            .clone(),
                                        ui.visuals().text_color(),
                                    );
                                    let res = val.edit_ui_with_id(ui, id.with(key));
                                    changed |= res.changed();
                                    ui.allocate_space(
                                        [
                                            ui.available_width()
                                                - gallery.rect.width()
                                                - ui.spacing().item_spacing.x,
                                            0.0,
                                        ]
                                        .into(),
                                    );
                                    ui.label(str_key);
                                },
                            );
                        }
                    }
                }
                if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(crate::icons::Icon::Check).clicked() {
                            if let Ok(k) = <&str as TryInto<T>>::try_into(new_key.read().as_str()) {
                                self.insert(k, U::default());
                                ui.clear_temp_string(id.with("new_key"));
                            }
                        }
                    });
                }
                if ui.icon_button(crate::icons::Icon::Add).clicked() {
                    ui.create_temp_string(id.with("new_key"), None);
                }
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

impl<T, U> EditableValue for BTreeMap<T, U>
where
    T: FromStr + Display + Debug + Hash + Eq + Ord,
    U: PartialEq + Clone + EditableValue + Default,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "btree_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut max_height = ui.spacing().interact_size.y;
        let mut res = ui
            .scope(|ui| {
                ui.allocate_space([ui.available_width(), 0.0].into());
                for (key, val) in self.iter_mut() {
                    let str_key = format!("{}", &key).trim_matches('"').to_owned();
                    match <U as EditableValue>::DISPLAY {
                        EditableDisplay::Block => {
                            egui::CollapsingHeader::new(&str_key)
                                .id_source(id.with(key))
                                .show(ui, |ui| {
                                    let res =
                                        val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                    changed |= res.changed();
                                    max_height = res.rect.height();
                                });
                        }
                        EditableDisplay::Inline => {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), ui.spacing().interact_size.y].into(),
                                Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let gallery = ui.fonts().layout_no_wrap(
                                        str_key.clone(),
                                        ui.style()
                                            .text_styles
                                            .get(&egui::TextStyle::Body)
                                            .unwrap()
                                            .clone(),
                                        ui.visuals().text_color(),
                                    );
                                    let res = val.edit_ui_with_id(ui, id.with(key));
                                    changed |= res.changed();
                                    ui.allocate_space(
                                        [
                                            ui.available_width()
                                                - gallery.rect.width()
                                                - ui.spacing().item_spacing.x,
                                            0.0,
                                        ]
                                        .into(),
                                    );
                                    ui.label(str_key);
                                },
                            );
                        }
                    }
                }
                if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(crate::icons::Icon::Check).clicked() {
                            if let Ok(k) = new_key.read().as_str().parse() {
                                self.insert(k, U::default());
                                ui.clear_temp_string(id.with("new_key"));
                            }
                        }
                    });
                }
                if ui.icon_button(crate::icons::Icon::Add).clicked() {
                    ui.create_temp_string(id.with("new_key"), None);
                }
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

pub fn edit_int_map<U>(
    map: &mut BTreeMap<usize, U>,
    ui: &mut egui::Ui,
    id: impl Hash,
) -> egui::Response
where
    U: PartialEq + Clone + EditableValue + Default,
{
    let id = egui::Id::new(id);
    let mut changed = false;
    let mut max_height = ui.spacing().interact_size.y;
    let mut res = ui
        .scope(|ui| {
            ui.allocate_space([ui.available_width(), 0.0].into());
            for (key, val) in map.iter_mut() {
                let str_key = format!("{}", &key).trim_matches('"').to_owned();
                match <U as EditableValue>::DISPLAY {
                    EditableDisplay::Block => {
                        egui::CollapsingHeader::new(&str_key)
                            .id_source(id.with(key))
                            .show(ui, |ui| {
                                let res = val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                changed |= res.changed();
                                max_height = res.rect.height();
                            });
                    }
                    EditableDisplay::Inline => {
                        ui.allocate_ui_with_layout(
                            [ui.available_width(), ui.spacing().interact_size.y].into(),
                            Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let gallery = ui.fonts().layout_no_wrap(
                                    str_key.clone(),
                                    ui.style()
                                        .text_styles
                                        .get(&egui::TextStyle::Body)
                                        .unwrap()
                                        .clone(),
                                    ui.visuals().text_color(),
                                );
                                let res = val.edit_ui_with_id(ui, id.with(key));
                                changed |= res.changed();
                                ui.allocate_space(
                                    [
                                        ui.available_width()
                                            - gallery.rect.width()
                                            - ui.spacing().item_spacing.x,
                                        0.0,
                                    ]
                                    .into(),
                                );
                                ui.label(str_key);
                            },
                        );
                    }
                }
            }
            if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(new_key.write().deref_mut());
                    if ui.icon_button(crate::icons::Icon::Check).clicked() {
                        if let Ok(k) = new_key.read().as_str().parse() {
                            map.insert(k, U::default());
                            ui.clear_temp_string(id.with("new_key"));
                        }
                    }
                });
            }
            if ui.icon_button(crate::icons::Icon::Add).clicked() {
                ui.create_temp_string(id.with("new_key"), None);
            }
        })
        .response;
    if changed {
        res.mark_changed();
    }
    res
}

impl<T, U, S> EditableValue for std::collections::hash_map::HashMap<T, U, S>
where
    T: std::fmt::Debug + Eq + Hash + for<'a> TryFrom<&'a str>,
    U: PartialEq + Clone + EditableValue + Default,
    S: BuildHasher,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "hash_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut max_height = ui.spacing().interact_size.y;
        let mut res = ui
            .scope(|ui| {
                ui.allocate_space([ui.available_width(), 0.0].into());
                for (key, val) in self.iter_mut() {
                    let str_key = format!("{:?}", &key).trim_matches('"').to_owned();
                    match <U as EditableValue>::DISPLAY {
                        EditableDisplay::Block => {
                            egui::CollapsingHeader::new(&str_key)
                                .id_source(id.with(key))
                                .show(ui, |ui| {
                                    let res =
                                        val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                    changed |= res.changed();
                                    max_height = res.rect.height();
                                });
                        }
                        EditableDisplay::Inline => {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), ui.spacing().interact_size.y].into(),
                                Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let gallery = ui.fonts().layout_no_wrap(
                                        str_key.clone(),
                                        ui.style()
                                            .text_styles
                                            .get(&egui::TextStyle::Body)
                                            .unwrap()
                                            .clone(),
                                        ui.visuals().text_color(),
                                    );
                                    let res = val.edit_ui_with_id(ui, id.with(key));
                                    changed |= res.changed();
                                    ui.allocate_space(
                                        [
                                            ui.available_width()
                                                - gallery.rect.width()
                                                - ui.spacing().item_spacing.x,
                                            0.0,
                                        ]
                                        .into(),
                                    );
                                    ui.label(str_key);
                                },
                            );
                        }
                    }
                }
                if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(crate::icons::Icon::Check).clicked() {
                            if let Ok(k) = <&str as TryInto<T>>::try_into(new_key.read().as_str()) {
                                self.insert(k, U::default());
                                ui.clear_temp_string(id.with("new_key"));
                            }
                        }
                    });
                }
                if ui.icon_button(crate::icons::Icon::Add).clicked() {
                    ui.create_temp_string(id.with("new_key"), None);
                }
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}
