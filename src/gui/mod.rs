mod info;
mod visuals;
use crate::{core::Manager, mods::Mod};
use eframe::NativeOptions;
use egui::{self, style::Widgets, Ui, Visuals};
use flume::{Receiver, Sender};
use std::sync::Arc;

enum Message {}

enum Tabs {
    Info,
    Install,
}

struct App {
    core: Arc<Manager>,
    channel: (Sender<Message>, Receiver<Message>),
    mods: Vec<Mod>,
    selected: Vec<Mod>,
    tab: Tabs,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        // cc.egui_ctx.set_visuals(visuals::default());
        let core = Arc::new(Manager::init().unwrap());
        let mods: Vec<_> = core.mod_manager().all_mods().map(|m| m.clone()).collect();
        Self {
            channel: flume::unbounded(),
            selected: mods.first().cloned().into_iter().collect(),
            mods,
            core,
            tab: Tabs::Info,
        }
    }

    fn file_menu(ui: &mut Ui) {
        ui.button("Open mod…");
    }

    fn edit_menu(ui: &mut Ui) {
        ui.button("Settings");
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.visuals_mut().widgets.noninteractive.bg_stroke.width = 0.;
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("File", Self::file_menu);
                    ui.menu_button("Edit", Self::edit_menu);
                });
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Mod list goes here");
                egui::TopBottomPanel::bottom("log")
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.label("Log goes here");
                        ui.allocate_space(ui.available_size());
                    });
            });
        });
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ctx, |ui| {
                match self.tab {
                    Tabs::Info => {
                        if let Some(mod_) = self.selected.first() {
                            info::mod_info(mod_, ui);
                        } else {
                            ui.label("No mod selected");
                        }
                    }
                    Tabs::Install => {}
                }
                ui.allocate_space(ui.available_size());
            });
    }
}

pub fn main() {
    crate::logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    eframe::run_native(
        "U-King Mod Manager",
        NativeOptions::default(),
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
