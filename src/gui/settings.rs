#![allow(unstable_name_collisions)]
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use uk_content::constants::Language;
use uk_manager::{localization::LocLang, settings::{DeployConfig, Platform, PlatformSettings}};
use uk_reader::ResourceReader;
use uk_ui::{
    egui::{self, Align, Checkbox, ImageButton, InnerResponse, Layout, RichText, TextStyle, Ui},
    ext::UiExt,
    icons::{self, IconButtonExt},
    visuals::Theme,
};
use uk_util::OptionResultExt;

use super::{App, Message, LOCALIZATION};

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
            ImageButton::new(icons::get_icon(ui.ctx(), icons::Icon::Info))
                .frame(false)
                .tint(ui.visuals().text_color()),
        )
        .on_hover_text(description);
    });
    ui.horizontal(|ui| add_contents(ui))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum DumpType {
    Unpacked {
        host_path:   PathBuf,
        content_dir: Option<PathBuf>,
        update_dir:  Option<PathBuf>,
        aoc_dir:     Option<PathBuf>,
    },
    ZArchive {
        content_dir: PathBuf,
        update_dir:  PathBuf,
        aoc_dir:     Option<PathBuf>,
        host_path:   PathBuf,
    },
}

impl DumpType {
    pub fn host_path(&self) -> &Path {
        match self {
            DumpType::Unpacked { host_path, .. } => host_path.as_path(),
            DumpType::ZArchive { host_path, .. } => host_path.as_path(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            DumpType::Unpacked {
                content_dir,
                update_dir,
                aoc_dir,
                ..
            } => {
                content_dir
                    .as_ref()
                    .map(|d| d.as_os_str().is_empty())
                    .unwrap_or(true)
                    && update_dir
                        .as_ref()
                        .map(|d| d.as_os_str().is_empty())
                        .unwrap_or(true)
                    && aoc_dir
                        .as_ref()
                        .map(|d| d.as_os_str().is_empty())
                        .unwrap_or(true)
            }
            DumpType::ZArchive { host_path, .. } => host_path.as_os_str().is_empty(),
        }
    }
}

impl From<&ResourceReader> for DumpType {
    fn from(reader: &ResourceReader) -> Self {
        serde_json::from_str(&reader.source_ser()).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformSettingsUI {
    pub language: Language,
    pub profile: String,
    pub dump: DumpType,
    pub deploy_config: DeployConfig,
}

impl Default for PlatformSettingsUI {
    fn default() -> Self {
        PlatformSettingsUI {
            language: Language::USen,
            profile: "Default".into(),
            dump: DumpType::Unpacked {
                host_path:   Default::default(),
                content_dir: Default::default(),
                update_dir:  Default::default(),
                aoc_dir:     Default::default(),
            },
            deploy_config: Default::default(),
        }
    }
}

impl TryFrom<PlatformSettingsUI> for PlatformSettings {
    type Error = anyhow::Error;

    fn try_from(settings: PlatformSettingsUI) -> Result<Self> {
        let dump = match settings.dump {
            DumpType::Unpacked {
                content_dir,
                update_dir,
                aoc_dir,
                ..
            } => {
                Arc::new(ResourceReader::from_unpacked_dirs(
                    content_dir,
                    update_dir,
                    aoc_dir,
                )?)
            }
            DumpType::ZArchive { host_path, .. } => {
                Arc::new(ResourceReader::from_zarchive(host_path)?)
            }
        };
        Ok(Self {
            language: settings.language,
            profile: settings.profile.into(),
            dump,
            deploy_config: if settings.deploy_config.output.as_os_str().is_empty() {
                None
            } else {
                Some(settings.deploy_config)
            },
        })
    }
}

impl From<&PlatformSettings> for PlatformSettingsUI {
    fn from(settings: &PlatformSettings) -> Self {
        Self {
            language: settings.language,
            profile: settings.profile.to_string(),
            dump: settings.dump.as_ref().into(),
            deploy_config: settings.deploy_config.as_ref().cloned().unwrap_or_default(),
        }
    }
}

impl PartialEq<PlatformSettings> for PlatformSettingsUI {
    fn eq(&self, other: &PlatformSettings) -> bool {
        self.language == other.language
            && other.deploy_config.contains(&self.deploy_config)
            && self.dump.host_path() == other.dump.source().host_path()
    }
}

pub static CONFIG: LazyLock<RwLock<FxHashMap<Platform, PlatformSettingsUI>>> =
    LazyLock::new(|| RwLock::new(Default::default()));

fn render_deploy_config(config: &mut DeployConfig, platform: Platform, ui: &mut Ui) -> bool {
    let loc = LOCALIZATION.read();
    ui.label(loc.get("Settings_Platform_Deploy"));
    let mut changed = false;
    ui.group(|ui| {
        ui.allocate_space([ui.available_width(), -8.0].into());
        render_setting(
            loc.get("Settings_Platform_Deploy_Method"),
            loc.get("Settings_Platform_Deploy_Method_Desc"),
            ui,
            |ui| {
                changed |= ui
                    .radio_value(
                        &mut config.method,
                        uk_manager::settings::DeployMethod::Copy,
                        loc.get("Settings_Platform_Deploy_Method_Copy"),
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut config.method,
                        uk_manager::settings::DeployMethod::HardLink,
                        loc.get("Settings_Platform_Deploy_Method_HardLink"),
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut config.method,
                        uk_manager::settings::DeployMethod::Symlink,
                        loc.get("Settings_Platform_Deploy_Method_Symlink"),
                    )
                    .changed();
            },
        );
        render_setting(
            loc.get("Settings_Platform_Deploy_Layout"),
            match platform {
                Platform::WiiU => loc.get("Settings_Platform_Deploy_Layout_WiiU_Desc"),
                Platform::Switch => loc.get("Settings_Platform_Deploy_Layout_NX_Desc"),
            },
            ui,
            |ui| {
                changed |= ui
                    .radio_value(
                        &mut config.layout,
                        uk_manager::settings::DeployLayout::WithoutName,
                        match platform {
                            Platform::WiiU =>
                                loc.get("Settings_Platform_Deploy_Layout_WiiU_WithoutName"),
                            Platform::Switch =>
                                loc.get("Settings_Platform_Deploy_Layout_NX_WithoutName"),
                        },
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut config.layout,
                        uk_manager::settings::DeployLayout::WithName,
                        match platform {
                            Platform::WiiU =>
                                loc.get("Settings_Platform_Deploy_Layout_WiiU_WithName"),
                            Platform::Switch =>
                                loc.get("Settings_Platform_Deploy_Layout_NX_WithName"),
                        },
                    )
                    .changed();
            }
        );
        render_setting(
            loc.get("Settings_Platform_Deploy_Auto"),
            loc.get("Settings_Platform_Deploy_Auto_Desc"),
            ui,
            |ui| {
                changed |= ui.checkbox(&mut config.auto, "").changed();
            },
        );
        if platform == Platform::WiiU {
            render_setting(
                loc.get("Settings_Platform_Deploy_Rules"),
                loc.get("Settings_Platform_Deploy_Rules_Desc"),
                ui,
                |ui| {
                    changed |= ui.checkbox(&mut config.cemu_rules, "").changed();
                },
            );
            ui.add_space(8.0);
        }
        render_setting(
            loc.get("Settings_Platform_Deploy_Output"),
            loc.get("Settings_Platform_Deploy_Output_Desc"),
            ui,
            |ui| {
                changed |= ui.folder_picker(&mut config.output).changed();
            },
        );
        render_setting(
            loc.get("Settings_Platform_Deploy_Emu"),
            loc.get("Settings_Platform_Deploy_Emu_Desc"),
            ui,
            |ui| {
                changed |= ui
                    .file_picker_string(config.executable.get_or_insert_default())
                    .changed();
            },
        );
    });
    changed
}

fn render_platform_config(
    config: &mut Option<PlatformSettings>,
    platform: Platform,
    ui: &mut Ui,
) -> bool {
    let mut changed = false;
    let mut conf_lock = CONFIG.write();
    let config = conf_lock
        .entry(platform)
        .or_insert_with(|| config.as_ref().map(|c| c.into()).unwrap_or_default());
    let loc = LOCALIZATION.read();
    render_setting(
        loc.get("Settings_Platform_Language"),
        loc.get("Settings_Platform_Language_Desc"),
        ui,
        |ui| {
            egui::ComboBox::new(format!("lang-{platform}"), "")
                .selected_text(config.language.to_str())
                .show_ui(ui, |ui| {
                    Language::iter().for_each(|lang| {
                        changed |= ui
                            .selectable_value(&mut config.language, *lang, lang.to_str())
                            .changed();
                    });
                });
        },
    );
    ui.add_space(8.0);
    ui.label(loc.get("Settings_Platform_Dump"));
    ui.group(|ui| {
        ui.allocate_space([ui.available_width(), -8.0].into());
        if platform == Platform::WiiU {
            render_setting(
                loc.get("Settings_Platform_Dump_Type"),
                loc.get("Settings_Platform_Dump_Type_Desc"),
                ui,
                |ui| {
                    if ui
                        .radio(matches!(
                            config.dump, DumpType::Unpacked { .. }),
                            loc.get("Settings_Platform_Dump_Type_Unpacked"
                        ))
                        .clicked()
                    {
                        config.dump = DumpType::Unpacked {
                            host_path:   Default::default(),
                            content_dir: Default::default(),
                            update_dir:  Default::default(),
                            aoc_dir:     Default::default(),
                        };
                        changed = true;
                    }
                    if ui
                        .radio(matches!(
                            config.dump, DumpType::ZArchive { .. }),
                            loc.get("Settings_Platform_Dump_Type_WUA"
                        ))
                        .clicked()
                    {
                        config.dump = DumpType::ZArchive {
                            content_dir: Default::default(),
                            update_dir:  Default::default(),
                            aoc_dir:     Default::default(),
                            host_path:   Default::default(),
                        };
                        changed = true;
                    }
                },
            );
        }
        match &mut config.dump {
            DumpType::Unpacked {
                host_path,
                content_dir,
                update_dir,
                aoc_dir,
            } => {
                render_setting(
                    match platform {
                        Platform::WiiU => loc.get("Settings_Platform_Dump_WiiU_Base"),
                        Platform::Switch => loc.get("Settings_Platform_Dump_NX_Base"),
                    },
                    match platform {
                        Platform::WiiU => loc.get("Settings_Platform_Dump_WiiU_Base_Desc"),
                        Platform::Switch => loc.get("Settings_Platform_Dump_NX_Base_Desc"),
                    },
                    ui,
                    |ui| {
                        if ui
                            .folder_picker(content_dir.get_or_insert_default())
                            .changed()
                        {
                            changed = true;
                            *host_path = "/".into();
                        }
                    },
                );
                if platform == Platform::WiiU {
                    render_setting(
                        loc.get("Settings_Platform_Dump_Update"),
                        loc.get("Settings_Platform_Dump_Update_Desc"),
                        ui,
                        |ui| {
                            if ui
                                .folder_picker(update_dir.get_or_insert_default())
                                .changed()
                            {
                                changed = true;
                                *host_path = "/".into();
                            }
                        },
                    );
                }
                render_setting(
                    loc.get("Settings_Platform_Dump_DLC"),
                    match platform {
                        Platform::WiiU => loc.get("Settings_Platform_Dump_DLC_WiiU_Desc"),
                        Platform::Switch => loc.get("Settings_Platform_Dump_DLC_NX_Desc"),
                    },
                    ui,
                    |ui| {
                        if ui.folder_picker(aoc_dir.get_or_insert_default()).changed() {
                            changed = true;
                            *host_path = "/".into();
                        }
                    },
                );
            }
            DumpType::ZArchive {
                content_dir: _,
                update_dir: _,
                aoc_dir: _,
                host_path,
            } => {
                render_setting(
                    loc.get("Settings_Platform_Dump_WUA"),
                    loc.get("Settings_Platform_Dump_WUA_Desc"),
                    ui,
                    |ui| {
                        changed |= ui.file_picker(host_path).changed();
                    },
                );
            }
        }
    });
    changed |= render_deploy_config(&mut config.deploy_config, platform, ui);
    changed
}

impl App {
    pub fn render_settings(&mut self, ui: &mut Ui) {
        let loc = LOCALIZATION.read();
        egui::Frame::none().inner_margin(4.0).show(ui, |ui| {
            let mut wiiu_changed = false;
            let mut switch_changed = false;
            ui.horizontal(|ui| {
                let platform_config_changed = self.temp_settings.ne(self.core.settings().deref())
                    || wiiu_changed
                    || switch_changed;
                ui.add_enabled_ui(platform_config_changed, |ui| {
                    if ui
                        .icon_button(icons::Icon::Save)
                        .on_hover_text(loc.get("Generic_Save"))
                        .clicked()
                    {
                        if wiiu_changed {
                            let wiiu_config_ui =
                                CONFIG.write().get(&Platform::WiiU).unwrap().clone();
                            let wiiu_config = wiiu_config_ui.try_into();
                            match wiiu_config {
                                Ok(conf) => {
                                    CONFIG.write().remove(&Platform::WiiU);
                                    self.temp_settings.wiiu_config = Some(conf)
                                }
                                Err(e) => {
                                    self.do_update(Message::Error(e));
                                    return;
                                }
                            }
                        }
                        if switch_changed {
                            let switch_config_ui =
                                CONFIG.write().get(&Platform::Switch).unwrap().clone();
                            let switch_config = switch_config_ui.try_into();
                            match switch_config {
                                Ok(conf) => {
                                    CONFIG.write().remove(&Platform::Switch);
                                    self.temp_settings.switch_config = Some(conf)
                                }
                                Err(e) => {
                                    self.do_update(Message::Error(e));
                                    return;
                                }
                            }
                        }
                        self.do_update(Message::SaveSettings);
                    }
                    if ui
                        .icon_button(icons::Icon::Reset)
                        .on_hover_text(loc.get("Generic_Reset"))
                        .clicked()
                    {
                        self.temp_settings.lang = self.core.settings().lang;
                        CONFIG.write().clear();
                        self.do_update(Message::ResetSettings);
                    }
                })
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                let settings = &mut self.temp_settings;
                let mut theme_change: Option<Theme> = None;
                let mut lang_change: Option<LocLang> = None;
                egui::CollapsingHeader::new(loc.get("Settings_General"))
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui
                            .icon_text_button(loc.get("Settings_Migrate"), icons::Icon::Import)
                            .clicked()
                        {
                            self.channel
                                .0
                                .clone()
                                .send(Message::MigrateBcml)
                                .expect("Broken channel");
                        }
                        if ui
                            .button(loc.get("Settings_OneClick"))
                            .on_hover_text(loc.get("Settings_OneClick_Desc"))
                            .clicked()
                        {
                            match crate::gui::tasks::register_handlers() {
                                Ok(()) => log::info!("GameBanana 1-click handler registered"),
                                Err(e) => {
                                    self.channel
                                        .0
                                        .clone()
                                        .send(Message::Error(e))
                                        .expect("Broken channel")
                                }
                            }
                        }
                        render_setting(
                            loc.get("Settings_Theme"),
                            loc.get("Settings_Theme_Desc"),
                            ui,
                            |ui| {
                                egui::ComboBox::new("ui-theme", "")
                                    .selected_text(self.theme.name())
                                    .show_ui(ui, |ui| {
                                        let mut current_theme = self.theme;
                                        for theme in uk_ui::visuals::Theme::iter() {
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
                            }
                        );
                        render_setting(
                            loc.get("Settings_Language"),
                            loc.get("Settings_Language_Desc"),
                            ui,
                            |ui| {
                                egui::ComboBox::new("lang-ukmm", "")
                                    .selected_text(settings.lang.to_str())
                                    .show_ui(ui, |ui| {
                                        for lang in LocLang::iter() {
                                            if ui
                                                .selectable_value(
                                                    &mut settings.lang,
                                                    *lang,
                                                    lang.to_str()
                                                )
                                                .changed()
                                            {
                                                lang_change = Some(*lang);
                                            }
                                        };
                                    });
                            },
                        );
                        render_setting(
                            loc.get("Settings_Mode"),
                            loc.get("Settings_Mode_Desc"),
                            ui,
                            |ui| {
                                ui.radio_value(
                                    &mut settings.current_mode,
                                    Platform::WiiU,
                                    loc.get("Settings_Mode_WiiU"),
                                );
                                ui.radio_value(
                                    &mut settings.current_mode,
                                    Platform::Switch,
                                    loc.get("Settings_Mode_Switch"),
                                );
                            },
                        );
                        render_setting(
                            loc.get("Settings_Storage"),
                            loc.get("Settings_Storage_Desc"),
                            ui,
                            |ui| {
                                ui.folder_picker(&mut settings.storage_dir);
                            },
                        );
                        render_setting(
                            loc.get("Settings_Sys7z"),
                            loc.get("Settings_Sys7z_Desc"),
                            ui,
                            |ui| ui.checkbox(&mut settings.system_7z, ""),
                        );
                        render_setting(
                            loc.get("Settings_Changelog"),
                            loc.get("Settings_Changelog_Desc"),
                            ui,
                            |ui| ui.add(Checkbox::new(&mut settings.show_changelog, "")),
                        );
                    });
                egui::CollapsingHeader::new(loc.get("Settings_Config_WiiU")).show(ui, |ui| {
                    if ui
                        .icon_text_button(
                            loc.get("Settings_Config_WiiU_ImportCemu"),
                            icons::Icon::Import
                        )
                        .clicked()
                    {
                        self.channel
                            .0
                            .clone()
                            .send(Message::ImportCemu)
                            .expect("Broken channel");
                    }
                    wiiu_changed =
                        render_platform_config(&mut settings.wiiu_config, Platform::WiiU, ui);
                });
                egui::CollapsingHeader::new(loc.get("Settings_Config_NX")).show(ui, |ui| {
                    switch_changed =
                        render_platform_config(&mut settings.switch_config, Platform::Switch, ui);
                });
                if let Some(theme) = theme_change {
                    self.do_update(Message::SetTheme(theme));
                }
                if let Some(lang) = lang_change {
                    self.do_update(Message::SetLanguage(lang));
                }
            });
            switch_changed |= {
                match (
                    CONFIG.read().get(&Platform::Switch),
                    self.temp_settings.switch_config.as_ref(),
                ) {
                    (None, None) | (None, Some(_)) => false,
                    (Some(config), None) => {
                        !config.dump.is_empty()
                            || !config.deploy_config.output.as_os_str().is_empty()
                    }
                    (Some(tmp_config), Some(config)) => tmp_config.ne(config),
                }
            };
            wiiu_changed |= {
                match (
                    CONFIG.read().get(&Platform::WiiU),
                    self.temp_settings.wiiu_config.as_ref(),
                ) {
                    (None, None) | (None, Some(_)) => false,
                    (Some(config), None) => {
                        !config.dump.is_empty()
                            || !config.deploy_config.output.as_os_str().is_empty()
                    }
                    (Some(tmp_config), Some(config)) => tmp_config.ne(config),
                }
            };
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let platform_config_changed =
                        self.temp_settings.ne(self.core.settings().deref())
                            || wiiu_changed
                            || switch_changed;
                    ui.add_enabled_ui(platform_config_changed, |ui| {
                        if ui.button(loc.get("Generic_Save")).clicked() {
                            if wiiu_changed {
                                let wiiu_config_ui =
                                    CONFIG.write().get(&Platform::WiiU).unwrap().clone();
                                let wiiu_config = wiiu_config_ui.try_into();
                                match wiiu_config {
                                    Ok(conf) => {
                                        CONFIG.write().remove(&Platform::WiiU);
                                        self.temp_settings.wiiu_config = Some(conf)
                                    }
                                    Err(e) => {
                                        self.do_update(Message::Error(e));
                                        return;
                                    }
                                }
                            }
                            if switch_changed {
                                let switch_config_ui =
                                    CONFIG.write().get(&Platform::Switch).unwrap().clone();
                                let switch_config = switch_config_ui.try_into();
                                match switch_config {
                                    Ok(conf) => {
                                        CONFIG.write().remove(&Platform::Switch);
                                        self.temp_settings.switch_config = Some(conf)
                                    }
                                    Err(e) => {
                                        self.do_update(Message::Error(e));
                                        return;
                                    }
                                }
                            }
                            self.do_update(Message::SaveSettings);
                        }
                        if ui.button(loc.get("Generic_Reset")).clicked() {
                            CONFIG.write().clear();
                            self.do_update(Message::ResetSettings);
                        }
                    })
                });
            });
        });
    }
}
