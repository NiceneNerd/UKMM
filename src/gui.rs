mod icons;
mod info;
mod mods;
mod options;
mod picker;
mod settings;
mod tabs;
mod tasks;
mod util;
mod visuals;
use crate::{core::Manager, logger::Entry, mods::Mod, settings::Settings};
use anyhow::Result;
use eframe::{
    egui::{FontData, FontDefinitions},
    epaint::{text::TextWrapping, FontFamily},
    NativeOptions,
};
use egui::{
    self, mutex::RwLock, style::Margin, text::LayoutJob, Align, Align2, Color32, ComboBox, FontId,
    Frame, Id, Label, LayerId, Layout, RichText, Rounding, Spinner, TextFormat, TextStyle, Ui,
    Vec2,
};
use egui_dock::{NodeIndex, Tree};
use egui_notify::Toast;
use egui_stylist::StylistState;
use flume::{Receiver, Sender};
use font_loader::system_fonts::FontPropertyBuilder;
use icons::IconButtonExt;
use im::Vector;
use join_str::jstr;
use once_cell::sync::OnceCell;
use picker::FilePickerState;
use std::{
    ops::DerefMut,
    path::PathBuf,
    sync::{Arc, Once},
    thread,
    time::Duration,
};
use uk_mod::Manifest;

use self::icons::Icon;

fn load_fonts(context: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let font_to_try = if cfg!(windows) {
        "Segoe UI".to_owned()
    } else {
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "font-name"])
            .output()
            .and_then(|o| {
                String::from_utf8(o.stdout)
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Bah"))
            })
            .unwrap_or_else(|_| "Ubuntu".to_owned())
    };
    if let Some(system_font) =
        font_loader::system_fonts::get(&FontPropertyBuilder::new().family(&font_to_try).build())
    {
        fonts
            .font_data
            .insert("System".to_owned(), FontData::from_owned(system_font.0));
    }
    if let Some(system_font) = font_loader::system_fonts::get(
        &FontPropertyBuilder::new()
            .family(&font_to_try)
            .bold()
            .build(),
    )
    .or_else(|| {
        let property = FontPropertyBuilder::new()
            .family(&(font_to_try + " Bold"))
            .build();
        font_loader::system_fonts::get(&property)
    }) {
        fonts
            .font_data
            .insert("Bold".to_owned(), FontData::from_owned(system_font.0));
    }
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "System".to_owned());
    fonts
        .families
        .insert(FontFamily::Name("Bold".into()), vec!["Bold".to_owned()]);
    context.set_fonts(fonts);
}

