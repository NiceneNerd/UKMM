use super::{visuals, App, Message};
use anyhow::Result;
use fs_err as fs;
use parking_lot::RwLock;
use std::{
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_manager::{mods::Mod, settings::Platform};
use uk_mod::Meta;
use uk_ui::{
    editor::EditableValue,
    egui::{self, text::LayoutJob, Id, Layout, Response, RichText, TextStyle, Ui},
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
                    render_field("URL", ui, |ui| {});
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
