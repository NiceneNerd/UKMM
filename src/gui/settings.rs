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
use uk_manager::settings::{DeployConfig, Platform, PlatformSettings};
use uk_reader::ResourceReader;
use uk_ui::{
    egui::{self, Align, Checkbox, ImageButton, InnerResponse, Layout, RichText, TextStyle, Ui},
    ext::UiExt,
    icons::{self, IconButtonExt},
    visuals::Theme,
};
use uk_util::{OptionExt, OptionResultExt};

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
    ui.label("Deployment");
    let mut changed = false;
    ui.group(|ui| {
        ui.allocate_space([ui.available_width(), -8.0].into());
        render_setting(
            "Deploy Method",
            "There are three methods of deployment: copying, hard linking, and symlinking. \
             Copying is slow and should only be used to deploy for consoles. \
             Hard links are faster and the most well-supported by Windows. \
             Symlinks are the fastest, but may fail to deploy automatically on Windows. \
             Always use Copy for consoles. Probably use Symlinks for emulators. \
             For more on this, consult the docs.",
            ui,
            |ui| {
                changed |= ui
                    .radio_value(
                        &mut config.method,
                        uk_manager::settings::DeployMethod::Copy,
                        "Copy",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut config.method,
                        uk_manager::settings::DeployMethod::HardLink,
                        "Hard Links",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut config.method,
                        uk_manager::settings::DeployMethod::Symlink,
                        "Symlink",
                    )
                    .changed();
            },
        );
        render_setting(
            "Deploy Layout",
            "There are two methods of deployment layout: without a folder named for UKMM, \
             and with a folder named for UKMM. If you select With Name, UKMM will add a \
             BreathOfTheWild_UKMM folder to the end of your Output Folder path, where appropriate. \
             If you don't know what to choose for this: On WiiU, choose With Name. On Switch consoles or \
             when your output folder is an atmosphere folder, choose Without Name. On Switch emulators \
             where your output folder is NOT an atmosphere folder, choose With Name. For more on this, \
             consult the docs.",
            ui,
            |ui| {
                changed |= ui
                    .radio_value(
                        &mut config.layout,
                        uk_manager::settings::DeployLayout::WithoutName,
                        "Without Name",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut config.layout,
                        uk_manager::settings::DeployLayout::WithName,
                        "With Name",
                    )
                    .changed();
            }
        );
        render_setting(
            "Auto Deploy",
            "Whether to automatically deploy changes to the mod configuration every time they are \
             applied.",
            ui,
            |ui| {
                changed |= ui.checkbox(&mut config.auto, "").changed();
            },
        );
        if platform == Platform::WiiU {
            render_setting(
                "Deploy rules.txt",
                "Automatically adds a rules.txt file when deploying for Cemu integration.",
                ui,
                |ui| {
                    changed |= ui.checkbox(&mut config.cemu_rules, "").changed();
                },
            );
            ui.add_space(8.0);
        }
        render_setting(
            "Output Folder",
            "Where to deploy the final merged mod pack.",
            ui,
            |ui| {
                changed |= ui.folder_picker(&mut config.output).changed();
            },
        );
        render_setting(
            "Emulator Executable",
            "Command line for the emulator to run for playing the game. This can be an \
             arbitrarily complex command which will be passed to your default shell.",
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
    render_setting(
        "Language",
        "Select the language and region corresponding to your game version and settings.",
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
    ui.label("Game Dump");
    ui.group(|ui| {
        ui.allocate_space([ui.available_width(), -8.0].into());
        if platform == Platform::WiiU {
            render_setting(
                "Dump Type",
                "For Wii U, you have two supported dump options: unpacked MLC files (most common) \
                 or a .wua file (Cemu-specific format).",
                ui,
                |ui| {
                    if ui
                        .radio(matches!(config.dump, DumpType::Unpacked { .. }), "Unpacked")
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
                        .radio(matches!(config.dump, DumpType::ZArchive { .. }), "WUA")
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
                if platform == Platform::WiiU {
                    render_setting(
                        "Base Folder",
                        "This folder is the root of the plain, v1.0 BOTW assets which were \
                         included on the disk. If you are using Cemu, it will usually be in your \
                         MLC folder, with a path such as this (part of the title ID will be \
                         different for the EU or JP versions): \
                         mlc01/usr/title/00050000/101C9400/content",
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
                }
                if platform == Platform::Switch {
                    render_setting(
                        "Base with Update Folder",
                        "Following the usual guides with nxdumptool, this will usually be the \
                         combined base game and v1.6.0 update files. The path will probably \
                         contain the title ID of 01007EF00011E800 and end in romfs.",
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
                }
                if platform == Platform::WiiU {
                    render_setting(
                        "Update Folder",
                        "The contains the BOTW v1.5.0 update data. It is absolutely necessary for \
                         the game to even run. If you are using Cemu, it will usually have a \
                         similar path to the base folder, but with an E at the end of the first \
                         half of the title ID: mlc01/usr/title/0005000E/101C9400/content",
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
                if platform == Platform::WiiU {
                    render_setting(
                        "DLC Folder",
                        "This contains most of the assets for the BOTW DLC. This one does not \
                         usually end in content, but must go one level further into a 0010 folder \
                         because of the way multiple kinds of add-on content are handled. If you \
                         are using Cemu, it will usually have a similar path to the base folder, \
                         but with a C at the end of the first half of the title ID: \
                         mlc01/usr/title/0005000C/101C9400/content/0010",
                        ui,
                        |ui| {
                            if ui.folder_picker(aoc_dir.get_or_insert_default()).changed() {
                                changed = true;
                                *host_path = "/".into();
                            }
                        },
                    );
                }
                if platform == Platform::Switch {
                    render_setting(
                        "DLC Folder",
                        "This contains most of the assets for the BOTW DLC. The path will \
                         probably contain a title ID like 01007EF00011F001 and end in romfs.",
                        ui,
                        |ui| {
                            if ui.folder_picker(aoc_dir.get_or_insert_default()).changed() {
                                changed = true;
                                *host_path = "/".into();
                            }
                        },
                    );
                }
            }
            DumpType::ZArchive {
                content_dir: _,
                update_dir: _,
                aoc_dir: _,
                host_path,
            } => {
                render_setting(
                    "WUA Path",
                    "This should contain the entire BOTW game with the Base, Update, and DLC and \
                     should have a file extension of .wua",
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
                        .on_hover_text("Save")
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
                        .on_hover_text("Reset")
                        .clicked()
                    {
                        CONFIG.write().clear();
                        self.do_update(Message::ResetSettings);
                    }
                })
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                let settings = &mut self.temp_settings;
                let mut theme_change: Option<Theme> = None;
                egui::CollapsingHeader::new("General")
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui
                            .icon_text_button("Migrate from BCML", icons::Icon::Import)
                            .clicked()
                        {
                            self.channel
                                .0
                                .clone()
                                .send(Message::MigrateBcml)
                                .expect("Broken channel");
                        }
                        if ui
                            .button("Register 1-Click Handler")
                            .on_hover_text(
                                "Sets up UKMM on your system to handle GameBanana 1-click links",
                            )
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
                        render_setting("Theme", "User interface theme", ui, |ui| {
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
                        });
                        render_setting(
                            "Current Mode",
                            "Select whether to manage the Wii U or Switch version of the game",
                            ui,
                            |ui| {
                                ui.radio_value(&mut settings.current_mode, Platform::WiiU, "Wii U");
                                ui.radio_value(
                                    &mut settings.current_mode,
                                    Platform::Switch,
                                    "Switch",
                                );
                            },
                        );
                        render_setting(
                            "Storage Folder",
                            "UKMM will store mods, profiles, and similar data here.",
                            ui,
                            |ui| {
                                ui.folder_picker(&mut settings.storage_dir);
                            },
                        );
                        render_setting(
                            "Use System 7z",
                            "By default UKMM will attempt to use 7z from your system PATH to \
                             extract 7-Zip files (like BNPs). Otherwise it will fall back to a \
                             slower built-in 7z extraction library. If you have 7z-related \
                             errors, try disabling this option.",
                            ui,
                            |ui| ui.checkbox(&mut settings.system_7z, ""),
                        );
                        render_setting(
                            "Show Changelog",
                            "Show a summary of recent changes after UKMM updates.",
                            ui,
                            |ui| ui.add(Checkbox::new(&mut settings.show_changelog, "")),
                        );
                    });
                egui::CollapsingHeader::new("Wii U Config").show(ui, |ui| {
                    if ui
                        .icon_text_button("Import Cemu Settings", icons::Icon::Import)
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
                egui::CollapsingHeader::new("Switch Config").show(ui, |ui| {
                    switch_changed =
                        render_platform_config(&mut settings.switch_config, Platform::Switch, ui);
                });
                if let Some(theme) = theme_change {
                    self.do_update(Message::SetTheme(theme));
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
                        if ui.button("Save").clicked() {
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
                        if ui.button("Reset").clicked() {
                            CONFIG.write().clear();
                            self.do_update(Message::ResetSettings);
                        }
                    })
                });
            });
        });
    }
}
