use std::path::Path;
pub use std::path::PathBuf;

use egui::{epaint::text::TextWrapping, text::LayoutJob, FontId, Label, Sense, TextFormat, Ui};

// A recursive type to represent a directory tree.
// Simplification: If it has children, it is considered
// a directory, else considered a file.
#[derive(Debug, Clone, Hash)]
pub struct PathNode {
    name:     String,
    path:     Option<PathBuf>,
    children: Vec<PathNode>,
}

impl PathNode {
    pub fn new(name: &str) -> PathNode {
        PathNode {
            name:     name.into(),
            path:     None,
            children: Vec::<PathNode>::new(),
        }
    }

    pub fn find_child(&mut self, name: &str) -> Option<&mut PathNode> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    pub fn add_child<T>(&mut self, leaf: T) -> &mut Self
    where
        T: Into<PathNode>,
    {
        self.children.push(leaf.into());
        self
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn dir(val: &str) -> Self {
        Self::new(val)
    }

    pub fn build_tree(&mut self, parts: &Vec<String>, depth: usize) {
        if depth < parts.len() {
            let item = &parts[depth];

            let dir = match self.find_child(item) {
                Some(d) => d,
                None => {
                    let d = PathNode::new(item);
                    self.add_child(d);
                    match self.find_child(item) {
                        Some(d2) => d2,
                        None => unreachable!(),
                    }
                }
            };
            if depth + 1 == parts.len() {
                dir.set_path(parts.iter().collect());
            }
            dir.build_tree(parts, depth + 1);
        }
    }

    pub fn render_dir(&self, ui: &mut Ui) {
        if !self.children.is_empty() {
            egui::CollapsingHeader::new(self.name.as_str())
                .id_salt(self)
                .show(ui, |ui| {
                    self.children.iter().for_each(|subdir| {
                        subdir.render_dir(ui);
                    })
                });
        } else {
            let mut job = LayoutJob::single_section(self.name.clone(), TextFormat {
                font_id: FontId::proportional(
                    ui.style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .unwrap()
                        .size,
                ),
                ..Default::default()
            });
            job.wrap = TextWrapping {
                max_width: ui.available_width(),
                max_rows: 1,
                break_anywhere: true,
                ..Default::default()
            };
            let label = ui.add(Label::new(job).sense(Sense::hover()));
            if let Some(path) = self.path.as_ref() {
                label.on_hover_text(path.display().to_string());
            }
        }
    }

    pub fn render_dir_selectable<C>(&self, ui: &mut Ui, selected: Option<&Path>, on_select: C)
    where
        C: Fn(PathBuf) + Clone,
    {
        if !self.children.is_empty() {
            egui::CollapsingHeader::new(self.name.as_str())
                .id_salt(self)
                .show(ui, |ui| {
                    self.children.iter().for_each(|subdir| {
                        subdir.render_dir_selectable(ui, selected, on_select.clone());
                    })
                });
        } else {
            let mut job = LayoutJob::single_section(self.name.clone(), TextFormat {
                font_id: FontId::proportional(
                    ui.style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .unwrap()
                        .size,
                ),
                ..Default::default()
            });
            job.wrap = TextWrapping {
                max_width: ui.available_width(),
                max_rows: 1,
                break_anywhere: true,
                ..Default::default()
            };
            let label = ui.selectable_label(
                self.path
                    .as_ref()
                    .and_then(|p| selected.map(|s| p == s))
                    .unwrap_or(false),
                job,
            );
            if let Some(path) = self.path.as_ref() {
                if label
                    .on_hover_text(path.display().to_string())
                    .double_clicked()
                {
                    on_select(path.clone());
                }
            }
        }
    }
}
