use super::*;

impl App {
    pub fn render_deploy_tab(&self, ui: &mut Ui) {
        match self
            .core
            .settings()
            .platform_config()
            .and_then(|c| c.deploy_config.as_ref())
        {
            Some(config) => {
                egui::Frame::none().inner_margin(4.0).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        let pending = self.core.deploy_manager().pending();
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Method")
                                    .family(egui::FontFamily::Name("Bold".into())),
                            );
                            // ui.add_space(8.);
                            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                                ui.label(config.method.name());
                            })
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Auto Deploy")
                                    .family(egui::FontFamily::Name("Bold".into())),
                            );
                            // ui.add_space(8.);
                            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                                ui.label(if config.auto {
                                    RichText::new("Yes").color(visuals::GREEN)
                                } else {
                                    RichText::new("No").color(visuals::RED)
                                });
                            })
                        });
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Target Folder")
                                    .family(egui::FontFamily::Name("Bold".into())),
                            );
                            let mut job = LayoutJob::simple_singleline(
                                config.output.to_string_lossy().into(),
                                ui.style()
                                    .text_styles
                                    .get(&egui::TextStyle::Body)
                                    .unwrap()
                                    .clone(),
                                ui.visuals().text_color(),
                            );
                            job.wrap = TextWrapping {
                                max_rows: 1,
                                max_width: ui.available_size_before_wrap().x,
                                ..Default::default()
                            };
                            if ui
                                .link(job)
                                .on_hover_text(config.output.to_string_lossy())
                                .clicked()
                            {
                                ui.close_menu();
                                open::that(if config.output.is_dir() {
                                    &config.output
                                } else {
                                    config.output.parent().unwrap()
                                })
                                .unwrap_or(());
                            }
                        });
                        ui.add_space(4.);
                        ui.with_layout(
                            Layout::from_main_dir_and_cross_align(
                                egui::Direction::BottomUp,
                                Align::Center,
                            ),
                            |ui| {
                                egui::Frame::none().show(ui, |ui| {
                                    if let Some(ref exe) = config.executable {
                                        ui.add_space(4.);
                                        if ui.button("Open Emulator").clicked() {
                                            let _ = std::process::Command::new(exe).spawn();
                                        }
                                    }
                                    if !config.auto || self.core.deploy_manager().pending() {
                                        if ui
                                            .add_enabled(pending, egui::Button::new("Deploy"))
                                            .clicked()
                                        {
                                            self.do_update(super::Message::Deploy);
                                        }
                                        if config.auto {
                                            ui.label(
                                                RichText::new(
                                                    "Auto deploy incomplete, please deploy \
                                                     manually",
                                                )
                                                .color(visuals::RED),
                                            );
                                        }
                                    }
                                });
                            },
                        );
                    });
                });
            }
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("No deployment config for current platform");
                });
            }
        }
    }
}
