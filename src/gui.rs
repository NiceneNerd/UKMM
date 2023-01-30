mod info;
mod menus;
mod modals;
mod mods;
mod options;
mod package;
mod picker;
mod profiles;
mod settings;
mod tabs;
mod tasks;
mod util;
use std::{
    ops::DerefMut,
    path::PathBuf,
    sync::{Arc, Once},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use eframe::{epaint::text::TextWrapping, IconData, NativeOptions};
use egui_dock::{NodeIndex, Tree};
use egui_notify::Toast;
use flume::{Receiver, Sender};
use fs_err as fs;
use im::{vector, Vector};
use join_str::jstr;
use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
use picker::FilePickerState;
use rustc_hash::FxHashSet;
use uk_manager::{
    core::Manager,
    mods::{LookupMod, Mod},
    settings::Settings,
};
use uk_mod::{pack::sanitise, Manifest};
pub use uk_ui::visuals;
use uk_ui::{
    egui::{
        self, style::Margin, text::LayoutJob, Align, Align2, Color32, ComboBox, FontId, Frame, Id,
        Label, LayerId, Layout, RichText, Rounding, Spinner, TextFormat, TextStyle, Ui, Vec2,
    },
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};

use self::package::ModPackerBuilder;
use crate::{gui::modals::MetaInputModal, logger::Entry};

impl Entry {
    pub fn format(&self, job: &mut LayoutJob) {
        job.append(&jstr!("[{&self.timestamp}] "), 0., TextFormat {
            color: Color32::GRAY,
            font_id: FontId::monospace(10.),
            ..Default::default()
        });
        job.append(&jstr!("{&self.level} "), 0., TextFormat {
            color: match self.level.as_str() {
                "INFO" => visuals::GREEN,
                "WARN" => visuals::ORGANGE,
                "ERROR" => visuals::RED,
                "DEBUG" => visuals::BLUE,
                _ => visuals::YELLOW,
            },
            font_id: FontId::monospace(10.),
            ..Default::default()
        });
        job.append(&self.args, 1., TextFormat {
            color: Color32::WHITE,
            font_id: FontId::monospace(10.),
            ..Default::default()
        });
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
    Package,
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
            Sort::Category => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| {
                    a.meta.category.cmp(&b.meta.category)
                })
            }
            Sort::Version => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| {
                    a.meta
                        .version
                        .partial_cmp(&b.meta.version)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            }
            Sort::Priority => Box::new(|&(a, _), &(b, _)| a.cmp(&b)),
        }
    }
}

