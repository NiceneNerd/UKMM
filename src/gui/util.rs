use egui::{Direction, Response, RichText, Ui};
use std::path::PathBuf;

pub trait UkWidgetExt {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response;
    fn file_picker(&mut self, value: &mut PathBuf) -> Response;
    fn strong_heading(&mut self, text: impl Into<String>) -> Response;
}

fn render_picker(folder: bool, ui: &mut Ui, value: &mut PathBuf) -> Response {
    let mut path = value.display().to_string();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if ui.layout().main_dir() == Direction::LeftToRight {
            let res = ui.text_edit_singleline(&mut path);
            if res.changed() {
                *value = path.into();
            }
            res
        } else {
            let mut changed = false;
            if ui.button("Browse…").clicked()
                && let Some(folder) = {
                    if folder {
                        rfd::FileDialog::new().pick_folder()
                    } else {
                        rfd::FileDialog::new().pick_file()
                    }
                }
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

impl UkWidgetExt for Ui {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response {
        render_picker(true, self, value)
    }
    fn file_picker(&mut self, value: &mut PathBuf) -> Response {
        render_picker(false, self, value)
    }

    fn strong_heading(&mut self, text: impl Into<String>) -> Response {
        self.label(
            RichText::new(text)
                .color(self.style().visuals.widgets.inactive.bg_fill)
                .heading(),
        )
    }
}