impl Entry {
    pub fn format(&self, job: &mut LayoutJob) {
        job.append(
            &jstr!("[{&self.timestamp}] "),
            0.,
            TextFormat {
                color: Color32::GRAY,
                font_id: FontId::monospace(10.),
                ..Default::default()
            },
        );
        job.append(
            &jstr!("{&self.level} "),
            0.,
            TextFormat {
                color: match self.level.as_str() {
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
            &self.args,
            1.,
            TextFormat {
                color: Color32::WHITE,
                font_id: FontId::monospace(10.),
                ..Default::default()
            },
        );
        job.append("\n", 0.0, Default::default());
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Tabs {
    Info,
    Install,
    Deploy,
    Mods,
    Log,
    Settings,
    Theme,
}

impl std::fmt::Display for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FocusedPane {
    ModList,
    FilePicker,
    None,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sort {
    Enabled,
    Name,
    Category,
    Version,
    Priority,
}

type Orderer = dyn Fn(&(usize, Mod), &(usize, Mod)) -> std::cmp::Ordering;

impl Sort {
    pub fn orderer(&self) -> Box<Orderer> {
        match self {
            Sort::Enabled => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| a.enabled.cmp(&b.enabled))
            }
            Sort::Name => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| a.meta.name.cmp(&b.meta.name))
            }
            Sort::Category => Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| {
                a.meta.category.cmp(&b.meta.category)
            }),
            Sort::Version => Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| {
                a.meta
                    .version
                    .partial_cmp(&b.meta.version)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            Sort::Priority => Box::new(|&(a, _), &(b, _)| a.cmp(&b)),
        }
    }
}

pub enum Message {
    Noop,
    Log(Entry),
    Confirm(Box<Message>, String),
    CloseConfirm,
    CloseError,
    ShowAbout,
    CloseAbout,
    SelectOnly(usize),
    SelectAlso(usize),
    Deselect(usize),
    ClearSelect,
    StartDrag(usize),
    ClearDrag,
    MoveSelected(usize),
    FilePickerUp,
    FilePickerBack,
    FilePickerSet(Option<PathBuf>),
    ChangeProfile(String),
    SetFocus(FocusedPane),
    SelectFile,
    OpenMod(PathBuf),
    HandleMod(Mod),
    RequestOptions(Mod),
    InstallMod(Mod),
    UninstallMods(Option<Vector<Mod>>),
    AddMod(Mod),
    RemoveMods(Vector<Mod>),
    ToggleMods(Option<Vector<Mod>>, bool),
    Apply,
    Remerge,
    // UpdateMods(Vector<Mod>),
    Error(anyhow::Error),
    ChangeSort(Sort, bool),
    RefreshModsDisplay,
    ClearChanges,
    Deploy,
    ResetSettings,
    SaveSettings,
}

struct App {
    core: Arc<Manager>,
    channel: (Sender<Message>, Receiver<Message>),
    mods: Vector<Mod>,
    displayed_mods: Vector<Mod>,
    selected: Vector<Mod>,
    drag_index: Option<usize>,
    hover_index: Option<usize>,
    picker_state: FilePickerState,
    closed_tabs: im::HashMap<Tabs, NodeIndex>,
    tree: Arc<RwLock<Tree<Tabs>>>,
    focused: FocusedPane,
    logs: Vector<Entry>,
    log: LayoutJob,
    error: Option<anyhow::Error>,
    confirm: Option<(Message, String)>,
    busy: bool,
    about: bool,
    dirty: Manifest,
    sort: (Sort, bool),
    options_mod: Option<Mod>,
    temp_settings: Settings,
    toasts: egui_notify::Toasts,
    style: StylistState,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        load_fonts(&cc.egui_ctx);
        // cc.egui_ctx.set_pixels_per_point(1.);
        visuals::default_dark(&cc.egui_ctx);
        let core = Arc::new(Manager::init().unwrap());
        let mods: Vector<_> = core.mod_manager().all_mods().collect();
        let (send, recv) = flume::unbounded();
        crate::logger::LOGGER.set_sender(send.clone());
        log::info!("Logger initialized");
        let temp_settings = core.settings().clone();
        Self {
            channel: (send, recv),
            selected: mods.front().cloned().into_iter().collect(),
            drag_index: None,
            hover_index: None,
            picker_state: Default::default(),
            displayed_mods: mods.clone(),
            mods,
            temp_settings,
            core,
            logs: Vector::new(),
            log: LayoutJob::default(),
            closed_tabs: im::HashMap::new(),
            focused: FocusedPane::None,
            error: None,
            confirm: None,
            about: false,
            busy: false,
            dirty: Manifest::default(),
            sort: (Sort::Priority, false),
            options_mod: None,
            tree: Arc::new(RwLock::new(tabs::default_ui())),
            toasts: egui_notify::Toasts::new().with_anchor(egui_notify::Anchor::BottomRight),
            style: StylistState::default(),
        }
    }

    #[inline(always)]
    fn modal_open(&self) -> bool {
        self.error.is_some() || self.busy || self.options_mod.is_some() || self.confirm.is_some()
    }

    fn do_update(&self, message: Message) {
        self.channel.0.send(message).unwrap();
    }

