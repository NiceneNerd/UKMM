use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use nk_ui::{
    egui::{self, Ui},
    PathNode,
};
use nk_util::Lazy;
use parking_lot::RwLock;
use path_slash::PathBufExt;
use uk_content::util::HashMap;

use crate::Message;

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
                root.render_dir_selectable(ui, self.focused.as_deref(), |path| {
                    self.do_update(Message::OpenResource(path));
                });
            }
        });
    }
}
