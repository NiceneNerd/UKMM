use std::path::PathBuf;

use fs_err as fs;
use smartstring::alias::String as SmartString;
use uk_content::util::{HashMap, HashSet};
use uk_manager::mods::Profile as ProfileData;
use uk_ui::{
    egui::{self, text::LayoutJob, Layout, TextStyle},
    icons::IconButtonExt,
};

use super::{App, Message};

#[derive(Debug, Default)]
pub struct ProfileManagerState {
    pub dir: PathBuf,
    pub profiles: HashMap<SmartString, ProfileData>,
    pub selected: Option<SmartString>,
    pub rename: Option<String>,
    pub show: bool,
}

impl ProfileManagerState {
    pub fn new(core: &uk_manager::core::Manager) -> Self {
        let settings = core.settings();
        let dir = settings.profiles_dir();
        let profiles = settings
            .profiles()
            .filter_map(|name| -> Option<(SmartString, ProfileData)> {
                let path = dir.join(name.as_str()).join("profile.yml");
                let text = fs::read_to_string(path).ok()?;
                let data = serde_yaml::from_str(&text).ok()?;
                Some((name, data))
            })
            .collect::<_>();
        Self {
            dir,
            profiles,
            selected: None,
            rename: None,
            show: false,
        }
    }

    pub fn reload(&mut self, core: &uk_manager::core::Manager) {
        let settings = core.settings();
        self.profiles = settings
            .profiles()
            .filter_map(|name| -> Option<(SmartString, ProfileData)> {
                let path = self.dir.join(name.as_str()).join("profile.yml");
                let text = fs::read_to_string(path).ok()?;
                let data = serde_yaml::from_str(&text).ok()?;
                Some((name, data))
            })
            .collect::<_>();
    }

    fn render_selected_profile(&mut self, app: &App, ui: &mut egui::Ui) {
        let name = self
            .selected
            .as_ref()
            .map(|n| n.as_str())
            .unwrap_or("Default");
        if let Some(profile) = self.profiles.get(name) {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    egui::ScrollArea::new([true, true])
                        .min_scrolled_height(128.0)
                        .id_source("mods_scroll")
                        .show(ui, |ui| {
                            let mods = profile.mods();
                            if !mods.is_empty() {
                                profile
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
                    if let Some(new_name) = self.rename.as_mut() {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(new_name);
                            if ui
                                .icon_button(uk_ui::icons::Icon::Check)
                                .on_hover_text("Save")
                                .clicked()
                            {
                                app.do_update(Message::RenameProfile(
                                    name.to_string(),
                                    new_name.clone(),
                                ));
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Rename").clicked() {
                            self.rename = Some(name.to_string());
                        }
                        if ui.button("Duplicate").clicked() {
                            app.do_update(Message::DuplicateProfile(name.to_string()));
                        }
                        if ui.button("Delete").clicked() {
                            app.do_update(Message::Confirm(
                                Message::DeleteProfile(name.to_string()).into(),
                                format!("Are you sure you want to delete the profile {}?", name),
                            ));
                        }
                    });
                });
            });
            ui.end_row();
            ui.allocate_space(ui.available_size());
        }
    }

    pub fn render(&mut self, app: &App, ctx: &egui::Context) {
        if self.show {
            egui::Window::new("Profiles")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .resizable(true)
                .default_size([320.0, 240.0])
                .show(ctx, |ui| {
                    egui::Grid::new("profiles_grid")
                        .num_columns(2)
                        .striped(false)
                        .show(ui, |ui| {
                            let sender = app.channel.0.clone();
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    self.profiles.keys().for_each(|p| {
                                        let font = ui
                                            .style()
                                            .text_styles
                                            .get(&TextStyle::Body)
                                            .expect("Body style is real, bro")
                                            .clone();
                                        let color = ui.style().visuals.text_color();
                                        let label = ui.fonts(|f| {
                                            f.layout_no_wrap(p.as_str().into(), font, color)
                                        });
                                        if ui
                                            .selectable_label(
                                                self.selected
                                                    .as_ref()
                                                    .map(|v| v.as_str())
                                                    .unwrap_or_default()
                                                    == p.as_str(),
                                                label,
                                            )
                                            .clicked()
                                        {
                                            sender
                                                .send(Message::SelectProfileManage(p.clone()))
                                                .unwrap();
                                        }
                                    });
                                });
                                ui.allocate_space(ui.available_size());
                            });
                            self.render_selected_profile(app, ui);
                        });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                app.do_update(Message::CloseProfiles);
                            }
                        });
                    });
                });
        }
    }

    #[inline]
    pub fn all_assigned_mod_hashes(&self) -> HashSet<usize> {
        self.profiles
            .values()
            .map(|m| m.mods()
                .keys()
                .copied()
                .collect::<Vec<_>>()
            )
            .flatten()
            .collect()
    }
}