    fn do_task(
        &mut self,
        task: impl 'static + Send + Sync + FnOnce(Arc<Manager>) -> Result<Message>,
    ) {
        let sender = self.channel.0.clone();
        let core = self.core.clone();
        let task = Box::new(task);
        self.busy = true;
        thread::spawn(move || {
            sender
                .send(match task(core) {
                    Ok(msg) => msg,
                    Err(e) => Message::Error(e),
                })
                .unwrap();
        });
    }

    fn handle_update(&mut self, ctx: &eframe::egui::Context) {
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Noop => self.busy = false,
                Message::Log(entry) => {
                    entry.format(&mut self.log);
                    self.logs.push_back(entry);
                    if self.logs.len() > 100 {
                        self.logs.pop_front();
                        for _ in 0..4 {
                            self.log.sections.remove(0);
                        }
                    }
                }
                Message::ClearChanges => {
                    self.busy = false;
                    self.dirty.clear();
                    self.mods = self.core.mod_manager().all_mods().collect();
                    self.do_update(Message::RefreshModsDisplay);
                }
                Message::RefreshModsDisplay => {
                    self.do_update(Message::ChangeSort(self.sort.0, self.sort.1));
                }
                Message::ChangeSort(sort, rev) => {
                    let orderer = sort.orderer();
                    let mut temp = self.mods.iter().cloned().enumerate().collect::<Vector<_>>();
                    temp.sort_by(orderer);
                    self.displayed_mods = if rev {
                        temp.into_iter().rev().map(|(_, m)| m).collect()
                    } else {
                        temp.into_iter().map(|(_, m)| m).collect()
                    };
                    self.sort = (sort, rev);
                }
                Message::CloseError => self.error = None,
                Message::CloseConfirm => self.confirm = None,
                Message::ShowAbout => self.about = true,
                Message::CloseAbout => self.about = false,
                Message::Confirm(msg, prompt) => {
                    self.confirm = Some((*msg, prompt));
                }
                Message::SelectOnly(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    if self.selected.contains(mod_) {
                        self.selected.retain(|m| m == mod_);
                    } else {
                        self.selected.clear();
                        self.selected.push_back(self.mods[index].clone());
                    }
                    self.drag_index = None;
                }
                Message::SelectAlso(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    if !self.selected.contains(mod_) {
                        self.selected.push_back(mod_.clone());
                    }
                    self.drag_index = None;
                }
                Message::Deselect(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    self.selected.retain(|m| m != mod_);
                    self.drag_index = None;
                }
                Message::ClearSelect => {
                    self.selected.clear();
                    self.drag_index = None;
                }
                Message::StartDrag(i) => {
                    if ctx.input().pointer.any_released() {
                        self.drag_index = None;
                    }
                    self.drag_index = Some(i);
                    let mod_ = &self.mods[i];
                    if !self.selected.contains(mod_) {
                        if !ctx.input().modifiers.ctrl {
                            self.selected.clear();
                        }
                        self.selected.push_back(mod_.clone());
                    }
                }
                Message::ClearDrag => {
                    self.drag_index = None;
                }
                Message::MoveSelected(dest_index) => {
                    let dest_index = dest_index.clamp(0, self.mods.len() - 1);
                    if self.selected.len() == self.mods.len() {
                        return;
                    }
                    self.mods.retain(|m| !self.selected.contains(m));
                    for (i, selected_mod) in self.selected.iter().enumerate() {
                        self.mods
                            .insert((dest_index + i).min(self.mods.len()), selected_mod.clone());
                    }
                    self.hover_index = None;
                    self.drag_index = None;
                    match self.selected.iter().try_for_each(|m| {
                        self.dirty
                            .extend(m.manifest_with_options(&m.enabled_options)?.as_ref());
                        Ok(())
                    }) {
                        Ok(()) => self.do_update(Message::RefreshModsDisplay),
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::FilePickerUp => {
                    let has_parent = self.picker_state.path.parent().is_some();
                    if has_parent {
                        self.picker_state
                            .history
                            .push(self.picker_state.path.clone());
                        self.picker_state
                            .set_path(self.picker_state.path.parent().unwrap().to_path_buf());
                    }
                }
                Message::FilePickerBack => {
                    if let Some(prev) = self.picker_state.history.pop() {
                        self.picker_state.set_path(prev);
                    }
                }
                Message::FilePickerSet(path) => {
                    let path = match path {
                        Some(path) => path,
                        None => self.picker_state.path_input.as_str().into(),
                    };
                    if path.is_dir() {
                        self.picker_state.selected = None;
                        self.picker_state
                            .history
                            .push(self.picker_state.path.clone());
                        self.picker_state.set_path(path);
                    }
                }
                Message::ChangeProfile(_profile) => {
                    todo!("Change profile");
                }
                Message::SetFocus(pane) => {
                    self.focused = pane;
                }
                Message::SelectFile => {
                    self.do_task(move |_| {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("UKMM Mod (*.zip)", &["*.zip"])
                            .pick_file()
                        {
                            tasks::open_mod(&path)
                        } else {
                            Ok(Message::Noop)
                        }
                    });
                }
                Message::OpenMod(path) => {
                    self.do_task(move |_| tasks::open_mod(&path));
                }
                Message::HandleMod(mod_) => {
                    self.busy = false;
                    log::debug!("{:?}", &mod_);
                    if !mod_.meta.options.is_empty() {
                        self.do_update(Message::RequestOptions(mod_));
                    } else {
                        self.do_update(Message::InstallMod(mod_));
                    }
                }
                Message::InstallMod(mod_) => {
                    self.do_task(move |core| {
                        let mods = core.mod_manager();
                        let mod_ = mods.add(&mod_.path)?;
                        let hash = mod_.hash;
                        if !mod_.enabled_options.is_empty() {
                            mods.set_enabled_options(hash, mod_.enabled_options)?;
                        }
                        mods.save()?;
                        log::info!("Added mod {} to current profile", mod_.meta.name.as_str());
                        let mod_ = unsafe { mods.get_mod(hash).unwrap_unchecked() };
                        Ok(Message::AddMod(mod_))
                    });
                }
                Message::UninstallMods(mods) => {
                    let mods = mods.unwrap_or_else(|| self.selected.clone());
                    self.do_task(move |core| {
                        let manager = core.mod_manager();
                        mods.iter().try_for_each(|m| -> Result<()> {
                            manager.del(m.hash)?;
                            log::info!("Removed mod {} from current profile", m.meta.name.as_str());
                            Ok(())
                        })?;
                        manager.save()?;
                        Ok(Message::RemoveMods(mods))
                    });
                }
                Message::ToggleMods(mods, enabled) => {
                    let mods = mods.as_ref().unwrap_or(&self.selected);
                    match mods.iter().try_for_each(|m| -> Result<()> {
                        let mod_ =
                            unsafe { self.mods.iter_mut().find(|m2| m.eq(m2)).unwrap_unchecked() };
                        mod_.enabled = enabled;
                        self.dirty.extend(m.manifest()?.as_ref());
                        Ok(())
                    }) {
                        Ok(()) => self.do_update(Message::RefreshModsDisplay),
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::AddMod(mod_) => {
                    if let Ok(manifest) = mod_.manifest() {
                        self.dirty.extend(&manifest);
                    }
                    self.mods.push_back(mod_);
                    self.do_update(Message::RefreshModsDisplay);
                    self.busy = false;
                }
                Message::RemoveMods(mods) => {
                    self.mods.retain(|m| !mods.contains(m));
                    self.selected.retain(|m| !mods.contains(m));
                    mods.iter().for_each(|m| {
                        if let Ok(manifest) = m.manifest() {
                            self.dirty.extend(&manifest);
                        }
                    });
                    self.do_update(Message::RefreshModsDisplay);
                    self.busy = false;
                }
                Message::Apply => {
                    let mods = self.mods.clone();
                    let dirty = self.dirty.clone();
                    self.do_task(move |core| tasks::apply_changes(&core, mods, dirty));
                }
                Message::Deploy => self.do_task(move |core| {
                    log::info!("Deploying current mod configuration");
                    core.deploy_manager().deploy()?;
                    Ok(Message::ClearChanges)
                }),
                Message::Remerge => {
                    self.do_task(|core| {
                        let deploy_manager = core.deploy_manager();
                        deploy_manager.apply(None)?;
                        Ok(Message::ClearChanges)
                    });
                }
                Message::ResetSettings => {
                    self.temp_settings = self.core.settings().clone();
                }
                Message::SaveSettings => {
                    match self.temp_settings.save().and_then(|_| {
                        Arc::make_mut(&mut self.core).reload()?;
                        Ok(())
                    }) {
                        Ok(()) => {
                            self.toasts.add({
                                let mut toast = Toast::success("Settings saved");
                                toast.set_duration(Some(Duration::new(2, 0)));
                                toast
                            });
                            self.do_update(Message::ClearSelect);
                            self.do_update(Message::ClearChanges);
                        }
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::RequestOptions(mod_) => {
                    self.options_mod = Some(mod_);
                }
                Message::Error(error) => {
                    log::error!("{:?}", &error);
                    self.busy = false;
                    self.error = Some(error);
                }
            }
            ctx.request_repaint();
        }
    }

    fn render_error(&mut self, ctx: &egui::Context) {
        if let Some(err) = self.error.as_ref() {
            egui::Window::new("Error")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label(err.to_string());
                    ui.add_space(8.);
                    egui::CollapsingHeader::new("Details").show(ui, |ui| {
                        err.chain().for_each(|e| {
                            ui.label(RichText::new(e.to_string()).code());
                        });
                    });
                    ui.add_space(8.);
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::CloseError);
                                }
                                if ui.button("Copy").clicked() {
                                    ui.output().copied_text = format!("{:?}", &err);
                                    egui::popup::show_tooltip(ctx, Id::new("copied"), |ui| {
                                        ui.label("Copied")
                                    });
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    fn render_confirm(&mut self, ctx: &egui::Context) {
        let is_confirm = self.confirm.is_some();
        if is_confirm {
            egui::Window::new("Confirm")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .auto_sized()
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.add_space(8.);
                    ui.label(&self.confirm.as_ref().unwrap().1);
                    ui.add_space(8.);
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    let msg = self.confirm.take().unwrap().0;
                                    self.do_update(msg);
                                    self.do_update(Message::CloseConfirm);
                                }
                                if ui.button("Close").clicked() {
                                    self.do_update(Message::CloseConfirm);
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    fn render_busy(&self, ctx: &egui::Context) {
        if self.busy {
            egui::Window::new("Working")
                .default_size([240., 80.])
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .collapsible(false)
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    let max_width = ui.available_width() / 2.;
                    ui.vertical_centered(|ui| {
                        let text_height = ui.text_style_height(&TextStyle::Body) * 2.;
                        let padding = 80. - text_height - 8.;
                        ui.allocate_space([max_width, padding / 2.].into());
                        ui.horizontal(|ui| {
                            ui.add_space(8.);
                            ui.add(Spinner::new().size(text_height));
                            ui.add_space(8.);
                            ui.vertical(|ui| {
                                ui.label("Processing…");
                                let mut job = LayoutJob::single_section(
                                    self.logs
                                        .iter()
                                        .rev()
                                        .find(|l| l.level == "INFO")
                                        .map(|l| l.args.as_str())
                                        .unwrap_or_default()
                                        .to_owned(),
                                    TextFormat::default(),
                                );
                                job.wrap = TextWrapping {
                                    max_width,
                                    max_rows: 1,
                                    break_anywhere: true,
                                    ..Default::default()
                                };
                                ui.add(Label::new(job).wrap(false));
                            });
                            ui.shrink_width_to_current();
                        });
                        ui.allocate_space([0., padding / 2.].into());
                    });
                });
        }
    }

    fn render_about(&self, ctx: &egui::Context) {
        if self.about {
            egui::Window::new("About")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, Vec2::default())
                .fixed_size([360.0, 240.0])
                .frame(Frame::window(&ctx.style()).inner_margin(8.))
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.vertical_centered(|ui| {
                        ui.heading("U-King Mod Manager");
                        ui.label("© 2022 Caleb Smith - GPLv3");
                        ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
                    });
                    egui::Grid::new("about_box").num_columns(2).show(ui, |ui| {
                        ui.label("GitHub:");
                        if ui.link("https://github.com/NiceneNerd/ukmm").clicked() {
                            open::that("https://github.com/NiceneNerd/ukmm").unwrap_or(());
                        }
                        ui.end_row();
                        ui.label("GUI library:");
                        if ui.link("egui (forked)").clicked() {
                            open::that("https://github.com/NiceneNerd/egui").unwrap_or(());
                        }
                        ui.end_row();
                    });
                    let width = ui.min_size().x;
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.min_size().y),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                if ui.button("OK").clicked() {
                                    self.do_update(Message::CloseAbout);
                                }
                                ui.shrink_width_to_current();
                            },
                        );
                    });
                });
        }
    }

    fn render_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.style_mut().visuals.button_frame = false;
            ui.set_enabled(!self.modal_open());
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| self.file_menu(ui));
                ui.menu_button("Tools", |ui| self.tool_menu(ui));
                ui.menu_button("Window", |ui| self.window_menu(ui));
                ui.menu_button("Help", |ui| self.help_menu(ui))
            });
        });
    }

    fn file_menu(&self, ui: &mut Ui) {
        if ui.button("Open mod…").clicked() {
            ui.close_menu();
            self.do_update(Message::SelectFile);
        }
    }

    fn tool_menu(&mut self, ui: &mut Ui) {
        if ui.button("Refresh Merge").clicked() {
            ui.close_menu();
            self.do_update(Message::Remerge);
        }
    }

    fn window_menu(&mut self, ui: &mut Ui) {
        if ui.button("Reset").clicked() {
            ui.close_menu();
            *self.tree.write() = tabs::default_ui();
        }
        ui.separator();
        for tab in [
            Tabs::Info,
            Tabs::Install,
            Tabs::Deploy,
            Tabs::Mods,
            Tabs::Settings,
            Tabs::Log,
        ] {
            let disabled = self.closed_tabs.contains_key(&tab);
            if ui
                .icon_text_button(
                    format!(" {tab}"),
                    if disabled { Icon::Blank } else { Icon::Check },
                )
                .clicked()
            {
                ui.close_menu();
                let mut tree = self.tree.write();
                if let Some((tab, parent)) = self.closed_tabs.remove_with_key(&tab) {
                    if let Some(parent) = tree.iter_mut().nth(parent.0) && parent.tabs_count() > 0 {
                        parent.append_tab(tab);
                    } else {
                        tree.push_to_focused_leaf(tab);
                    }
                } else if let Some((parent_index, node_index)) = tree.find_tab(&tab) {
                    let parent = tree.iter_mut().nth(parent_index.0).unwrap();
                    parent.remove_tab(node_index);
                    self.closed_tabs.insert(tab, parent_index);
                    tree.remove_empty_leaf();
                }
            }
        }
    }

    fn help_menu(&self, ui: &mut Ui) {
        if ui.button("Help").clicked() {
            ui.close_menu();
            todo!("You need help");
        }
        if ui.button("About").clicked() {
            ui.close_menu();
            self.do_update(Message::ShowAbout);
        }
    }

    fn render_profile_menu(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .inner_margin(Margin {
                left: 2.0,
                ..Default::default()
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let current_profile = self
                        .core
                        .settings()
                        .platform_config()
                        .map(|c| c.profile.to_string())
                        .unwrap_or_else(|| "Default".to_owned());
                    ComboBox::from_id_source("profiles")
                        .selected_text(&current_profile)
                        .show_ui(ui, |ui| {
                            self.core.settings().profiles().for_each(|profile| {
                                if ui
                                    .selectable_label(
                                        profile.as_str() == current_profile,
                                        profile.as_str(),
                                    )
                                    .clicked()
                                {
                                    self.do_update(Message::ChangeProfile(profile.into()));
                                }
                            });
                        })
                        .response
                        .on_hover_text("Select Mod Profile");
                    ui.icon_button(Icon::Delete).on_hover_text("Delete Profile");
                    ui.icon_button(Icon::Add).on_hover_text("New Profile");
                    ui.icon_button(Icon::Menu).on_hover_text("Manage Profiles…");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(20.);
                        ui.label(format!(
                            "{} Mods / {} Active",
                            self.mods.len(),
                            self.mods.iter().filter(|m| m.enabled).count()
                        ));
                    });
                });
            });
    }

    fn render_pending(&self, ui: &mut Ui) {
        if !self.dirty.is_empty() {
            egui::Window::new("Pending Changes")
                .anchor(Align2::RIGHT_BOTTOM, [-32.0, -32.0])
                .collapsible(true)
                .show(ui.ctx(), |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        egui::ScrollArea::new([false, true])
                            .id_source("pending_files")
                            .auto_shrink([true, true])
                            .max_height(200.)
                            .show(ui, |ui| {
                                egui::CollapsingHeader::new("Files Pending Update").show(
                                    ui,
                                    |ui| {
                                        info::render_manifest(&self.dirty, ui);
                                    },
                                );
                            });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.icon_text_button("Apply", Icon::Check).clicked() {
                                    self.do_update(Message::Apply);
                                }
                                if ui.icon_text_button("Cancel", Icon::Cancel).clicked() {
                                    self.do_update(Message::ClearChanges);
                                }
                            });
                        });
                    });
                });
        }
    }
}

