use super::{visuals, App, Message};
use anyhow::Result;
use eframe::emath::Align;
use fs_err as fs;
use parking_lot::RwLock;
use rustc_hash::FxHashSet;
use std::{
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_manager::{mods::Mod, settings::Platform};
use uk_mod::{Meta, ModOptionGroup, OptionGroup};
use uk_ui::{
    editor::EditableValue,
    egui::{self, text::LayoutJob, Align2, Context, Id, Layout, Response, RichText, TextStyle, Ui},
    ext::UiExt,
    icons::IconButtonExt,
};

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
    pub dest: PathBuf,
    pub meta: Meta,
}

impl ModPackerBuilder {
    pub fn new(platform: Platform) -> Self {
        ModPackerBuilder {
            source: Default::default(),
            dest: Default::default(),
            meta: Meta {
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
    ui.allocate_ui_with_layout(
        [ui.available_width(), ui.spacing().item_spacing.y].into(),
        Layout::right_to_left(egui::Align::Center),
        |ui| {
            add_contents(ui);
        },
    );
    ui.end_row();
}

impl App {
    fn render_package_deps(&self, ctx: &Context, builder: &mut ModPackerBuilder) {
        if self.show_package_deps {
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
                                    let mut in_deps =
                                        builder.meta.masters.contains_key(&mod_.hash());
                                    if ui
                                        .checkbox(
                                            &mut in_deps,
                                            format!(
                                                " {} (v{})",
                                                mod_.meta.name.as_str(),
                                                mod_.meta.version
                                            ),
                                        )
                                        .changed()
                                    {
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
    }

    fn render_package_opts(&self, ctx: &Context, builder: &mut ModPackerBuilder) {
        if let Some(ref folders) = self.opt_folders {
            egui::Window::new("Configure Mod Options")
                .anchor(Align2::CENTER_CENTER, [0., 0.])
                .show(ctx, |ui| {
                    ui.allocate_ui_with_layout(
                        [ui.available_width(), ui.spacing().interact_size.y].into(),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                                todo!()
                            }
                        },
                    );
                    let id = Id::new("opt-groups-");
                    for (i, opt_group) in builder.meta.options.iter_mut().enumerate() {
                        let group_name = if opt_group.name().is_empty() {
                            "New Option Group"
                        } else {
                            opt_group.name()
                        };
                        egui::CollapsingHeader::new(group_name)
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Name");
                                    opt_group.name_mut().edit_ui_with_id(ui, id.with(i));
                                });
                                ui.label("Description");
                                ui.text_edit_multiline(&mut uk_ui::editor::SmartStringWrapper(
                                    opt_group.description_mut(),
                                ));
                                ui.horizontal(|ui| {});
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
            ui.horizontal(|ui| {
                ui.icon_text_button("Manage Options", uk_ui::icons::Icon::Tune);
                let source_set = builder.source.exists();
                ui.add_enabled_ui(source_set, |ui| {
                    if ui
                        .icon_text_button("Set Dependencies", uk_ui::icons::Icon::List)
                        .clicked()
                    {
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
                });
                if ui
                    .icon_text_button("Help", uk_ui::icons::Icon::Help)
                    .clicked()
                {
                    open::that("https://nicenenerd.github.io/ukmm/packaging.html").unwrap_or(());
                }
            });
            ui.add_space(8.0);
            egui::Grid::new("packer_grid1")
                .num_columns(2)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    render_field("Source", ui, |ui| ui.folder_picker(&mut builder.source));
                    render_field("Name", ui, |ui| {
                        builder.meta.name.edit_ui_with_id(ui, id.with("Name"))
                    });
                    render_field("Version", ui, |ui| {
                        builder.meta.version.edit_ui_with_id(ui, id.with("Version"))
                    });
                    render_field("Author", ui, |ui| {
                        builder.meta.author.edit_ui_with_id(ui, id.with("Author"))
                    });
                    render_field("Category", ui, |ui| {
                        egui::ComboBox::new(id.with("category"), "")
                            .selected_text(builder.meta.category.as_str())
                            .show_ui(ui, |ui| {
                                CATEGORIES.iter().for_each(|cat| {
                                    ui.selectable_value(
                                        &mut builder.meta.category,
                                        (*cat).into(),
                                        *cat,
                                    );
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
                });
            ui.add_space(8.0);
            ui.label("Description");
            ui.small("Some Markdown formatting supported");
            ui.add_space(8.0);
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
