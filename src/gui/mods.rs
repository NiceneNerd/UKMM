use crate::mods::Mod;

use super::{visuals, App, Message, FocusedPane};
use egui::{
    Align, Checkbox, CursorIcon, Id, Key, Label, LayerId, Layout, Response, Sense, TextStyle, Ui,
    Vec2,
};
use egui_extras::{Size, TableBuilder, TableRow};

impl App {
    pub fn render_modlist(&mut self, ui: &mut Ui) {
        let text_height = ui.text_style_height(&TextStyle::Body) + 4.;
        TableBuilder::new(ui)
            .cell_sense(Sense::click_and_drag())
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Size::exact(16.))
            .column(Size::remainder())
            .column(Size::Absolute {
                initial: 80.,
                range: (16., 240.),
            })
            .column(Size::exact(48.))
            .column(Size::exact(48.))
            .header(text_height, |mut header| {
                header.col(|ui| {
                    ui.add_space(16.);
                });
                header.col(|ui| {
                    ui.label("Mod Name");
                });
                header.col(|ui| {
                    ui.label("Category");
                });
                header.col(|ui| {
                    ui.label("Version");
                });
                header.col(|ui| {
                    ui.label("Priority");
                });
            })
            .body(|body| {
                body.rows(text_height, self.mods.len(), |index, row| {
                    self.render_mod_row(index, row);
                });
            });
        if ui.memory().focus().is_none() && self.focused == FocusedPane::ModList {
            if ui.input().key_pressed(Key::ArrowDown)
                && let Some((last_index, _)) = self.mods.iter().enumerate().filter(|(_, m)| self.selected.contains(m)).last()
            {
                if !ui.input().modifiers.shift {
                    self.do_update(Message::SelectOnly(last_index + 1));
                } else {
                    self.do_update(Message::SelectAlso(last_index + 1));
                }
            } else if ui.input().key_pressed(Key::ArrowUp)
                && let Some((first_index, _)) = self.mods.iter().enumerate().find(|(_, m)| self.selected.contains(m))
            {
                let index = first_index.max(1);
                if !ui.input().modifiers.shift {
                    self.do_update(Message::SelectOnly(index - 1));
                } else {
                    self.do_update(Message::SelectAlso(index - 1));
                }
            }
        } 
        self.render_drag_state(text_height, ui);
        if ui.input().pointer.any_released() {
            if let Some(start_index) = self.drag_index
                && let Some(dest_index) = self.hover_index
                && start_index != dest_index
            {
                self.do_update(Message::MoveSelected(dest_index))
            } else {
                self.do_update(Message::ClearDrag);
            }
            ui.output().cursor_icon = CursorIcon::Default;
        }
    }

    fn render_drag_state(&mut self, text_height: f32, ui: &mut Ui) {
        let being_dragged = ui.memory().is_anything_being_dragged();
        if being_dragged && let Some(drag_index) = self.drag_index {
            ui.output().cursor_icon = CursorIcon::Grabbing;
            let layer_id = LayerId::new(egui::Order::Tooltip, Id::new("mod_list").with(drag_index));
            let res = ui.with_layer_id(layer_id, |ui| {
                TableBuilder::new(ui).column(Size::exact(16.))
                .column(Size::remainder())
                .column(Size::Absolute {
                    initial: 80.,
                    range: (16., 240.),
                })
                .column(Size::exact(48.))
                .column(Size::exact(48.)).body(|body| {
                    body.rows(text_height, self.selected.len(), |index, mut row| {
                        let mod_ = &self.selected[index];
                        let mut enabled = mod_.enabled;
                        row.col(|ui| {
                            ui.checkbox(&mut enabled, "");
                        });
                        for label in [
                            mod_.meta.name.as_str(),
                            mod_.meta.category.as_str(),
                            mod_.meta.version.to_string().as_str(),
                            self.mods.iter().position(|m| m == mod_).unwrap().to_string().as_str(),
                        ] {
                            row.col(|ui| {
                                ui.label(label);
                            });
                        };
                    });
                });
            }).response;
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos.y - res.rect.center().y;
                ui.ctx().translate_layer(layer_id, Vec2::new(0.0, delta));
                
            }
        }
    }

    fn render_mod_row(&mut self, index: usize, mut row: TableRow) {
        let mod_ = unsafe { self.mods.get_mut(index).unwrap_unchecked() };
        let selected = self.selected.contains(mod_);
        let mut clicked = false;
        let mut drag_started = false;
        let mut ctrl = false;
        let mut hover = false;

        let mut process_col_res = |res: Response| {
            clicked = clicked || res.clicked();
            hover = hover || res.hovered();
            drag_started = drag_started || res.drag_started();
        };

        if selected {
            row = row.selected(true);
        }
        process_col_res(row.col(|ui| {
            ui.checkbox(&mut mod_.enabled, "");
            ctrl = ui.input().modifiers.ctrl;
        }));
        for label in [
            mod_.meta.name.as_str(),
            mod_.meta.category.as_str(),
            mod_.meta.version.to_string().as_str(),
            index.to_string().as_str(),
        ] {
            process_col_res(row.col(|ui| {
                ui.label(label);
            }));
        }
        if clicked {
            self.do_update(Message::SetFocus(FocusedPane::ModList));
            if selected && ctrl {
                self.do_update(Message::Deselect(index));
            } else if ctrl {
                self.do_update(Message::SelectAlso(index));
            } else {
                self.do_update(Message::SelectOnly(index));
            }
        } else if drag_started {
            if !ctrl {
                self.do_update(Message::StartDrag(index));
            }
        } else if self.drag_index != Some(index) && hover {
            self.hover_index = Some(index);
        }
    }
}
