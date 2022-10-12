use super::{
    icons::{self},
    util::UkWidgetExt,
    App, Message,
};
use anyhow::Result;
use egui::{Align, Checkbox, ImageButton, InnerResponse, Layout, RichText, TextStyle, Ui};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_manager::settings::{DeployConfig, Language, Platform, PlatformSettings};
use uk_reader::ResourceReader;

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

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
enum DumpType {
    Unpacked {
        host_path: PathBuf,
        content_dir: Option<PathBuf>,
        update_dir: Option<PathBuf>,
        aoc_dir: Option<PathBuf>,
    },
    ZArchive {
        content_dir: PathBuf,
        update_dir: PathBuf,
        aoc_dir: Option<PathBuf>,
        host_path: PathBuf,
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

#[derive(Debug, Clone, PartialEq)]
struct PlatformSettingsUI {
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
                host_path: Default::default(),
                content_dir: Default::default(),
                update_dir: Default::default(),
                aoc_dir: Default::default(),
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
            } => Arc::new(ResourceReader::from_unpacked_dirs(
                content_dir,
                update_dir,
                aoc_dir,
            )?),
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

static CONFIG: Lazy<RwLock<FxHashMap<Platform, PlatformSettingsUI>>> =
    Lazy::new(|| RwLock::new(Default::default()));

fn render_deploy_config(config: &mut DeployConfig, ui: &mut Ui) -> bool {
    ui.label("Deployment");
    ui.separator();
    ui.end_row();
    let mut changed = false;
    render_setting("Deploy Method", "There are three methods of deployment: copying, hard linking, and symlinking. Generally copying is slow and should be avoided if possible. For more on this, consult the docs.", ui, |ui| {
        changed = changed || ui.radio_value(&mut config.method, uk_manager::settings::DeployMethod::Copy, "Copy").changed();
        changed = changed || ui.radio_value(&mut config.method, uk_manager::settings::DeployMethod::HardLink, "Hard Links").changed();
        changed = changed || ui.radio_value(&mut config.method, uk_manager::settings::DeployMethod::Symlink, "Symlink").changed();
    });
    render_setting("Auto Deploy", "Whether to automatically deploy changes to the mod configuration every time they are applied.", ui, |ui| {
        changed = changed || ui.checkbox(&mut config.auto, "").changed();
    });
    render_setting(
        "Output Folder",
        "Where to deploy the final merged mod pack.",
        ui,
        |ui| {
            changed = changed || ui.folder_picker(&mut config.output).changed();
        },
    );
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
                    enum_iterator::all::<Language>().for_each(|lang| {
                        changed = changed
                            || ui
                                .selectable_value(&mut config.language, lang, lang.to_str())
                                .changed();
                    });
                });
        },
    );
    ui.label("Game Dump");
    ui.separator();
    ui.end_row();
    if platform == Platform::WiiU {
        render_setting("Dump Type", "Blah blah", ui, |ui| {
            if ui
                .radio(matches!(config.dump, DumpType::Unpacked { .. }), "Unpacked")
                .clicked()
            {
                config.dump = DumpType::Unpacked {
                    host_path: Default::default(),
                    content_dir: Default::default(),
                    update_dir: Default::default(),
                    aoc_dir: Default::default(),
                };
                changed = true;
            }
            if ui
                .radio(matches!(config.dump, DumpType::ZArchive { .. }), "WUA")
                .clicked()
            {
                config.dump = DumpType::ZArchive {
                    content_dir: Default::default(),
                    update_dir: Default::default(),
                    aoc_dir: Default::default(),
                    host_path: Default::default(),
                };
                changed = true;
            }
        });
    }
    match &mut config.dump {
        DumpType::Unpacked {
            host_path,
            content_dir,
            update_dir,
            aoc_dir,
        } => {
            render_setting("Base Folder", "Blah blah", ui, |ui| {
                if ui
                    .folder_picker(content_dir.get_or_insert_default())
                    .changed()
                {
                    changed = true;
                    *host_path = "/".into();
                }
            });
            if platform == Platform::WiiU {
                render_setting("Update Folder", "Blah blah", ui, |ui| {
                    if ui
                        .folder_picker(update_dir.get_or_insert_default())
                        .changed()
                    {
                        changed = true;
                        *host_path = "/".into();
                    }
                });
            }
            render_setting("DLC Folder", "Blah blah", ui, |ui| {
                if ui.folder_picker(aoc_dir.get_or_insert_default()).changed() {
                    changed = true;
                    *host_path = "/".into();
                }
            });
        }
        DumpType::ZArchive {
            content_dir: _,
            update_dir: _,
            aoc_dir: _,
            host_path,
        } => {
            render_setting("WUA Path", "Blah blah", ui, |ui| {
                changed = changed || ui.file_picker(host_path).changed();
            });
        }
    }
    changed = changed || render_deploy_config(&mut config.deploy_config, ui);
    changed
}

impl App {
    pub fn render_settings(&mut self, ui: &mut Ui) {
        egui::Frame::none().inner_margin(4.0).show(ui, |ui| {
            let mut wiiu_changed = false;
            let mut switch_changed = false;
            ui.vertical(|ui| {
                let settings = &mut self.temp_settings;
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
                        egui::Grid::new("wiiu_config").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
                            wiiu_changed = render_platform_config(&mut settings.wiiu_config, Platform::WiiU, ui);
                        });
                    });
                egui::CollapsingHeader::new("Switch Config")
                .show(ui, |ui| {
                    egui::Grid::new("switch_config").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
                        switch_changed = render_platform_config(&mut settings.switch_config, Platform::Switch, ui);
                    });
                });
            });
            switch_changed = switch_changed || {
                match (CONFIG.read().get(&Platform::Switch), self.temp_settings.switch_config.as_ref()) {
                    (None, None) | (None, Some(_)) => false,
                    (Some(config), None) => !config.dump.is_empty() || !config.deploy_config.output.as_os_str().is_empty(),
                    (Some(tmp_config), Some(config)) => tmp_config.ne(config),
                }
            };
            wiiu_changed = wiiu_changed || {
                match (CONFIG.read().get(&Platform::WiiU), self.temp_settings.wiiu_config.as_ref()) {
                    (None, None) | (None, Some(_)) => false,
                    (Some(config), None) => !config.dump.is_empty() || !config.deploy_config.output.as_os_str().is_empty(),
                    (Some(tmp_config), Some(config)) => tmp_config.ne(config),
                }
            };
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let platform_config_changed = self.temp_settings.ne(self.core.settings().deref() )|| wiiu_changed || switch_changed;
                    ui.add_enabled_ui(platform_config_changed, |ui| {
                        if ui.button("Save").clicked() {
                            if wiiu_changed {
                                let wiiu_config = CONFIG.write().remove(&Platform::WiiU).unwrap().try_into();
                                match wiiu_config {
                                    Ok(conf) => self.temp_settings.wiiu_config = Some(conf),
                                    Err(e) => {
                                        self.do_update(Message::Error(e));
                                        return;
                                    },
                                }
                            }
                            if switch_changed {
                                let switch_config = CONFIG.write().remove(&Platform::Switch).unwrap().try_into();
                                match switch_config {
                                    Ok(conf) => self.temp_settings.switch_config = Some(conf),
                                    Err(e) => {
                                        self.do_update(Message::Error(e));
                                        return;
                                    },
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
