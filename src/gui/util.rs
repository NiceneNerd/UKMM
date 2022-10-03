use egui::{Align, Direction, Layout, Response, Ui};
use std::path::PathBuf;

pub trait FolderPickerExt {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response;
}

impl FolderPickerExt for Ui {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response {
        let mut path = value.display().to_string();
        self.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if ui.layout().main_dir() == Direction::LeftToRight {
                let res = ui.text_edit_singleline(&mut path);
                if res.changed() {
                    *value = path.into();
                }
                res
            } else {
                let mut changed = false;
                if ui.small_button("Browse…").clicked()
                    && let Some(folder) = rfd::FileDialog::new().pick_folder()
                {
                    *value = folder;
                    changed = true;
                }
                let mut res = ui.text_edit_singleline(&mut path);
                if res.changed() {
                    *value = path.into();
                }
                if changed {
                    res.mark_changed();
                }
                res
            }
        })
        .inner
    }
}
