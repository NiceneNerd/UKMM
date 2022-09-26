use super::App;
use egui::{Button, Id, RichText, Ui};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FilePickerState {
    pub path: String,
    pub history: Vec<PathBuf>,
}

impl FilePickerState {
    pub fn path(&self) -> PathBuf {
        self.path.as_str().into()
    }
}

impl Default for FilePickerState {
    fn default() -> Self {
        Self {
            path: dirs2::download_dir().unwrap().display().to_string(),
            history: vec![],
        }
    }
}

impl App {
    pub fn render_file_picker(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                for (icon, tooltip) in [("🗁", "Open Mod…"), ("⏶", "Up One Level"), ("⮪", "Back")]
                {
                    ui.add(Button::new(icon).small()).on_hover_text(tooltip);
                }
            });
        });
    }
}
