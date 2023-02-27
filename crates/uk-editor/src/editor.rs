use std::{cell::RefCell, ops::Deref, path::PathBuf};

use uk_content::resource::ResourceData;
use uk_ui::{
    editor::EditableValue,
    egui_dock::{TabViewer, Tree},
};

#[derive(Debug, Clone, PartialEq)]
pub struct EditorTab {
    pub path: PathBuf,
    pub ref_data: ResourceData,
    pub edit_data: RefCell<ResourceData>,
}

pub fn default_ui() -> Tree<EditorTab> {
    Tree::new(vec![])
}

impl TabViewer for super::App {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        let EditorTab {
            path,
            ref_data,
            edit_data,
        } = tab;
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

    fn ui(&mut self, ui: &mut eframe::egui::Ui, tab: &mut Self::Tab) {
        match *tab.edit_data.borrow_mut() {
            ResourceData::Mergeable(ref mut resource) => {
                resource.edit_ui(ui);
            }
            ResourceData::Binary(_) => (),
            ResourceData::Sarc(ref mut sarc_map) => {
                sarc_map.files.edit_ui(ui);
            }
        }
    }
}
