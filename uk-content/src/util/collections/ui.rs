use std::ops::DerefMut;
use std::sync::Arc;

use super::*;
use uk_ui::editor::{EditableDisplay, EditableValue};
use uk_ui::egui;
use uk_ui::egui::mutex::RwLock;
use uk_ui::egui_extras;
use uk_ui::icons::IconButtonExt;

impl<T: Default + EditableValue + Clone + PartialEq> EditableValue for DeleteVec<T> {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "delete-vec")
    }
    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut do_add = false;
        let mut res = ui
            .group(|ui| {
                for (i, (val, del)) in self.0.iter_mut().enumerate() {
                    egui::Frame::none()
                        .fill(if *del {
                            uk_ui::visuals::error_bg(ui.visuals())
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .show(ui, |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                changed = changed
                                    || ui
                                        .checkbox(del, "")
                                        .on_hover_text(if *del {
                                            "Uncheck to restore"
                                        } else {
                                            "Check to delete"
                                        })
                                        .changed();
                                ui.scope(|ui| {
                                    changed =
                                        changed || val.edit_ui_with_id(ui, id.with(i)).changed();
                                });
                            });
                        });
                }
                do_add = ui.icon_button(uk_ui::icons::Icon::Add).clicked();
            })
            .response;
        if do_add {
            self.0.push((T::default(), false));
        }
        if changed || do_add {
            res.mark_changed();
        }
        res
    }
}

impl<T> EditableValue for DeleteSet<T>
where
    T: std::fmt::Debug + for<'a> TryFrom<&'a str> + Default + DeleteKey,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "delete_set")
    }
    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut res = ui
            .group(|ui| {
                for (val, del) in self.0.iter_mut() {
                    changed = changed || ui.checkbox(del, format!("{:#?}", val)).changed();
                }
                let new_value = ui
                    .data()
                    .get_temp::<Arc<RwLock<String>>>(id.with("new_val"));
                if let Some(new_value) = new_value {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_value.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            if let Ok(val) = T::try_from(new_value.read().as_str()) {
                                self.0.insert(val, false);
                                ui.data().remove::<Arc<RwLock<String>>>(id.with("new_val"))
                            }
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                    ui.data()
                        .insert_temp(id.with("new_val"), Arc::new(RwLock::new(String::new())));
                }
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

impl<T> EditableValue for SortedDeleteSet<T>
where
    T: std::fmt::Debug + for<'a> TryFrom<&'a str> + Default + DeleteKey + Ord,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "delete_set")
    }
    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut res = ui
            .group(|ui| {
                for (val, del) in self.0.iter_mut() {
                    changed = changed || ui.checkbox(del, format!("{:#?}", val)).changed();
                }
                let new_value = ui
                    .data()
                    .get_temp::<Arc<RwLock<String>>>(id.with("new_val"));
                if let Some(new_value) = new_value {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_value.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            if let Ok(val) = T::try_from(new_value.read().as_str()) {
                                self.0.insert(val, false);
                                ui.data().remove::<Arc<RwLock<String>>>(id.with("new_val"))
                            }
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                    ui.data()
                        .insert_temp(id.with("new_val"), Arc::new(RwLock::new(String::new())));
                }
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

impl<T, U> EditableValue for DeleteMap<T, U>
where
    T: std::fmt::Debug + DeleteKey + for<'a> TryFrom<&'a str>,
    U: PartialEq + Clone + EditableValue,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "delete_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut max_height = ui.spacing().interact_size.y;
        let mut res = ui
            .scope(|ui| {
                ui.allocate_space([ui.available_width(), 0.0].into());
                for (key, (val, del)) in self.0.iter_mut() {
                    let str_key = format!("{:#?}", &key).trim_matches('"').to_owned();
                    match <U as EditableValue>::DISPLAY {
                        EditableDisplay::Block => {
                            egui::CollapsingHeader::new(if *del {
                                egui::RichText::new(&str_key).color(uk_ui::visuals::RED)
                            } else {
                                egui::RichText::new(&str_key)
                            })
                            .id_source(id.with(key))
                            .show(ui, |ui| {
                                changed = changed
                                    || ui
                                        .checkbox(
                                            del,
                                            if *del {
                                                egui::RichText::new("Delete")
                                                    .color(uk_ui::visuals::RED)
                                            } else {
                                                egui::RichText::new("Delete")
                                            },
                                        )
                                        .changed();
                                let res = val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                changed = changed || res.changed();
                                max_height = res.rect.height();
                            });
                        }
                        EditableDisplay::Inline => {
                            let label_width = ui.available_width() / 4.0;
                            let chk_width = ui.spacing().interact_size.x;
                            egui_extras::StripBuilder::new(ui)
                                .size(egui_extras::Size::initial(30.0).at_most(label_width))
                                .size(egui_extras::Size::remainder())
                                .size(egui_extras::Size::exact(chk_width))
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        ui.label(&str_key);
                                    });
                                    strip.cell(|ui| {
                                        let res = val.edit_ui_with_id(ui, id.with(key));
                                        changed = changed || res.changed();
                                        max_height = res.rect.height();
                                    });
                                    strip.cell(|ui| {
                                        changed = changed
                                            || ui
                                                .checkbox(del, "")
                                                .on_hover_text(if *del {
                                                    "Check to restore"
                                                } else {
                                                    "Uncheck to delete"
                                                })
                                                .changed();
                                    });
                                });
                        }
                    }
                }
            })
            .response;
        ui.set_max_height(10.0);
        if changed {
            res.mark_changed();
        }
        res
    }
}
