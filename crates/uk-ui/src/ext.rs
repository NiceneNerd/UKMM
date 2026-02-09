use std::{hash::Hash, path::PathBuf, str::FromStr, sync::Arc};

use egui::{
    epaint::text::TextWrapping, mutex::RwLock, text::LayoutJob, Button, Direction, Id, NumExt,
    Response, RichText, Ui,
};

pub trait UiExt {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response;
    fn file_picker(&mut self, value: &mut PathBuf) -> Response;
    fn file_picker_string(&mut self, value: &mut String) -> Response;
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
        let button_size = egui::Vec2::new(
            ui.spacing().interact_size.x,
            (ui.text_style_height(&egui::TextStyle::Body) + (ui.spacing().button_padding.y * 2.0))
                .at_least(ui.spacing().interact_size.y),
        );
        if ui.layout().main_dir() == Direction::LeftToRight {
            let mut changed = false;
            let mut res = ui.text_edit_singleline(&mut path);
            if let Some(folder) = ui
                .add(Button::new("Browse…").small().min_size(button_size))
                .clicked()
                .then(|| {
                    if folder {
                        rfd::FileDialog::new().pick_folder()
                    } else {
                        rfd::FileDialog::new().pick_file()
                    }
                })
                .flatten()
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
            if let Some(folder) = ui
                .add(Button::new("Browse…").small().min_size(button_size))
                .clicked()
                .then(|| {
                    if folder {
                        rfd::FileDialog::new().pick_folder()
                    } else {
                        rfd::FileDialog::new().pick_file()
                    }
                })
                .flatten()
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

    fn file_picker_string(&mut self, value: &mut String) -> Response {
        let mut path = PathBuf::from_str(value).unwrap();
        let res = render_picker(false, self, &mut path);
        if res.changed() {
            *value = path.to_str().unwrap().to_owned();
        }
        res
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
        let gallery = self.fonts_mut(|f| f.layout(text.clone(), font, color, f32::INFINITY));
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
        self.data_mut(|d| d.insert_temp(Id::new(id), string.clone()));
        string
    }

    #[inline]
    fn get_temp_string(&mut self, id: impl Hash) -> Option<Arc<RwLock<String>>> {
        self.data_mut(|d| d.get_temp::<Arc<RwLock<String>>>(Id::new(id)))
    }

    #[inline]
    fn clear_temp_string(&mut self, id: impl Hash) {
        self.data_mut(|d| d.remove::<Arc<RwLock<String>>>(Id::new(id)))
    }
}
