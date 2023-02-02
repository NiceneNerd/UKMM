#![feature(let_chains)]
mod project;

use std::sync::Arc;

use anyhow::Context;
use flume::{Receiver, Sender};
use fs_err as fs;
use serde::Deserialize;
use uk_content::resource::ResourceData;
use uk_manager::core::Manager;
use uk_ui::egui;

use crate::project::Project;

#[derive(Debug, Clone)]
enum Message {}

#[derive(Debug, Default, Deserialize)]
struct UiState {
    theme: uk_ui::visuals::Theme,
}

struct App {
    core:     Arc<Manager>,
    project:  Option<Project>,
    projects: Vec<Project>,
    channel:  (Sender<Message>, Receiver<Message>),
    opened:   Vec<ResourceData>,
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
        }
    }

    fn file_menu(&self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if ui.button("New Project…").clicked() {
            ui.close_menu();
            todo!("New Project");
        }
        if ui.button("Import Mod…").clicked() {
            ui.close_menu();
            todo!("Open Mod");
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu")
            .exact_height(ctx.style().spacing.interact_size.y)
            .show(ctx, |ui| {
                ui.style_mut().visuals.button_frame = false;
                ui.menu_button("File", |ui| self.file_menu(ui, frame));
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
