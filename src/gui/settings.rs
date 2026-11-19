#![allow(unstable_name_collisions)]
use uk_app::{
    model::{DeployConfig, Language, Platform},
    settings::{DumpDraft, PlatformDraft},
};
use uk_localization::{LocLang, string_ext::LocString};
use uk_ui::{
    egui::{self, Align, Button, Checkbox, InnerResponse, Layout, RichText, TextStyle, Ui},
    ext::UiExt,
    icons::{self, IconButtonExt},
    visuals::Theme,
};

use super::{App, Message};

fn render_setting<R>(
    name: &str,
    description: &str,
    ui: &mut Ui,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let _icon_height = ui.text_style_height(&TextStyle::Small);
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).family(egui::FontFamily::Name("Bold".into())));
        ui.add(
            Button::image(icons::get_icon(ui.ctx(), icons::Icon::Info))
                .frame(false)
                .image_tint_follows_text_color(true),
        )
        .on_hover_text(description);
    });
    ui.horizontal(|ui| add_contents(ui))
}

fn render_deploy_config(config: &mut DeployConfig, platform: Platform, ui: &mut Ui) -> bool {
    ui.label("Settings_Platform_Deploy".localize());
    let mut changed = false;
    ui.group(|ui| {
        let width = ui.available_width().max(0.0);
        ui.allocate_space([width, 0.0].into());
        let mut name = "Settings_Platform_Deploy_Method".localize();
        let mut description = "Settings_Platform_Deploy_Method_Desc".localize();
        render_setting(&name, &description, ui, |ui| {
            changed |= ui
                .radio_value(
                    &mut config.method,
                    uk_app::model::DeployMethod::Copy,
                    "Settings_Platform_Deploy_Method_Copy".localize(),
                )
                .changed();
            changed |= ui
                .radio_value(
                    &mut config.method,
                    uk_app::model::DeployMethod::HardLink,
                    "Settings_Platform_Deploy_Method_HardLink".localize(),
                )
                .changed();
            changed |= ui
                .radio_value(
                    &mut config.method,
                    uk_app::model::DeployMethod::Symlink,
                    "Settings_Platform_Deploy_Method_Symlink".localize(),
                )
                .changed();
        });
        name = "Settings_Platform_Deploy_Layout".localize();
        description = match platform {
            Platform::WiiU => "Settings_Platform_Deploy_Layout_WiiU_Desc".localize(),
            Platform::Switch => "Settings_Platform_Deploy_Layout_NX_Desc".localize(),
        };
        render_setting(&name, &description, ui, |ui| {
            changed |= ui
                .radio_value(
                    &mut config.layout,
                    uk_app::model::DeployLayout::WithoutName,
                    match platform {
                        Platform::WiiU => {
                            "Settings_Platform_Deploy_Layout_WiiU_WithoutName".localize()
                        }
                        Platform::Switch => {
                            "Settings_Platform_Deploy_Layout_NX_WithoutName".localize()
                        }
                    },
                )
                .changed();
            changed |= ui
                .radio_value(
                    &mut config.layout,
                    uk_app::model::DeployLayout::WithName,
                    match platform {
                        Platform::WiiU => {
                            "Settings_Platform_Deploy_Layout_WiiU_WithName".localize()
                        }
                        Platform::Switch => {
                            "Settings_Platform_Deploy_Layout_NX_WithName".localize()
                        }
                    },
                )
                .changed();
        });
        name = "Settings_Platform_Deploy_Auto".localize();
        description = "Settings_Platform_Deploy_Auto_Desc".localize();
        render_setting(&name, &description, ui, |ui| {
            changed |= ui.checkbox(&mut config.auto, "").changed();
        });
        if platform == Platform::WiiU {
            name = "Settings_Platform_Deploy_Rules".localize();
            description = "Settings_Platform_Deploy_Rules_Desc".localize();
            render_setting(&name, &description, ui, |ui| {
                changed |= ui.checkbox(&mut config.cemu_rules, "").changed();
            });
            ui.add_space(8.0);
        }
        name = "Settings_Platform_Deploy_Output".localize();
        description = "Settings_Platform_Deploy_Output_Desc".localize();
        render_setting(&name, &description, ui, |ui| {
            changed |= ui.folder_picker(&mut config.output).changed();
        });
        name = "Settings_Platform_Deploy_Emu".localize();
        description = "Settings_Platform_Deploy_Emu_Desc".localize();
        render_setting(&name, &description, ui, |ui| {
            changed |= ui
                .file_picker_string(config.executable.get_or_insert_default())
                .changed();
        });
    });
    changed
}

