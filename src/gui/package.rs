use std::{ops::DerefMut, path::PathBuf};

use eframe::emath::Align;
use parking_lot::Mutex;
use rustc_hash::FxHashSet;
use uk_manager::settings::Platform;
use uk_mod::{
    ExclusiveOptionGroup, Meta, ModOption, ModOptionGroup, ModPlatform, MultipleOptionGroup,
    OptionGroup, CATEGORIES,
};
use uk_ui::{
    editor::EditableValue,
    egui::{self, Align2, Context, Id, Layout, Response, TextStyle, Ui},
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};

use super::{App, Message};

fn render_field(name: &str, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> Response) {
    ui.label(name);
    ui.horizontal(add_contents);
    ui.add_space(4.0);
}
#[derive(Debug, Clone)]
pub struct ModPackerBuilder {
    pub source: PathBuf,
    pub dest:   PathBuf,
    pub meta:   Meta,
}

impl ModPackerBuilder {
    pub fn new(platform: Platform) -> Self {
        ModPackerBuilder {
            source: Default::default(),
            dest:   Default::default(),
            meta:   Meta {
                api: env!("CARGO_PKG_VERSION").into(),
                name: Default::default(),
                version: "1.0.0".into(),
                author: Default::default(),
                category: "Other".into(),
                description: Default::default(),
                platform: uk_mod::ModPlatform::Specific(platform.into()),
                url: Default::default(),
                options: Default::default(),
                masters: Default::default(),
            },
        }
    }

    pub fn reset(&mut self, platform: Platform) {
        *self = Self::new(platform);
    }