pub enum Message {
    AddMod(Mod),
    AddProfile,
    Apply,
    ChangeProfile(String),
    ChangeSort(Sort, bool),
    ClearDrag,
    ClearSelect,
    CloseAbout,
    CloseConfirm,
    CloseError,
    ClosePackagingOptions,
    ClosePackagingDependencies,
    CloseProfiles,
    Confirm(Box<Message>, String),
    DeleteProfile(String),
    Deploy,
    Deselect(usize),
    DuplicateProfile(String),
    Error(anyhow::Error),
    FilePickerBack,
    FilePickerSet(Option<PathBuf>),
    FilePickerUp,
    HandleMod(Mod),
    ImportCemu,
    InstallMod(Mod),
    Log(Entry),
    MoveSelected(usize),
    NewProfile,
    Noop,
    OpenMod(PathBuf),
    PackageMod(Arc<RwLock<ModPackerBuilder>>),
    RefreshModsDisplay,
    Remerge,
    RemoveMods(Vector<Mod>),
    RenameProfile(String, String),
    RequestMeta(PathBuf),
    RequestOptions(Mod, bool),
    ResetMods,
    ResetSettings,
    SaveSettings,
    SelectAlso(usize),
    SelectFile,
    SelectOnly(usize),
    SelectProfileManage(smartstring::alias::String),
    SetFocus(FocusedPane),
    ShowAbout,
    ShowPackagingOptions(FxHashSet<PathBuf>),
    ShowPackagingDependencies,
    StartDrag(usize),
    ToggleMods(Option<Vector<Mod>>, bool),
    UninstallMods(Option<Vector<Mod>>),
    UpdateOptions(Mod),
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
    profiles_state: profiles::ProfileManagerState,
    meta_input: modals::MetaInputModal,
    closed_tabs: im::HashMap<Tabs, NodeIndex>,
    tree: Arc<RwLock<Tree<Tabs>>>,
    focused: FocusedPane,
    logs: Vector<Entry>,
    log: LayoutJob,
    error: Option<anyhow::Error>,
    new_profile: Option<String>,
    confirm: Option<(Message, String)>,
    busy: bool,
    show_about: bool,
    show_package_deps: bool,
    opt_folders: Option<Mutex<FxHashSet<PathBuf>>>,
    dirty: Manifest,
    sort: (Sort, bool),
    options_mod: Option<(Mod, bool)>,
    temp_settings: Settings,
    toasts: egui_notify::Toasts,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        uk_ui::icons::load_icons();
        uk_ui::load_fonts(&cc.egui_ctx);
        uk_ui::visuals::default_dark(&cc.egui_ctx);
        let core = Arc::new(Manager::init().unwrap());
        let mods: Vector<_> = core.mod_manager().all_mods().collect();
        let (send, recv) = flume::unbounded();
        crate::logger::LOGGER.set_sender(send.clone());
        log::info!("Logger initialized");
        let temp_settings = core.settings().clone();
        let picker_state = fs::read_to_string(core.settings().state_file())
            .context("")
            .and_then(|s| serde_json::from_str(&s).context(""))
            .unwrap_or_default();
        Self {
            selected: mods.front().cloned().into_iter().collect(),
            drag_index: None,
            hover_index: None,
            picker_state,
            profiles_state: Default::default(),
            meta_input: MetaInputModal::new(send.clone()),
            channel: (send, recv),
            displayed_mods: mods.clone(),
            mods,
            temp_settings,
            core,
            logs: Vector::new(),
            log: LayoutJob::default(),
            closed_tabs: im::HashMap::new(),
            focused: FocusedPane::None,
            error: None,
            new_profile: None,
            confirm: None,
            show_about: false,
            show_package_deps: false,
            opt_folders: None,
            busy: false,
            dirty: Manifest::default(),
            sort: (Sort::Priority, false),
            options_mod: None,
            tree: Arc::new(RwLock::new(tabs::default_ui())),
            toasts: egui_notify::Toasts::new().with_anchor(egui_notify::Anchor::BottomRight),
        }
    }

    #[inline(always)]
    fn modal_open(&self) -> bool {
        self.error.is_some()
            || self.busy
            || self.options_mod.is_some()
            || self.confirm.is_some()
            || self.show_about
            || self.new_profile.is_some()
            || self.show_package_deps
            || self.opt_folders.is_some()
            || self.meta_input.is_open()
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

    fn handle_update(&mut self, ctx: &eframe::egui::Context) {
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Noop => self.busy = false,
                Message::Log(entry) => {
                    if !entry.args.starts_with("PROGRESS") {
                        entry.format(&mut self.log);
                        if self.logs.len() > 100 {
                            self.logs.pop_front();
                            for _ in 0..4 {
                                if !self.log.sections.is_empty() {
                                    self.log.sections.remove(0);
                                }
                            }
                        }
                    }
                    self.logs.push_back(entry);
                }
                Message::ResetMods => {
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
                Message::ShowAbout => self.show_about = true,
                Message::CloseAbout => self.show_about = false,
                Message::CloseProfiles => self.profiles_state.show = false,
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
                Message::ChangeProfile(profile) => {
                    match self.core.change_profile(profile) {
                        Ok(()) => self.do_update(Message::ResetMods),
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::NewProfile => self.new_profile = Some("".into()),
                Message::AddProfile => {
                    if let Some(profile) = self.new_profile.take() {
                        match self.core.change_profile(profile) {
                            Ok(()) => self.do_update(Message::ResetMods),
                            Err(e) => self.do_update(Message::Error(e)),
                        };
                    }
                }
                Message::DeleteProfile(profile) => {
                    self.do_task(move |core| {
                        let path = core.settings().profiles_dir().join(profile);
                        fs::remove_dir_all(path)?;
                        Ok(Message::Noop)
                    })
                }
                Message::DuplicateProfile(profile) => {
                    self.do_task(move |core| {
                        let profiles_dir = core.settings().profiles_dir();
                        dircpy::copy_dir(
                            profiles_dir.join(&profile),
                            profiles_dir.join(profile + "_copy"),
                        )?;
                        Ok(Message::Noop)
                    });
                }
                Message::RenameProfile(profile, rename) => {
                    self.do_task(move |core| {
                        let profiles_dir = core.settings().profiles_dir();
                        fs::rename(profiles_dir.join(&profile), profiles_dir.join(rename))?;
                        Ok(Message::Noop)
                    })
                }
                Message::SelectProfileManage(name) => {
                    self.profiles_state.selected = Some(profiles::SelectedProfile::load(
                        &self.core.settings().profiles_dir(),
                        name.as_str(),
                    ));
                }
                Message::SetFocus(pane) => {
                    self.focused = pane;
                }
                Message::SelectFile => {
                    let core = self.core.clone();
                    self.do_task(move |_| {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("UKMM Mod (*.zip)", &["zip"])
                            .add_filter("BCML Mod (*.bnp)", &["bnp"])
                            .add_filter("Legacy Mod (*.zip, *.7z)", &["zip", "7z"])
                            .pick_file()
                        {
                            tasks::open_mod(&core, &path, None)
                        } else {
                            Ok(Message::Noop)
                        }
                    });
                }
                Message::OpenMod(path) => {
                    let core = self.core.clone();
                    let meta = self.meta_input.take();
                    self.do_task(move |_| tasks::open_mod(&core, &path, meta));
                }
                Message::HandleMod(mod_) => {
                    self.busy = false;
                    log::debug!("{:#?}", &mod_);
                    for (hash, (name, version)) in mod_.meta.masters.iter() {
                        if !self.mods.iter().any(|m| m.hash() == *hash) {
                            self.do_update(Message::Error(anyhow::anyhow!(
                                "Could not find required mod dependency {} (version {})",
                                name,
                                version
                            )));
                        }
                    }
                    if !mod_.meta.options.is_empty() {
                        self.do_update(Message::RequestOptions(mod_, false));
                    } else {
                        self.do_update(Message::InstallMod(mod_));
                    }
                }
                Message::InstallMod(tmp_mod_) => {
                    self.do_task(move |core| {
                        let mods = core.mod_manager();
                        let mod_ = mods.add(&tmp_mod_.path)?;
                        let hash = mod_.as_hash_id();
                        if !tmp_mod_.enabled_options.is_empty() {
                            mods.set_enabled_options(hash, tmp_mod_.enabled_options)?;
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
                            manager.del(m.as_hash_id())?;
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
                    let dirty = std::mem::take(&mut self.dirty);
                    self.do_task(move |core| tasks::apply_changes(&core, mods, Some(dirty)));
                }
                Message::Deploy => {
                    self.do_task(move |core| {
                        log::info!("Deploying current mod configuration");
                        core.deploy_manager().deploy()?;
                        Ok(Message::ResetMods)
                    })
                }
                Message::Remerge => {
                    self.do_task(|core| tasks::apply_changes(&core, vector![], None));
                }
                Message::ResetSettings => {
                    self.busy = false;
                    self.temp_settings = self.core.settings().clone();
                    settings::CONFIG.write().clear();
                }
                Message::SaveSettings => {
                    match self.temp_settings.save().and_then(|_| {
                        self.core.reload()?;
                        Ok(())
                    }) {
                        Ok(()) => {
                            self.toasts.add({
                                let mut toast = Toast::success("Settings saved");
                                toast.set_duration(Some(Duration::new(2, 0)));
                                toast
                            });
                            ctx.data().remove_by_type::<Arc<RwLock<ModPackerBuilder>>>();
                            self.do_update(Message::ClearSelect);
                            self.do_update(Message::ResetMods);
                        }
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::RequestOptions(mut mod_, update) => {
                    if !update {
                        mod_.enable_default_options();
                    }
                    self.options_mod = Some((mod_, update));
                }
                Message::UpdateOptions(mod_) => {
                    let opts = mod_.enabled_options.clone();
                    match self
                        .core
                        .mod_manager()
                        .set_enabled_options(mod_.hash(), opts)
                    {
                        Ok(manifest) => {
                            self.dirty.extend(&manifest);
                            if let Some(old_mod) =
                                self.mods.iter_mut().find(|m| m.hash() == mod_.hash())
                            {
                                *old_mod = mod_.clone();
                            }
                            if let Some(old_mod) =
                                self.selected.iter_mut().find(|m| m.hash() == mod_.hash())
                            {
                                *old_mod = mod_;
                            }
                            self.do_update(Message::RefreshModsDisplay);
                        }
                        Err(e) => self.do_update(Message::Error(e)),
                    }
                }
                Message::Error(error) => {
                    log::error!("{:?}", &error);
                    self.busy = false;
                    self.error = Some(error);
                }
                Message::ShowPackagingOptions(folders) => {
                    self.opt_folders = Some(Mutex::new(folders));
                }
                Message::ShowPackagingDependencies => {
                    self.show_package_deps = true;
                }
                Message::ClosePackagingOptions => self.opt_folders = None,
                Message::ClosePackagingDependencies => self.show_package_deps = false,
                Message::PackageMod(builder) => {
                    let mut builder = ModPackerBuilder::clone(&builder.read());
                    let default_name = sanitise(&builder.meta.name) + ".zip";
                    if let Some(dest) = rfd::FileDialog::new()
                        .add_filter("UKMM Mod", &["zip"])
                        .set_title("Save Mod Package")
                        .set_file_name(&default_name)
                        .save_file()
                    {
                        builder.dest = dest;
                        self.do_task(move |core| tasks::package_mod(&core, builder));
                    }
                }
                Message::ImportCemu => {
                    let mut dialog = rfd::FileDialog::new()
                        .add_filter("Cemu executable", &["exe", "AppImage", "*"])
                        .set_title("Select Cemu Executable");
                    if cfg!(windows) {
                        dialog = dialog.set_file_name("Cemu.exe");
                    }
                    if let Some(path) = dialog.pick_file() {
                        self.do_task(move |core| tasks::import_cemu_settings(&core, &path));
                    }
                }
                Message::RequestMeta(path) => {
                    self.meta_input
                        .open(path, self.core.settings().current_mode);
                }
            }
            ctx.request_repaint();
        }
    }
}

static LAYOUT_FIX: Once = Once::new();

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_update(ctx);
        self.render_error(ctx);
        self.render_confirm(ctx);
        self.render_new_profile(ctx);
        self.render_about(ctx);
        self.render_menu(ctx, frame);
        self.render_option_picker(ctx);
        self.render_profiles_modal(ctx);
        self.meta_input.ui(ctx);
        let layer_id = LayerId::background();
        let max_rect = ctx.available_rect();
        let clip_rect = ctx.available_rect();
        let id = Id::new("egui_dock::DockArea");
        let mut ui = Ui::new(ctx.clone(), layer_id, id, max_rect, clip_rect);
        ui.spacing_mut().item_spacing = [8.0, 8.0].into();
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
                            .with_tab_text_color_focused(ui.style().visuals.strong_text_color())
                            .with_tab_text_color_unfocused(ui.style().visuals.weak_text_color())
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        fs::write(
            self.core.settings().state_file(),
            serde_json::to_string_pretty(&self.picker_state).unwrap(),
        )
        .unwrap_or(());
        uk_manager::util::clear_temp();
    }
}

pub fn main() {
    crate::logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    eframe::run_native(
        "U-King Mod Manager",
        NativeOptions {
            icon_data: Some(IconData {
                height: 256,
                width:  256,
                rgba:   image::load_from_memory(include_bytes!("../assets/ukmm.png"))
                    .unwrap()
                    .to_rgba8()
                    .into_vec(),
            }),
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
