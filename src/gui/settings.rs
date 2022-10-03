use super::{
    icons::{self, IconButtonExt},
    util::FolderPickerExt,
    App, Message,
};
use crate::settings::{Language, Platform, PlatformSettings, Settings};
use egui::{
    Align, Button, Checkbox, Id, ImageButton, InnerResponse, Layout, Response, RichText, TextStyle,
    Ui,
};
use std::{borrow::Cow, ops::Deref};

fn render_setting<R>(
    name: &str,
    description: &str,
    ui: &mut Ui,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let icon_height = ui.text_style_height(&TextStyle::Small);
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).family(egui::FontFamily::Name("Bold".into())));
        ui.add(
            ImageButton::new(
                icons::get_icon(ui.ctx(), icons::Icon::Info),
                [icon_height, icon_height],
            )
            .frame(false)
            .tint(ui.visuals().text_color()),
        )
        .on_hover_text(description);
    });
    let res = ui.with_layout(Layout::right_to_left(Align::Center), add_contents);
    ui.end_row();
    res
}

fn render_platform_config(config: &mut Option<PlatformSettings>, platform: Platform, ui: &mut Ui) {
    let mut language = config
        .as_ref()
        .map(|c| c.language)
        .unwrap_or(Language::USen);
    render_setting(
        "Language",
        "Select the language and region corresponding to your game version and settings.",
        ui,
        |ui| {
            egui::ComboBox::new(format!("lang-{platform}"), language.to_string())
                .selected_text(language.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_label(false, "USen");
                });
        },
    );
}

impl App {
    pub fn render_settings(&mut self, ui: &mut Ui) {
        egui::Frame::none().inner_margin(4.0).show(ui, |ui| {
            let settings = &mut self.temp_settings;
            ui.vertical(|ui| {
                egui::CollapsingHeader::new("General")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("general_settings")
                            .num_columns(2)
                            .spacing([8.0, 8.0])
                            .show(ui, |ui| {
                                render_setting("Current Mode", "Select whether to manage the Wii U or Switch version of the game", ui, |ui| {
                                    ui.radio_value(
                                        &mut settings.current_mode,
                                        Platform::WiiU,
                                        "Wii U",
                                    );
                                    ui.radio_value(
                                        &mut settings.current_mode,
                                        Platform::Switch,
                                        "Switch",
                                    );
                                });
                                render_setting(
                                    "Storage Folder",
                                    "UKMM will store mods, profiles, and similar data here.",
                                    ui,
                                    |ui| {
                                        ui.folder_picker(&mut settings.storage_dir);
                                    },
                                );
                                render_setting("Unpack Mods", "By default UKMM stores mods as ZIP files with ZSTD compression. Turn on this option to unpack them instead, which will improve performance at the cost of disk space.", ui, |ui| {
                                    ui.add(Checkbox::new(&mut settings.unpack_mods, ""))
                                });
                                render_setting("Show Changelog", "Show a summary of recent changes after UKMM updates.", ui, |ui| {
                                    ui.add(Checkbox::new(&mut settings.show_changelog, ""))
                                });
                            });
                    });
                egui::CollapsingHeader::new("Wii U Config")
                    .show(ui, |ui| {
                        render_platform_config(&mut settings.wiiu_config, Platform::WiiU, ui);
                    });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_enabled_ui(self.temp_settings.ne(self.core.settings().deref()), |ui| {
                        if ui.button("Save").clicked() {
                            self.do_update(Message::SaveSettings);
                        }
                        if ui.button("Reset").clicked() {
                            self.do_update(Message::ResetSettings);
                        }
                    })
                });
            });
        });
    }
}
