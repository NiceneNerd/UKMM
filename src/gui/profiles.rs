use smartstring::alias::String as SmartString;
use strfmt::Format;
use uk_app::{Command, ProfileService};
use uk_content::util::HashMap;
use uk_localization::string_ext::LocString;
use uk_manager::mods::Profile as ProfileData;
use uk_ui::{
    egui::{self, Layout, TextStyle, text::LayoutJob},
    icons::IconButtonExt,
};

use super::{App, Message};

#[derive(Debug, Default)]
pub struct ProfileManagerState {
    pub profiles: HashMap<SmartString, ProfileData>,
    pub selected: Option<SmartString>,
    pub rename:   Option<String>,
    pub show:     bool,
}

impl ProfileManagerState {
    pub fn new(core: &uk_manager::core::Manager) -> Self {
        let mut state = Self::default();
        state.reload(core);
        state
    }

    /// Re-reads every profile's data through the app-layer profile service
    /// (validated names, no raw fs in the frontend).
    pub fn reload(&mut self, core: &uk_manager::core::Manager) {
        let settings = core.settings();
        let service = ProfileService::new(&settings);
        self.profiles = service
            .list()
            .into_iter()
            .filter_map(|name| {
                service
                    .read(&name)
                    .ok()
                    .flatten()
                    .map(|data| (name.into_string().into(), data))
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
                        .id_salt("mods_scroll")
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
                                    ui.label("Profile_NoMods".localize());
                                });
                            }
                            let available = ui.available_size();
                            ui.allocate_space([available.x.max(0.0), available.y.max(0.0)].into());
                        });
                    ui.add_space(8.0);
                    let rename_target = self.rename.clone();
                    if let Some(new_name) = rename_target {
                        ui.horizontal(|ui| {
                            let mut new_name = new_name;
                            ui.text_edit_singleline(&mut new_name);
                            if ui
                                .icon_button(uk_ui::icons::Icon::Check)
                                .on_hover_text("Generic_Save".localize())
                                .clicked()
                            {
                                match uk_app::ProfileName::parse(&new_name) {
                                    Ok(to) => {
                                        app.session.dispatch(Command::RenameProfile {
                                            from: uk_app::ProfileName::from_existing(name),
                                            to,
                                        });
                                        self.rename = None;
                                    }
                                    Err(e) => app.do_update(Message::Error(e)),
                                }
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Profile_Rename".localize()).clicked() {
                            self.rename = Some(name.to_string());
                        }
                        if ui.button("Profile_Duplicate".localize()).clicked() {
                            app.session.dispatch(Command::DuplicateProfile(
                                uk_app::ProfileName::from_existing(name),
                            ));
                        }
                        if ui.button("Generic_Delete".localize()).clicked() {
                            let message = "Profile_Delete_Confirmation".localize();
                            let vars = std::collections::HashMap::from([(
                                "profile_name".to_string(),
                                name.to_string(),
                            )]);
                            app.do_update(Message::Confirm(
                                Command::DeleteProfile(uk_app::ProfileName::from_existing(name))
                                    .into(),
                                message.format(&vars).unwrap(),
                            ));
                        }
                    });
                });
            });
            ui.end_row();
            let available = ui.available_size();
            ui.allocate_space([available.x.max(0.0), available.y.max(0.0)].into());
        }
    }

    pub fn render(&mut self, app: &App, ctx: &egui::Context) {
        if self.show {
            egui::Window::new("Profile_Label".localize())
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
                                        let label = ui.fonts_mut(|f| {
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
                                let available = ui.available_size();
                                ui.allocate_space(
                                    [available.x.max(0.0), available.y.max(0.0)].into(),
                                );
                            });
                            self.render_selected_profile(app, ui);
                        });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Generic_Close".localize()).clicked() {
                                app.do_update(Message::CloseProfiles);
                            }
                        });
                    });
                });
        }
    }
}
