use crate::mods::Mod;

use super::{visuals, App};
use egui::{Align, Key, Label, Layout, Response, Sense, TextStyle, Ui};
use egui_extras::{Size, TableBuilder, TableRow};

impl App {
    pub fn render_modlist(&mut self, ui: &mut Ui) {
        let text_height = ui.text_style_height(&TextStyle::Body) + 4.;
        TableBuilder::new(ui)
            .cell_layout(Layout::left_to_right(Align::Center))
            .striped(true)
            .clip(true)
            .column(Size::exact(16.))
            .column(Size::remainder())
            .column(Size::remainder())
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
                if self.drag_index.is_some() && self.drag_index == drag_up {
                    if let Some(start_index) = self.drag_index.take()
                        && let Some(dest_index) = self.hover_index.take()
                    {
                        if start_index != dest_index {
                            self.mods.retain(|m| !self.selected.contains(m));
                            for (i, selected_mod) in self.selected.iter().enumerate() {
                                self.mods.insert(dest_index + i, selected_mod.clone());
                            }
                        }
                    }
                }
            });
        ui.set_max_width(ui.available_width());
    }

    fn render_mod_row(&mut self, index: usize, mut row: TableRow) -> Option<usize> {
        let mod_ = unsafe { self.mods.get_unchecked_mut(index) };
        let selected = self.selected.contains(mod_);
        let mut clicked = false;
        let mut drag_started = false;
        let mut drag_ended = false;
        let mut ctrl = false;
        let mut hover = false;
        if selected {
            row.set_selected(true);
        }
        row.col(|ui| {
            ui.checkbox(&mut mod_.enabled, "");
            ctrl = ui.input().modifiers.ctrl;
        });
        row.col(|ui| {
            let res = Self::render_mod_cell(&mod_.meta.name, selected, ui);
            clicked = clicked || res.clicked();
            hover = hover || res.hovered();
            drag_started = drag_started || res.drag_started();
            drag_ended = drag_ended || res.drag_released();
        });
        row.col(|ui| {
            let res = Self::render_mod_cell(&mod_.meta.category, selected, ui);
            clicked = clicked || res.clicked();
            hover = hover || res.hovered();
            drag_started = drag_started || res.drag_started();
            drag_ended = drag_ended || res.drag_released();
        });
        row.col(|ui| {
            let res = Self::render_mod_cell(mod_.meta.version.to_string(), selected, ui);
            clicked = clicked || res.clicked();
            hover = hover || res.hovered();
            drag_started = drag_started || res.drag_started();
            drag_ended = drag_ended || res.drag_released();
        });
        row.col(|ui| {
            let res = Self::render_mod_cell(index.to_string(), selected, ui);
            hover = hover || res.hovered();
            clicked = clicked || res.clicked();
            drag_started = drag_started || res.drag_started();
            drag_ended = drag_ended || res.drag_released();
        });
        if clicked {
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
        } else if drag_started {
            self.drag_index = Some(index);
        } else if hover {
            self.hover_index = Some(index);
        } else {
            return drag_ended.then_some(index);
        }
        None
    }

    fn render_mod_cell(label: impl AsRef<str>, selected: bool, ui: &mut Ui) -> Response {
        if selected {
            ui.style_mut().visuals.override_text_color =
                Some(ui.style().visuals.selection.stroke.color);
        }
        let label = if label.as_ref().is_empty() {
            "   "
        } else {
            label.as_ref()
        };
        ui.add(Label::new(label).sense(Sense::click_and_drag()).wrap(false))
    }
}
