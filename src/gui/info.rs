use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::mods::Mod;
use egui::{Align, FontId, Label, Layout, RichText, TextStyle, Ui};
use egui_extras::{Size, TableBuilder};
use smartstring::alias::String;
use uk_mod::Manifest;

pub fn render_mod_info(mod_: &Mod, ui: &mut Ui) {
    ui.vertical(|ui| {
        let ver = mod_.meta.version.to_string();
        [
            ("Name", mod_.meta.name.as_str()),
            ("Version", ver.as_str()),
            ("Category", mod_.meta.category.as_str()),
            ("Author", mod_.meta.author.as_str()),
        ]
        .into_iter()
        .for_each(|(label, value)| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label));
                ui.add_space(8.);
                ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                    ui.add(Label::new(value).wrap(true));
                })
            });
        });
        ui.label("Description");
        ui.add(Label::new(mod_.meta.description.as_str()).wrap(true));
        ui.label("Manifest");
        render_manifest(&mod_.manifest, ui);
    });
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Default)]
struct PathNode {
    name: String,
    path: PathBuf,
    children: BTreeSet<PathNode>,
}

impl From<&BTreeSet<String>> for PathNode {
    fn from(files: &BTreeSet<String>) -> Self {
        files.iter().fold(PathNode::default(), |root, file| root)
    }
}

pub fn render_manifest(manifest: &Manifest, ui: &mut Ui) {
    egui::CollapsingHeader::new("Base Files").show(ui, |ui| {});
}
