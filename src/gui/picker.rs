use std::path::{Path, PathBuf};

use fs_err as fs;
use serde::{Deserialize, Serialize};
use uk_localization::string_ext::LocString;
use uk_ui::{
    egui::{self, Button, Key, Ui, Vec2},
    icons::{Icon, IconButtonExt, get_icon},
};

use super::{App, FocusedPane, Message};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "FilePickerStateDe")]
pub struct FilePickerState {
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub path_input: String,
    #[serde(skip)]
    pub selected: Option<PathBuf>,
    pub entries: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct FilePickerStateDe {
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub path_input: String,
}

impl From<FilePickerStateDe> for FilePickerState {
    fn from(de: FilePickerStateDe) -> Self {
        Self {
            selected: None,
            entries:  Self::load_entries(&de.path),

            path: de.path,
            history: de.history,
            path_input: de.path_input,
        }
    }
}

impl Default for FilePickerState {
    fn default() -> Self {
        let path = dirs2::download_dir().or_else(dirs2::home_dir).unwrap();
        Self {
            path_input: path.display().to_string(),
            entries: Self::load_entries(&path),
            path,
            history: vec![],
            selected: None,
        }
    }
}

impl FilePickerState {
    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        self.path_input = path.display().to_string();
        self.entries = Self::load_entries(&path);
        self.path = path;
    }

    fn load_entries(path: &Path) -> Vec<PathBuf> {
        if let Ok(dir_entries) =
            fs::read_dir(path).map(|entries| entries.filter_map(std::result::Result::ok))
        {
            let mut entries = dir_entries
                .filter_map(|e| {
                    let path = e.path();
                    let filename = e.file_name()
                        .to_str()
                        .unwrap_or("")
                        .to_lowercase();
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    (
                        (
                            matches!(ext.as_str(), "zip" | "rar" | "7z" | "bnp") ||
                            matches!(filename.as_str(), "rules.txt" | "info.json") ||
                            path.is_dir()
                        )
                        && !filename.starts_with('.'))
                    .then_some(path)
                })
                .collect::<Vec<_>>();
            entries.sort_by(|a, b| {
                if a.is_file() != b.is_file() {
                    b.is_dir().cmp(&a.is_dir())
                } else {
                    a.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(
                            &b.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_lowercase(),
                        )
                }
            });
            entries
        } else {
            Default::default()
        }
    }
}

impl App {
    pub fn render_file_picker(&mut self, ui: &mut Ui) {
        egui::Frame::none().inner_margin(2.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (icon, tooltip, cb) in [
                    (
                        Icon::FolderOpen,
                        "Menu_File_Open".localize(),
                        Box::new(|| self.do_update(Message::SelectFile)) as Box<dyn FnOnce()>,
                    ),
                    (
                        Icon::ArrowUp,
                        "FilePicker_Up".localize(),
                        Box::new(|| self.do_update(Message::FilePickerUp)) as Box<dyn FnOnce()>,
                    ),
                    (
                        Icon::ArrowBack,
                        "FilePicker_Back".localize(),
                        Box::new(|| self.do_update(Message::FilePickerBack)) as Box<dyn FnOnce()>,
                    ),
                    (
                        Icon::Refresh,
                        "FilePicker_Refresh".localize(),
                        Box::new(|| {
                            self.do_update(Message::FilePickerSet(Some(
                                self.picker_state.path.clone(),
                            )))
                        }) as Box<dyn FnOnce()>,
                    ),
                ] {
                    if ui.icon_button(icon).on_hover_text(tooltip).clicked() {
                        cb();
                    }
                }
                let res = ui.text_edit_singleline(&mut self.picker_state.path_input);
                if res.changed() {
                    self.do_update(Message::FilePickerSet(None));
                }
            });
            egui::ScrollArea::both()
                .id_source("file_picker")
                .show(ui, |ui| {
                    ui.add_space(8.);
                    ui.style_mut().spacing.item_spacing.y = 4.;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke.width = 0.0;
                    let entries = &self.picker_state.entries;
                    if self.focused == FocusedPane::FilePicker && !self.modal_open() {
                        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                            let pos = match entries
                                .iter()
                                .position(|p| self.picker_state.selected.as_ref() == Some(p))
                            {
                                Some(p) => (p + 1).min(entries.len() - 1),
                                None => 0,
                            };
                            self.picker_state.selected = Some(entries[pos].to_path_buf());
                        } else if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                            let pos = match entries
                                .iter()
                                .position(|p| self.picker_state.selected.as_ref() == Some(p))
                            {
                                Some(p) => p.max(1) - 1,
                                None => 0,
                            };
                            self.picker_state.selected = Some(entries[pos].to_path_buf());
                        }
                    }
                    entries.clone().iter().for_each(|path| {
                        self.render_picker_dir_entry(path, ui);
                    });
                    let available = ui.available_size();
                    ui.allocate_space([available.x.max(0.0), available.y.max(0.0)].into());
                });
        });
    }

    fn render_picker_dir_entry(&mut self, path: &Path, ui: &mut Ui) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let is_dir = path.is_dir();
        let selected = self.picker_state.selected.as_ref().map(|p| p.as_ref()) == Some(path);
        let _icon_size: Vec2 = [ui.spacing().icon_width, ui.spacing().icon_width].into();
        let res = ui.add(
            Button::image_and_text(
                get_icon(
                    ui.ctx(),
                    if is_dir {
                        Icon::Folder
                    } else {
                        Icon::FolderZip
                    },
                ),
                name,
            )
            .wrap_mode(egui::TextWrapMode::Truncate)
            // .tint(if is_dir {
            //     visuals::YELLOW
            // } else {
            //     visuals::GREEN
            // })
            .fill(if selected {
                ui.style().visuals.selection.bg_fill
            } else {
                ui.style().visuals.noninteractive().bg_fill
            }),
        );
        if res.double_clicked()
            || (ui.input(|i| i.key_pressed(Key::Enter)) && selected && !self.modal_open())
        {
            self.do_update(Message::SetFocus(FocusedPane::FilePicker));
            if path.is_dir() {
                self.do_update(Message::FilePickerSet(Some(path.to_path_buf())));
            } else {
                self.do_update(Message::OpenMod(path.to_path_buf()));
            }
        } else if res.clicked() {
            self.do_update(Message::SetFocus(FocusedPane::FilePicker));
            self.picker_state.selected = Some(path.to_path_buf());
        }
    }
}
