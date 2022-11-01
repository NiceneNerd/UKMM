use roead::byml::Byml;
use std::ops::{Deref, DerefMut};
use uk_ui::{
    editor::{EditableDisplay, EditableValue},
    egui,
    egui_extras::{self, Size},
    ext::UiExt,
    icons::IconButtonExt,
};

impl EditableValue for super::SaveData {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "save_data")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let mut changed = false;
        let id = egui::Id::new(id);
        let res = egui::CollapsingHeader::new("SaveData")
            .id_source(id)
            .show(ui, |ui| {
                let row_height = ui.spacing().interact_size.y;
                let len = self.flags.len();
                egui::ScrollArea::new([false, true]).show_rows(ui, row_height, len, |ui, range| {
                    self.flags
                        .iter_full_mut()
                        .skip(range.start)
                        .take(range.end - range.start)
                        .for_each(|(flag, del)| {
                            ui.horizontal(|ui| {
                                if *del {
                                    ui.visuals_mut().override_text_color =
                                        Some(uk_ui::visuals::RED);
                                }
                                ui.label(flag.name.as_str());
                                changed |= ui
                                    .checkbox(del, if *del { "Disabled" } else { "Enabled" })
                                    .changed();
                            });
                        });
                });
            });
        let tmp_id = id.with("new_flag");
        let mut clear_new = false;
        if let Some(new_flag) = ui.get_temp_string(tmp_id) {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(new_flag.write().deref_mut());
                if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                    self.flags
                        .insert(super::Flag::from(new_flag.read().as_str()));
                    clear_new = true;
                }
            });
        } else if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
            ui.create_temp_string(tmp_id, None);
        }
        if clear_new {
            ui.clear_temp_string(tmp_id);
        }
        let mut res = res.body_response.unwrap_or(res.header_response);
        if changed {
            res.mark_changed();
        }
        res
    }
}
