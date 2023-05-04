mod info;
mod menus;
mod modals;
mod mods;
mod options;
pub(crate) mod package;
mod picker;
mod profiles;
mod settings;
mod tabs;
pub(crate) mod tasks;
mod util;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ops::DerefMut,
    path::PathBuf,
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow_ext::{Context, Result};
use eframe::{egui::InnerResponse, epaint::text::TextWrapping, IconData, NativeOptions};
use egui_notify::Toast;
use flume::{Receiver, Sender};
use fs_err as fs;
use join_str::jstr;
use parking_lot::{Mutex, RwLock};
use picker::FilePickerState;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use uk_content::util::HashMap;
use uk_manager::{
    core::Manager,
    mods::{LookupMod, Mod},
    settings::{Platform, Settings},
};
use uk_mod::{pack::sanitise, Manifest, Meta, ModPlatform};
pub use uk_ui::visuals;
use uk_ui::{
    egui::{
        self, style::Margin, text::LayoutJob, Align, Align2, Color32, ComboBox, FontId, Frame, Id,
        Label, LayerId, Layout, RichText, Spinner, TextFormat, TextStyle, Ui, Vec2,
    },
    egui_dock::{DockArea, NodeIndex, Tree},
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};

use self::{package::ModPackerBuilder, tasks::VersionResponse};
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

pub trait Component {
    type Message;
    fn show(&self, ui: &mut Ui) -> InnerResponse<Option<Self::Message>>;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug)]
pub enum Message {
    AddMod(Mod),
    AddProfile,
    Apply,
    ChangeProfile(String),
    ChangeSort(Sort, bool),
    CheckMeta,
    ClearDrag,
    ClearSelect,
    CloseAbout,
    CloseConfirm,
    CloseError,
    CloseChangelog,
    ClosePackagingOptions,
    ClosePackagingDependencies,
    CloseProfiles,
    Confirm(Box<Message>, String),
    DeleteProfile(String),
    Deploy,
    Deselect(usize),
    DoUpdate,
    DuplicateProfile(String),
    Error(anyhow_ext::Error),
    FilePickerBack,
    FilePickerSet(Option<PathBuf>),
    FilePickerUp,
    GetPackagingOptions,
    HandleMod(Mod),
    HandleSettings,
    ImportCemu,
    InstallMod(Mod),
    Log(Entry),
    MigrateBcml,
    MoveSelected(usize),
    NewProfile,
    Noop,
    OfferUpdate(VersionResponse),
    OpenMod(PathBuf),
    PackageMod,
    RefreshModsDisplay,
    Remerge,
    ReloadProfiles,
    RemoveMods(Vec<Mod>),
    RenameProfile(String, String),
    RequestMeta(PathBuf),
    RequestOptions(Mod, bool),
    ResetMods,
    ResetPacker,
    ResetPending,
    ResetSettings,
    Restart,
    SaveSettings,
    SelectAlso(usize),
    SelectFile,
    SelectOnly(usize),
    SelectThrough(usize),
    SelectProfileManage(smartstring::alias::String),
    SetChangelog(String),
    SetFocus(FocusedPane),
    SetTheme(uk_ui::visuals::Theme),
    ShowAbout,
    ShowPackagingOptions(FxHashSet<PathBuf>),
    ShowPackagingDependencies,
    StartDrag(usize),
    Toast(String),
    ToggleMods(Option<Vec<Mod>>, bool),
    UpdatePackageMeta(Meta),
    UninstallMods(Option<Vec<Mod>>),
    UpdateOptions(Mod),
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct UiState {
    theme: uk_ui::visuals::Theme,
    picker_state: FilePickerState,
    #[serde(default = "tabs::default_ui")]
    tree: Tree<Tabs>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: uk_ui::visuals::Theme::Sheikah,
            picker_state: FilePickerState::default(),
            tree: tabs::default_ui(),
        }
    }
}

