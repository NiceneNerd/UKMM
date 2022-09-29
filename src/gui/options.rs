use super::App;
use crate::mods::Mod;
use anyhow::Result;
use egui::{Align, Checkbox, Context, Layout, Vec2};
use uk_mod::ModOptionGroup;

impl App {
    pub fn render_option_picker(&mut self, ctx: &Context) {
        let is_opt_mod = self.options_mod.is_some();
        if is_opt_mod {
            egui::Window::new("Select Mod Options")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::default())
                .show(ctx, |ui| {
                    if let Some(mod_) = self.options_mod.as_mut() {
                        mod_.meta.options.iter().for_each(|group| {
                            egui::CollapsingHeader::new(group.name())
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(group.description());
                                        match group {
                                            uk_mod::OptionGroup::Exclusive(group) => {
                                                group.options.iter().for_each(|opt| {
                                                    if ui
                                                        .radio(
                                                            mod_.enabled_options.contains(opt),
                                                            opt.name.as_str(),
                                                        )
                                                        .clicked()
                                                    {
                                                        mod_.enabled_options
                                                            .retain(|o| !group.options.contains(o));
                                                        mod_.enabled_options.push(opt.clone());
                                                    }
                                                });
                                            }
                                            uk_mod::OptionGroup::Multiple(group) => {
                                                group.options.iter().for_each(|opt| {
                                                    let mut checked =
                                                        mod_.enabled_options.contains(opt);
                                                    if ui
                                                        .add(Checkbox::new(
                                                            &mut checked,
                                                            opt.name.as_str(),
                                                        ))
                                                        .clicked()
                                                    {
                                                        if checked {
                                                            mod_.enabled_options.push(opt.clone());
                                                        } else {
                                                            mod_.enabled_options
                                                                .retain(|o| o != opt);
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    });
                                });
                        });
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("OK").clicked() {
                            let mod_ = self.options_mod.as_ref().unwrap().clone();
                            self.do_update(todo!());
                        }
                        if ui.button("Cancel").clicked() {
                            self.options_mod = None;
                        }
                        ui.shrink_width_to_current();
                    });
                });
        }
    }
}
