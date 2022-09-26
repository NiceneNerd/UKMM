use crate::mods::Mod;

use super::{visuals, App};
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
                let mut drag_up = None;
                body.rows(text_height, self.mods.len(), |index, row| {
                    let up = self.render_mod_row(index, row);
                    drag_up = drag_up.or(up);
                });
                if self.drag_index.is_some()
                    && self.drag_index == drag_up
                    && let Some(start_index) = self.drag_index.take()
                    && let Some(dest_index) = self.hover_index.take()
                    && start_index != dest_index
                {
                    self.mods.retain(|m| !self.selected.contains(m));
                    for (i, selected_mod) in self.selected.iter().enumerate() {
                        self.mods.insert(dest_index + i, selected_mod.clone());
                    }
                }
            });
        if ui.memory().focus().is_none() {
            if ui.input().key_pressed(Key::ArrowDown)
                && let Some((last_index, _)) = self.mods.iter().enumerate().filter(|(_, m)| self.selected.contains(m)).last()
            {
                if !ui.input().modifiers.shift {
                    self.selected.clear();
                } 
                self.selected.push(self.mods[std::cmp::min(last_index + 1, self.mods.len())].clone()); 
                
            } else if ui.input().key_pressed(Key::ArrowUp)
                && let Some((first_index, _)) = self.mods.iter().enumerate().find(|(_, m)| self.selected.contains(m))
            {
                if !ui.input().modifiers.shift {
                    self.selected.clear();
                } 
                self.selected.push(self.mods[std::cmp::max(first_index - 1, 0)].clone());
                
            }
        } 
        self.render_drag_state(text_height, ui);
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
                        let mod_ = unsafe { self.selected.get_unchecked(index) };
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
        } else if ui.input().pointer.any_released() {
            self.drag_index = None;
            ui.output().cursor_icon = CursorIcon::Default;
        }
    }

    fn render_mod_row(&mut self, index: usize, mut row: TableRow) -> Option<usize> {
        let mod_ = unsafe { self.mods.get_unchecked_mut(index) };
        let selected = self.selected.contains(mod_);
        let mut clicked = false;
        let mut drag_started = false;
        let mut drag_ended = false;
        let mut ctrl = false;
        let mut hover = false;

        let mut process_col_res = |res: Response| {
            clicked = clicked || res.clicked();
            hover = hover || res.hovered();
            drag_started = drag_started || res.drag_started();
            drag_ended = drag_ended || res.drag_released();
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
        let mut process_click = || {
            if selected && ctrl {
                self.selected.retain(|m| m != mod_);
            } else if selected && self.selected.len() > 1 {
                self.selected.retain(|m| m == mod_);
            } else {
                if !ctrl {
                    self.selected.clear();
                }
                self.selected.push(mod_.clone());
            }
        };
        if clicked {
            if self.drag_index != Some(index) {
                process_click();
            }
        } else if drag_started {
            self.drag_index = Some(index);
            if !selected {
                process_click();
            }
        } else if hover {
            self.hover_index = Some(index);
        } else {
            return drag_ended.then_some(index);
        }
        None
    }
}
