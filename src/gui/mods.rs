use std::{collections::HashMap, sync::OnceLock};

use serde::{Deserialize, Serialize};
use strfmt::Format;
use uk_app::{Command, ModId, ProfileName, view::load_index_of};
use uk_localization::string_ext::LocString;
use uk_ui::{
    egui::{
        self,
        Align,
        Button,
        Color32,
        CursorIcon,
        Id,
        Key,
        /* LayerId, */ Layout,
        Response,
        Sense,
        TextStyle,
        Ui, //, Vec2, emath::TSTransform
        epaint::Margin,
        text::LayoutJob,
    },
    egui_extras::{Column, TableBuilder, TableRow},
    ext::UiExt,
};

use super::{App, FocusedPane, Message, Sort};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModListState {
    pub sort: Sort,
    pub descending: bool,
}

impl Default for ModListState {
    fn default() -> Self {
        Self {
            sort: Sort::Priority,
            descending: false,
        }
    }
}

enum ContextMenuMessage {
    CopyToProfile(smartstring::alias::String),
    Extract,
    Update,
    DevUpdate,
    Uninstall,
    Toggle(bool),
    Move(usize),
    View,
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
            ui.fonts_mut(|f| {
                f.layout_job(LayoutJob::simple_singleline(
                    "PriorityWW".to_owned(),
                    ui.style()
                        .text_styles
                        .get(&TextStyle::Body)
                        .unwrap()
                        .clone(),
                    ui.style().visuals.text_color(),
                ))
            })
            .size()
            .x
        });
        static CATEGORY_WIDTH: OnceLock<f32> = OnceLock::new();

        // Show helper message if no mods installed
        if self.snap.entries.is_empty() {
            egui::Frame::NONE
                .inner_margin(Margin::same(40))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);

                        let has_config = self.core.settings().platform_config().is_some();

                        // Main message
                        if !has_config {
                            let message = "Helper_Modlist_NoConfig".localize();
                            let vars = HashMap::from([(
                                "platform".to_string(),
                                self.core.settings().current_mode.to_string(),
                            )]);
                            let prompt = message.format(&vars).unwrap();

                            ui.label(
                                egui::RichText::new(prompt)
                                    .size(16.0)
                                    .color(ui.style().visuals.text_color()),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Helper_Modlist_NoMods".localize())
                                    .size(16.0)
                                    .color(ui.style().visuals.text_color()),
                            );
                        }

                        ui.add_space(8.0);

                        // Hint text
                        let hint = if !has_config {
                            "Helper_Modlist_NoConfig_Hint".localize()
                        } else {
                            "Helper_Modlist_NoMods_Hint".localize()
                        };

                        ui.label(
                            egui::RichText::new(hint)
                                .size(13.0)
                                .color(ui.style().visuals.weak_text_color()),
                        );
                    });
                });
            return;
        }

        egui::Frame::NONE
            .inner_margin(Margin {
                bottom: 12,
                top:    4,
                left:   4,
                right:  -12,
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
                    .sense(Sense::click_and_drag())
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
                            let is_current = self.list_state.sort == Sort::Enabled;
                            let label = if is_current {
                                if self.list_state.descending {
                                    "⏷"
                                } else {
                                    "⏶"
                                }
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
                                        !self.list_state.descending
                                    } else {
                                        self.list_state.descending
                                    },
                                ));
                            }
                        });
                        [
                            ("Info_Name".localize(), Sort::Name),
                            ("Info_Category".localize(), Sort::Category),
                            ("Info_Version".localize(), Sort::Version),
                            ("Info_Priority".localize(), Sort::Priority),
                        ]
                        .into_iter()
                        .for_each(|(label, sort)| {
                            let width = header
                                .col(|ui| {
                                    let is_current = self.list_state.sort == sort;
                                    let mut label = label.into_owned();
                                    if is_current {
                                        if self.list_state.descending {
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
                                                    !self.list_state.descending
                                                } else {
                                                    self.list_state.descending
                                                },
                                            ));
                                        }
                                    });
                                })
                                .0
                                .width();
                            if CATEGORY_WIDTH.get().is_none() && sort == Sort::Category {
                                CATEGORY_WIDTH.set(width).unwrap();
                            }
                        });
                    })
                    .body(|body| {
                        let visible = self.visible.clone();
                        body.rows(*text_height, visible.len(), |row| {
                            let id = visible[row.index()];
                            self.render_mod_row(id, row);
                        });
                    });
            });
        if ui.memory(|m| m.focused().is_some())
            && self.focused == FocusedPane::ModList
            && !self.modal_open()
        {
            if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                if let Some(index) = self
                    .visible
                    .iter()
                    .rposition(|id| self.selected.contains(id))
                    .map(|i| (i + 1).min(self.visible.len().saturating_sub(1)))
                {
                    let id = self.visible[index];
                    if ui.input(|i| i.modifiers.shift) {
                        self.do_update(Message::SelectAlso(id));
                    } else {
                        self.do_update(Message::SelectOnly(id));
                    }
                }
            } else if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                if let Some(index) = self
                    .visible
                    .iter()
                    .position(|id| self.selected.contains(id))
                    .map(|i| i.saturating_sub(1))
                {
                    let id = self.visible[index];
                    if ui.input(|i| i.modifiers.shift) {
                        self.do_update(Message::SelectAlso(id));
                    } else {
                        self.do_update(Message::SelectOnly(id));
                    }
                }
            }
        }
        self.render_drag_state(*text_height, *icon_width, *numeric_col_width, ui);
        if ui.input_mut(|i| i.pointer.any_released()) {
            ui.memory_mut(|m| m.data.insert_temp(Id::new("drag_delay_frames"), 0usize));
            if let (Some(dragged), Some(hovered)) = (self.drag, self.hover) {
                if dragged != hovered {
                    // The destination is a load-order index, not a row index.
                    let index = self.drag_dest(hovered);
                    let ids = self.selected.clone();
                    self.session.dispatch(Command::MoveTo { ids, index });
                }
            }
            self.drag = None;
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Default);
        }
    }

    fn render_drag_state(
        &mut self,
        _text_height: f32,
        _icon_width: f32,
        _numeric_col_width: f32,
        ui: &mut Ui,
    ) {
        let being_dragged = ui.ctx().dragged_id().is_some();
        let delay_id = Id::new("drag_delay_frames");
        let mut delay_frames: usize = ui.memory_mut(|m| *m.data.get_temp_mut_or_default(delay_id));
        if being_dragged {
            if delay_frames < 6 {
                delay_frames += 1;
            } else if self.drag.is_some() {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
            }
        } else {
            delay_frames = 0;
        }
        ui.memory_mut(|m| m.data.insert_temp(delay_id, delay_frames));
    }

    fn render_mod_row(&mut self, id: ModId, mut row: TableRow) {
        let Some(entry) = self.snap.entries.iter().find(|e| e.id == id).cloned() else {
            return;
        };
        // The Priority column shows the load-order index (GUI_REF §22.1).
        let load_index = load_index_of(&self.snap.entries, id).unwrap_or(0);
        let selected = self.selected.contains(&id);
        let mut clicked = false;
        let mut drag_started = false;
        let mut ctrl = false;
        let mut shift = false;
        let mut hover = false;
        let mut toggled = false;
        let mut ctx_action = None;
        let menu_enabled = entry.enabled;

        let mut process_col_res = |res: Response| {
            clicked = clicked || res.clicked();
            hover = hover || res.hovered();
            drag_started = drag_started || res.drag_started();
            res.context_menu(|ui| {
                if let Some(action) =
                    Self::render_mod_context_menu(&self.core, &self.snap.profiles, menu_enabled, ui)
                {
                    ctx_action.replace(action);
                }
            });
        };

        if selected {
            row.set_selected(true);
        }
        let mut enabled = entry.enabled;
        process_col_res(
            row.col(|ui| {
                toggled = ui.checkbox(&mut enabled, "").clicked();
                shift = ui.input(|i| i.modifiers.shift);
                ctrl = ui.input(|i| i.modifiers.ctrl);
            })
            .1,
        );
        process_col_res(
            row.col(|ui| {
                ui.clipped_label(entry.meta.name.as_str());
            })
            .1,
        );
        for label in [
            entry.meta.category.to_loc_str().localize().as_ref(),
            entry.meta.version.to_string().as_str(),
            load_index.to_string().as_str(),
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
            // Context-menu actions target the clicked mod, or the whole
            // selection when the clicked mod is part of it.
            let ids = self.context_target(id);
            match action {
                ContextMenuMessage::CopyToProfile(profile) => {
                    self.session.dispatch(Command::AddToProfile {
                        ids,
                        profile: ProfileName::from_existing(profile),
                    });
                }
                ContextMenuMessage::Extract => {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("Mod_Unpack_Folder".localize())
                        .pick_folder()
                    {
                        self.session
                            .dispatch(Command::Extract { ids, dest: folder });
                    }
                }
                ContextMenuMessage::Update => {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Mod_Select_Title".localize())
                        .add_filter("Any mod (*.zip, *.rar, *.7z, *.bnp, rules.txt)", &[
                            "zip", "rar", "7z", "bnp", "txt",
                        ])
                        .add_filter("UKMM Mod (*.zip)", &["zip"])
                        .add_filter("BCML Mod (*.bnp)", &["bnp"])
                        .add_filter("Legacy Mod (*.zip, *.rar, *.7z, rules.txt)", &[
                            "zip", "rar", "7z", "txt",
                        ])
                        .add_filter("All files (*.*)", &["*"])
                        .pick_file()
                    {
                        self.session.dispatch(Command::UpdateFromPath {
                            id,
                            path,
                            meta: None,
                        });
                    }
                }
                ContextMenuMessage::DevUpdate => {
                    // Pick the source folder for each target mod here in render
                    // code; the worker stays dialog-free.
                    let mut folders = Vec::with_capacity(ids.len());
                    let mut aborted = false;
                    for target in &ids {
                        let name = self
                            .snap
                            .entries
                            .iter()
                            .find(|e| &e.id == target)
                            .map(|e| e.meta.name.to_string())
                            .unwrap_or_default();
                        let message = "Mod_Update_Folder".localize();
                        let vars =
                            std::collections::HashMap::from([("mod_name".to_string(), name)]);
                        match message.format(&vars) {
                            Ok(title) => {
                                match rfd::FileDialog::new().set_title(title).pick_folder() {
                                    Some(folder) => folders.push(folder),
                                    None => {
                                        aborted = true;
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                self.do_update(Message::Error(e.into()));
                                aborted = true;
                                break;
                            }
                        }
                    }
                    if !aborted {
                        self.session.dispatch(Command::DevUpdate { ids, folders });
                    }
                }
                ContextMenuMessage::Uninstall => {
                    let message = "Mod_Uninstall_Confirmation".localize();
                    let vars =
                        HashMap::from([("mod_name".to_string(), entry.meta.name.to_string())]);
                    let prompt = message.format(&vars).unwrap();
                    self.confirm = Some((Command::Uninstall(ids).into(), prompt));
                }
                ContextMenuMessage::Toggle(state) => {
                    self.session.dispatch(Command::SetEnabled {
                        ids,
                        enabled: state,
                    });
                }
                ContextMenuMessage::Move(dest) => {
                    self.session.dispatch(Command::MoveTo { ids, index: dest });
                }
                ContextMenuMessage::View => {
                    self.session
                        .dispatch(Command::OpenFolder(uk_app::FolderKind::Mod(id)));
                }
            }
        }
        if toggled {
            self.session.dispatch(Command::SetEnabled {
                ids: vec![id],
                enabled,
            });
        } else if clicked {
            self.do_update(Message::SetFocus(FocusedPane::ModList));
            if selected && ctrl {
                self.do_update(Message::Deselect(id));
            } else if shift {
                self.do_update(Message::SelectThrough(id));
            } else if ctrl {
                self.do_update(Message::SelectAlso(id));
            } else {
                self.do_update(Message::SelectOnly(id));
            }
        } else if drag_started {
            if !ctrl && !shift {
                self.do_update(Message::StartDrag(id));
            }
        } else if self.drag != Some(id) && hover {
            self.hover = Some(id);
        }
    }

    fn render_mod_context_menu(
        core: &uk_app::model::Manager,
        profiles: &[uk_app::ProfileName],
        enabled: bool,
        ui: &mut Ui,
    ) -> Option<ContextMenuMessage> {
        let mut result = None;
        ui.menu_button("Mod_Send".localize(), |ui| {
            for profile in profiles
                .iter()
                .filter(|p| core.with_mod_manager(|mm| mm.profile().key() != p.as_str()))
            {
                if ui.button(profile.as_str()).clicked() {
                    result = Some(ContextMenuMessage::CopyToProfile(profile.as_str().into()));
                    ui.close_kind(egui::UiKind::Menu)
                }
            }
        });
        if ui.button("Generic_Update".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::Update);
        }
        if ui.button("Mod_Dev_Update".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::DevUpdate);
        }
        if ui.button("Mod_Uninstall".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::Uninstall);
        }
        if ui
            .button(if enabled {
                "Mod_Disable".localize()
            } else {
                "Mod_Enable".localize()
            })
            .clicked()
        {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::Toggle(!enabled));
        }
        if ui.button("Mod_View".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::View);
        }
        if ui.button("Mod_Extract".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::Extract);
        }
        if ui.button("Mod_Move_Start".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::Move(0));
        }
        if ui.button("Mod_Move_End".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            result = Some(ContextMenuMessage::Move(9999));
        }
        result
    }
}
