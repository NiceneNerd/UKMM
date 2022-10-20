use super::*;
use uk_ui::editor::{EditableDisplay, EditableValue};
use uk_ui::egui;
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
                            egui::Color32::DARK_RED
                        } else {
                            ui.style().noninteractive().bg_fill
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
                            do_add = ui.icon_button(uk_ui::icons::Icon::Add).clicked();
                        });
                }
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

impl<T: std::fmt::Debug + Into<String> + Default + Clone + PartialEq + Hash + Eq> EditableValue
    for DeleteSet<T>
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
                for (i, (val, del)) in self.0.iter_mut() {
                    changed = changed || ui.checkbox(del, format!("{:#?}", val)).changed();
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {}
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}
