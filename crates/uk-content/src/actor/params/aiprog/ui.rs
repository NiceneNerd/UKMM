use std::{hash::Hash, ops::DerefMut};

use uk_ui::{
    editor::{EditableDisplay, EditableValue},
    egui::{self, Layout},
    ext::UiExt,
    icons::{self, IconButtonExt},
};

use super::BehaviorMap;

impl EditableValue for BehaviorMap {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "index_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let mut res = ui
            .scope(|ui| {
                ui.allocate_space([ui.available_width(), 0.0].into());
                for (key, val) in self.0.iter_mut() {
                    let str_key = format!("{}", &key).trim_matches('"').to_owned();
                    ui.allocate_ui_with_layout(
                        [ui.available_width(), ui.spacing().interact_size.y].into(),
                        Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let gallery = ui.fonts(|f| {
                                f.layout_no_wrap(
                                    str_key.clone(),
                                    ui.style()
                                        .text_styles
                                        .get(&egui::TextStyle::Body)
                                        .expect("Bad egui config")
                                        .clone(),
                                    ui.visuals().text_color(),
                                )
                            });
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
                if let Some(new_key) = ui.get_temp_string(id.with("new_key")) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(icons::Icon::Check).clicked() {
                            if let Ok(k) = new_key.read().as_str().parse::<u32>() {
                                self.0.insert(k, Default::default());
                                ui.clear_temp_string(id.with("new_key"));
                            }
                        }
                    });
                }
                if ui.icon_button(icons::Icon::Add).clicked() {
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