pub struct App {
    core: Arc<Manager>,
    channel: (Sender<Message>, Receiver<Message>),
    mods: Vec<Mod>,
    displayed_mods: Vec<Mod>,
    selected: Vec<Mod>,
    install_queue: VecDeque<PathBuf>,
    error_queue: VecDeque<anyhow_ext::Error>,
    drag_index: Option<usize>,
    hover_index: Option<usize>,
    picker_state: FilePickerState,
    profiles_state: RefCell<profiles::ProfileManagerState>,
    meta_input: modals::MetaInputModal,
    closed_tabs: HashMap<Tabs, NodeIndex>,
    tree: Arc<RwLock<Tree<Tabs>>>,
    focused: FocusedPane,
    logs: Vec<Entry>,
    log: LayoutJob,
    error: Option<anyhow_ext::Error>,
    new_profile: Option<String>,
    confirm: Option<(Message, String)>,
    busy: Cell<bool>,
    show_about: bool,
    package_builder: RefCell<ModPackerBuilder>,
    show_package_deps: bool,
    opt_folders: Option<Mutex<FxHashSet<PathBuf>>>,
    dirty: Manifest,
    sort: (Sort, bool),
    options_mod: Option<(Mod, bool)>,
    temp_settings: Settings,
    toasts: egui_notify::Toasts,
    theme: uk_ui::visuals::Theme,
    dock_style: uk_ui::egui_dock::Style,
    changelog: Option<String>,
    new_version: Option<VersionResponse>,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        if option_env!("UPDATE_PLATFORM").unwrap_or_default() == "steamdeck" {
            cc.egui_ctx.set_pixels_per_point(
                option_env!("WINIT_X11_SCALE_FACTOR")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0),
            );
        }
        uk_ui::icons::load_icons();
        uk_ui::load_fonts(&cc.egui_ctx);
        let core = Arc::new(Manager::init().unwrap());
        let ui_state: UiState = fs::read_to_string(core.settings().state_file())
            .context("")
            .and_then(|s| serde_json::from_str(&s).context(""))
            .unwrap_or_default();
        ui_state.theme.set_theme(&cc.egui_ctx);
        let mods: Vec<_> = core.mod_manager().all_mods().collect();
        let (send, recv) = flume::unbounded();
        crate::logger::LOGGER.set_sender(send.clone());
        crate::logger::LOGGER.set_file(Settings::config_dir().join("log.txt"));
        log::info!("Logger initialized");
        let temp_settings = core.settings().clone();
        let platform = core.settings().current_mode;
        Self {
            selected: mods.first().cloned().into_iter().collect(),
            drag_index: None,
            hover_index: None,
            package_builder: RefCell::new(ModPackerBuilder::new(platform)),
            picker_state: ui_state.picker_state,
            profiles_state: RefCell::new(profiles::ProfileManagerState::new(&core)),
            meta_input: MetaInputModal::new(send.clone()),
            displayed_mods: mods.clone(),
            mods,
            temp_settings,
            changelog: {
                let last_version = core
                    .settings()
                    .last_version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "0.0.0".into());
                if last_version == "0.0.0" {
                    Some(include_str!("../assets/intro.md").into())
                } else {
                    tasks::get_releases(core.clone(), send.clone());
                    None
                }
            },
            channel: (send, recv),
            core,
            logs: Vec::new(),
            log: LayoutJob::default(),
            closed_tabs: Default::default(),
            focused: FocusedPane::None,
            error: None,
            new_profile: None,
            confirm: None,
            show_about: false,
            show_package_deps: false,
            opt_folders: None,
            busy: Cell::new(false),
            dirty: Manifest::default(),
            sort: (Sort::Priority, false),
            options_mod: None,
            tree: Arc::new(RwLock::new(ui_state.tree)),
            toasts: egui_notify::Toasts::new().with_anchor(egui_notify::Anchor::BottomRight),
            theme: ui_state.theme,
            dock_style: uk_ui::visuals::style_dock(&cc.egui_ctx.style()),
            install_queue: Default::default(),
            error_queue: Default::default(),
            new_version: None,
        }
    }

    #[inline(always)]
    fn platform(&self) -> Platform {
        self.core.settings().current_mode
    }

    #[inline(always)]
    fn modal_open(&self) -> bool {
        self.error.is_some()
            || self.busy.get()
            || self.options_mod.is_some()
            || self.confirm.is_some()
            || self.show_about
            || self.new_profile.is_some()
            || self.show_package_deps
            || self.opt_folders.is_some()
            || self.meta_input.is_open()
            || self.changelog.is_some()
    }

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
            let response = match std::panic::catch_unwind(|| task(core.clone())) {
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
            };
            if let Some(d) = core.settings().dump() {
                d.clear_cache()
            }
            sender.send(response).unwrap();
        });
    }

    fn handle_drops(&mut self, ctx: &eframe::egui::Context) {
        let files = &ctx.input().raw.dropped_files;
        if !(self.modal_open() || files.is_empty()) {
            let first = files.first().and_then(|f| f.path.clone()).unwrap();
            self.install_queue
                .extend(files.iter().skip(1).filter_map(|f| f.path.clone()));
            self.error_queue.clear();
            self.do_task(move |core| tasks::open_mod(&core, &first, None));
        }
    }

    fn handle_update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Noop => self.busy.set(false),
                Message::Log(entry) => {
                    if !entry.args.starts_with("PROGRESS") {
                        entry.format(&mut self.log);
                        if self.logs.len() > 100 {
                            self.logs.remove(0);
                            for _ in 0..4 {
                                if !self.log.sections.is_empty() {
                                    self.log.sections.remove(0);
                                }
                            }
                        }
                    }
                    self.logs.push(entry);
                }
                Message::ResetMods => {
                    self.busy.set(false);
                    self.dirty.clear();
                    self.mods = self.core.mod_manager().all_mods().collect();
                    self.do_update(Message::RefreshModsDisplay);
                    self.do_update(Message::ReloadProfiles);
                }
                Message::RefreshModsDisplay => {
                    self.do_update(Message::ChangeSort(self.sort.0, self.sort.1));
                }
                Message::ChangeSort(sort, rev) => {
                    let orderer = sort.orderer();
                    let mut temp = self.mods.iter().cloned().enumerate().collect::<Vec<_>>();
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
                Message::CloseProfiles => self.profiles_state.borrow_mut().show = false,
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
                        self.selected.push(self.mods[index].clone());
                    }
                    self.drag_index = None;
                }
                Message::SelectAlso(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    let mod_ = &self.mods[index];
                    if !self.selected.contains(mod_) {
                        self.selected.push(mod_.clone());
                    }
                    self.drag_index = None;
                }
                Message::SelectThrough(i) => {
                    let index = i.clamp(0, self.mods.len() - 1);
                    if let Some(start_index) = self
                        .selected
                        .first()
                        .and_then(|sm| self.mods.iter().position(|m| m == sm))
                    {
                        let range = if start_index < index {
                            start_index..=index
                        } else {
                            index..=start_index
                        };
                        self.selected = self
                            .mods
                            .iter()
                            .enumerate()
                            .filter_map(|(i, m)| range.contains(&i).then(|| m.clone()))
                            .collect();
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
                        self.selected.push(mod_.clone());
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
                        Ok(Message::ReloadProfiles)
                    })
                }
                Message::DuplicateProfile(profile) => {
                    self.do_task(move |core| {
                        let profiles_dir = core.settings().profiles_dir();
                        dircpy::copy_dir(
                            profiles_dir.join(&profile),
                            profiles_dir.join(profile + "_copy"),
                        )?;
                        Ok(Message::ReloadProfiles)
                    });
                }
                Message::RenameProfile(profile, rename) => {
                    self.do_task(move |core| {
                        let profiles_dir = core.settings().profiles_dir();
                        fs::rename(profiles_dir.join(&profile), profiles_dir.join(rename))?;
                        Ok(Message::ReloadProfiles)
                    })
                }
                Message::ReloadProfiles => {
                    self.profiles_state.borrow_mut().reload(&self.core);
                    self.busy.set(false);
                }
                Message::SelectProfileManage(name) => {
                    self.profiles_state.borrow_mut().selected = Some(name);
                }
                Message::SetFocus(pane) => {
                    self.focused = pane;
                }
                Message::SetTheme(theme) => {
                    theme.set_theme(ctx);
                    self.theme = theme;
                    self.dock_style = uk_ui::visuals::style_dock(&ctx.style());
                }
                Message::SelectFile => {
                    if let Some(mut paths) = rfd::FileDialog::new()
                            .add_filter("Any mod (*.zip, *.7z, *.bnp)", &["zip", "bnp", "7z"])
                            .add_filter("UKMM Mod (*.zip)", &["zip"])
                            .add_filter("BCML Mod (*.bnp)", &["bnp"])
                            .add_filter("Legacy Mod (*.zip, *.7z)", &["zip", "7z"])
                            .add_filter("All files (*.*)", &["*"])
                            .pick_files() && !paths.is_empty()
                    {
                        let first = paths.remove(0);
                        self.install_queue.extend(paths);
                        self.error_queue.clear();
                        self.do_task(move |core| tasks::open_mod(&core, &first, None));
                    }
                }
                Message::OpenMod(path) => {
                    let core = self.core.clone();
                    let meta = self.meta_input.take();
                    self.do_task(move |_| tasks::open_mod(&core, &path, meta));
                }
                Message::HandleMod(mod_) => {
                    self.busy.set(false);
                    log::debug!("{:#?}", &mod_);
                    for (hash, (name, version)) in mod_.meta.masters.iter() {
                        if !self.mods.iter().any(|m| m.hash() == *hash) {
                            self.do_update(Message::Error(anyhow_ext::anyhow!(
                                "Could not find required mod dependency {} (version {})",
                                name,
                                version
                            )));
                        }
                    }
                    if let ModPlatform::Specific(platform) = mod_.meta.platform
                        && Platform::from(platform) != self.platform()
                    {
                        self.do_update(Message::Error(anyhow_ext::anyhow!(
                            "Mod is for {}, current mode is {}",
                            platform,
                            self.platform()
                        )));
                    } else if !mod_.meta.options.is_empty() {
                        self.do_update(Message::RequestOptions(mod_, false));
                    } else {
                        self.do_update(Message::InstallMod(mod_));
                    }
                }
                Message::InstallMod(tmp_mod_) => {
                    self.do_task(move |core| {
                        let mods = core.mod_manager();
                        let mod_ = mods.add(&tmp_mod_.path, None)?;
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
                            manager.del(m.as_hash_id(), None)?;
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
                    self.mods = self.core.mod_manager().all_mods().collect();
                    self.do_update(Message::RefreshModsDisplay);
                    self.busy.set(false);
                    if let Some(path) = self.install_queue.pop_front() {
                        self.do_task(move |core| tasks::open_mod(&core, &path, None));
                    } else if !self.error_queue.is_empty() {
                        let msg = self
                            .error_queue
                            .drain(..)
                            .map(|e| format!("{e:?}\n"))
                            .collect::<String>();
                        self.do_update(Message::Error(
                            anyhow_ext::anyhow!("{msg}")
                                .context("One or more errors occured while installing your mods. Please see full details."),
                        ));
                    }
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
                    self.busy.set(false);
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
                Message::ResetPending => {
                    self.do_task(|core| {
                        log::info!("Resetting pending deployment data");
                        core.deploy_manager().reset_pending()?;
                        Ok(Message::Noop)
                    })
                }
                Message::Remerge => {
                    self.do_task(|core| tasks::apply_changes(&core, vec![], None));
                }
                Message::ResetSettings => {
                    self.busy.set(false);
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
                            if let Some(dump) = self.core.settings().dump() { dump.clear_cache() }
                            self.package_builder.borrow_mut().reset(self.platform());
                            self.do_update(Message::ClearSelect);
                            self.do_update(Message::ResetMods);
                        }
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::HandleSettings => {
                    self.toasts.add({
                        let mut toast = Toast::success("Settings saved");
                        toast.set_duration(Some(Duration::new(2, 0)));
                        toast
                    });
                    if let Some(dump) = self.core.settings().dump() { dump.clear_cache() }
                    self.package_builder.borrow_mut().reset(self.platform());
                    self.do_update(Message::ClearSelect);
                    self.do_update(Message::ResetMods);
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
                    if self.install_queue.is_empty() {
                        self.busy.set(false);
                        self.error = Some(error);
                    } else {
                        log::warn!("More operations in queue, stashing error and continuing…");
                        self.error_queue.push_back(error);
                        if let Some(path) = self.install_queue.pop_front() {
                            self.do_task(move |core| tasks::open_mod(&core, &path, None));
                        }
                    }
                }
                #[allow(irrefutable_let_patterns)]
                Message::CheckMeta => {
                    let source = &self.package_builder.borrow().source;
                    for file in ["info.json", "rules.txt", "meta.yml"] {
                        if let file = source.join(file) && file.exists() {
                            self.do_task(move |_| tasks::parse_meta(file));
                            break;
                        }
                    }
                }
                Message::GetPackagingOptions => {
                    let folder = self.package_builder.borrow().source.join("options");
                    if let Ok(reader) = fs::read_dir(folder) {
                        let files = reader
                            .filter_map(|res| {
                                res.ok().and_then(|e| {
                                    e.file_type()
                                        .ok()
                                        .and_then(|t| t.is_dir().then(|| e.path()))
                                })
                            })
                            .collect();
                        self.do_update(Message::ShowPackagingOptions(files));
                    }
                }
                Message::ShowPackagingOptions(folders) => {
                    self.opt_folders = Some(Mutex::new(folders));
                }
                Message::ShowPackagingDependencies => {
                    self.show_package_deps = true;
                }
                Message::ClosePackagingOptions => self.opt_folders = None,
                Message::ClosePackagingDependencies => self.show_package_deps = false,
                Message::PackageMod => {
                    let mut builder = self.package_builder.borrow().clone();
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
                Message::ResetPacker => {
                    self.package_builder.borrow_mut().reset(self.platform());
                    self.busy.set(false);
                }
                Message::ImportCemu => {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Cemu Directory").pick_folder()
                    {
                        self.do_task(move |core| tasks::import_cemu_settings(&core, &path));
                    }
                }
                Message::MigrateBcml => {
                    self.do_task(tasks::migrate_bcml);
                }
                Message::RequestMeta(path) => {
                    self.meta_input
                        .open(path, self.platform());
                }
                Message::SetChangelog(msg) => self.changelog = Some(msg),
                Message::CloseChangelog => self.changelog = None,
                Message::OfferUpdate(version) => {
                    self.changelog = Some(format!("A new update is available!\n\n{}", version.description()));
                    self.new_version = Some(version)                    ;
                }
                Message::DoUpdate => {
                    let version = self.new_version.take().unwrap();
                    self.changelog = None;
                    self.do_task(move |_| {
                        tasks::do_update(version)
                    });
                }
                Message::Restart => {
                    let mut exe = std::env::current_exe().unwrap();
                    if exe.extension().and_then(|x| x.to_str()).contains(&"bak") {
                        exe.set_extension("");
                    }
                    let mut command = std::process::Command::new(exe);
                    #[cfg(unix)]
                    {
                        std::os::unix::process::CommandExt::process_group(&mut command, 0);
                    }
                    command.spawn().unwrap();
                    frame.close();
                }
                Message::Toast(msg) => {
                    self.toasts.add({
                        let mut toast = Toast::info(msg);
                        toast.set_duration(Some(Duration::new(2, 0)));
                        toast
                    });
                }
                Message::UpdatePackageMeta(meta) => {
                    self.package_builder.borrow_mut().meta = meta;
                    self.busy.set(false);
                },
            }
        } else {
            self.handle_drops(ctx);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_update(ctx, frame);
        self.render_menu(ctx, frame);
        self.render_error(ctx);
        self.render_confirm(ctx);
        self.render_new_profile(ctx);
        self.render_about(ctx);
        self.render_option_picker(ctx);
        self.profiles_state.borrow_mut().render(self, ctx);
        self.render_changelog(ctx);
        self.meta_input.ui(ctx);
        let layer_id = LayerId::background();
        let max_rect = ctx.available_rect();
        let clip_rect = ctx.available_rect();
        let id = Id::new("egui_dock::DockArea");
        let mut ui = Ui::new(ctx.clone(), layer_id, id, max_rect, clip_rect);
        ui.spacing_mut().item_spacing = [8.0, 8.0].into();
        DockArea::new(self.tree.clone().write().deref_mut())
            .style(self.dock_style.clone())
            .show_inside(&mut ui, self);
        self.render_busy(ctx, frame);
        self.toasts.show(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::logger::LOGGER.save_log();
        self.core.settings_mut().last_version = Some(env!("CARGO_PKG_VERSION").into());
        self.core.settings().save().unwrap_or(());
        let ui_state = UiState {
            theme: self.theme,
            picker_state: std::mem::take(&mut self.picker_state),
            tree: std::mem::take(&mut self.tree.write()),
        };
        fs::write(
            self.core.settings().state_file(),
            serde_json::to_string_pretty(&ui_state).unwrap(),
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
            min_window_size: Some(egui::Vec2::new(850.0, 500.0)),
            initial_window_size: Some(egui::Vec2::new(1200.0, 800.0)),
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
