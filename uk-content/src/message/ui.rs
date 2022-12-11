use std::ops::DerefMut;

use ::msyt::model::MsbtInfo;
use uk_ui::{editor::*, egui, ext::UiExt, icons::IconButtonExt};

use super::MessagePack;

impl EditableValue for MessagePack {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "message_pack")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let res = egui::CollapsingHeader::new("MessagePack")
            .id_source(id)
            .show(ui, |ui| {
                self.0.iter_mut().for_each(|(file, msyt)| {
                    egui::CollapsingHeader::new(file.as_str())
                        .id_source(id.with(file.as_str()))
                        .show(ui, |ui| {
                            changed |= msyt
                                .edit_ui_with_id(ui, id.with(file).with("inner"))
                                .changed();
                        });
                });
                let tmp_id = id.with("new_key");
                let mut clear_tmp = false;
                if let Some(new_key) = ui.get_temp_string(tmp_id) {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(new_key.write().deref_mut());
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            self.0.insert(new_key.read().as_str().into(), ::msyt::Msyt {
                                msbt:    MsbtInfo {
                                    group_count: 0,
                                    atr1_unknown: None,
                                    ato1: None,
                                    tsy1: None,
                                    nli1: None,
                                },
                                entries: Default::default(),
                            });
                            clear_tmp = true;
                        }
                    });
                } else if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                    ui.create_temp_string(tmp_id, None);
                }
                if clear_tmp {
                    ui.clear_temp_string(tmp_id);
                }
            });
        let mut res = res.body_response.unwrap_or(res.header_response);
        if changed {
            res.mark_changed();
        }
        res
    }
}
