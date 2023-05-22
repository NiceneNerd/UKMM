use std::{process::Command, sync::OnceLock};

use join_str::jstr;
use uk_manager::mods::Mod;
use uk_ui::{
    egui::{
        self, style::Margin, text::LayoutJob, Align, Button, Color32, CursorIcon, Id, Key, LayerId,
        Layout, Response, Sense, TextStyle, Ui, Vec2,
    },
    egui_extras::{Column, TableBuilder, TableRow},
    ext::UiExt,
};

use super::{App, FocusedPane, Message, Sort};

enum ContextMenuMessage {
    CopyToProfile(smartstring::alias::String),
    Uninstall,
    Toggle(bool),
    Move(usize),
}

impl App {
    pub fn render_modlist(&mut self, ui: &mut Ui) {
        static TEXT_HEIGHT: OnceLock<f32> = OnceLock::new();
        let text_height = TEXT_HEIGHT.get_or_init(|| ui.text_style_height(&TextStyle::Body) + 4.);
        static ICON_WIDTH: OnceLock<f32> = OnceLock::new();
        let icon_width =
            ICON_WIDTH.get_or_init(|| ui.spacing().icon_width + ui.spacing().button_padding.x);
        static NUMERIC_COL_WIDTH: OnceLock<f32> = OnceLock::new();
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
        static CATEGORY_WIDTH: OnceLock<f32> = OnceLock::new();
        egui::Frame::none()
            .inner_margin(Margin {
                bottom: 4.0,
                top:    4.0,
                left:   4.0,
                right:  -12.0,
            })
            .show(ui, |ui| {
                ui.style_mut()
                    .visuals
                    .widgets
                    .noninteractive
                    .fg_stroke
                    .color = ui.style().visuals.strong_text_color();
                let max_width = ui.available_width();
                TableBuilder::new(ui)
                    .cell_sense(Sense::click_and_drag())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(Column::exact(*icon_width))
                    .column(
                        CATEGORY_WIDTH
                            .get()
                            .map(|w| {
                                Column::exact(
                                    max_width
                                        - *icon_width
                                        - (*numeric_col_width * 2.0)
                                        - *w
                                        - (8.0 * 5.0),
                                )
                            })
                            .unwrap_or_else(Column::remainder),
                    )
                    .column(
                        CATEGORY_WIDTH
                            .get()
                            .map(|w| Column::exact(*w))
                            .unwrap_or_else(|| Column::initial(100.).at_least(16.).at_most(240.)),
                    )
                    .columns(Column::exact(*numeric_col_width), 2)
                    .header(*text_height, |mut header| {
                        header.col(|ui| {
                            let is_current = self.sort.0 == Sort::Enabled;
                            let label = if is_current {
                                if self.sort.1 { "⏷" } else { "⏶" }
                            } else {
                                "  "
                            };
                            ui.style_mut().visuals.widgets.inactive.bg_stroke.width = 0.0;
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
                            let width = header
                                .col(|ui| {
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
                                        ui.style_mut().visuals.widgets.inactive.bg_stroke.width =
                                            0.0;
                                        if ui
                                            .add(
                                                Button::new(label)
                                                    .small()
                                                    .fill(Color32::TRANSPARENT),
                                            )
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
                                })
                                .0
                                .width();
                            if CATEGORY_WIDTH.get().is_none() && label == "Category" {
                                CATEGORY_WIDTH.set(width).unwrap();
                            }
                        });
                    })
                    .body(|body| {
                        body.rows(*text_height, self.displayed_mods.len(), |index, row| {
                            self.render_mod_row(index, row);
                        });
                    });
            });
        if ui.memory().focus().is_none()
            && self.focused == FocusedPane::ModList
            && !self.modal_open()
        {
            if let Some((last_index, _)) = ui
                .input()
                .key_pressed(Key::ArrowDown)
                .then(|| {
                    self.mods
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| self.selected.contains(m))
                        .last()
                })
                .flatten()
            {
                if !ui.input().modifiers.shift {
                    self.do_update(Message::SelectOnly(last_index + 1));
                } else {
                    self.do_update(Message::SelectAlso(last_index + 1));
                }
            } else if let Some((first_index, _)) = ui
                .input()
                .key_pressed(Key::ArrowUp)
                .then(|| {
                    self.mods
                        .iter()
                        .enumerate()
                        .find(|(_, m)| self.selected.contains(m))
                })
                .flatten()
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
            if let Some((_start_index, dest_index)) = self
                .drag_index
                .and_then(|d| self.hover_index.map(|h| (d, h)))
                .filter(|(s, d)| s != d)
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
                                .column(Column::exact(icon_width))
                                .column(Column::remainder())
                                .column(Column::initial(80.).at_least(16.).at_most(260.))
                                .column(Column::exact(numeric_col_width))
                                .column(Column::exact(numeric_col_width))
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
        if let Some(index) = self.mods.iter().position(|m| m == mod_) {
            let selected = self.selected.contains(mod_);
            let mut clicked = false;
            let mut drag_started = false;
            let mut ctrl = false;
            let mut shift = false;
            let mut hover = false;
            let mut toggled = false;
            let mut ctx_action = None;
            let menu_mod = mod_.clone();

            let mut process_col_res = |res: Response| {
                clicked = clicked || res.clicked();
                hover = hover || res.hovered();
                drag_started = drag_started || res.drag_started();
                res.context_menu(|ui| {
                    if let Some(action) =
                        Self::render_mod_context_menu(&self.core, menu_mod.clone(), ui)
                    {
                        ctx_action.replace(action);
                    }
                });
            };

            if selected {
                row = row.selected(true);
            }
            let mut enabled = mod_.enabled;
            process_col_res(
                row.col(|ui| {
                    toggled = ui.checkbox(&mut enabled, "").clicked();
                    shift = ui.input().modifiers.shift;
                    ctrl = ui.input().modifiers.ctrl;
                })
                .1,
            );
            process_col_res(
                row.col(|ui| {
                    ui.clipped_label(mod_.meta.name.as_str());
                })
                .1,
            );
            for label in [
                mod_.meta.category.as_str(),
                mod_.meta.version.to_string().as_str(),
                index.to_string().as_str(),
            ] {
                process_col_res(
                    row.col(|ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(label);
                        });
                    })
                    .1,
                );
            }
            if let Some(action) = ctx_action {
                match action {
                    ContextMenuMessage::CopyToProfile(profile) => {
                        self.do_update(Message::AddToProfile(profile));
                    }
                    ContextMenuMessage::Uninstall => {
                        let prompt = jstr!("Are you sure you want to uninstall {&mod_.meta.name}?");
                        self.do_update(Message::Confirm(
                            Message::UninstallMods(None).into(),
                            prompt,
                        ));
                    }
                    ContextMenuMessage::Toggle(state) => {
                        self.do_update(Message::ToggleMods(None, state));
                    }
                    ContextMenuMessage::Move(dest) => {
                        self.do_update(Message::MoveSelected(dest));
                    }
                }
            }
            if toggled {
                self.do_update(Message::ToggleMods(Some(vec![menu_mod.clone()]), enabled));
            } else if clicked {
                self.do_update(Message::SetFocus(FocusedPane::ModList));
                if selected && ctrl {
                    self.do_update(Message::Deselect(index));
                } else if shift {
                    self.do_update(Message::SelectThrough(index));
                } else if ctrl {
                    self.do_update(Message::SelectAlso(index));
                } else {
                    self.do_update(Message::SelectOnly(index));
                }
            } else if drag_started {
                if !ctrl && !shift {
                    self.do_update(Message::StartDrag(index));
                }
            } else if self.drag_index != Some(index) && hover {
                self.hover_index = Some(index);
            }
        }
    }

    fn render_mod_context_menu(
        core: &uk_manager::core::Manager,
        mod_: Mod,
        ui: &mut Ui,
    ) -> Option<ContextMenuMessage> {
        let mut result = None;
        ui.menu_button("Send to profile", |ui| {
            for profile in core
                .settings()
                .profiles()
                .filter(|p| core.mod_manager().profile().key() != p)
            {
                if ui.button(profile.as_str()).clicked() {
                    result = Some(ContextMenuMessage::CopyToProfile(profile));
                    ui.close_menu();
                }
            }
        });
        if ui.button("Uninstall").clicked() {
            ui.close_menu();
            result = Some(ContextMenuMessage::Uninstall);
        }
        if ui
            .button(if mod_.enabled { "Disable" } else { "Enable" })
            .clicked()
        {
            ui.close_menu();
            result = Some(ContextMenuMessage::Toggle(!mod_.enabled));
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
            result = Some(ContextMenuMessage::Move(0));
        }
        if ui.button("Move to end").clicked() {
            ui.close_menu();
            result = Some(ContextMenuMessage::Move(9999));
        }
        result
    }
}
