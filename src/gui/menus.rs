use super::*;

impl App {
    pub fn render_menu(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.style_mut().visuals.button_frame = false;
            ui.set_enabled(!self.modal_open());
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| self.file_menu(ui, frame));
                ui.menu_button("Tools", |ui| self.tool_menu(ui));
                ui.menu_button("Window", |ui| self.window_menu(ui));
                ui.menu_button("Help", |ui| self.help_menu(ui));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(self.platform().to_string().to_uppercase())
                            .family(egui::FontFamily::Name("Bold".into())),
                    );
                });
            });
        });
    }

    pub fn file_menu(&self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if ui.button("Open mod…").clicked() {
            ui.close_menu();
            self.do_update(Message::SelectFile);
        }
        if ui.button("Exit").clicked() {
            frame.close();
        }
    }

    pub fn tool_menu(&mut self, ui: &mut Ui) {
        if ui.button("Refresh Merge").clicked() {
            ui.close_menu();
            self.do_update(Message::Remerge);
        }
        if ui.button("Reset Pending").clicked() {
            ui.close_menu();
            self.do_update(Message::ResetPending);
        }
    }

    pub fn window_menu(&mut self, ui: &mut Ui) {
        if ui.button("Reset").clicked() {
            ui.close_menu();
            *self.tree.write() = tabs::default_ui();
        }
        ui.separator();
        for tab in [
            Tabs::Info,
            Tabs::Install,
            Tabs::Deploy,
            Tabs::Mods,
            Tabs::Settings,
            Tabs::Log,
        ] {
            let disabled = self.closed_tabs.contains_key(&tab);
            if ui
                .icon_text_button(
                    format!(" {tab}"),
                    if disabled { Icon::Blank } else { Icon::Check },
                )
                .clicked()
            {
                ui.close_menu();
                let mut tree = self.tree.write();
                if let Some(parent) = self.closed_tabs.remove(&tab) {
                    if let Some(parent) =
                        tree.iter_mut().nth(parent.0).filter(|p| p.tabs_count() > 0)
                    {
                        parent.append_tab(tab);
                    } else {
                        tree.push_to_focused_leaf(tab);
                    }
                } else if let Some((parent_index, node_index)) = tree.find_tab(&tab) {
                    let parent = tree.iter_mut().nth(parent_index.0).unwrap();
                    parent.remove_tab(node_index);
                    self.closed_tabs.insert(tab, parent_index);
                    tree.remove_empty_leaf();
                }
            }
        }
    }

    pub fn help_menu(&self, ui: &mut Ui) {
        let verbose = crate::logger::LOGGER.debug();
        let verbose_button = if verbose {
            ui.icon_text_button(" Verbose Logging", Icon::Check)
        } else {
            ui.button("Verbose Logging")
        };
        ui.separator();
        if verbose_button.clicked() {
            ui.close_menu();
            crate::logger::LOGGER.set_debug(!verbose);
            log::debug!("Verbose logging enabled"); // Think about it for a second
        }
        if ui.button("Help").clicked() {
            ui.close_menu();
            open::that("https://nicenenerd.github.io/ukmm").unwrap_or(());
        }
        if ui.button("About").clicked() {
            ui.close_menu();
            self.do_update(Message::ShowAbout);
        }
    }
}
