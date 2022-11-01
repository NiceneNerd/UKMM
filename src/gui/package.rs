use super::{visuals, App, Message};
use anyhow::Result;
use fs_err as fs;
use parking_lot::RwLock;
use std::{path::Path, sync::Arc};
use uk_manager::{mods::Mod, settings::Platform};
use uk_mod::Meta;
use uk_ui::{
    egui::{self, text::LayoutJob, Id, Layout, RichText, TextStyle, Ui},
    icons::IconButtonExt,
};

pub static CATEGORIES: &[&str] = &[
    "Balance",
    "Crafting",
    "Customization",
    "Difficulty",
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
    pub source: String,
    pub dest: String,
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

impl App {
    pub fn render_packger(&self, ui: &mut Ui) {
        ui.heading("Package UKMM Mod");
        let data_id = Id::new("packer_data");
        let builder = ui
            .data()
            .get_temp_mut_or_insert_with::<Arc<RwLock<ModPackerBuilder>>>(data_id, || {
                Arc::new(RwLock::new(ModPackerBuilder::new(
                    self.core.settings().current_mode,
                )))
            })
            .clone();
    }
}
