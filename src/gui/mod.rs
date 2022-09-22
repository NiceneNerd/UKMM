mod info;
mod visuals;
use crate::{core::Manager, logger::Entry, mods::Mod};
use eframe::NativeOptions;
use egui::{
    self,
    style::{Margin, Widgets},
    text::LayoutJob,
    Color32, FontId, Frame, Grid, Label, Layout, RichText, Sense, Stroke, TextBuffer, TextEdit,
    TextFormat, TextStyle, Ui, Visuals, WidgetText,
};
use egui_extras::{Size, TableBuilder};
use flume::{Receiver, Sender};
use join_str::jstr;
use std::{collections::VecDeque, sync::Arc};

#[inline(always)]
fn common_frame() -> Frame {
    Frame {
        stroke: Stroke::new(0.1, Color32::DARK_GRAY),
        inner_margin: Margin::same(4.),
        ..Default::default()
    }
}

pub enum Message {
    Log(Entry),
}

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
    logs: VecDeque<Entry>,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        // cc.egui_ctx.set_visuals(visuals::default());
        let core = Arc::new(Manager::init().unwrap());
        let mods: Vec<_> = core.mod_manager().all_mods().map(|m| m.clone()).collect();
        let (send, recv) = flume::unbounded();
        crate::logger::LOGGER.set_sender(send.clone());
        crate::logger::LOGGER.flush_queue();
        log::info!("Logger initialized");
        Self {
            channel: (send, recv),
            selected: mods.first().cloned().into_iter().collect(),
            mods,
            core,
            logs: VecDeque::new(),
            tab: Tabs::Info,
        }
    }

    fn handle_update(&mut self) {
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Log(entry) => self.logs.push_back(entry),
            }
        }
    }

    fn render_menu(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar")
            .frame(common_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("File", Self::file_menu);
                    ui.menu_button("Edit", Self::edit_menu);
                });
            });
    }

    fn file_menu(ui: &mut Ui) {
        if ui.button("Open mod…").clicked() {
            // todo!("Open mod");
        }
    }

    fn edit_menu(ui: &mut Ui) {
        if ui.button("Settings").clicked() {
            todo!("Settings");
        }
    }

    fn render_log(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("log")
            .frame(Frame {
                fill: Color32::BLACK,
                inner_margin: Margin::same(6.),
                stroke: Stroke::new(0.1, Color32::DARK_GRAY),
                ..Default::default()
            })
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let mut job = LayoutJob::default();
                        self.logs.iter().for_each(|entry| {
                            job.append(
                                &jstr!("[{&entry.timestamp}] "),
                                0.,
                                TextFormat {
                                    color: Color32::GRAY,
                                    font_id: FontId::monospace(10.),
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &jstr!("{&entry.level} "),
                                0.,
                                TextFormat {
                                    color: match entry.level.as_str() {
                                        "INFO" => visuals::GREEN,
                                        "WARN" => visuals::ORGANGE,
                                        "ERROR" => visuals::RED,
                                        "DEBUG" => visuals::BLUE,
                                        _ => visuals::YELLOW,
                                    },
                                    font_id: FontId::monospace(10.),
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &entry.args,
                                1.,
                                TextFormat {
                                    color: Color32::WHITE,
                                    font_id: FontId::monospace(10.),
                                    ..Default::default()
                                },
                            );
                            job.append("\n", 0.0, Default::default());
                        });
                        let text = job.text.clone();
                        if ui
                            .add(Label::new(job).sense(Sense::click()))
                            .on_hover_text("Click to copy")
                            .clicked()
                        {
                            ui.output().copied_text = text;
                        }
                    });
            });
    }

    fn render_modlist(&mut self, ui: &mut Ui) {
        println!("{}", ui.available_width());
        let size = ui.available_size();
        ui.set_max_size(size);
        Grid::new("mod_list").striped(true).show(ui, |ui| {
            self.mods.iter_mut().enumerate().for_each(|(i, mod_)| {
                ui.checkbox(&mut mod_.enabled, "");
                ui.label(mod_.meta.name.as_str());
                ui.allocate_space(ui.available_size());
                ui.label(mod_.meta.category.as_str());
                ui.label(&mod_.meta.version.to_string());
                ui.label(&i.to_string());
                ui.end_row();
            });
        });
        ui.set_min_size(size);
        // TableBuilder::new(ui)
        //     .resizable(true)
        //     .striped(true)
        //     // .column(Size::exact(16.))
        //     .columns(Size::remainder(), 5)
        //     // .column(Size::exact(16.))
        //     // .column(Size::exact(16.))
        //     .clip(true)
        //     .header(text_height, |mut header| {
        //         header.col(|ui| {
        //             ui.add_space(16.);
        //         });
        //         header.col(|ui| {
        //             ui.label("Mod Name");
        //         });
        //         header.col(|ui| {
        //             ui.label("Category");
        //         });
        //         header.col(|ui| {
        //             ui.label("Version");
        //         });
        //         header.col(|ui| {
        //             ui.label("Priority");
        //         });
        //     })
        //     .body(|mut body| {
        //         body.rows(text_height, self.mods.len(), |index, mut row| {
        //             let mod_ = unsafe { self.mods.get_unchecked_mut(index) };
        //             row.col(|ui| {
        //                 ui.checkbox(&mut mod_.enabled, "");
        //             });
        //             row.col(|ui| {
        //                 ui.label(mod_.meta.name.as_str());
        //             });
        //             row.col(|ui| {
        //                 ui.label(mod_.meta.category.as_str());
        //             });
        //             row.col(|ui| {
        //                 ui.label(&mod_.meta.version.to_string());
        //             });
        //             row.col(|ui| {
        //                 ui.label(&index.to_string());
        //             });
        //         });
        //     });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_update();
        self.render_menu(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_modlist(ui);
            egui::SidePanel::right("right_panel")
                .resizable(true)
                .min_width(0.)
                .frame(common_frame())
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_source("right_panel_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(matches!(self.tab, Tabs::Info), "Mod Info")
                                    .clicked()
                                {
                                    self.tab = Tabs::Info;
                                }
                                if ui
                                    .selectable_label(matches!(self.tab, Tabs::Install), "Install")
                                    .clicked()
                                {
                                    self.tab = Tabs::Install;
                                }
                            });
                            match self.tab {
                                Tabs::Info => {
                                    if let Some(mod_) = self.selected.first() {
                                        info::render_mod_info(mod_, ui);
                                    } else {
                                        ui.label("No mod selected");
                                    }
                                }
                                Tabs::Install => {}
                            }
                        });
                    ui.allocate_space(ui.available_size());
                });
        });
        self.render_log(ctx);
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
