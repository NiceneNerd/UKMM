use std::{ops::DerefMut, path::PathBuf, sync::Arc};

use eframe::emath::Align;
use fs_err as fs;
use parking_lot::RwLock;
use uk_manager::settings::Platform;
use uk_mod::{ExclusiveOptionGroup, Meta, ModOptionGroup, MultipleOptionGroup, OptionGroup};
use uk_ui::{
    editor::EditableValue,
    egui::{self, Align2, Context, Id, Layout, Response, TextStyle, Ui},
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};

use super::{App, Message};

pub static CATEGORIES: &[&str] = &[
    "Animations",
    "Balance",
    "Crafting",
    "Customization",
    "Difficulty",
    "Enemies",
    "Expansion",
    "Meme/Gimmick",
    "Other",
    "Overhaul",
    "Overworld",
    "Player",
    "Quest",
    "Shrine",
    "Skin/Texture",
];

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
                        let id = Id::new("opt-groups-");
                        for (i, opt_group) in builder.meta.options.iter_mut().enumerate() {
                            render_opt(i, opt_group, id, ui);
                        }
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

        #[inline]
        fn render_opt(i: usize, opt_group: &mut OptionGroup, id: Id, ui: &mut Ui) {
            let group_name = if opt_group.name().is_empty() {
                "New Option Group"
            } else {
                opt_group.name()
            };
            egui::CollapsingHeader::new(group_name)
                .default_open(true)
                .show(ui, |ui| {
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
                                    if ui.selectable_label(selected, opt.name.as_str()).clicked() {
                                        group.default = Some(opt.path.clone());
                                    }
                                });
                            });
                    }
                });
        }
    }

    pub fn render_packger(&self, ui: &mut Ui) {
        egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
            let id = Id::new("packer_data");
            let builder = ui
                .data()
                .get_temp_mut_or_insert_with::<Arc<RwLock<ModPackerBuilder>>>(id, || {
                    Arc::new(RwLock::new(ModPackerBuilder::new(
                        self.core.settings().current_mode,
                    )))
                })
                .clone();
            let mut builder = builder.write();
            self.render_package_deps(ui.ctx(), &mut builder);
            self.render_package_opts(ui.ctx(), &mut builder);
            ui.horizontal(|ui| {
                if ui.icon_text_button("Manage Options", Icon::Tune).clicked() {
                    if let Ok(reader) = fs::read_dir(&builder.source) {
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
                let source_set = builder.source.exists();
                ui.add_enabled_ui(source_set, |ui| {
                    if ui
                        .icon_text_button("Set Dependencies", Icon::List)
                        .clicked()
                    {
                        self.do_update(Message::ShowPackagingDependencies);
                    }
                });
                if ui.icon_text_button("Help", Icon::Help).clicked() {
                    open::that("https://nicenenerd.github.io/ukmm/packaging.html").unwrap_or(());
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
            let string = ui
                .get_temp_string(id.with("Description"))
                .get_or_insert_default()
                .clone();
            if egui::TextEdit::multiline(string.write().deref_mut())
                .desired_width(f32::INFINITY)
                .show(ui)
                .response
                .changed()
            {
                builder.meta.description = string.read().as_str().into();
            }
        });
    }
}
