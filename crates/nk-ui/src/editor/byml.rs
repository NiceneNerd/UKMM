use std::{hash::Hash, ops::DerefMut, sync::Arc};

use egui::{mutex::RwLock, Align, Id, Layout, Response, Ui};

use super::EditableDisplay;
use crate::{icons::IconButtonExt, visuals};

impl super::EditableValue for roead::byml::Byml {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let id = Id::new(&self);
        self.edit_ui_with_id(ui, id)
    }

    fn edit_ui_with_id(&mut self, ui: &mut Ui, id: impl Hash) -> Response {
        let id = Id::new(id);
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            let yaml = ui
                .memory()
                .data
                .get_temp_mut_or_insert_with(id, || Arc::new(RwLock::new(self.to_text())))
                .clone();
            ui.allocate_ui_with_layout(
                [
                    ui.spacing().icon_width + ui.spacing().item_spacing.x * 2.0,
                    ui.available_height(),
                ]
                .into(),
                Layout::top_down(Align::Center),
                |ui| {
                    if ui
                        .icon_button(crate::icons::Icon::Check)
                        .on_hover_text("Save")
                        .clicked()
                    {
                        match roead::byml::Byml::from_text(yaml.read().as_str()) {
                            Ok(val) => {
                                ui.memory()
                                    .data
                                    .insert_temp::<bool>(id.with("error"), false);
                                *self = val;
                            }
                            Err(_) => ui.memory().data.insert_temp(id.with("error"), true),
                        }
                    }
                    if ui
                        .icon_button(crate::icons::Icon::Cancel)
                        .on_hover_text("Reset")
                        .clicked()
                    {
                        *yaml.write() = self.to_text();
                        ui.memory()
                            .data
                            .insert_temp::<bool>(id.with("error"), false);
                    }
                },
            );
            let has_err = ui.memory().data.get_temp(id.with("error")).unwrap_or(false);
            if has_err {
                ui.visuals_mut().extreme_bg_color = visuals::error_bg(ui.visuals());
            }
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job = crate::syntect::highlight(
                    ui.ctx(),
                    &crate::syntect::CodeTheme::dark(),
                    string,
                    "yaml",
                );
                layout_job.wrap.max_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };
            let res = egui::TextEdit::multiline(yaml.write().deref_mut())
                .layouter(&mut layouter)
                .code_editor()
                .desired_width(ui.available_width())
                .show(ui);
            if has_err {
                res.response.on_hover_text_at_pointer("Invalid YAML")
            } else {
                res.response
            }
        })
        .inner
    }
}
