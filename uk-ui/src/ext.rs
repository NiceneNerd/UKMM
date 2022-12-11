use std::{hash::Hash, path::PathBuf, sync::Arc};

use egui::{
    epaint::text::TextWrapping, mutex::RwLock, text::LayoutJob, Direction, Id, Response, RichText,
    Ui,
};

pub trait UiExt {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response;
    fn file_picker(&mut self, value: &mut PathBuf) -> Response;
    fn strong_heading(&mut self, text: impl Into<String>) -> Response;
    fn clipped_label(&mut self, text: impl Into<String>) -> Response;
    fn create_temp_string(&mut self, id: impl Hash, init: Option<String>) -> Arc<RwLock<String>>;
    fn get_temp_string(&mut self, id: impl Hash) -> Option<Arc<RwLock<String>>>;
    fn clear_temp_string(&mut self, id: impl Hash);
}

fn render_picker(folder: bool, ui: &mut Ui, value: &mut PathBuf) -> Response {
    let mut path = value.display().to_string();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if ui.layout().main_dir() == Direction::LeftToRight {
            let mut changed = false;
            let mut res = ui.text_edit_singleline(&mut path);
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
            if res.changed() {
                *value = path.into();
            }
            if changed {
                res.mark_changed();
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

impl UiExt for Ui {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response {
        render_picker(true, self, value)
    }

    fn file_picker(&mut self, value: &mut PathBuf) -> Response {
        render_picker(false, self, value)
    }

    fn strong_heading(&mut self, text: impl Into<String>) -> Response {
        let text = text.into();
        self.label(
            RichText::new(text)
                .color(self.style().visuals.widgets.inactive.bg_fill)
                .heading(),
        )
    }

    fn clipped_label(&mut self, text: impl Into<String>) -> Response {
        let text = text.into();
        let font = self
            .style()
            .text_styles
            .get(&egui::TextStyle::Body)
            .unwrap()
            .clone();
        let color = self.visuals().text_color();
        let mut job = LayoutJob::simple_singleline(text.clone(), font.clone(), color);
        let max_width = self.available_width();
        job.wrap = TextWrapping {
            break_anywhere: false,
            max_rows: 1,
            max_width,
            ..Default::default()
        };
        let gallery = self
            .fonts()
            .layout(text.clone(), font, color, f32::INFINITY);
        let width = gallery.size().x;
        let res = self.label(job);
        if width > self.available_width() {
            res.on_hover_text(text)
        } else {
            res
        }
    }

    #[inline]
    fn create_temp_string(&mut self, id: impl Hash, init: Option<String>) -> Arc<RwLock<String>> {
        let string = Arc::new(RwLock::new(init.unwrap_or_default()));
        self.data().insert_temp(Id::new(id), string.clone());
        string
    }

    #[inline]
    fn get_temp_string(&mut self, id: impl Hash) -> Option<Arc<RwLock<String>>> {
        self.data().get_temp::<Arc<RwLock<String>>>(Id::new(id))
    }

    #[inline]
    fn clear_temp_string(&mut self, id: impl Hash) {
        self.data().remove::<Arc<RwLock<String>>>(Id::new(id))
    }
}
