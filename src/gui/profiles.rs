use std::path::Path;

use anyhow::Result;
use fs_err as fs;
use uk_ui::{
    egui::{self, text::LayoutJob, Layout, RichText, TextStyle, Ui},
    icons::IconButtonExt,
};

use super::{visuals, App, Message};

#[derive(Debug, Default)]
pub struct ProfileManagerState {
    pub selected: Option<Result<SelectedProfile>>,
    pub rename:   Option<String>,
    pub show:     bool,
}

#[derive(Debug, Clone)]
pub struct SelectedProfile {
    name: smartstring::alias::String,
    data: uk_manager::mods::Profile,
}

impl SelectedProfile {
    pub fn load(profiles_dir: &Path, name: &str) -> Result<Self> {
        let path = profiles_dir.join(name).join("profile.yml");
        let text = fs::read_to_string(path)?;
        let data = serde_yaml::from_str(&text)?;
        Ok(Self {
            name: name.into(),
            data,
        })
    }
}

impl App {
    fn render_selected_profile(
        sender: flume::Sender<Message>,
        profile: &SelectedProfile,
        rename: &mut Option<String>,
        ui: &mut Ui,
    ) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::new([true, true])
                    .min_scrolled_height(128.0)
                    .id_source("mods_scroll")
                    .show(ui, |ui| {
                        let mods = profile.data.mods();
                        if !mods.is_empty() {
                            profile
                                .data
                                .load_order()
                                .iter()
                                .map(|h| mods.get(h).unwrap())
                                .for_each(|m| {
                                    let mut job = LayoutJob::simple_singleline(
                                        m.meta.name.as_str().to_owned(),
                                        ui.style()
                                            .text_styles
                                            .get(&TextStyle::Body)
                                            .unwrap()
                                            .clone(),
                                        ui.style().visuals.text_color(),
                                    );
                                    job.wrap.break_anywhere = true;
                                    job.wrap.max_rows = 1;
                                    ui.label(job);
                                });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("No mods in profile");
                            });
                        }
                        ui.allocate_space(ui.available_size());
                    });
                ui.add_space(8.0);
                if let Some(name) = rename {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(name);
                        if ui
                            .icon_button(uk_ui::icons::Icon::Check)
                            .on_hover_text("Save")
                            .clicked()
                        {
                            sender
                                .send(Message::RenameProfile(
                                    profile.name.to_string(),
                                    name.clone(),
                                ))
                                .unwrap();
                        }
                    });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        *rename = Some(profile.name.to_string());
                    }
                    if ui.button("Duplicate").clicked() {
                        sender
                            .send(Message::DuplicateProfile(profile.name.to_string()))
                            .unwrap();
                    }
                    if ui.button("Delete").clicked() {
                        sender
                            .send(Message::Confirm(
                                Message::DeleteProfile(profile.name.to_string()).into(),
                                format!(
                                    "Are you sure you want to delete the profile {}?",
                                    profile.name.as_str()
                                ),
                            ))
                            .unwrap();
                    }
                });
            });
        });
        ui.end_row();
        ui.allocate_space(ui.available_size());
    }

    pub fn render_profiles_modal(&mut self, ctx: &egui::Context) {
        if self.profiles_state.show {
            egui::Window::new("Profiles")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .resizable(true)
                .default_size([320.0, 240.0])
                .show(ctx, |ui| {
                    let current_profile = {
                        let settings = self.core.settings();
                        self.profiles_state.selected.get_or_insert_with(|| {
                            SelectedProfile::load(
                                &settings.profiles_dir(),
                                settings
                                    .platform_config()
                                    .as_ref()
                                    .map(|c| c.profile.as_str())
                                    .unwrap_or(""),
                            )
                        })
                    };
                    match current_profile {
                        Ok(current_profile) => {
                            egui::Grid::new("profiles_grid")
                                .num_columns(2)
                                .striped(false)
                                .show(ui, |ui| {
                                    let sender = self.channel.0.clone();
                                    let settings = self.core.settings();
                                    ui.group(|ui| {
                                        ui.vertical(|ui| {
                                            settings.profiles().for_each(|p| {
                                                if ui
                                                    .selectable_label(
                                                        current_profile.name == p,
                                                        p.as_str(),
                                                    )
                                                    .clicked()
                                                {
                                                    sender
                                                        .send(Message::SelectProfileManage(p))
                                                        .unwrap();
                                                }
                                            });
                                        });
                                        ui.allocate_space(ui.available_size());
                                    });
                                    Self::render_selected_profile(
                                        self.channel.0.clone(),
                                        &*current_profile,
                                        &mut self.profiles_state.rename,
                                        ui,
                                    );
                                });
                        }
                        Err(e) => {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new(format!("Error loading profile: {:#?}", e))
                                        .color(visuals::RED),
                                );
                            });
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.do_update(Message::CloseProfiles);
                            }
                        });
                    });
                });
        }
    }
}
