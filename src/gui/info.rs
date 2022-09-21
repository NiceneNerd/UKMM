use crate::mods::Mod;
use egui::{Align, Label, Layout, Ui};
use egui_extras::{Size, TableBuilder};

pub fn mod_info(mod_: &Mod, ui: &mut Ui) {
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
                ui.label(label);
                ui.add_space(4.);
                ui.add(Label::new(value).wrap(true));
            });
        });
    });
}
