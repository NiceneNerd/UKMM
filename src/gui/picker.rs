use super::{App, FocusedPane, Message};
use egui::{Button, Id, Key, Ui};
use fs_err as fs;
use std::path::{PathBuf, Path};

#[derive(Clone)]
pub struct FilePickerState {
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub path_input: String,
    pub selected: Option<PathBuf>,
}

impl Default for FilePickerState {
    fn default() -> Self {
        let path = dirs2::download_dir().unwrap();
        Self {
            path_input: path.display().to_string(),
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
        self.path = path;
    }
}

impl App {
    pub fn render_file_picker(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                for (icon, tooltip, cb) in [
                    (
                        "🗁",
                        "Open Mod…",
                        Box::new(|| self.do_update(Message::ClearSelect)) as Box<dyn FnOnce()>,
                    ),
                    (
                        "⏶",
                        "Up One Level",
                        Box::new(|| self.do_update(Message::FilePickerUp)) as Box<dyn FnOnce()>,
                    ),
                    (
                        "⮪",
                        "Back",
                        Box::new(|| self.do_update(Message::FilePickerBack)) as Box<dyn FnOnce()>,
                    ),
                ] {
                    if ui
                        .add(Button::new(icon).small())
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        cb();
                    }
                }
                if ui
                    .text_edit_singleline(&mut self.picker_state.path_input)
                    .has_focus()
                    && ui.input().key_pressed(Key::Enter)
                {
                    self.do_update(Message::FilePickerSet(None));
                }
            });
            egui::ScrollArea::both()
                .id_source("file_picker")
                .show(ui, |ui| {
                    ui.add_space(8.);
                    ui.style_mut().spacing.item_spacing.y = 4.;
                    if let Ok(dir_entries) = fs::read_dir(&self.picker_state.path)
                        .map(|entries| entries.filter_map(std::result::Result::ok))
                    {
                        let mut entries = dir_entries
                            .filter_map(|e| {
                                let path = e.path();
                                let ext = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();
                                ((ext == "zip" || ext == "7z" || path.is_dir())
                                    && !e.file_name().to_str().unwrap_or("").starts_with('.'))
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
                        if self.focused == FocusedPane::FilePicker {
                            if ui.input().key_pressed(Key::ArrowDown)
                                && let Some(pos) = entries.iter().position(|p| self.picker_state.selected.as_ref() == Some(p))
                            {
                                self.picker_state.selected = Some(entries[(pos + 1).min(entries.len() - 1)].to_path_buf());
                            } else if ui.input().key_pressed(Key::ArrowUp)
                                && let Some(pos) = entries.iter().position(|p| self.picker_state.selected.as_ref() == Some(p))
                            {
                                let index = pos.max(1);
                                self.picker_state.selected = Some(entries[index - 1].to_path_buf());
                            }
                        } 
                        entries.into_iter().for_each(|path| {
                            self.render_picker_dir_entry(path, ui);
                        });
                    }
                    ui.allocate_space(ui.available_size());
                });
        });
    }

    fn render_picker_dir_entry(&mut self, path: PathBuf, ui: &mut Ui) {
        let name = if path.is_dir() {
            "🗀 ".to_owned()
        } else {
            "🗄 ".to_owned()
        } + path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let selected = self.picker_state.selected.as_ref() == Some(&path);
        let res = ui.add(Button::new(name).wrap(false).fill(
            if selected {
                ui.style().visuals.selection.bg_fill
            } else {
                ui.style().visuals.noninteractive().bg_fill
            },
        ));
        if res.double_clicked() || (ui.input().key_pressed(Key::Enter) && selected) {
            self.do_update(Message::SetFocus(FocusedPane::FilePicker));
            if path.is_dir() {
                self.do_update(Message::FilePickerSet(Some(path)));
            } else {
                self.do_update(Message::OpenMod(path));
            }
        } else if res.clicked() {
            self.do_update(Message::SetFocus(FocusedPane::FilePicker));
            self.picker_state.selected = Some(path);
        }
    }
}
