use std::path::PathBuf;

use crate::mods::Mod;
use eframe::epaint::text::TextWrapping;
use egui::{text::LayoutJob, Align, Label, Layout, RichText, Sense, TextFormat, Ui};
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

// A recursive type to represent a directory tree.
// Simplification: If it has children, it is considered
// a directory, else considered a file.
#[derive(Debug, Clone)]
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
        egui::CollapsingHeader::new(dir.name.as_str()).show(ui, |ui| {
            dir.children.iter().for_each(|subdir| {
                render_dir(subdir, ui);
            })
        });
    } else {
        let mut job = LayoutJob::single_section(dir.name.clone(), TextFormat::default());
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
    let mut content_root = dir("Base Files");
    manifest.content_files.iter().for_each(|file| {
        build_tree(
            &mut content_root,
            &file.split('/').map(|s| s.to_owned()).collect(),
            0,
        );
    });
    render_dir(&content_root, ui);
    let mut aoc_root = dir("DLC Files");
    manifest.aoc_files.iter().for_each(|file| {
        build_tree(
            &mut aoc_root,
            &file.split('/').map(|s| s.to_owned()).collect(),
            0,
        );
    });
    render_dir(&aoc_root, ui);
}
