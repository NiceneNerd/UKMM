use std::{ops::DerefMut, path::PathBuf, sync::Arc};

use eframe::emath::Align;
use fs_err as fs;
use parking_lot::{Mutex, RwLock};
use rustc_hash::FxHashSet;
use uk_manager::settings::Platform;
use uk_mod::{
    ExclusiveOptionGroup, Meta, ModOption, ModOptionGroup, MultipleOptionGroup, OptionGroup,
    CATEGORIES,
};
use uk_ui::{
    editor::EditableValue,
    egui::{self, Align2, Context, Id, Layout, Response, TextStyle, Ui},
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};

use super::{App, Message};

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
                name: Default::default(),
                version: 1.0,
                author: Default::default(),
                category: "Other".into(),
                description: Default::default(),
                platform: platform.into(),
                url: Default::default(),
                options: Default::default(),
                masters: Default::default(),
            },
        }
    }
}

fn render_field(name: &str, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> Response) {
    ui.label(name);
    ui.horizontal(add_contents);
    ui.add_space(4.0);
}

impl App {
    fn render_package_deps(&self, ctx: &Context, builder: &mut ModPackerBuilder) {
        if !self.show_package_deps {
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
                        self.mods.len(),
                        |ui, range| {
                            for mod_ in self
                                .mods
                                .iter()
                                .skip(range.start)
                                .take(range.end - range.start)
                            {
                                let mut in_deps = builder.meta.masters.contains_key(&mod_.hash());
                                let friendly = format!(
                                    " {} (v{})",
                                    mod_.meta.name.as_str(),
                                    mod_.meta.version
                                );
                                if ui.checkbox(&mut in_deps, friendly).changed() {
                                    if in_deps {
                                        builder.meta.masters.insert(
                                            mod_.hash(),
                                            (mod_.meta.name.clone(), mod_.meta.version),
                                        );
                                    } else {
                                        builder.meta.masters.shift_remove(&mod_.hash());
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
                            self.do_update(Message::ClosePackagingDependencies);
                        }
                        ui.shrink_width_to_current();
                    },
                );
            });
    }

