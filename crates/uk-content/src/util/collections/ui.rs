use std::{ops::DerefMut, str::FromStr, sync::Arc};

use uk_ui::{
    editor::{EditableDisplay, EditableValue},
    egui::{self, mutex::RwLock, Layout, RichText},
    ext::UiExt,
    icons::IconButtonExt,
};

use super::*;

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
                                    changed |= val.edit_ui_with_id(ui, id.with(i)).changed();
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
        if changed | do_add {
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
                    changed |= ui.checkbox(del, format!("{:#?}", val)).changed();
                }
                let new_value = ui.data(|d| d.get_temp::<Arc<RwLock<String>>>(id.with("new_val")));
                if let Some(new_value) = new_value {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_value.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            if let Ok(val) = T::try_from(new_value.read().as_str()) {
                                self.0.insert(val, false);
                                ui.data_mut(|d| d.remove::<Arc<RwLock<String>>>(id.with("new_val")))
                            }
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                    ui.data_mut(|d| {
                        d.insert_temp(id.with("new_val"), Arc::new(RwLock::new(String::new())))
                    });
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
    T: std::fmt::Display + for<'a> TryFrom<&'a str> + Default + DeleteKey + Ord,
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
                    let mut label = RichText::new(format!("{}", val));
                    if *del {
                        label = label.color(uk_ui::visuals::RED);
                    }
                    changed |= ui
                        .checkbox(del, label)
                        .on_hover_text(if *del {
                            "Uncheck to restore"
                        } else {
                            "Check to delete"
                        })
                        .changed();
                }
                let new_value = ui.data(|d| d.get_temp::<Arc<RwLock<String>>>(id.with("new_val")));
                if let Some(new_value) = new_value {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_value.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            if let Ok(val) = T::try_from(new_value.read().as_str()) {
                                self.0.insert(val, false);
                                ui.data_mut(|d| d.remove::<Arc<RwLock<String>>>(id.with("new_val")))
                            }
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                    ui.data_mut(|d| {
                        d.insert_temp(id.with("new_val"), Arc::new(RwLock::new(String::new())))
                    });
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
    U: PartialEq + Clone + EditableValue + Default,
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
                                ui.add_enabled_ui(!*del, |ui| {
                                    let res =
                                        val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                    changed |= res.changed();
                                    max_height = res.rect.height();
                                });
                            });
                        }
                        EditableDisplay::Inline => {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), ui.spacing().interact_size.y].into(),
                                Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let gallery = ui.fonts(|f| {
                                        f.layout_no_wrap(
                                            str_key,
                                            ui.style()
                                                .text_styles
                                                .get(&egui::TextStyle::Body)
                                                .expect("Bad egui config")
                                                .clone(),
                                            if *del {
                                                ui.visuals().error_fg_color
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        )
                                    });
                                    changed = changed
                                        || ui
                                            .checkbox(del, "")
                                            .on_hover_text(if *del {
                                                "Uncheck to restore"
                                            } else {
                                                "Mark for delete"
                                            })
                                            .changed();
                                    ui.add_enabled_ui(!*del, |ui| {
                                        let res = val.edit_ui_with_id(ui, id.with(key));
                                        changed |= res.changed();
                                    });
                                    ui.allocate_space(
                                        [
                                            ui.available_width()
                                                - gallery.rect.width()
                                                - ui.spacing().item_spacing.x,
                                            0.0,
                                        ]
                                        .into(),
                                    );
                                    ui.label(gallery);
                                },
                            );
                        }
                    }
                }
                if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            if let Ok(k) = <&str as TryInto<T>>::try_into(new_key.read().as_str()) {
                                self.0.insert(k, (U::default(), false));
                                ui.clear_temp_string(id.with("new_key"));
                            }
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
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

impl<T, U> EditableValue for SortedDeleteMap<T, U>
where
    T: std::fmt::Debug + DeleteKey + FromStr + Ord,
    U: PartialEq + Clone + EditableValue + Default,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "sorted_delete_map")
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
                                ui.add_enabled_ui(!*del, |ui| {
                                    let res =
                                        val.edit_ui_with_id(ui, id.with(key).with("child-ui"));
                                    changed |= res.changed();
                                    max_height = res.rect.height();
                                });
                            });
                        }
                        EditableDisplay::Inline => {
                            ui.allocate_ui_with_layout(
                                [ui.available_width(), ui.spacing().interact_size.y].into(),
                                Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let gallery = ui.fonts(|f| {
                                        f.layout_no_wrap(
                                            str_key,
                                            ui.style()
                                                .text_styles
                                                .get(&egui::TextStyle::Body)
                                                .expect("Bad egui config")
                                                .clone(),
                                            if *del {
                                                ui.visuals().error_fg_color
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        )
                                    });
                                    changed = changed
                                        || ui
                                            .checkbox(del, "")
                                            .on_hover_text(if *del {
                                                "Uncheck to restore"
                                            } else {
                                                "Mark for delete"
                                            })
                                            .changed();
                                    ui.add_enabled_ui(!*del, |ui| {
                                        let res = val.edit_ui_with_id(ui, id.with(key));
                                        changed |= res.changed();
                                    });
                                    ui.allocate_space(
                                        [
                                            ui.available_width()
                                                - gallery.rect.width()
                                                - ui.spacing().item_spacing.x,
                                            0.0,
                                        ]
                                        .into(),
                                    );
                                    ui.label(gallery);
                                },
                            );
                        }
                    }
                }
                if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            if let Ok(k) = new_key.read().as_str().parse() {
                                self.0.insert(k, (U::default(), false));
                                ui.clear_temp_string(id.with("new_key"));
                            }
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
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
