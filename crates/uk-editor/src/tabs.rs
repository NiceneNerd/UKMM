use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use path_slash::PathBufExt;
use uk_content::util::{HashMap, HashSet};
use uk_mod::Manifest;
use uk_ui::{
    editor::EditableValue,
    egui::{self, Ui},
    egui_dock::{TabViewer, Tree},
    PathNode,
};

use crate::Message;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Tabs {
    Files,
    Editor,
}

impl std::fmt::Display for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn default_ui() -> Tree<Tabs> {
    let mut tree = Tree::new(vec![Tabs::Files]);
    tree.split_right(0.into(), 0.25, vec![Tabs::Editor]);
    tree
}

impl TabViewer for super::App {
    type Tab = Tabs;

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tabs::Files => {
                if let Some(project) = self.project.as_ref() {
                    self.render_file_tree(&project.files, ui);
                }
            }
            Tabs::Editor => {
                if let Some((_, uk_content::resource::ResourceData::Mergeable(resource))) =
                    self.opened.last_mut()
                {
                    resource.edit_ui(ui);
                }
            }
        }
    }
}

impl super::App {
    pub fn render_file_tree(&self, files: &BTreeSet<PathBuf>, ui: &mut Ui) {
        ui.scope(|ui| {
            static ROOTS: Lazy<RwLock<HashMap<u64, PathNode>>> =
                Lazy::new(|| RwLock::new(HashMap::default()));
            ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
            ui.spacing_mut().item_spacing.y = 4.;
            if !files.is_empty() {
                let mut hasher = DefaultHasher::default();
                for file in files {
                    file.hash(&mut hasher);
                }
                let mut roots = ROOTS.write();
                let root = roots.entry(hasher.finish()).or_insert_with(|| {
                    let mut root = PathNode::dir("Mod Root");
                    files.iter().for_each(|file| {
                        root.build_tree(
                            &file
                                .to_slash_lossy()
                                .split('/')
                                .map(|s| s.to_owned())
                                .collect(),
                            0,
                        );
                    });
                    root
                });
                root.render_dir_selectable(ui, self.opened.last().map(|o| o.0.as_path()), |path| {
                    self.do_update(Message::OpenResource(path));
                });
            }
        });
    }
}