    fn render_package_opts(&self, ctx: &Context, builder: &mut ModPackerBuilder) {
        if let Some(ref folders) = self.opt_folders {
            egui::Window::new("Configure Mod Options")
                .anchor(Align2::CENTER_CENTER, [0., 0.])
                .scroll2([false, true])
                .show(ctx, |ui| {
                    egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.horizontal(|ui| {
                            if ui.icon_text_button("Add Option Group", Icon::Add).clicked() {
                                builder
                                    .meta
                                    .options
                                    .push(OptionGroup::Multiple(Default::default()));
                            }
                        });
                        render_opt_groups(
                            &mut builder.meta.options,
                            folders,
                            Id::new("opt-groups-"),
                            ui,
                        );
                        ui.allocate_ui_with_layout(
                            [ui.available_width(), ui.spacing().interact_size.y].into(),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::ClosePackagingOptions);
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
                    "New Option Group"
                } else {
                    opt_group.name()
                };
                egui::CollapsingHeader::new(group_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui.icon_text_button("Delete", Icon::Delete).clicked() {
                            delete = Some(i);
                        }
                        ui.label("Group Name");
                        opt_group.name_mut().edit_ui_with_id(ui, id.with(i));
                        ui.label("Group Description");
                        ui.text_edit_multiline(&mut uk_ui::editor::SmartStringWrapper(
                            opt_group.description_mut(),
                        ));
                        ui.label("Group Type");
                        ui.horizontal(|ui| {
                            if ui
                                .radio(matches!(opt_group, OptionGroup::Exclusive(_)), "Exclusive")
                                .clicked()
                            {
                                *opt_group = OptionGroup::Exclusive(ExclusiveOptionGroup {
                                    default: None,
                                    name: std::mem::take(opt_group.name_mut()),
                                    description: std::mem::take(opt_group.description_mut()),
                                    options: std::mem::take(opt_group.options_mut()),
                                });
                            }
                            if ui
                                .radio(matches!(opt_group, OptionGroup::Multiple(_)), "Multiple")
                                .clicked()
                            {
                                *opt_group = OptionGroup::Multiple(MultipleOptionGroup {
                                    defaults: Default::default(),
                                    name: std::mem::take(opt_group.name_mut()),
                                    description: std::mem::take(opt_group.description_mut()),
                                    options: std::mem::take(opt_group.options_mut()),
                                });
                            }
                        });
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
                                .unwrap_or("None");
                            egui::ComboBox::new(id, "Default Option")
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
                        if let OptionGroup::Multiple(group) = opt_group
                            && let Some(defaults) = defaults
                            && defaults != group.defaults
                        {
                            group.defaults = defaults;
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
                                if let Some(defaults) = defaults && defaults.contains(&old_folder) {
                                    defaults.remove(&old_folder);
                                    defaults.insert(option.path.clone());
                                }
                            }
                        });
                });
        }
    }

    pub fn render_packger(&self, ui: &mut Ui) {
        egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
            let id = Id::new("packer_data");
            let builder_ref = ui
                .data()
                .get_temp_mut_or_insert_with::<Arc<RwLock<ModPackerBuilder>>>(id, || {
                    Arc::new(RwLock::new(ModPackerBuilder::new(
                        self.core.settings().current_mode,
                    )))
                })
                .clone();
            let mut builder = builder_ref.write();
            self.render_package_deps(ui.ctx(), &mut builder);
            self.render_package_opts(ui.ctx(), &mut builder);
            ui.horizontal(|ui| {
                let source_set = builder.source.exists();
                ui.add_enabled_ui(source_set, |ui| {
                    if ui.icon_text_button("Manage Options", Icon::Tune).clicked() {
                        if let Ok(reader) = fs::read_dir(builder.source.join("options")) {
                            self.do_update(Message::ShowPackagingOptions(
                                reader
                                    .filter_map(|res| {
                                        res.ok().and_then(|e| {
                                            e.file_type()
                                                .ok()
                                                .and_then(|t| t.is_dir().then(|| e.path()))
                                        })
                                    })
                                    .collect(),
                            ));
                        }
                    }
                });
                if ui
                    .icon_text_button("Set Dependencies", Icon::List)
                    .clicked()
                {
                    self.do_update(Message::ShowPackagingDependencies);
                }
                if ui.icon_text_button("Help", Icon::Help).clicked() {
                    open::that("https://nicenenerd.github.io/ukmm/mod_format.html").unwrap_or(());
                }
            });
            ui.add_space(8.0);
            render_field("Source", ui, |ui| ui.folder_picker(&mut builder.source));
            render_field("Name", ui, |ui| {
                builder.meta.name.edit_ui_with_id(ui, id.with("Name"))
            });
            render_field("Version", ui, |ui| {
                ui.add(
                    egui::DragValue::new(&mut builder.meta.version)
                        .clamp_range(0.0..=3000.0)
                        .speed(0.1)
                        .max_decimals(2)
                        .min_decimals(1),
                )
            });
            render_field("Author", ui, |ui| {
                builder.meta.author.edit_ui_with_id(ui, id.with("Author"))
            });
            render_field("Category", ui, |ui| {
                egui::ComboBox::new(id.with("category"), "")
                    .selected_text(builder.meta.category.as_str())
                    .show_ui(ui, |ui| {
                        CATEGORIES.iter().for_each(|cat| {
                            ui.selectable_value(&mut builder.meta.category, (*cat).into(), *cat);
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
                            builder.meta.url.as_ref().map(|u| u.as_str().into()),
                        )
                    })
                    .clone();
                let res = {
                    let mut url = url.write();
                    url.edit_ui_with_id(ui, id)
                };
                if res.changed() {
                    let url = url.read();
                    builder.meta.url = if url.is_empty() {
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
                Some(builder.meta.description.as_str().into()),
            );
            if egui::TextEdit::multiline(string.write().deref_mut())
                .desired_width(f32::INFINITY)
                .show(ui)
                .response
                .changed()
            {
                builder.meta.description = string.read().as_str().into();
            }
            let is_valid = || {
                builder.source != PathBuf::default()
                    && builder.source.exists()
                    && !builder.meta.name.is_empty()
            };
            ui.add_enabled_ui(is_valid(), |ui| {
                ui.allocate_ui_with_layout(
                    [ui.available_width(), ui.spacing().interact_size.y].into(),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        if ui.button("Package Mod").clicked() {
                            self.do_update(Message::PackageMod(builder_ref.clone()));
                        }
                    },
                );
            });
        });
    }
}