static LAYOUT_FIX: Once = Once::new();

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.handle_update(ctx);
        self.render_error(ctx);
        self.render_confirm(ctx);
        self.render_about(ctx);
        self.render_menu(ctx);
        self.render_option_picker(ctx);
        let layer_id = LayerId::background();
        let max_rect = ctx.available_rect();
        let clip_rect = ctx.available_rect();
        let id = Id::new("egui_dock::DockArea");
        let mut ui = Ui::new(ctx.clone(), layer_id, id, max_rect, clip_rect);
        static DOCK_STYLE: OnceCell<egui_dock::Style> = OnceCell::new();
        egui_dock::DockArea::new(self.tree.clone().write().deref_mut())
            .style(
                DOCK_STYLE
                    .get_or_init(|| {
                        egui_dock::StyleBuilder::from_egui(&ui.ctx().style())
                            .show_close_buttons(false)
                            .with_tab_rounding(Rounding {
                                ne: 2.0,
                                nw: 2.0,
                                ..Default::default()
                            })
                            .with_tab_outline_color(
                                ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                            )
                            .with_border_width(1.0)
                            .with_border_color(
                                ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                            )
                            .with_separator_width(1.0)
                            .with_separator_color(
                                ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                            )
                            .with_padding(Margin::default())
                            .build()
                    })
                    .clone(),
            )
            .show_inside(&mut ui, self);
        self.render_busy(ctx);
        self.toasts.show(ctx);
        LAYOUT_FIX.call_once(|| {
            *self.tree.write() = tabs::default_ui();
        });
    }
}

pub fn main() {
    icons::load_icons();
    crate::logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    eframe::run_native(
        "U-King Mod Manager",
        NativeOptions::default(),
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
