use std::process::Command;

use crate::mods::Mod;

use super::{App, FocusedPane, Message, Sort};
use eframe::epaint::text::TextWrapping;
use egui::{
    text::LayoutJob, Align, Button, Color32, CursorIcon, Id, Key, Label, LayerId, Layout, Response,
    Sense, TextStyle, Ui, Vec2,
};
use egui_extras::{Size, TableBuilder, TableRow};
use join_str::jstr;
use once_cell::sync::OnceCell;

impl App {
    pub fn render_modlist(&mut self, ui: &mut Ui) {
        static TEXT_HEIGHT: OnceCell<f32> = OnceCell::new();
        let text_height = TEXT_HEIGHT.get_or_init(|| ui.text_style_height(&TextStyle::Body) + 4.);
        static ICON_WIDTH: OnceCell<f32> = OnceCell::new();
        let icon_width =
            ICON_WIDTH.get_or_init(|| ui.spacing().icon_width + ui.spacing().button_padding.x);
        static NUMERIC_COL_WIDTH: OnceCell<f32> = OnceCell::new();
        let numeric_col_width = NUMERIC_COL_WIDTH.get_or_init(|| {
            ui.fonts()
                .layout_job(LayoutJob::simple_singleline(
                    "PriorityWW".to_owned(),
                    ui.style()
                        .text_styles
                        .get(&TextStyle::Body)
                        .unwrap()
                        .clone(),
                    ui.style().visuals.text_color(),
                ))
                .size()
                .x
        });
        TableBuilder::new(ui)
            .cell_sense(Sense::click_and_drag())
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Size::exact(*icon_width))
            .column(Size::remainder())
            .column(Size::Absolute {
                initial: 100.,
                range: (16., 240.),
            })
            .column(Size::exact(*numeric_col_width))
            .column(Size::exact(*numeric_col_width))
            .header(*text_height, |mut header| {
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
                body.rows(*text_height, self.displayed_mods.len(), |index, row| {
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
        self.render_drag_state(*text_height, *icon_width, *numeric_col_width, ui);
        if ui.input().pointer.any_released() {
            ui.memory()
                .data
                .insert_temp(Id::new("drag_delay_frames"), 0usize);
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

    fn render_drag_state(
        &mut self,
        text_height: f32,
        icon_width: f32,
        numeric_col_width: f32,
        ui: &mut Ui,
    ) {
        let being_dragged = ui.memory().is_anything_being_dragged();
        let mut memory = ui.memory();
        let delay_frames: &mut usize = memory
            .data
            .get_temp_mut_or_default(Id::new("drag_delay_frames"));
        if being_dragged {
            if *delay_frames < 6 {
                *delay_frames += 1;
            } else {
                drop(memory);
                if let Some(drag_index) = self.drag_index {
                    ui.output().cursor_icon = CursorIcon::Grabbing;
                    let layer_id =
                        LayerId::new(egui::Order::Tooltip, Id::new("mod_list").with(drag_index));
                    let res = ui
                        .with_layer_id(layer_id, |ui| {
                            TableBuilder::new(ui)
                                .column(Size::exact(icon_width))
                                .column(Size::remainder())
                                .column(Size::Absolute {
                                    initial: 80.,
                                    range: (16., 240.),
                                })
                                .column(Size::exact(numeric_col_width))
                                .column(Size::exact(numeric_col_width))
                                .body(|body| {
                                    body.rows(
                                        text_height,
                                        self.selected.len(),
                                        |index, mut row| {
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
                                        },
                                    );
                                });
                        })
                        .response;
                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                        let delta = pointer_pos.y - res.rect.center().y;
                        ui.ctx().translate_layer(layer_id, Vec2::new(0.0, delta));
                    }
                }
            }
        } else {
            *delay_frames = 0;
        }
    }

    fn render_mod_row(&mut self, index: usize, mut row: TableRow) {
        let mod_ = unsafe { self.displayed_mods.get_mut(index).unwrap_unchecked() };
        if let Some(index) = self.mods.index_of(mod_) {
            let selected = self.selected.contains(mod_);
            let mut clicked = false;
            let mut drag_started = false;
            let mut ctrl = false;
            let mut hover = false;
            let menu_mod = mod_.clone();

            let mut process_col_res = |res: Response| {
                clicked = clicked || res.clicked();
                hover = hover || res.hovered();
                drag_started = drag_started || res.drag_started();
                res.context_menu(|ui| {
                    Self::render_mod_context_menu(self.channel.0.clone(), menu_mod.clone(), ui);
                });
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
                let unclipped = ui.fonts().layout_job(job.clone());
                let max_width = ui.available_width();
                job.wrap = TextWrapping {
                    max_rows: 1,
                    max_width,
                    ..Default::default()
                };
                let clipped = ui.fonts().layout_job(job);
                let res = ui.add(Label::new(clipped));
                if unclipped.size().x > max_width {
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

    fn render_mod_context_menu(sender: flume::Sender<Message>, mod_: Mod, ui: &mut Ui) {
        if ui.button("Uninstall").clicked() {
            let prompt = jstr!("Are you sure you want to uninstall the selected mod(s)?");
            ui.close_menu();
            sender
                .send(Message::Confirm(
                    Message::UninstallMods(None).into(),
                    prompt,
                ))
                .unwrap();
        }
        if ui
            .button(if mod_.enabled { "Disable" } else { "Enable" })
            .clicked()
        {
            ui.close_menu();
            sender
                .send(Message::ToggleMods(None, !mod_.enabled))
                .unwrap();
        }
        if ui.button("View folder").clicked() {
            ui.close_menu();
            let _ = Command::new(if cfg!(windows) {
                "explorer"
            } else {
                "xdg-open"
            })
            .arg(if mod_.path.is_dir() {
                &mod_.path
            } else {
                mod_.path.parent().unwrap()
            })
            .output();
        }
        if ui.button("Move to start").clicked() {
            ui.close_menu();
            sender.send(Message::MoveSelected(0)).unwrap();
        }
        if ui.button("Move to end").clicked() {
            ui.close_menu();
            sender.send(Message::MoveSelected(9999)).unwrap();
        }
    }
}
