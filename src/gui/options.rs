use eframe::egui::Button;
use uk_app::{Command, ModId, OptionsPrompt};
use uk_localization::string_ext::LocString;
use uk_mod::{ModOption, ModOptionGroup, OptionGroup};
use uk_ui::{
    egui::{self, Align, Checkbox, Context, Layout, Vec2},
    visuals,
};

use super::App;

/// The options modal: either resolving an install prompt (from the session)
/// or editing an installed mod's picks (from the snapshot).
pub enum OptionsModal {
    Install {
        prompt: OptionsPrompt,
        picks:  Vec<ModOption>,
    },
    Edit {
        id:     ModId,
        groups: Vec<OptionGroup>,
        picks:  Vec<ModOption>,
    },
}

impl OptionsModal {
    /// Opens the modal to edit an installed mod's options.
    pub fn for_mod(id: ModId, groups: Vec<OptionGroup>, picks: Vec<ModOption>) -> Self {
        Self::Edit { id, groups, picks }
    }
}

impl App {
    pub fn render_option_picker(&mut self, ctx: &Context) {
        if self.options_modal.is_none() {
            return;
        }
        egui::Window::new("Options_Select".localize())
            .collapsible(false)
            .scroll([false, true])
            .anchor(egui::Align2::CENTER_CENTER, Vec2::default())
            .show(ctx, |ui| {
                let mut done = true;
                let mut ok_clicked = false;
                {
                    let modal = self.options_modal.as_mut().unwrap();
                    let (groups, picks): (&[OptionGroup], &mut Vec<ModOption>) = match modal {
                        OptionsModal::Install { prompt, picks } => (prompt.groups(), picks),
                        OptionsModal::Edit { groups, picks, .. } => (groups, picks),
                    };
                    for group in groups {
                        egui::CollapsingHeader::new(group.name())
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 8.0;
                                    let desc = match group {
                                        OptionGroup::Multiple(g) => g.description(),
                                        OptionGroup::Exclusive(g) => g.description(),
                                    };
                                    if !desc.is_empty() {
                                        ui.label(desc);
                                    }
                                    match group {
                                        OptionGroup::Exclusive(group) => {
                                            if !group.required
                                                && ui
                                                    .radio(
                                                        !group
                                                            .options
                                                            .iter()
                                                            .any(|opt| picks.contains(opt)),
                                                        "Options_None".localize(),
                                                    )
                                                    .clicked()
                                            {
                                                picks.retain(|opt| !group.options.contains(opt));
                                            }
                                            group.options.iter().for_each(|opt| {
                                                if ui
                                                    .radio(picks.contains(opt), opt.name.as_str())
                                                    .clicked()
                                                {
                                                    picks.retain(|o| !group.options.contains(o));
                                                    picks.push(opt.clone());
                                                }
                                                if !opt.description.is_empty() {
                                                    ui.small(opt.description.as_str());
                                                }
                                            });
                                        }
                                        OptionGroup::Multiple(group) => {
                                            group.options.iter().for_each(|opt| {
                                                let mut checked = picks.contains(opt);
                                                if ui
                                                    .add(Checkbox::new(
                                                        &mut checked,
                                                        opt.name.as_str(),
                                                    ))
                                                    .clicked()
                                                {
                                                    if checked {
                                                        picks.push(opt.clone());
                                                    } else {
                                                        picks.retain(|o| o != opt);
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
                        if !uk_app::required_satisfied(std::slice::from_ref(group), picks) {
                            done = false;
                        }
                    }
                }
                if !done {
                    ui.colored_label(visuals::RED, "Options_Required".localize());
                }
                ui.horizontal(|ui| {
                    ui.add_space(2.);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(done, Button::new("Generic_OK".localize()))
                            .clicked()
                        {
                            ok_clicked = true;
                        }
                        if ui.button("Generic_Cancel".localize()).clicked() {
                            self.options_modal = None;
                        }
                    });
                });
                if ok_clicked {
                    let modal = self.options_modal.take().unwrap();
                    match modal {
                        OptionsModal::Install { prompt, picks } => {
                            if let Some(candidate) = prompt.resolve(picks) {
                                self.session.dispatch(Command::Install(candidate));
                            }
                        }
                        OptionsModal::Edit { id, picks, .. } => {
                            self.session.dispatch(Command::SetOptions { id, picks });
                        }
                    }
                }
            });
    }
}
