#![feature(let_chains, lazy_cell)]
mod editor;
mod files;
mod modals;
mod project;
mod tasks;

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    sync::Arc,
    thread,
};

use anyhow::{Context, Error, Result};
use editor::EditorTab;
use eframe::egui::{panel::Side, Frame};
use flume::{Receiver, Sender};
use fs_err as fs;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::Deserialize;
use uk_content::{canonicalize, prelude::Mergeable, resource::ResourceData};
use uk_manager::core::Manager;
use uk_ui::{
    egui,
    egui_dock::{self, Tree},
};

use crate::project::Project;

#[derive(Debug)]
pub enum Message {
    CloseError,
    Error(Error),
    ImportMod,
    OpenProject(Project),
    OpenResource(PathBuf),
    LoadResource(PathBuf, ResourceData),
    Save,
}

#[derive(Debug, Default, Deserialize)]
struct UiState {
    theme: uk_ui::visuals::Theme,
}

struct App {
    core: Arc<Manager>,
    project: Option<Project>,
    channel: (Sender<Message>, Receiver<Message>),
    tree: Arc<RwLock<Tree<EditorTab>>>,
    focused: Option<PathBuf>,
    dock_style: egui_dock::Style,
    busy: Cell<bool>,
    error: Option<Error>,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        uk_ui::icons::load_icons();
        uk_ui::load_fonts(&cc.egui_ctx);
        let core = Arc::new(Manager::init().expect("Core manager failed to initialize"));
        let ui_state: UiState = fs::read_to_string(core.settings().state_file())
            .context("")
            .and_then(|s| serde_json::from_str(&s).context(""))
            .unwrap_or_default();
        ui_state.theme.set_theme(&cc.egui_ctx);
        let mut dock_style = uk_ui::visuals::style_dock(&cc.egui_ctx.style());
        dock_style.show_close_buttons = true;
        Self {
            core,
            project: None,
            channel: flume::unbounded(),
            tree: Arc::new(RwLock::new(editor::default_ui())),
            focused: None,
            dock_style,
            busy: Cell::new(false),
            error: None,
        }
    }

    fn active_tab(&self) -> Option<MappedRwLockReadGuard<'_, editor::EditorTab>> {
        let tree = self.tree.read();
        if tree.focused_leaf().is_none() {
            None
        } else {
            Some(RwLockReadGuard::map(tree, |tree| {
                let leaf = tree.focused_leaf().unwrap();
                let node = tree.iter().nth(leaf.0).unwrap();
                match node {
                    egui_dock::Node::Leaf { tabs, active, .. } => &tabs[active.0],
                    _ => unreachable!(),
                }
            }))
        }
    }

    #[inline]
    fn do_update(&self, message: Message) {
        self.channel.0.send(message).unwrap();
    }

    fn do_task(
        &self,
        task: impl 'static
        + Send
        + Sync
        + FnOnce(Arc<Manager>) -> Result<Message>
        + std::panic::UnwindSafe,
    ) {
        let sender = self.channel.0.clone();
        let core = self.core.clone();
        let task = Box::new(task);
        self.busy.set(true);
        thread::spawn(move || {
            sender
                .send(match std::panic::catch_unwind(|| task(core)) {
                    Ok(Ok(msg)) => msg,
                    Ok(Err(e)) => Message::Error(e),
                    Err(e) => {
                        Message::Error(anyhow::format_err!(
                            "{}",
                            e.downcast::<String>().unwrap_or_else(|_| {
                                Box::new(
                                    "An unknown error occured, check the log for possible details."
                                        .to_string(),
                                )
                            })
                        ))
                    }
                })
                .unwrap();
        });
    }

    fn file_menu(&self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if ui.button("New Project…").clicked() {
            ui.close_menu();
            todo!("New Project");
        }
        if ui.button("Open Project…").clicked() {
            ui.close_menu();
            if let Some(folder) = rfd::FileDialog::new()
                .set_title("Select Project Folder")
                .set_directory(self.core.settings().projects_dir())
                .pick_folder()
            {
                self.do_task(move |_core| {
                    let project = project::Project::open(&folder)?;
                    Ok(Message::OpenProject(project))
                });
            }
        }
        if ui.button("Import Mod…").clicked() {
            ui.close_menu();
            self.do_update(Message::ImportMod);
        }
        ui.separator();
        ui.add_enabled_ui(self.project.is_some(), |ui| {
            if ui.button("Save").clicked() {
                ui.close_menu();
                self.do_update(Message::Save);
            }
            if ui.button("Save As…").clicked() {
                ui.close_menu();
                todo!("Save project as");
            }
            if ui.button("Package…").clicked() {
                ui.close_menu();
                todo!("Package mod");
            }
        });
        ui.separator();
        if ui.button("Exit").clicked() {
            frame.close();
        }
    }

    fn handle_update(&mut self) {
        if let Some(path) = self
            .tree
            .write()
            .find_active_focused()
            .map(|(_, tab)| tab.path.as_path())
            && self.focused.as_ref().map(|p| p != path).unwrap_or(true)
        {
            self.focused = Some(path.to_path_buf());
        }
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::CloseError => {
                    self.error = None;
                }
                Message::Error(e) => {
                    self.error = Some(e);
                    self.busy.set(false);
                }
                Message::ImportMod => {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Import Mod")
                        .add_filter("UKMM Mod (*.zip)", &["zip"])
                        .pick_file()
                    {
                        self.do_task(move |core| tasks::import_mod(&core, path));
                    }
                }
                Message::LoadResource(path, res) => {
                    let new_tab = EditorTab {
                        path,
                        ref_data: res.clone(),
                        edit_data: RefCell::new(res),
                    };
                    self.tree.write().push_to_first_leaf(new_tab);
                    self.busy.set(false);
                }
                Message::OpenProject(project) => {
                    self.project = Some(project);
                    self.busy.set(false);
                }
                Message::OpenResource(path) => {
                    if let Some(project) = self.project.as_ref() {
                        let root = project.path.clone();
                        self.do_task(move |core| tasks::open_resource(&core, root, path));
                    }
                }
                Message::Save => {
                    if let Some(EditorTab {
                        path,
                        ref_data: _,
                        edit_data: _,
                    }) = self.active_tab().as_deref()
                    {
                        dbg!(&path);
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_update();
        self.render_modals(ctx);
        egui::TopBottomPanel::top("menu")
            .exact_height(ctx.style().spacing.interact_size.y)
            .show(ctx, |ui| {
                ui.style_mut().visuals.button_frame = false;
                ui.menu_button("File", |ui| self.file_menu(ui, frame));
            });
        egui::SidePanel::new(Side::Left, "files-panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    if let Some(project) = self.project.as_ref() {
                        self.render_file_tree(&project.files, ui);
                    }
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_dock::DockArea::new(&mut self.tree.clone().write())
                .id("editor-dock".into())
                .style(self.dock_style.clone())
                .show_inside(ui, self);
        });
    }
}

fn main() {
    eframe::run_native(
        "U-King Mod Maker",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(App::new(cc))),
    )
}
