use super::{App, Message};
use egui::{Button, Id, Key, RichText, SelectableLabel, TextStyle, Ui, WidgetText};
use fs_err as fs;
use std::path::{Path, PathBuf};

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
            ui.vertical(|ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    if let Ok(dir_entries) = fs::read_dir(&self.picker_state.path)
                        .map(|entries| entries.filter_map(std::result::Result::ok))
                    {
                        dir_entries
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
                            .for_each(|path| {
                                let mut name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or_default()
                                    .to_owned();
                                if ui
                                    .selectable_value(
                                        &mut self.picker_state.selected,
                                        Some(path.clone()),
                                        name,
                                    )
                                    .double_clicked()
                                {
                                    if path.is_dir() {
                                        self.do_update(Message::FilePickerSet(Some(path)));
                                    } else {
                                        todo!()
                                    }
                                }
                            });
                    }
                });
            });
        });
    }
}
