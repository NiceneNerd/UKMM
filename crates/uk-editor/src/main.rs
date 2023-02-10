#![feature(let_chains)]
mod modals;
mod project;
mod tabs;
mod tasks;

use std::{path::PathBuf, sync::Arc, thread};

use anyhow::{Context, Error, Result};
use eframe::egui::{Frame, Id};
use flume::{Receiver, Sender};
use fs_err as fs;
use parking_lot::RwLock;
use serde::Deserialize;
use tabs::Tabs;
use uk_content::{canonicalize, resource::ResourceData};
use uk_manager::core::Manager;
use uk_ui::{
    egui,
    egui_dock::{self, DockArea, Tree},
};

use crate::project::Project;

#[derive(Debug)]
pub enum Message {
    Error(Error),
    ImportMod,
    OpenProject(Project),
    OpenResource(PathBuf),
    LoadResource(PathBuf, ResourceData),
}

#[derive(Debug, Default, Deserialize)]
struct UiState {
    theme: uk_ui::visuals::Theme,
}

struct App {
    core: Arc<Manager>,
    project: Option<Project>,
    projects: Vec<Project>,
    channel: (Sender<Message>, Receiver<Message>),
    opened: Vec<(PathBuf, ResourceData)>,
    tree: Arc<RwLock<Tree<Tabs>>>,
    dock_style: egui_dock::Style,
    busy: bool,
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
        Self {
            core,
            project: None,
            projects: vec![],
            channel: flume::unbounded(),
            opened: vec![],
            tree: Arc::new(RwLock::new(tabs::default_ui())),
            dock_style: uk_ui::visuals::style_dock(&cc.egui_ctx.style()),
            busy: false,
        }
    }

    fn do_update(&self, message: Message) {
        self.channel.0.send(message).unwrap();
    }

    fn do_task(
        &mut self,
        task: impl 'static
        + Send
        + Sync
        + FnOnce(Arc<Manager>) -> Result<Message>
        + std::panic::UnwindSafe,
    ) {
        let sender = self.channel.0.clone();
        let core = self.core.clone();
        let task = Box::new(task);
        self.busy = true;
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
        if ui.button("Import Mod…").clicked() {
            ui.close_menu();
            self.do_update(Message::ImportMod);
        }
        if ui.button("Open Project…").clicked() {
            ui.close_menu();
            todo!("Open Project");
        }
        ui.separator();
        ui.add_enabled_ui(self.project.is_some(), |ui| {
            if ui.button("Save").clicked() {
                ui.close_menu();
                todo!("Save project");
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
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Error(e) => {
                    dbg!(e);
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
                Message::OpenProject(project) => {
                    self.project = Some(project);
                }
                Message::OpenResource(path) => {
                    if let Some(project) = self.project.as_ref() {
                        let root = project.path.clone();
                        self.do_task(move |_| {
                            let file = root.join(canonicalize(&path).as_str());
                            let resource: ResourceData = ron::from_str(&fs::read_to_string(file)?)?;
                            Ok(Message::LoadResource(path, resource))
                        });
                    }
                }
                Message::LoadResource(path, res) => {
                    self.opened.push((path, res));
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_update();
        egui::TopBottomPanel::top("menu")
            .exact_height(ctx.style().spacing.interact_size.y)
            .show(ctx, |ui| {
                ui.style_mut().visuals.button_frame = false;
                ui.menu_button("File", |ui| self.file_menu(ui, frame));
            });
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                DockArea::new(&mut self.tree.clone().write())
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
