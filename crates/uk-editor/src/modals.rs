use eframe::{
    egui::{Id, Layout, RichText},
    emath::Align,
};
use uk_ui::egui::{Align2, Spinner, TextStyle, Vec2};

use super::*;

impl App {
    pub fn render_modals(&self, ctx: &egui::Context) {
        self.render_error(ctx);
        self.render_busy(ctx);
    }

    pub fn render_error(&self, ctx: &egui::Context) {
        if let Some(err) = self.error.as_ref() {
            egui::Window::new("错误")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label(err.to_string());
                    ui.add_space(8.);
                    egui::CollapsingHeader::new("详情").show(ui, |ui| {
                        err.chain().enumerate().for_each(|(i, e)| {
                            ui.label(RichText::new(format!("{i}. {e}")).code());
                        });
                    });
                    ui.add_space(8.);
                    if let Some(context) = err.chain().find_map(|e| {
                        e.downcast_ref::<uk_content::UKError>()
                            .and_then(|e| e.context_data())
                    }) {
                        egui::CollapsingHeader::new("数据上下文").show(ui, |ui| {
                            ui.label(format!("{:#?}", context));
                        });
                    }
                    ui.add_space(8.);
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("确定").clicked() {
                                    self.do_update(Message::CloseError);
                                }
                                if ui.button("复制").clicked() {
                                    ui.output_mut(|o| o.copied_text = format!("{:?}", &err));
                                    egui::popup::show_tooltip(ctx, Id::new("copied"), |ui| {
                                        ui.label("已复制")
                                    });
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    pub fn render_busy(&self, ctx: &egui::Context) {
        if self.busy.get() {
            egui::Window::new("正在处理")
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
                                ui.label("处理中…");
                            });
                            ui.shrink_width_to_current();
                        });
                        ui.allocate_space([0., padding / 2.].into());
                    });
                });
        }
    }
}
