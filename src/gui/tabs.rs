use super::Tabs;
use egui::{Label, Sense, Ui, WidgetText};
use egui_dock::{NodeIndex, TabViewer, Tree};
use join_str::jstr;

pub fn default_ui() -> Tree<Tabs> {
    let mut tree = Tree::new(vec![Tabs::Mods, Tabs::Settings]);
    let [main, side] = tree.split_right(0.into(), 0.9, vec![Tabs::Info, Tabs::Install]);
    let [side_top, side_bottom] = tree.split_below(side, 0.5, vec![Tabs::Deploy]);
    let [main, log] = tree.split_below(main, 0.99, vec![Tabs::Log]);
    tree
}

impl TabViewer for super::App {
    type Tab = Tabs;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.to_string().into()
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        self.closed_tabs.insert(*tab, NodeIndex::root());
        true
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.set_enabled(!self.modal_open());
        match tab {
            Tabs::Info => {
                if let Some(mod_) = self.selected.front() {
                    super::info::render_mod_info(mod_, ui);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No mod selected");
                    });
                }
            }
            Tabs::Install => {
                self.render_file_picker(ui);
            }
            Tabs::Deploy => {
                if let Some(config) = self
                    .core
                    .settings()
                    .platform_config()
                    .and_then(|c| c.deploy_config.as_ref())
                {
                    ui.label(jstr!("Deployment method: {config.method.name()}"));
                    ui.label(config.output.display().to_string());
                    ui.label(jstr!(
                        "Deployment pending: {&self.core.deploy_manager().pending().to_string()}"
                    ));
                }
            }
            Tabs::Mods => {
                self.render_profile_menu(ui);
                ui.add_space(4.);
                self.render_modlist(ui);
                ui.allocate_space(ui.available_size());
            }
            Tabs::Log => {
                egui::ScrollArea::new([true, true])
                    .auto_shrink([false, true])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if ui
                            .add(Label::new(self.log.clone()).sense(Sense::click()))
                            .on_hover_text("Click to copy")
                            .clicked()
                        {
                            ui.output().copied_text = self.log.text.clone();
                        }
                        ui.shrink_height_to_current();
                    });
                ui.shrink_height_to_current();
            }
            Tabs::Settings => {
                ui.label("Settings stuff");
            }
        }
    }
}
