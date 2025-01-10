use uk_ui::{
    egui::{self, Ui, WidgetText},
    egui_dock::{DockState, Node, NodeIndex, TabViewer},
    visuals::Theme,
};

use super::{info, visuals, Component, Tabs, LOCALIZATION};

pub fn default_ui() -> DockState<Tabs> {
    let mut state = DockState::new(vec![Tabs::Mods, Tabs::Package, Tabs::Settings]);
    let [main, side] = state.split(
        (0.into(), 0.into()),
        uk_ui::egui_dock::Split::Right,
        0.85,
        Node::leaf_with(vec![Tabs::Info, Tabs::Install]),
    );
    let [_side_top, _side_bottom] = state.split(
        (0.into(), side),
        uk_ui::egui_dock::Split::Below,
        0.6,
        Node::leaf(Tabs::Deploy),
    );
    let [main, _log] = state.split(
        (0.into(), main),
        uk_ui::egui_dock::Split::Below,
        0.75,
        Node::leaf(Tabs::Log),
    );
    state.set_focused_node_and_surface((0.into(), main));
    state
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
        ui.add_enabled_ui(!self.modal_open(), |ui| {
            match tab {
                Tabs::Info => {
                    if let Some(mod_) = self.selected.first() {
                        if let Some(info::Message::RequestOptions) =
                            info::ModInfo(mod_).show(ui).inner
                        {
                            self.do_update(super::Message::RequestOptions(mod_.clone(), true));
                        }
                    } else {
                        let loc = LOCALIZATION.read();
                        ui.centered_and_justified(|ui| {
                            ui.label(loc.get("Mod_Selected_None"));
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
                            egui_logger::logger_ui().show(ui);
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
        });
    }
}
