use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
    ops::Deref,
    path::PathBuf,
    sync::LazyLock,
};

use parking_lot::RwLock;
use path_slash::PathBufExt;
use uk_content::{resource::ResourceData, util::HashMap};
use uk_ui::{
    editor::EditableValue,
    egui::{self, Ui},
    egui_dock::{TabViewer, Tree},
    PathNode,
};

use crate::Message;

#[derive(Debug, Clone, PartialEq)]
pub enum Tabs {
    Files,
    Editor(PathBuf, ResourceData, RefCell<ResourceData>),
}

pub fn default_ui() -> Tree<Tabs> {
    let mut tree = Tree::new(vec![Tabs::Files]);
    tree.split_right(0.into(), 0.25, vec![]);
    tree
}

impl TabViewer for super::App {
    type Tab = Tabs;

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        match tab {
            Tabs::Files => "Files".into(),
            Tabs::Editor(path, ref_data, edit_data) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if ref_data != edit_data.borrow().deref() {
                    format!("*{}", name).into()
                } else {
                    name.into()
                }
            }
        }
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tabs::Files => {
                if let Some(project) = self.project.as_ref() {
                    self.render_file_tree(&project.files, ui);
                }
            }
            Tabs::Editor(_path, _saved_data, edit_data) => {
                if let ResourceData::Mergeable(ref mut resource) = *edit_data.borrow_mut() {
                    resource.edit_ui(ui);
                }
            }
        }
    }
}

impl super::App {
    pub fn render_file_tree(&self, files: &BTreeSet<PathBuf>, ui: &mut Ui) {
        ui.scope(|ui| {
            static ROOTS: LazyLock<RwLock<HashMap<u64, PathNode>>> =
                LazyLock::new(|| RwLock::new(HashMap::default()));
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
                root.render_dir_selectable(ui, self.focused.as_deref(), |path| {
                    self.do_update(Message::OpenResource(path));
                });
            }
        });
    }
}
