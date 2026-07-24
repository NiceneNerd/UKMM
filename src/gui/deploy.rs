use uk_localization::string_ext::LocString;
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
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Settings_Platform_Deploy_Method".localize())
                                    .family(egui::FontFamily::Name("Bold".into())),
                            );
                            // ui.add_space(8.);
                            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                                ui.label(config.method.name().localize());
                            })
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Settings_Platform_Deploy_Auto".localize())
                                    .family(egui::FontFamily::Name("Bold".into())),
                            );
                            // ui.add_space(8.);
                            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                                ui.label(if config.auto {
                                    RichText::new("Generic_Yes".localize())
                                        .color(visuals::GREEN)
                                } else {
                                    RichText::new("Generic_No".localize())
                                        .color(visuals::RED)
                                });
                            })
                        });
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Settings_Platform_Deploy_Output".localize())
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
                                        if ui.button("Deploy_OpenEmu".localize()).clicked() {
                                            let mut iter = shlex::split(&exe.replace("\\","/"))
                                                .unwrap_or_default()
                                                .into_iter();
                                            let (cmd, args) = (
                                                iter.next().unwrap(),
                                                iter.collect::<Vec<_>>()
                                            );
                                            match std::process::Command::new(cmd)
                                                .args(args)
                                                .spawn() {
                                                Ok(mut child) => {
                                                    thread::sleep(Duration::from_secs(1));
                                                    match child.try_wait() {
                                                        Ok(Some(status)) => if !status.success() {
                                                            self.do_update(Message::Error(
                                                                anyhow::anyhow!("Emulator exited with non-zero exit status {status}"),
                                                            ))
                                                        },
                                                        Ok(None) => {}
                                                        Err(e) =>
                                                            self.do_update(Message::Error(
                                                                anyhow::Error::from(e)
                                                                    .context("There was an error running the emulator"),
                                                            ))
                                                    }
                                                },
                                                Err(e) => {
                                                    self.do_update(Message::Error(
                                                        anyhow::Error::from(e)
                                                            .context("Failed to open emulator"),
                                                    ))
                                                }
                                            }
                                        }
                                    }
                                    if ui
                                        .add(egui::Button::new("Tab_Deploy".localize()))
                                        .clicked()
                                    {
                                        self.do_update(super::Message::Deploy);
                                    }
                                    if config.auto && self.core.deploy_manager().pending() {
                                        ui.label(
                                            RichText::new(
                                                "Deploy_Auto_Failed".localize()
                                            )
                                            .color(visuals::RED),
                                        );
                                    }
                                });
                            },
                        );
                    });
                });
            }
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("Deploy_NoConfig".localize());
                });
            }
        }
    }
}
