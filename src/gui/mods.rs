use crate::mods::Mod;

use super::{visuals, App, FocusedPane, Message, Sort};
use eframe::epaint::{text::TextWrapping, Galley};
use egui::{
    text::LayoutJob, Align, Button, Checkbox, Color32, CursorIcon, Id, Key, Label, LayerId, Layout,
    Response, Sense, TextStyle, Ui, Vec2,
};
use egui_extras::{Size, TableBuilder, TableRow};

impl App {
    pub fn render_modlist(&mut self, ui: &mut Ui) {
        let text_height = ui.text_style_height(&TextStyle::Body) + 4.;
        let icon_width = ui.spacing().icon_width + ui.spacing().button_padding.x;
        TableBuilder::new(ui)
            .cell_sense(Sense::click_and_drag())
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Size::exact(icon_width))
            .column(Size::remainder())
            .column(Size::Absolute {
                initial: 100.,
                range: (16., 240.),
            })
            .column(Size::exact(72.))
            .column(Size::exact(72.))
            .header(text_height, |mut header| {
                header.col(|ui| {
                    let is_current = self.sort.0 == Sort::Enabled;
                    let label = if is_current {
                        if self.sort.1 {
                            "⏷"
                        } else {
                            "⏶"
                        }
                    } else {
                        "  "
                    };
                    if ui
                        .add(Button::new(label).small().fill(Color32::TRANSPARENT))
                        .clicked()
                    {
                        self.do_update(Message::ChangeSort(
                            Sort::Enabled,
                            if is_current {
                                !self.sort.1
                            } else {
                                self.sort.1
                            },
                        ));
                    }
                });
                [
                    ("Mod Name", Sort::Name),
                    ("Category", Sort::Category),
                    ("Version", Sort::Version),
                    ("Priority", Sort::Priority),
                ]
                .into_iter()
                .for_each(|(label, sort)| {
                    header.col(|ui| {
                        let is_current = self.sort.0 == sort;
                        let mut label = label.to_owned();
                        if is_current {
                            if self.sort.1 {
                                label += " ⏷";
                            } else {
                                label += " ⏶";
                            }
                        } else {
                            label += "  ";
                        }
                        ui.centered_and_justified(|ui| {
                            if ui
                                .add(Button::new(label).small().fill(Color32::TRANSPARENT))
                                .clicked()
                            {
                                self.do_update(Message::ChangeSort(
                                    sort,
                                    if is_current {
                                        !self.sort.1
                                    } else {
                                        self.sort.1
                                    },
                                ));
                            }
                        });
                    });
                });
            })
            .body(|body| {
                body.rows(text_height, self.displayed_mods.len(), |index, row| {
                    self.render_mod_row(index, row);
                });
            });
        if ui.memory().focus().is_none() && self.focused == FocusedPane::ModList {
            if ui.input().key_pressed(Key::ArrowDown) && let Some((last_index, _)) = self
                .mods
                .iter()
                .enumerate()
                .filter(|(_, m)| self.selected.contains(m))
                .last()
            {
                if !ui.input().modifiers.shift {
                    self.do_update(Message::SelectOnly(last_index + 1));
                } else {
                    self.do_update(Message::SelectAlso(last_index + 1));
                }
            } else if ui.input().key_pressed(Key::ArrowUp) && let Some((first_index, _)) = self
                .mods
                .iter()
                .enumerate()
                .find(|(_, m)| self.selected.contains(m))
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
        let icon_width = ui.spacing().icon_width + ui.spacing().button_padding.x;
        if being_dragged && let Some(drag_index) = self.drag_index {
            ui.output().cursor_icon = CursorIcon::Grabbing;
            let layer_id = LayerId::new(egui::Order::Tooltip, Id::new("mod_list").with(drag_index));
            let res = ui
                .with_layer_id(layer_id, |ui| {
                    TableBuilder::new(ui)
                        .column(Size::exact(icon_width))
                        .column(Size::remainder())
                        .column(Size::Absolute {
                            initial: 80.,
                            range: (16., 240.),
                        })
                        .column(Size::exact(64.))
                        .column(Size::exact(64.))
                        .body(|body| {
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
                                    self.mods
                                        .iter()
                                        .position(|m| m == mod_)
                                        .unwrap()
                                        .to_string()
                                        .as_str(),
                                ] {
                                    row.col(|ui| {
                                        ui.label(label);
                                    });
                                }
                            });
                        });
                })
                .response;
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos.y - res.rect.center().y;
                ui.ctx().translate_layer(layer_id, Vec2::new(0.0, delta));
            }
        }
    }

    fn render_mod_row(&mut self, index: usize, mut row: TableRow) {
        let mod_ = unsafe { self.displayed_mods.get_mut(index).unwrap_unchecked() };
        let index = unsafe { self.mods.index_of(mod_).unwrap_unchecked() };
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
        process_col_res(row.col(|ui| {
            let mut job = LayoutJob::simple_singleline(
                mod_.meta.name.to_string(),
                ui.style()
                    .text_styles
                    .get(&TextStyle::Body)
                    .unwrap()
                    .clone(),
                ui.style().visuals.text_color(),
            );
            let max_width = ui.available_width();
            job.wrap = TextWrapping {
                max_rows: 1,
                max_width,
                ..Default::default()
            };
            let gallery = ui.fonts().layout_job(job);
            let res = ui.add(Label::new(gallery));
            if (mod_.meta.name.len() * 10) as f32 > max_width {
                res.on_hover_text(mod_.meta.name.as_str());
            }
        }));
        for label in [
            mod_.meta.category.as_str(),
            mod_.meta.version.to_string().as_str(),
            index.to_string().as_str(),
        ] {
            process_col_res(row.col(|ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(label);
                });
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
