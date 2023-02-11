use uk_ui::egui::{Align2, Spinner, TextStyle, Vec2};

use super::*;

impl App {
    pub fn render_busy(&self, ctx: &egui::Context) {
        if self.busy {
            egui::Window::new("Working")
                .default_size([240., 80.])
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .collapsible(false)
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    let max_width = ui.available_width() / 2.;
                    ui.vertical_centered(|ui| {
                        let text_height = ui.text_style_height(&TextStyle::Body) * 2.;
                        let padding = 80. - text_height - 8.;
                        ui.allocate_space([max_width, padding / 2.].into());
                        ui.horizontal(|ui| {
                            ui.add_space(8.);
                            ui.add(Spinner::new().size(text_height));
                            ui.add_space(8.);
                            ui.vertical(|ui| {
                                ui.label("Processing…");
                            });
                            ui.shrink_width_to_current();
                        });
                        ui.allocate_space([0., padding / 2.].into());
                    });
                });
        }
    }
}
