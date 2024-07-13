mod deploy;
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
    fmt::Write,
    ops::DerefMut,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow_ext::{Context, Result};
use eframe::{
    egui::{IconData, InnerResponse},
    epaint::text::TextWrapping,
    NativeOptions,
};
use egui_notify::Toast;
use flume::{Receiver, Sender};
use fs_err as fs;
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
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
        self, epaint::Margin, text::LayoutJob, Align, Align2, ComboBox, Frame, Id, Label, LayerId,
        Layout, RichText, Spinner, TextStyle, Ui, Vec2, ViewportBuilder,
    },
    egui_dock::{DockArea, DockState, NodeIndex},
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};
use uk_util::OptionResultExt;

use self::{package::ModPackerBuilder, tasks::VersionResponse};
use crate::gui::modals::MetaInputModal;

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
    AddToProfile(smartstring::alias::String),
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
    Extract,
    FilePickerBack,
    FilePickerSet(Option<PathBuf>),
    FilePickerUp,
    GetPackagingOptions,
    HandleMod(Mod),
    HandleSettings,
    ImportCemu,
    InstallMod(Mod),
    MigrateBcml,
    ModUpdate,
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
    ResetMods(Option<Manifest>),
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
    SetDownloading(String),
    SetFocus(FocusedPane),
    SetTheme(uk_ui::visuals::Theme),
    ShowAbout,
    ShowPackagingOptions(FxHashSet<PathBuf>),
    ShowPackagingDependencies,
    StartDrag(usize),
    Toast(String),
    ToggleMods(Option<Vec<Mod>>, bool),
    DevUpdate,
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
    tree: DockState<Tabs>,
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
    update_mod: Option<Mod>,
    error_queue: VecDeque<anyhow_ext::Error>,
    drag_index: Option<usize>,
    hover_index: Option<usize>,
    picker_state: FilePickerState,
    profiles_state: RefCell<profiles::ProfileManagerState>,
    meta_input: modals::MetaInputModal,
    closed_tabs: HashMap<Tabs, NodeIndex>,
    tree: Rc<RefCell<DockState<Tabs>>>,
    focused: FocusedPane,
    error: Option<anyhow_ext::Error>,
    new_profile: Option<String>,
    confirm: Option<(Message, String)>,
    busy: Cell<bool>,
    show_about: bool,
    package_builder: RefCell<ModPackerBuilder>,
    show_package_deps: bool,
    opt_folders: Option<Mutex<FxHashSet<PathBuf>>>,
    dirty: RwLock<HashMap<String, Manifest>>,
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
                std::env::var("WINIT_X11_SCALE_FACTOR")
                    .ok()
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
        tasks::ONECLICK_SENDER.set(send.clone()).unwrap_or(());
        crate::logger::LOGGER.set_file(Settings::config_dir().join("log.txt"));
        log::info!("日志记录器已初始化");
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
            closed_tabs: Default::default(),
            focused: FocusedPane::None,
            error: None,
            new_profile: None,
            confirm: None,
            show_about: false,
            show_package_deps: false,
            opt_folders: None,
            busy: Cell::new(false),
            dirty: {
                let settings = core.settings();
                RwLock::new(
                    settings
                        .profiles()
                        .map(|p| (p.into(), Default::default()))
                        .collect(),
                )
            },
            sort: (Sort::Priority, false),
            options_mod: None,
            tree: Rc::new(RefCell::new(ui_state.tree)),
            toasts: egui_notify::Toasts::new().with_anchor(egui_notify::Anchor::BottomRight),
            theme: ui_state.theme,
            dock_style: uk_ui::visuals::style_dock(&cc.egui_ctx.style()),
            install_queue: Default::default(),
            update_mod: Default::default(),
            error_queue: Default::default(),
            new_version: None,
            core,
        }
    }

    #[inline(always)]
    fn platform(&self) -> Platform {
        self.core.settings().current_mode
    }

    #[inline(always)]
    fn dirty(&self) -> MappedRwLockReadGuard<'_, uk_mod::Manifest> {
        let dirty = self.dirty.read();
        RwLockReadGuard::map(dirty, |dirty| {
            dirty
                .get(self.core.mod_manager().profile().key().as_str())
                .or_else(|| dirty.values().next())
                .unwrap()
        })
    }

    #[inline(always)]
    fn dirty_mut(&self) -> MappedRwLockWriteGuard<'_, uk_mod::Manifest> {
        let dirty = self.dirty.write();
        RwLockWriteGuard::map(dirty, |dirty| {
            dirty.get_mut(self.core.mod_manager().profile().key().as_str())
            .map(|m| unsafe { &mut *(m as *mut _) }) // Classic Polonius situation
            .or_else(|| dirty.values_mut().next())
            .unwrap()
        })
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
                                "发生未知错误，请检查日志获取可能的详细信息。"
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
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        if !(self.modal_open() || files.is_empty()) {
            let first = files.first().and_then(|f| f.path.clone()).unwrap();
            self.install_queue
                .extend(files.iter().skip(1).filter_map(|f| f.path.clone()));
            self.error_queue.clear();
            self.do_task(move |core| tasks::open_mod(&core, &first, None));
        }
    }

    fn handle_update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(msg) = self.channel.1.try_recv() {
            match msg {
                Message::Noop => self.busy.set(false),
                Message::ResetMods(dirty) => {
                    self.busy.set(false);
                    self.dirty_mut().clear();
                    if let Some(dirty) = dirty {
                        self.dirty_mut().extend(&dirty);
                    }
                    self.mods = self.core.mod_manager().all_mods().collect();
                    self.selected.retain(|m| self.mods.contains(m));
                    self.do_update(Message::RefreshModsDisplay);
                    self.do_update(Message::ReloadProfiles);
                    ctx.data_mut(|d| {
                        d.remove::<Arc<Mutex<egui_commonmark::CommonMarkCache>>>(egui::Id::new(
                            "md_cache",
                        ))
                    });
                    info::ROOTS.write().clear();
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
                            .filter(|&(i, _m)| range.contains(&i))
                            .map(|(_i, m)| m.clone())
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
                    if ctx.input(|i| i.pointer.any_released()) {
                        self.drag_index = None;
                    }
                    self.drag_index = Some(i);
                    let mod_ = &self.mods[i];
                    if !self.selected.contains(mod_) {
                        if !ctx.input(|i| i.modifiers.ctrl) {
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
                        self.dirty_mut()
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
                        Ok(()) => {
                            self.mods = self.core.mod_manager().all_mods().collect();
                            self.do_update(Message::RefreshModsDisplay);
                            self.do_update(Message::ReloadProfiles);
                        }
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::NewProfile => self.new_profile = Some("".into()),
                Message::AddProfile => {
                    if let Some(profile) = self.new_profile.take() {
                        match self.core.change_profile(profile) {
                            Ok(()) => self.do_update(Message::ResetMods(None)),
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
                Message::SetDownloading(mod_name) => {
                    ctx.request_repaint();
                    self.busy.set(true);
                    log::info!("从GameBanana下载…{mod_name} ");
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
                        .set_title("选择一个Mod")
                        .add_filter("任何 Mod（*.zip, *.7z, *.bnp)", &["zip", "bnp", "7z"])
                        .add_filter("UKMM Mod (*.zip)", &["zip"])
                        .add_filter("BCML Mod (*.bnp)", &["bnp"])
                        .add_filter("Legacy Mod (*.zip, *.7z)", &["zip", "7z"])
                        .add_filter("所有文件 (*.*)", &["*"])
                        .pick_files()
                        .filter(|p| !p.is_empty())
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
                    ctx.request_repaint();
                    println!("Opening mod at {}", path.display());
                    self.do_task(move |_| tasks::open_mod(&core, &path, meta));
                }
                Message::HandleMod(mod_) => {
                    self.busy.set(false);
                    log::debug!("{:#?}", &mod_);
                    for (hash, (name, version)) in mod_.meta.masters.iter() {
                        if !self.mods.iter().any(|m| m.hash() == *hash) {
                            self.do_update(Message::Error(anyhow_ext::anyhow!(
                                "未找到所需的 Mod 依赖  {} (version {})",
                                name,
                                version
                            )));
                        }
                    }
                    if !matches!(mod_.meta.platform, ModPlatform::Universal)
                        && mod_.meta.platform != ModPlatform::Specific(self.platform().into())
                    {
                        self.do_update(Message::Error(anyhow_ext::anyhow!(
                            "Mod 适用于 {:?}, 当前模式是 {}",
                            mod_.meta.platform,
                            self.platform()
                        )));
                    } else if !mod_.meta.options.is_empty() {
                        self.do_update(Message::RequestOptions(mod_, false));
                    } else {
                        self.do_update(Message::InstallMod(mod_));
                    }
                }
                Message::InstallMod(tmp_mod_) => {
                    let update_mod = self.update_mod.take();
                    self.do_task(move |core| {
                        let mods = core.mod_manager();
                        if let Some(mod_) = update_mod {
                            let mut dirty = Manifest::default();
                            dirty.extend(&tmp_mod_.manifest().unwrap_or_default());
                            mods.replace(tmp_mod_, mod_.hash())?;
                            log::info!("Updated {}", mod_.meta.name);
                            dirty.extend(&mod_.manifest().unwrap_or_default());
                            Ok(Message::ResetMods(Some(dirty)))
                        } else {
                            let mod_ = mods.add(&tmp_mod_.path, None)?;
                            let hash = mod_.as_map_id();
                            if !tmp_mod_.enabled_options.is_empty() {
                                mods.set_enabled_options(hash, tmp_mod_.enabled_options)?;
                            }
                            mods.save()?;
                            log::info!("将 Mod {} 添加到当前配置文件", mod_.meta.name.as_str());
                            let mod_ = unsafe { mods.get_mod(hash).unwrap_unchecked() };
                            Ok(Message::AddMod(mod_))
                        }
                    });
                }
                Message::UninstallMods(mods) => {
                    let mods = mods.unwrap_or_else(|| self.selected.clone());
                    self.do_task(move |core| {
                        let manager = core.mod_manager();
                        mods.iter().try_for_each(|m| -> Result<()> {
                            manager.del(m.as_map_id(), None)?;
                            log::info!("从当前配置文件中移除 Mod {}", m.meta.name.as_str());
                            Ok(())
                        })?;
                        manager.save()?;
                        Ok(Message::RemoveMods(mods))
                    });
                }
                Message::ModUpdate => {
                    if let Some(file) = rfd::FileDialog::new()
                        .set_title("选择一个 Mod")
                        .add_filter("Any mod (*.zip, *.7z, *.bnp)", &["zip", "bnp", "7z"])
                        .add_filter("UKMM Mod (*.zip)", &["zip"])
                        .add_filter("BCML Mod (*.bnp)", &["bnp"])
                        .add_filter("Legacy Mod (*.zip, *.7z)", &["zip", "7z"])
                        .add_filter("All files (*.*)", &["*"])
                        .pick_file()
                    {
                        let path = file.clone();
                        self.update_mod = Some(self.selected.first().unwrap().clone());
                        self.do_task(move |core| tasks::open_mod(&core, &path, None));
                    }
                }
                Message::DevUpdate => {
                    let mods = self.selected.clone();
                    self.do_task(move |core| tasks::dev_update_mods(&core, mods));
                }
                Message::ToggleMods(mods, enabled) => {
                    let mods = mods.as_ref().unwrap_or(&self.selected);
                    let dirty = mods.iter().try_fold(
                        Manifest::default(),
                        |mut dirty, m| -> Result<Manifest> {
                            let mod_ = unsafe {
                                self.mods.iter_mut().find(|m2| m.eq(m2)).unwrap_unchecked()
                            };
                            mod_.enabled = enabled;
                            dirty.extend(m.manifest()?.as_ref());
                            Ok(dirty)
                        },
                    );
                    match dirty {
                        Ok(dirty) => {
                            self.dirty_mut().extend(&dirty);
                            self.do_update(Message::RefreshModsDisplay)
                        }
                        Err(e) => self.do_update(Message::Error(e)),
                    };
                }
                Message::AddMod(mod_) => {
                    if let Ok(manifest) = mod_.manifest() {
                        self.dirty_mut().extend(&manifest);
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
                            .fold(String::new(), |mut acc, e| {
                                writeln!(acc, "{:?}", e).expect("写入字符串失败");
                                acc
                            });
                        self.do_update(Message::Error(anyhow_ext::anyhow!("{msg}").context(
                            "安装 Mod 时发生一个或多个错误。请查看完整的详细信息。.",
                        )));
                    }
                }
                Message::Extract => {
                    let mods = self.selected.clone();
                    self.do_task(move |core| tasks::extract_mods(&core, mods));
                }
                Message::AddToProfile(profile) => {
                    let mut dirty = self.dirty.write();
                    let dirty = dirty.entry(profile.as_str().into()).or_default();
                    let mut err = false;
                    for mod_ in &self.selected {
                        match self.core.mod_manager().add(&mod_.path, Some(&profile)) {
                            Ok(_) => {
                                if let Ok(manifest) = mod_.manifest() {
                                    dirty.extend(&manifest);
                                }
                            }
                            Err(e) => {
                                self.do_update(Message::Error(e));
                                err = true;
                                break;
                            }
                        };
                    }
                    if !err {
                        self.toasts.add({
                            let mut toast =
                                Toast::success(format!("Mod(s) 已添加到配置文件 {}", profile));
                            toast.set_duration(Some(Duration::new(2, 0)));
                            toast
                        });
                    }
                }
                Message::RemoveMods(mods) => {
                    self.mods.retain(|m| !mods.contains(m));
                    self.selected.retain(|m| !mods.contains(m));
                    mods.iter().for_each(|m| {
                        if let Ok(manifest) = m.manifest() {
                            self.dirty_mut().extend(&manifest);
                        }
                    });
                    self.do_update(Message::RefreshModsDisplay);
                    self.busy.set(false);
                }
                Message::Apply => {
                    let mods = self.mods.clone();
                    let dirty = std::mem::take(self.dirty_mut().deref_mut());
                    self.do_task(move |core| tasks::apply_changes(&core, mods, Some(dirty)));
                }
                Message::Deploy => {
                    self.do_task(move |core| {
                        log::info!("部署当前的 Mod 配置");
                        core.deploy_manager().deploy()?;
                        Ok(Message::ResetMods(None))
                    })
                }
                Message::ResetPending => {
                    self.do_task(|core| {
                        log::info!("重置待处理的部署数据");
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
                    let save_res = self.temp_settings.save().and_then(|_| {
                        self.core.reload()?;
                        Ok(())
                    });
                    match save_res {
                        Ok(()) => {
                            self.toasts.add({
                                let mut toast = Toast::success("设置已保存");
                                toast.set_duration(Some(Duration::new(2, 0)));
                                toast
                            });
                            if let Some(dump) = self.core.settings().dump() {
                                dump.clear_cache()
                            }
                            self.package_builder.borrow_mut().reset(self.platform());
                            self.do_update(Message::ClearSelect);
                            self.do_update(Message::ResetMods(None));
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
                    if let Some(dump) = self.core.settings().dump() {
                        dump.clear_cache()
                    }
                    self.package_builder.borrow_mut().reset(self.platform());
                    self.do_update(Message::ClearSelect);
                    self.do_update(Message::ResetMods(None));
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
                            self.dirty_mut().extend(&manifest);
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
                        log::warn!("队列中有更多操作，正在暂存错误并继续…");
                        self.error_queue.push_back(error);
                        if let Some(path) = self.install_queue.pop_front() {
                            self.do_task(move |core| tasks::open_mod(&core, &path, None));
                        }
                    }
                }

                Message::CheckMeta => {
                    let source = &self.package_builder.borrow().source;
                    for file in ["info.json", "rules.txt", "meta.yml"] {
                        let file = source.join(file);
                        if file.exists() {
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
                        .set_title("保存 Mod 包")
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
                        .set_title("选择 Cemu 目录")
                        .pick_folder()
                    {
                        self.do_task(move |core| tasks::import_cemu_settings(&core, &path));
                    }
                }
                Message::MigrateBcml => {
                    self.do_task(tasks::migrate_bcml);
                }
                Message::RequestMeta(path) => {
                    self.meta_input.open(path, self.platform());
                }
                Message::SetChangelog(msg) => self.changelog = Some(msg),
                Message::CloseChangelog => self.changelog = None,
                Message::OfferUpdate(version) => {
                    self.changelog = Some(format!(
                        "有新的更新可用!\n\n{}",
                        version.description()
                    ));
                    self.new_version = Some(version);
                }
                Message::DoUpdate => {
                    let version = self.new_version.take().unwrap();
                    self.changelog = None;
                    self.do_task(move |_| tasks::do_update(version));
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
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
                }
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
        DockArea::new(&mut Rc::clone(&self.tree).borrow_mut())
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
            tree: std::mem::replace(&mut self.tree.borrow_mut(), tabs::default_ui()),
        };
        fs::write(
            self.core.settings().state_file(),
            serde_json::to_string_pretty(&ui_state).unwrap(),
        )
        .unwrap_or(());
        uk_manager::util::clear_temp();
    }
}

pub fn main() -> Result<(), eframe::Error> {
    crate::logger::init();
    log::debug!("日志记录器已初始化");
    log::info!("已启动 UKMM");
    eframe::run_native(
        "U-King Mod Manager",
        NativeOptions {
            viewport: ViewportBuilder {
                icon: Some(
                    IconData {
                        height: 256,
                        width:  256,
                        rgba:   image::load_from_memory(include_bytes!("../assets/ukmm.png"))
                            .unwrap()
                            .to_rgba8()
                            .into_vec(),
                    }
                    .into(),
                ),
                min_inner_size: Some(egui::Vec2::new(850.0, 500.0)),
                inner_size: Some(egui::Vec2::new(1200.0, 800.0)),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    )
}
