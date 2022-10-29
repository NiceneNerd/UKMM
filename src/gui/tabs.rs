use super::{visuals, Tabs};
use eframe::epaint::text::TextWrapping;
use egui_dock::{NodeIndex, TabViewer, Tree};
use uk_ui::egui::{
    self, text::LayoutJob, Align, Button, Label, Layout, RichText, Sense, Ui, WidgetText,
};

pub fn default_ui() -> Tree<Tabs> {
    let mut tree = Tree::new(vec![Tabs::Mods, Tabs::Settings]);
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
                if let Some(mod_) = self.selected.front() {
                    self.render_mod_info(mod_, ui);
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
                                    ui.label(job).on_hover_text(config.output.to_string_lossy())
                                });
                                if !config.auto || self.core.deploy_manager().pending() {
                                    ui.add_space(4.);
                                    ui.with_layout(Layout::from_main_dir_and_cross_align(egui::Direction::BottomUp, Align::Center), |ui| {
                                        egui::Frame::none().show(ui, |ui| {
                                            if ui.add_enabled(pending, Button::new("Deploy")).clicked()
                                            {
                                                self.do_update(super::Message::Deploy);
                                            }
                                            if config.auto {
                                                ui.label(RichText::new("Auto deploy incomplete, please deploy manually").color(visuals::RED));
                                            }
                                        });
                                    });
                                }
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
            Tabs::Mods => {
                self.render_profile_menu(ui);
                ui.add_space(4.);
                egui::Frame::none()
                    .inner_margin(0.0)
                    .outer_margin(0.0)
                    .show(ui, |ui| {
                        visuals::slate_grid(ui);
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
            Tabs::Theme => {
                self.style.ui(ui);
            }
        }
    }
}
