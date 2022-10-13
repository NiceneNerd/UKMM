use egui::{Direction, Response, RichText, Ui};
use std::path::PathBuf;

pub trait UiExt {
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

impl UiExt for Ui {
    fn folder_picker(&mut self, value: &mut PathBuf) -> Response {
        render_picker(true, self, value)
    }
    fn file_picker(&mut self, value: &mut PathBuf) -> Response {
        render_picker(false, self, value)
    }

    fn strong_heading(&mut self, text: impl Into<String>) -> Response {
        let text = text.into();
        // TODO: Figure out shadow
        // let heading_style = self.style().text_styles.get(&TextStyle::Heading).unwrap();
        // let gallery = self.fonts().layout_no_wrap(
        //     text.clone(),
        //     heading_style.clone(),
        //     self.style()
        //         .visuals
        //         .strong_text_color()
        //         .linear_multiply(0.5),
        // );
        // let mut mesh = Mesh::default();
        // let mut tessellator = Tessellator::new(
        //     self.fonts().pixels_per_point(),
        //     eframe::epaint::TessellationOptions {
        //         feathering: true,
        //         feathering_size_in_pixels: 16.0,
        //         ..Default::default()
        //     },
        //     self.fonts().font_image_size(),
        //     vec![],
        // );
        // let center_x =
        //     (self.cursor().min.x + self.available_width()) / 2.0 + (self.cursor().min.x / 2.0);
        // let x = center_x - gallery.size().x / 2.0;
        // let y = self.cursor().min.y + (self.text_style_height(&TextStyle::Heading) / 2.0);
        // tessellator.tessellate_line(
        //     [[x, y].into(), [x + gallery.size().x, y].into()],
        //     Stroke::new(
        //         1.0,
        //         self.style()
        //             .visuals
        //             .widgets
        //             .hovered
        //             .bg_fill
        //             .linear_multiply(0.5),
        //     ),
        //     &mut mesh,
        // );
        // self.painter().add(mesh);
        self.label(
            RichText::new(text)
                .color(self.style().visuals.widgets.inactive.bg_fill)
                .heading(),
        )
    }
}