    fn render_package_deps(&mut self, app: &App, ctx: &Context) {
        if !app.show_package_deps {
            return;
        }
        egui::Window::new("Select Dependencies")
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .show(ctx, |ui| {
                egui::ScrollArea::new([true, false])
                    .id_source("modal-pkg-deps")
                    .show_rows(
                        ui,
                        ui.text_style_height(&TextStyle::Body),
                        app.mods.len(),
                        |ui, range| {
                            for mod_ in app
                                .mods
                                .iter()
                                .skip(range.start)
                                .take(range.end - range.start)
                            {
                                let mut in_deps = self.meta.masters.contains_key(&mod_.hash());
                                let friendly = format!(
                                    " {} (v{})",
                                    mod_.meta.name.as_str(),
                                    mod_.meta.version
                                );
                                if ui.checkbox(&mut in_deps, friendly).changed() {
                                    if in_deps {
                                        self.meta.masters.insert(
                                            mod_.hash(),
                                            (mod_.meta.name.clone(), mod_.meta.version.clone()),
                                        );
                                    } else {
                                        self.meta.masters.shift_remove(&mod_.hash());
                                    }
                                }
                            }
                        },
                    );
                ui.allocate_ui_with_layout(
                    [ui.available_width(), ui.spacing().interact_size.y].into(),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        if ui.button("OK").clicked() {
                            app.do_update(Message::ClosePackagingDependencies);
                        }
                        ui.shrink_width_to_current();
                    },
                );
            });
    }

    fn render_package_opts(&mut self, app: &App, ctx: &Context) {
        if let Some(ref folders) = app.opt_folders {
            egui::Window::new("配置模组选项")
                .anchor(Align2::CENTER_CENTER, [0., 0.])
                .scroll2([false, true])
                .show(ctx, |ui| {
                    egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.horizontal(|ui| {
                            if ui.icon_text_button("添加选项组", Icon::Add).clicked() {
                                self.meta
                                    .options
                                    .push(OptionGroup::Multiple(Default::default()));
                            }
                        });
                        render_opt_groups(
                            &mut self.meta.options,
                            folders,
                            Id::new("opt-groups-"),
                            ui,
                        );
                        ui.allocate_ui_with_layout(
                            [ui.available_width(), ui.spacing().interact_size.y].into(),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("确定").clicked() {
                                    app.do_update(Message::ClosePackagingOptions);
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }

        fn render_opt_groups(
            opt_groups: &mut Vec<OptionGroup>,
            folders: &Mutex<FxHashSet<PathBuf>>,
            id: Id,
            ui: &mut Ui,
        ) {
            let mut delete = None;
            for (i, opt_group) in opt_groups.iter_mut().enumerate() {
                let id = id.with(i);
                let group_name = if opt_group.name().is_empty() {
                    "新选项组"
                } else {
                    opt_group.name()
                };
                egui::CollapsingHeader::new(group_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui.icon_text_button("删除", Icon::Delete).clicked() {
                            delete = Some(i);
                        }
                        ui.label("组名");
                        opt_group.name_mut().edit_ui_with_id(ui, id.with(i));
                        ui.label("组描述");
                        ui.text_edit_multiline(&mut uk_ui::editor::SmartStringWrapper(
                            opt_group.description_mut(),
                        ));
                        ui.label("组类型");
                        ui.horizontal(|ui| {
                            if ui
                                .radio(matches!(opt_group, OptionGroup::Exclusive(_)), "独占")
                                .clicked()
                            {
                                *opt_group = OptionGroup::Exclusive(ExclusiveOptionGroup {
                                    default: None,
                                    name: std::mem::take(opt_group.name_mut()),
                                    description: std::mem::take(opt_group.description_mut()),
                                    options: std::mem::take(opt_group.options_mut()),
                                    required: opt_group.required(),
                                });
                            }
                            if ui
                                .radio(matches!(opt_group, OptionGroup::Multiple(_)), "多选")
                                .clicked()
                            {
                                *opt_group = OptionGroup::Multiple(MultipleOptionGroup {
                                    defaults: Default::default(),
                                    name: std::mem::take(opt_group.name_mut()),
                                    description: std::mem::take(opt_group.description_mut()),
                                    options: std::mem::take(opt_group.options_mut()),
                                    required: opt_group.required(),
                                });
                            }
                        });
                        ui.checkbox(opt_group.required_mut(), "必选")
                            .on_hover_text("要求用户在此组中选择一个选项");
                        if let OptionGroup::Exclusive(group) = opt_group {
                            let id = Id::new(group.name.as_str()).with("default");
                            let def_name = group
                                .default
                                .as_ref()
                                .and_then(|opt| {
                                    group
                                        .options
                                        .iter()
                                        .find_map(|o| o.path.eq(opt).then(|| o.name.as_str()))
                                })
                                .unwrap_or("无");
                            egui::ComboBox::new(id, "默认选项")
                                .selected_text(def_name)
                                .show_ui(ui, |ui| {
                                    group.options.iter().for_each(|opt| {
                                        let selected = group.default.as_ref() == Some(&opt.path);
                                        if ui
                                            .selectable_label(selected, opt.name.as_str())
                                            .clicked()
                                        {
                                            group.default = Some(opt.path.clone());
                                        }
                                    });
                                });
                        }
                        ui.add_enabled_ui(!folders.lock().is_empty(), |ui| {
                            if ui.icon_text_button("Add Option", Icon::Add).clicked() {
                                opt_group.options_mut().push(ModOption {
                                    name: Default::default(),
                                    description: Default::default(),
                                    path: Default::default(),
                                    requires: vec![],
                                });
                            }
                        });
                        let mut delete = None;
                        let mut defaults = if let OptionGroup::Multiple(group) = opt_group {
                            Some(group.defaults.clone())
                        } else {
                            None
                        };
                        for (i, opt) in opt_group.options_mut().iter_mut().enumerate() {
                            render_option(opt, defaults.as_mut(), folders, &mut delete, i, id, ui);
                        }
                        if let OptionGroup::Multiple(group) = opt_group {
                            if let Some(defaults) = defaults.filter(|d| &group.defaults != d) {
                                group.defaults = defaults;
                            }
                        }
                        if let Some(i) = delete {
                            opt_group.options_mut().remove(i);
                        }
                    });
            }
            if let Some(i) = delete {
                opt_groups.remove(i);
            }
        }

        fn render_option(
            option: &mut ModOption,
            mut defaults: Option<&mut FxHashSet<PathBuf>>,
            folders: &Mutex<FxHashSet<PathBuf>>,
            delete: &mut Option<usize>,
            i: usize,
            id: Id,
            ui: &mut Ui,
        ) {
            let id = id.with(i);
            let opt_name = if option.name.is_empty() {
                "New Option"
            } else {
                option.name.as_str()
            };
            egui::CollapsingHeader::new(opt_name)
                .id_source(id.with("header"))
                .default_open(true)
                .show(ui, |ui| {
                    if ui.icon_text_button("Delete", Icon::Delete).clicked() {
                        *delete = Some(i);
                    }
                    ui.label("Option Name");
                    option.name.edit_ui_with_id(ui, id.with("name"));
                    ui.label("Option Description");
                    ui.text_edit_multiline(&mut uk_ui::editor::SmartStringWrapper(
                        &mut option.description,
                    ));
                    if let Some(ref mut defaults) = defaults {
                        let mut default = defaults.contains(&option.path);
                        if ui.checkbox(&mut default, "Enable by default").changed() {
                            if default {
                                defaults.insert(option.path.clone());
                            } else {
                                defaults.remove(&option.path);
                            }
                        }
                    }
                    egui::ComboBox::new(id.with("path"), "Option Folder")
                        .selected_text(option.path.display().to_string())
                        .show_ui(ui, |ui| {
                            let mut new_folder: Option<PathBuf> = None;
                            folders.lock().iter().for_each(|folder| {
                                let folder_name = folder.file_name().unwrap_or_default();
                                let selected = option.path.as_os_str() == folder_name;
                                if ui
                                    .selectable_label(
                                        selected,
                                        folder_name.to_str().unwrap_or_default(),
                                    )
                                    .clicked()
                                    && !selected
                                {
                                    new_folder = Some(folder.clone());
                                };
                            });
                            if let Some(new_folder) = new_folder {
                                let old_folder = option.path.clone();
                                let mut folders = folders.lock();
                                folders.remove(&new_folder);
                                if option.path != PathBuf::default() {
                                    folders.insert(new_folder.with_file_name(&option.path));
                                }
                                option.path = new_folder.file_name().unwrap().into();
                                if let Some(defaults) = defaults.filter(|d| d.contains(&old_folder))
                                {
                                    defaults.remove(&old_folder);
                                    defaults.insert(option.path.clone());
                                }
                            }
                        });
                });
        }
    }

    pub fn render(&mut self, app: &App, ui: &mut Ui) {
        egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
            let id = Id::new("packer_data");
            self.render_package_deps(app, ui.ctx());
            self.render_package_opts(app, ui.ctx());
            ui.horizontal(|ui| {
                let source_set = self.source.exists();
                ui.add_enabled_ui(source_set, |ui| {
                    if ui.icon_text_button("Manage Options", Icon::Tune).clicked() {
                        app.do_update(Message::GetPackagingOptions);
                    }
                });
                if ui
                    .icon_text_button("Set Dependencies", Icon::List)
                    .clicked()
                {
                    app.do_update(Message::ShowPackagingDependencies);
                }
                if ui.icon_text_button("Help", Icon::Help).clicked() {
                    open::that("https://nicenenerd.github.io/UKMM/mod_format.html").unwrap_or(());
                }
            });
            ui.add_space(8.0);
            render_field("Source", ui, |ui| {
                let res = ui.folder_picker(&mut self.source);
                if res.changed() {
                    app.do_update(Message::CheckMeta);
                }
                res
            });
            let mut cross = matches!(self.meta.platform, ModPlatform::Universal);
            if ui
                .checkbox(&mut cross, " Mark as cross-platform")
                .on_hover_text("Allow mod to be used for Switch or Wii U")
                .changed()
            {
                if cross {
                    self.meta.platform = ModPlatform::Universal;
                } else {
                    self.meta.platform = ModPlatform::Specific(app.platform().into());
                }
            }
            render_field("Name", ui, |ui| {
                self.meta.name.edit_ui_with_id(ui, id.with("Name"))
            });
            render_field("Version", ui, |ui| {
                let tmp_version = ui.create_temp_string(
                    "mod-self-version",
                    Some(self.meta.version.as_str().into()),
                );
                let res = tmp_version
                    .write()
                    .edit_ui(ui)
                    .on_hover_text("Must conform to semantic versioning");
                if res.changed() {
                    let ver = tmp_version.read();
                    if lenient_semver::Version::parse(ver.as_str()).is_ok() {
                        self.meta.version = ver.as_str().into()
                    }
                }
                res
            });
            render_field("Author", ui, |ui| {
                self.meta.author.edit_ui_with_id(ui, id.with("Author"))
            });
            render_field("Category", ui, |ui| {
                egui::ComboBox::new(id.with("category"), "")
                    .selected_text(self.meta.category.as_str())
                    .show_ui(ui, |ui| {
                        CATEGORIES.iter().for_each(|cat| {
                            ui.selectable_value(&mut self.meta.category, (*cat).into(), *cat);
                        });
                    })
                    .response
            });
            render_field("URL", ui, |ui| {
                let id = id.with("url");
                let url = ui
                    .get_temp_string(id.with("tmp"))
                    .get_or_insert_with(|| {
                        ui.create_temp_string(
                            id.with("tmp"),
                            self.meta.url.as_ref().map(|u| u.as_str().into()),
                        )
                    })
                    .clone();
                let res = {
                    let mut url = url.write();
                    url.edit_ui_with_id(ui, id)
                };
                if res.changed() {
                    let url = url.read();
                    self.meta.url = if url.is_empty() {
                        None
                    } else {
                        Some(url.as_str().into())
                    };
                }
                res
            });
            ui.add_space(8.0);
            ui.label("Description");
            ui.small("Some Markdown formatting supported");
            ui.add_space(4.0);
            let string = ui.create_temp_string(
                id.with("Description"),
                Some(self.meta.description.as_str().into()),
            );
            if egui::TextEdit::multiline(string.write().deref_mut())
                .desired_width(f32::INFINITY)
                .show(ui)
                .response
                .changed()
            {
                self.meta.description = string.read().as_str().into();
            }
            let is_valid = || {
                self.source != PathBuf::default()
                    && self.source.exists()
                    && !self.meta.name.is_empty()
            };
            ui.add_enabled_ui(is_valid(), |ui| {
                ui.allocate_ui_with_layout(
                    [ui.available_width(), ui.spacing().interact_size.y].into(),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        if ui.button("Package Mod").clicked() {
                            app.do_update(Message::PackageMod);
                        }
                    },
                );
            });
        });
    }
}
