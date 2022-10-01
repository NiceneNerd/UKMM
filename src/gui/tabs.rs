use egui::{Ui, WidgetText};
use egui_dock::TabViewer;

impl TabViewer for super::App {
    type Tab = super::Tabs;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.to_string().into()
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        self.closed_tabs.insert(*tab);
        true
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        egui::ScrollArea::vertical()
            .id_source("right_panel_scroll")
            .show(ui, |ui| match tab {
                super::Tabs::Info => {
                    if let Some(mod_) = self.selected.front() {
                        super::info::render_mod_info(mod_, ui);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("No mod selected");
                        });
                    }
                }
                super::Tabs::Install => {
                    self.render_file_picker(ui);
                }
                super::Tabs::Deploy => {
                    ui.label("Deployment stuff");
                }
            });
    }
}
