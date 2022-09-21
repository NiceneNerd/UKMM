use std::{
    cell::RefCell,
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

// A type to represent a path, split into its component parts
#[derive(Debug)]
struct PathNode {
    parts: Vec<String>,
}
impl PathNode {
    pub fn new(path: &str) -> Self {
        Self {
            parts: path.to_string().split('/').map(|s| s.into()).collect(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.parts.iter().map(|p| p.as_str()).collect()
    }
}

// A recursive type to represent a directory tree.
// Simplification: If it has children, it is considered
// a directory, else considered a file.
#[derive(Debug, Clone)]
struct Dir {
    name: String,
    children: Vec<Dir>,
}

impl Dir {
    fn new(name: &str) -> Dir {
        Dir {
            name: name.into(),
            children: Vec::<Dir>::new(),
        }
    }

    fn find_child(&mut self, name: &str) -> Option<&mut Dir> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    fn add_child<T>(&mut self, leaf: T) -> &mut Self
    where
        T: Into<Dir>,
    {
        self.children.push(leaf.into());
        self
    }
}

fn dir(val: &str) -> Dir {
    Dir::new(val)
}

fn build_tree(node: &mut Dir, parts: &Vec<String>, depth: usize) {
    if depth < parts.len() {
        let item = &parts[depth];

        let mut dir = match node.find_child(&item) {
            Some(d) => d,
            None => {
                let d = Dir::new(&item);
                node.add_child(d);
                match node.find_child(&item) {
                    Some(d2) => d2,
                    None => unreachable!(),
                }
            }
        };
        build_tree(dir, parts, depth + 1);
    }
}

fn render_dir(dir: &Dir, ui: &mut Ui) {
    if !dir.children.is_empty() {
        egui::CollapsingHeader::new(dir.name.as_str()).show(ui, |ui| {
            dir.children.iter().for_each(|subdir| {
                render_dir(subdir, ui);
            })
        });
    } else {
        ui.label(dir.name.as_str());
    }
}

pub fn render_manifest(manifest: &Manifest, ui: &mut Ui) {
    let mut content_root = dir("Base Files");
    build_tree(
        &mut content_root,
        &manifest.content_files.iter().cloned().collect(),
        0,
    );
    dbg!("{:?}", &content_root);
    render_dir(&content_root, ui);
    let mut aoc_root = dir("DLC Files");
    build_tree(
        &mut aoc_root,
        &manifest.aoc_files.iter().cloned().collect(),
        0,
    );
    render_dir(&aoc_root, ui);
}