fn render_platform_config(config: &mut PlatformDraft, platform: Platform, ui: &mut Ui) -> bool {
    let mut changed = false;
    let config = config;
    let mut name = "Settings_Platform_Language".localize();
    let mut description = "Settings_Platform_Language_Desc".localize();
    render_setting(&name, &description, ui, |ui| {
        egui::ComboBox::new(format!("lang-{platform}"), "")
            .selected_text(config.language.to_str())
            .show_ui(ui, |ui| {
                Language::iter().for_each(|lang| {
                    changed |= ui
                        .selectable_value(&mut config.language, *lang, lang.to_str())
                        .changed();
                });
            });
    });
    ui.add_space(8.0);
    ui.label("Settings_Platform_Dump".localize());
    ui.group(|ui| {
        let width = ui.available_width().max(0.0);
        ui.allocate_space([width, 0.0].into());
        if platform == Platform::WiiU {
            name = "Settings_Platform_Dump_Type".localize();
            description = "Settings_Platform_Dump_Type_Desc".localize();
            render_setting(&name, &description, ui, |ui| {
                if ui
                    .radio(
                        matches!(config.dump, DumpDraft::Unpacked { .. }),
                        "Settings_Platform_Dump_Type_Unpacked".localize(),
                    )
                    .clicked()
                {
                    config.dump = DumpDraft::Unpacked {
                        host_path:   Default::default(),
                        content_dir: Default::default(),
                        update_dir:  Default::default(),
                        aoc_dir:     Default::default(),
                    };
                    changed = true;
                }
                if ui
                    .radio(
                        matches!(config.dump, DumpDraft::ZArchive { .. }),
                        "Settings_Platform_Dump_Type_WUA".localize(),
                    )
                    .clicked()
                {
                    config.dump = DumpDraft::ZArchive {
                        content_dir: Default::default(),
                        update_dir:  Default::default(),
                        aoc_dir:     Default::default(),
                        host_path:   Default::default(),
                    };
                    changed = true;
                }
            });
        }
        match &mut config.dump {
            DumpDraft::Unpacked {
                host_path,
                content_dir,
                update_dir,
                aoc_dir,
            } => {
                (name, description) = match platform {
                    Platform::WiiU => {
                        (
                            "Settings_Platform_Dump_WiiU_Base".localize(),
                            "Settings_Platform_Dump_WiiU_Base_Desc".localize(),
                        )
                    }
                    Platform::Switch => {
                        (
                            "Settings_Platform_Dump_NX_Base".localize(),
                            "Settings_Platform_Dump_NX_Base_Desc".localize(),
                        )
                    }
                };
                render_setting(&name, &description, ui, |ui| {
                    if ui
                        .folder_picker(content_dir.get_or_insert_default())
                        .changed()
                    {
                        changed = true;
                        *host_path = "/".into();
                    }
                });
                if platform == Platform::WiiU {
                    name = "Settings_Platform_Dump_Update".localize();
                    description = "Settings_Platform_Dump_Update_Desc".localize();
                    render_setting(&name, &description, ui, |ui| {
                        if ui
                            .folder_picker(update_dir.get_or_insert_default())
                            .changed()
                        {
                            changed = true;
                            *host_path = "/".into();
                        }
                    });
                }
                name = "Settings_Platform_Dump_DLC".localize();
                description = match platform {
                    Platform::WiiU => "Settings_Platform_Dump_DLC_WiiU_Desc".localize(),
                    Platform::Switch => "Settings_Platform_Dump_DLC_NX_Desc".localize(),
                };
                render_setting(&name, &description, ui, |ui| {
                    if ui.folder_picker(aoc_dir.get_or_insert_default()).changed() {
                        changed = true;
                        *host_path = "/".into();
                    }
                });
            }
            DumpDraft::ZArchive {
                content_dir: _,
                update_dir: _,
                aoc_dir: _,
                host_path,
            } => {
                name = "Settings_Platform_Dump_WUA".localize();
                description = "Settings_Platform_Dump_WUA_Desc".localize();
                render_setting(&name, &description, ui, |ui| {
                    changed |= ui.file_picker(host_path).changed();
                });
            }
        }
    });
    changed |= render_deploy_config(&mut config.deploy_config, platform, ui);
    changed
}

