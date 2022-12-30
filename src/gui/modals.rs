use super::*;

impl App {
    pub fn render_error(&mut self, ctx: &egui::Context) {
        if let Some(err) = self.error.as_ref() {
            egui::Window::new("Error")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label(err.to_string());
                    ui.add_space(8.);
                    egui::CollapsingHeader::new("Details").show(ui, |ui| {
                        err.chain().enumerate().for_each(|(i, e)| {
                            ui.label(RichText::new(format!("{i}. {e}")).code());
                        });
                    });
                    ui.add_space(8.);
                    if let Some(context) = err.chain().find_map(|e| {
                        e.downcast_ref::<uk_content::UKError>()
                            .and_then(|e| e.context_data())
                    }) {
                        egui::CollapsingHeader::new("Data Context").show(ui, |ui| {
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
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::CloseError);
                                }
                                if ui.button("Copy").clicked() {
                                    ui.output().copied_text = format!("{:?}", &err);
                                    egui::popup::show_tooltip(ctx, Id::new("copied"), |ui| {
                                        ui.label("Copied")
                                    });
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    pub fn render_confirm(&mut self, ctx: &egui::Context) {
        let is_confirm = self.confirm.is_some();
        if is_confirm {
            egui::Window::new("Confirm")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label(&self.confirm.as_ref().unwrap().1);
                    ui.add_space(8.);
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    let msg = self.confirm.take().unwrap().0;
                                    self.do_update(msg);
                                    self.do_update(Message::CloseConfirm);
                                }
                                if ui.button("Close").clicked() {
                                    self.do_update(Message::CloseConfirm);
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    pub fn render_new_profile(&mut self, ctx: &egui::Context) {
        let is_open = self.new_profile.is_some();
        if is_open {
            egui::Window::new("New Profile")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label("Enter name for new profile");
                    ui.add_space(8.);
                    ui.text_edit_singleline(self.new_profile.as_mut().unwrap());
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::AddProfile);
                                }
                                if ui.button("Close").clicked() {
                                    self.new_profile = None;
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

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
                                let mut job = LayoutJob::single_section(
                                    self.logs
                                        .iter()
                                        .rev()
                                        .find(|l| {
                                            l.level == "INFO" || l.args.starts_with("PROGRESS")
                                        })
                                        .map(|l| l.args.as_str().trim_start_matches("PROGRESS"))
                                        .unwrap_or_default()
                                        .to_owned(),
                                    TextFormat::default(),
                                );
                                job.wrap = TextWrapping {
                                    max_width,
                                    max_rows: 1,
                                    break_anywhere: true,
                                    ..Default::default()
                                };
                                ui.add(Label::new(job).wrap(false));
                            });
                            ui.shrink_width_to_current();
                        });
                        ui.allocate_space([0., padding / 2.].into());
                    });
                });
        }
    }

    pub fn render_about(&self, ctx: &egui::Context) {
        if self.show_about {
            egui::Window::new("About")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .fixed_size([360.0, 240.0])
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.vertical_centered(|ui| {
                        ui.strong_heading("U-King Mod Manager");
                        ui.label("© 2022 Caleb Smith - GPLv3");
                        ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
                    });
                    egui::Grid::new("about_box").num_columns(2).show(ui, |ui| {
                        ui.label("GitHub:");
                        if ui.link("https://github.com/NiceneNerd/ukmm").clicked() {
                            open::that("https://github.com/NiceneNerd/ukmm").unwrap_or(());
                        }
                        ui.end_row();
                        ui.label("GUI library:");
                        if ui.link("egui (forked)").clicked() {
                            open::that("https://github.com/NiceneNerd/egui").unwrap_or(());
                        }
                        ui.end_row();
                    });
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::CloseAbout);
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    pub fn render_profile_menu(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .inner_margin(Margin {
                left: 2.0,
                ..Default::default()
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let current_profile = self
                        .core
                        .settings()
                        .platform_config()
                        .map(|c| c.profile.to_string())
                        .unwrap_or_else(|| "Default".to_owned());
                    ComboBox::from_id_source("profiles")
                        .selected_text(&current_profile)
                        .show_ui(ui, |ui| {
                            self.core.settings().profiles().for_each(|profile| {
                                if ui
                                    .selectable_label(
                                        profile.as_str() == current_profile,
                                        profile.as_str(),
                                    )
                                    .clicked()
                                    && current_profile != profile
                                {
                                    self.do_update(Message::ChangeProfile(profile.into()));
                                }
                            });
                        })
                        .response
                        .on_hover_text("Select Mod Profile");
                    if ui
                        .icon_button(Icon::Add)
                        .on_hover_text("New Profile")
                        .clicked()
                    {
                        self.do_update(Message::NewProfile);
                    };
                    if ui
                        .icon_button(Icon::Menu)
                        .on_hover_text("Manage Profiles…")
                        .clicked()
                    {
                        self.profiles_state.show = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(20.);
                        ui.label(
                            RichText::new(format!(
                                "{} Mods / {} Active",
                                self.mods.len(),
                                self.mods.iter().filter(|m| m.enabled).count()
                            ))
                            .strong(),
                        );
                    });
                });
            });
    }

    pub fn render_pending(&self, ui: &mut Ui) {
        if !self.dirty.is_empty() {
            egui::Window::new("Pending Changes")
                .anchor(Align2::RIGHT_BOTTOM, [-32.0, -32.0])
                .collapsible(true)
                .show(ui.ctx(), |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        egui::ScrollArea::new([false, true])
                            .id_source("pending_files")
                            .auto_shrink([true, true])
                            .max_height(200.)
                            .show(ui, |ui| {
                                egui::CollapsingHeader::new("Files Pending Update").show(
                                    ui,
                                    |ui| {
                                        info::render_manifest(&self.dirty, ui);
                                    },
                                );
                            });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.icon_text_button("Apply", Icon::Check).clicked() {
                                    self.do_update(Message::Apply);
                                }
                                if ui.icon_text_button("Cancel", Icon::Cancel).clicked() {
                                    self.do_update(Message::ResetMods);
                                }
                            });
                        });
                    });
                });
        }
    }
}
