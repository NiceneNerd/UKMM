use uk_localization::string_ext::LocString;
use super::*;

impl App {
    pub fn render_menu(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.style_mut().visuals.button_frame = false;
            ui.add_enabled_ui(!self.modal_open(), |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Menu_File".localize(), |ui| self.file_menu(ui, frame));
                    ui.menu_button("Menu_Tools".localize(), |ui| self.tool_menu(ui));
                    ui.menu_button("Menu_Window".localize(), |ui| self.window_menu(ui));
                    ui.menu_button("Menu_Help".localize(), |ui| self.help_menu(ui));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(self.platform().to_string().to_uppercase())
                                .family(egui::FontFamily::Name("Bold".into())),
                        );
                    });
                });
            });
        });
    }

    pub fn file_menu(&self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        if ui.button("Menu_File_Open".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            self.do_update(Message::SelectFile);
        }
        if ui.button("Generic_Exit".localize()).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub fn tool_menu(&mut self, ui: &mut Ui) {
        if ui.button("Menu_Tools_RefreshMerge".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            self.do_update(Message::Remerge);
        }
        if ui.button("Menu_Tools_ResetPending".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            self.do_update(Message::ResetPending);
        }
        if ui.button("Menu_Tools_ConfigFolder".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            open::that(Settings::config_dir()).unwrap_or(());
        }
        let settings = self.core.settings();
        if ui.button("Menu_Tools_StorageFolder".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            open::that(&settings.storage_dir).unwrap_or(());
        }
        let deploy_dir = settings.deploy_dir();
        if ui
            .add_enabled(
                deploy_dir.is_some(),
                egui::Button::new("Menu_Tools_DeployFolder".localize()),
            )
            .clicked()
        {
            ui.close_kind(egui::UiKind::Menu);
            open::that(deploy_dir.unwrap()).unwrap_or(());
        }
    }

    pub fn window_menu(&mut self, ui: &mut Ui) {
        if ui.button("Menu_Window_Reset".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            *self.tree.borrow_mut() = tabs::default_ui();
        }
        ui.separator();
        for tab in [
            Tabs::Info,
            Tabs::Install,
            Tabs::Deploy,
            Tabs::Mods,
            Tabs::Package,
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
                ui.close_kind(egui::UiKind::Menu);
                if let Some(parent) = self.closed_tabs.remove(&tab) {
                    let mut tree = self.tree.borrow_mut();
                    let mut has_parent = false;
                    if let Some(parent) = tree
                        .iter_all_nodes_mut()
                        .nth(parent.0)
                        .filter(|p| p.1.tabs_count() > 0)
                    {
                        has_parent = true;
                        parent.1.append_tab(tab);
                    }
                    if !has_parent {
                        tree.push_to_focused_leaf(tab);
                    }
                } else {
                    let mut tree = self.tree.borrow_mut();
                    if let Some((_, parent_index, node_index)) = tree.find_tab(&tab) {
                        let parent = tree.iter_all_nodes_mut().nth(parent_index.0).unwrap();
                        parent.1.remove_tab(node_index);
                        self.closed_tabs.insert(tab, parent_index);
                    }
                }
            }
        }
    }

    pub fn help_menu(&self, ui: &mut Ui) {
        if ui.button("Menu_Help".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            open::that("https://nicenenerd.github.io/UKMM").unwrap_or(());
        }
        if ui.button("Menu_Help_About".localize()).clicked() {
            ui.close_kind(egui::UiKind::Menu);
            self.do_update(Message::ShowAbout);
        }
    }
}
