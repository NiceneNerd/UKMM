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
mod update;
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
        Layout, RichText, Spinner, TextStyle, Ui, UiStackInfo, Vec2, ViewportBuilder,
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
    CleanProfile(String),
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
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        if !(self.modal_open() || files.is_empty()) {
            let first = files.first().and_then(|f| f.path.clone()).unwrap();
            self.install_queue
                .extend(files.iter().skip(1).filter_map(|f| f.path.clone()));
            self.error_queue.clear();
            self.do_task(move |core| tasks::open_mod(&core, &first, None));
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
        let mut ui = Ui::new(
            ctx.clone(),
            layer_id,
            id,
            max_rect,
            clip_rect,
            UiStackInfo::new(egui::UiKind::CentralPanel),
        );
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
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
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
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
