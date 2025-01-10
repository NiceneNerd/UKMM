use eframe::egui::Button;
use uk_mod::ModOptionGroup;
use uk_ui::{
    egui::{self, Align, Checkbox, Context, Layout, Vec2},
    visuals,
};

use super::{App, Message, LOCALIZATION};

impl App {
    pub fn render_option_picker(&mut self, ctx: &Context) {
        let is_opt_mod = self.options_mod.is_some();
        if !is_opt_mod {
            return;
        }
        let loc = LOCALIZATION.read();
        egui::Window::new(loc.get("Options_Select"))
            .collapsible(false)
            .scroll([false, true])
            .anchor(egui::Align2::CENTER_CENTER, Vec2::default())
            .show(ctx, |ui| {
                let mod_ = unsafe { &mut self.options_mod.as_mut().unwrap_unchecked().0 };
                let mut done = true;
                mod_.meta.options.iter().for_each(|group| {
                    egui::CollapsingHeader::new(group.name())
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                if !group.description().is_empty() {
                                    ui.label(group.description());
                                }
                                match group {
                                    uk_mod::OptionGroup::Exclusive(group) => {
                                        if !group.required
                                            && ui
                                                .radio(
                                                    !group.options.iter().any(|opt| {
                                                        mod_.enabled_options.contains(opt)
                                                    }),
                                                    loc.get("Options_None"),
                                                )
                                                .clicked()
                                        {
                                            mod_.enabled_options
                                                .retain(|opt| !group.options.contains(opt));
                                        }
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
                                            if !opt.description.is_empty() {
                                                ui.small(opt.description.as_str());
                                            }
                                        });
                                    }
                                    uk_mod::OptionGroup::Multiple(group) => {
                                        group.options.iter().for_each(|opt| {
                                            let mut checked = mod_.enabled_options.contains(opt);
                                            if ui
                                                .add(Checkbox::new(&mut checked, opt.name.as_str()))
                                                .clicked()
                                            {
                                                if checked {
                                                    mod_.enabled_options.push(opt.clone());
                                                } else {
                                                    mod_.enabled_options.retain(|o| o != opt);
                                                }
                                            }
                                            if !opt.description.is_empty() {
                                                ui.small(opt.description.as_str());
                                            }
                                        });
                                    }
                                }
                            });
                        });
                    if group.required()
                        && !group
                            .options()
                            .iter()
                            .any(|opt| mod_.enabled_options.contains(opt))
                    {
                        done = false;
                    }
                });
                if !done {
                    ui.colored_label(visuals::RED, loc.get("Options_Required"));
                }
                ui.horizontal(|ui| {
                    ui.add_space(2.);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add_enabled(done, Button::new(loc.get("Generic_OK"))).clicked() {
                            let (mod_, update) = self.options_mod.take().unwrap();
                            if update {
                                self.do_update(Message::UpdateOptions(mod_));
                            } else {
                                self.do_update(Message::InstallMod(mod_));
                            }
                        }
                        if ui.button(loc.get("Generic_Cancel")).clicked() {
                            self.options_mod = None;
                        }
                    });
                });
            });
    }
}