impl App {
    pub fn render_settings(&mut self, ui: &mut Ui) {
        egui::Frame::NONE.inner_margin(4.0).show(ui, |ui| {
            let scroll_height = ui.available_height() - 38.0_f32;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(scroll_height)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let settings = &mut self.settings_draft;
                        let mut theme_change: Option<Theme> = None;
                        let mut lang_change: Option<LocLang> = None;
                        egui::CollapsingHeader::new("Settings_General".localize())
                            .default_open(true)
                            .show(ui, |ui| {
                                if ui
                                    .icon_text_button(
                                        "Settings_Migrate".localize(),
                                        icons::Icon::Import,
                                    )
                                    .clicked()
                                {
                                    self.channel
                                        .0
                                        .clone()
                                        .send(Message::MigrateBcml)
                                        .expect("Broken channel");
                                }
                                if ui
                                    .button("Settings_OneClick".localize())
                                    .on_hover_text("Settings_OneClick_Desc".localize())
                                    .clicked()
                                {
                                    self.session.dispatch(uk_app::Command::RegisterUrlHandler);
                                }
                                let mut name = "Settings_Theme".localize();
                                let mut description = "Settings_Theme_Desc".localize();
                                render_setting(&name, &description, ui, |ui| {
                                    egui::ComboBox::new("ui-theme", "")
                                        .selected_text(self.theme.name())
                                        .show_ui(ui, |ui| {
                                            let mut current_theme = self.theme;
                                            for theme in Theme::iter() {
                                                if ui
                                                    .selectable_value(
                                                        &mut current_theme,
                                                        theme,
                                                        theme.name(),
                                                    )
                                                    .clicked()
                                                {
                                                    theme_change = Some(theme);
                                                }
                                            }
                                        });
                                });
                                name = "Settings_Language".localize();
                                description = "Settings_Language_Desc".localize();
                                render_setting(&name, &description, ui, |ui| {
                                    // UI language is frontend state (persisted
                                    // in ui-egui.json), not core settings.
                                    egui::ComboBox::new("lang-ukmm", "")
                                        .selected_text(self.lang.to_str())
                                        .show_ui(ui, |ui| {
                                            for lang in LocLang::iter() {
                                                if ui
                                                    .selectable_value(
                                                        &mut self.lang,
                                                        *lang,
                                                        lang.to_str(),
                                                    )
                                                    .changed()
                                                {
                                                    lang_change = Some(*lang);
                                                }
                                            }
                                        });
                                });
                                name = "Settings_Mode".localize();
                                description = "Settings_Mode_Desc".localize();
                                render_setting(&name, &description, ui, |ui| {
                                    ui.radio_value(
                                        &mut settings.current_mode,
                                        Platform::WiiU,
                                        "Settings_Mode_WiiU".localize(),
                                    );
                                    ui.radio_value(
                                        &mut settings.current_mode,
                                        Platform::Switch,
                                        "Settings_Mode_Switch".localize(),
                                    );
                                });
                                name = "Settings_Storage".localize();
                                description = "Settings_Storage_Desc".localize();
                                render_setting(&name, &description, ui, |ui| {
                                    ui.folder_picker(&mut settings.storage_dir);
                                });
                                name = "Settings_Sys7z".localize();
                                description = "Settings_Sys7z_Desc".localize();
                                render_setting(&name, &description, ui, |ui| {
                                    ui.checkbox(&mut settings.system_7z, "")
                                });
                                name = "Settings_Changelog".localize();
                                description = "Settings_Changelog_Desc".localize();
                                render_setting(&name, &description, ui, |ui| {
                                    ui.add(Checkbox::new(&mut settings.show_changelog, ""))
                                });
                            });
                        egui::CollapsingHeader::new("Settings_Config_WiiU".localize()).show(
                            ui,
                            |ui| {
                                if ui
                                    .icon_text_button(
                                        "Settings_Config_WiiU_ImportCemu".localize(),
                                        icons::Icon::Import,
                                    )
                                    .clicked()
                                {
                                    self.channel
                                        .0
                                        .clone()
                                        .send(Message::ImportCemu)
                                        .expect("Broken channel");
                                }
                                render_platform_config(
                                    settings.platform_draft_mut(Platform::WiiU),
                                    Platform::WiiU,
                                    ui,
                                );
                            },
                        );
                        egui::CollapsingHeader::new("Settings_Config_NX".localize()).show(
                            ui,
                            |ui| {
                                render_platform_config(
                                    settings.platform_draft_mut(Platform::Switch),
                                    Platform::Switch,
                                    ui,
                                );
                            },
                        );
                        if let Some(theme) = theme_change {
                            self.do_update(Message::SetTheme(theme));
                        }
                        if let Some(lang) = lang_change {
                            self.do_update(Message::SetLanguage(lang));
                        }
                    });
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // The draft is validated (all platforms) and saved by the
                    // session job; the button is enabled when the draft
                    // differs from the saved settings.
                    let platform_config_changed =
                        self.settings_draft.differs_from(&self.core.settings());
                    ui.add_enabled_ui(platform_config_changed, |ui| {
                        if ui.button("Generic_Save".localize()).clicked() {
                            self.do_update(Message::SaveSettings);
                        }
                        if ui.button("Generic_Reset".localize()).clicked() {
                            self.do_update(Message::ResetSettings);
                        }
                    });
                    if platform_config_changed {
                        ui.label(
                            RichText::new("Changes to settings have not been saved")
                                .color(uk_ui::visuals::RED)
                                .heading()
                                .size(13.0_f32),
                        );
                    }
                });
            });
        });
    }
}
