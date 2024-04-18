use nk_ui::{
    egui::{self, Label, Sense, Ui, WidgetText},
    egui_dock::{NodeIndex, TabViewer, Tree},
    visuals::Theme,
};

use super::{info, visuals, Component, Tabs};

pub fn default_ui() -> Tree<Tabs> {
    let mut tree = Tree::new(vec![Tabs::Mods, Tabs::Package, Tabs::Settings]);
    let [main, side] = tree.split_right(0.into(), 0.9, vec![Tabs::Info, Tabs::Install]);
    let [_side_top, _side_bottom] = tree.split_below(side, 0.6, vec![Tabs::Deploy]);
    let [main, _log] = tree.split_below(main, 0.99, vec![Tabs::Log]);
    tree.set_focused_node(main);
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
                if let Some(mod_) = self.selected.first() {
                    if let Some(info::Message::RequestOptions) = info::ModInfo(mod_).show(ui).inner
                    {
                        self.do_update(super::Message::RequestOptions(mod_.clone(), true));
                    }
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
                self.render_deploy_tab(ui);
            }
            Tabs::Mods => {
                self.render_profile_menu(ui);
                ui.add_space(4.);
                egui::Frame::none()
                    .inner_margin(0.0)
                    .outer_margin(0.0)
                    .show(ui, |ui| {
                        if self.theme == Theme::Sheikah {
                            visuals::slate_grid(ui);
                        }
                        self.render_modlist(ui);
                        ui.allocate_space(ui.available_size());
                        self.render_pending(ui);
                    });
            }
            Tabs::Log => {
                egui::Frame::none()
                    .fill(ui.style().visuals.extreme_bg_color)
                    .inner_margin(-2.0)
                    .outer_margin(0.0)
                    .show(ui, |ui| {
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
                                ui.allocate_space(ui.available_size());
                            });
                    });
                ui.shrink_height_to_current();
            }
            Tabs::Settings => {
                self.render_settings(ui);
            }
            Tabs::Package => {
                self.package_builder.borrow_mut().render(self, ui);
            }
        }
    }
}
