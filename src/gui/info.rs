use crate::mods::Mod;
use eframe::epaint::text::TextWrapping;
use egui::{text::LayoutJob, Align, FontId, Label, Layout, RichText, Sense, TextFormat, Ui};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHasher};
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    path::PathBuf,
};
use uk_mod::Manifest;

pub fn render_mod_info(mod_: &Mod, ui: &mut Ui) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.;
        ui.add_space(8.);
        if let Some(preview) = mod_.preview() {
            preview.show_max_size(ui, ui.available_size());
            ui.add_space(8.);
        }
        let ver = mod_.meta.version.to_string();
        [
            ("Name", mod_.meta.name.as_str()),
            ("Version", ver.as_str()),
            ("Category", mod_.meta.category.as_str()),
            ("Author", mod_.meta.author.as_str()),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .for_each(|(label, value)| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).family(egui::FontFamily::Name("Bold".into())));
                ui.add_space(8.);
                ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                    ui.add(Label::new(value).wrap(true));
                })
            });
        });
        ui.label(RichText::new("Description").family(egui::FontFamily::Name("Bold".into())));
        ui.add_space(4.);
        ui.add(Label::new(mod_.meta.description.as_str()).wrap(true));
        ui.add_space(4.);
        ui.label(RichText::new("Manifest").family(egui::FontFamily::Name("Bold".into())));
        match mod_.manifest() {
            Ok(manifest) => render_manifest(&manifest, ui),
            Err(e) => {
                ui.label(RichText::new("FAILED TO LOAD MANIFEST").strong());
            }
        }
    });
}

// A recursive type to represent a directory tree.
// Simplification: If it has children, it is considered
// a directory, else considered a file.
#[derive(Debug, Clone, Hash)]
struct PathNode {
    name: String,
    path: Option<PathBuf>,
    children: Vec<PathNode>,
}

impl PathNode {
    fn new(name: &str) -> PathNode {
        PathNode {
            name: name.into(),
            path: None,
            children: Vec::<PathNode>::new(),
        }
    }

    fn find_child(&mut self, name: &str) -> Option<&mut PathNode> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    fn add_child<T>(&mut self, leaf: T) -> &mut Self
    where
        T: Into<PathNode>,
    {
        self.children.push(leaf.into());
        self
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }
}

fn dir(val: &str) -> PathNode {
    PathNode::new(val)
}

fn build_tree(node: &mut PathNode, parts: &Vec<String>, depth: usize) {
    if depth < parts.len() {
        let item = &parts[depth];

        let dir = match node.find_child(item) {
            Some(d) => d,
            None => {
                let d = PathNode::new(item);
                node.add_child(d);
                match node.find_child(item) {
                    Some(d2) => d2,
                    None => unreachable!(),
                }
            }
        };
        if depth + 1 == parts.len() {
            dir.set_path(parts.iter().collect());
        }
        build_tree(dir, parts, depth + 1);
    }
}

fn render_dir(dir: &PathNode, ui: &mut Ui) {
    if !dir.children.is_empty() {
        ui.spacing_mut().icon_width_inner = 4.;
        egui::CollapsingHeader::new(dir.name.as_str())
            .id_source(dir)
            .show(ui, |ui| {
                dir.children.iter().for_each(|subdir| {
                    render_dir(subdir, ui);
                })
            });
    } else {
        let mut job = LayoutJob::single_section(dir.name.clone(), {
            let mut fmt = TextFormat::default();
            fmt.font_id.size = 10.;
            fmt
        });
        job.wrap = TextWrapping {
            max_width: ui.available_width(),
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let label = ui.add(Label::new(job).sense(Sense::hover()));
        if let Some(path) = dir.path.as_ref() {
            label.on_hover_text(path.display().to_string());
        }
    }
}

pub fn render_manifest(manifest: &Manifest, ui: &mut Ui) {
    ui.scope(|ui| {
        static ROOTS: Lazy<RwLock<FxHashMap<u64, PathNode>>> =
            Lazy::new(|| RwLock::new(FxHashMap::default()));
        ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
        ui.spacing_mut().item_spacing.y = 4.;
        if !manifest.content_files.is_empty() {
            let mut hasher = FxHasher::default();
            manifest.content_files.hash(&mut hasher);
            let mut roots = ROOTS.write();
            let content_root = roots.entry(hasher.finish()).or_insert_with(|| {
                let mut root = dir("Base Files");
                manifest.content_files.iter().for_each(|file| {
                    build_tree(
                        &mut root,
                        &file.split('/').map(|s| s.to_owned()).collect(),
                        0,
                    );
                });
                root
            });
            render_dir(content_root, ui);
        }
        if !manifest.aoc_files.is_empty() {
            let mut hasher = FxHasher::default();
            manifest.aoc_files.hash(&mut hasher);
            let mut roots = ROOTS.write();
            let aoc_root = roots.entry(hasher.finish()).or_insert_with(|| {
                let mut root = dir("DLC Files");
                manifest.aoc_files.iter().for_each(|file| {
                    build_tree(
                        &mut root,
                        &file.split('/').map(|s| s.to_owned()).collect(),
                        0,
                    );
                });
                root
            });
            render_dir(aoc_root, ui);
        }
    });
}
