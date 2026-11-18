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
mod update;
mod util;
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc, thread, time::Duration};

use anyhow::{Context, Result};
use eframe::{
    NativeOptions,
    egui::{IconData, InnerResponse},
    epaint::text::TextWrapping,
};
use egui_notify::Toast;
use flume::{Receiver, Sender};
use fs_err as fs;
use mods::ModListState;
use options::OptionsModal;
use parking_lot::Mutex;
use picker::FilePickerState;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use uk_app::{
    Command, Event, ModId, ProfileName, Session, Snapshot,
    host::updates::VersionResponse,
    view::{filter_entries, sort_entries},
};
use uk_content::util::HashMap;
use uk_localization::{string_ext::LocString, *};
use uk_manager::{
    core::Manager,
    settings::{Platform, Settings},
};
use uk_mod::pack::sanitise;
pub use uk_ui::visuals;
use uk_ui::{
    egui::{
        self, Align, Align2, ComboBox, Frame, Id, Label, LayerId, Layout, RichText, Spinner,
        TextStyle, Ui, UiStackInfo, Vec2, ViewportBuilder, epaint::Margin, text::LayoutJob,
    },
    egui_dock::{DockArea, DockState, NodeIndex},
    ext::UiExt,
    icons::{Icon, IconButtonExt},
};
use uk_util::OptionResultExt;

use self::package::ModPackerBuilder;
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
        let string = match self {
            Self::Info => "Tab_Info".localize(),
            Self::Install => "Tab_Install".localize(),
            Self::Deploy => "Tab_Deploy".localize(),
            Self::Mods => "Tab_Mods".localize(),
            Self::Log => "Tab_Log".localize(),
            Self::Settings => "Tab_Settings".localize(),
            Self::Package => "Tab_Package".localize(),
        };
        write!(f, "{}", string)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FocusedPane {
    ModList,
    FilePicker,
    None,
}

// The sort order lives in `uk-app` so every frontend shares it; the GUI
// re-exports it and keeps persisting it in `ui.json`.
pub use uk_app::view::Sort;

/// Frontend-only messages. Anything that mutates core state goes straight to
/// `Session::dispatch(Command)`; results come back as `Event`s.
#[derive(Debug)]
pub enum Message {
    ChangeSort(Sort, bool),
    CheckMeta,
    CloseAbout,
    CloseConfirm,
    CloseError,
    CloseChangelog,
    ClosePackagingOptions,
    ClosePackagingDependencies,
    CloseProfiles,
    /// Ask the user to confirm a session command; dispatched on OK.
    Confirm(Box<Command>, String),
    Deselect(ModId),
    DoUpdate,
    Error(anyhow_ext::Error),
    FilePickerBack,
    FilePickerSet(Option<PathBuf>),
    FilePickerUp,
    GetPackagingOptions,
    ImportCemu,
    MigrateBcml,
    NewProfile,
    OpenMod(PathBuf),
    PackageMod,
    ResetSettings,
    SaveSettings,
    SelectAlso(ModId),
    SelectFile,
    SelectOnly(ModId),
    SelectThrough(ModId),
    SelectProfileManage(smartstring::alias::String),
    SetDownloading(String),
    SetFocus(FocusedPane),
    SetLanguage(LocLang),
    SetTheme(visuals::Theme),
    ShowAbout,
    ShowPackagingOptions(FxHashSet<PathBuf>),
    ShowPackagingDependencies,
    StartDrag(ModId),
    Toast(String),
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct UiState {
    theme: visuals::Theme,
    list_state: ModListState,
    picker_state: FilePickerState,
    #[serde(default = "tabs::default_ui")]
    tree: DockState<Tabs>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: visuals::Theme::Sheikah,
            list_state: ModListState::default(),
            picker_state: FilePickerState::default(),
            tree: tabs::default_ui(),
        }
    }
}

pub struct App {
    core: Arc<Manager>,
    /// The session owns all mod/profile/pending state; the frontend only
    /// keeps view state and dispatches commands.
    session: Session,
    events: Receiver<Event>,
    /// Latest snapshot from the session (re-read after events).
    snap: Arc<Snapshot>,
    /// Visible rows (sorted + filtered), as mod ids; display indices never
    /// leave the frontend.
    visible: Vec<ModId>,
    visible_dirty: bool,
    channel: (Sender<Message>, Receiver<Message>),
    mod_list_filter: String,
    selected: Vec<ModId>,
    drag: Option<ModId>,
    hover: Option<ModId>,
    /// One-click download in progress (reported over IPC).
    downloading: Option<String>,
    list_state: ModListState,
    picker_state: FilePickerState,
    profiles_state: RefCell<profiles::ProfileManagerState>,
    meta_input: MetaInputModal,
    closed_tabs: HashMap<Tabs, NodeIndex>,
    tree: Rc<RefCell<DockState<Tabs>>>,
    focused: FocusedPane,
    error: Option<anyhow_ext::Error>,
    new_profile: Option<String>,
    confirm: Option<(Box<Command>, String)>,
    show_about: bool,
    package_builder: RefCell<ModPackerBuilder>,
    show_package_deps: bool,
    opt_folders: Option<Mutex<FxHashSet<PathBuf>>>,
    options_modal: Option<OptionsModal>,
    temp_settings: Settings,
    toasts: egui_notify::Toasts,
    theme: visuals::Theme,
    dock_style: uk_ui::egui_dock::Style,
    changelog: Option<String>,
    new_version: Option<VersionResponse>,
}

impl App {
    fn new(
        cc: &eframe::CreationContext,
        ipc: Receiver<uk_app::host::oneclick::IpcMessage>,
    ) -> Self {
        #[cfg(not(windows))]
        if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
            cc.egui_ctx.set_pixels_per_point(
                std::env::var("WINIT_X11_SCALE_FACTOR")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0),
            );
        }
        uk_ui::icons::load_icons(&cc.egui_ctx);
        uk_ui::load_fonts(&cc.egui_ctx);
        let core = Arc::new(Manager::init().unwrap());
        let session = Session::open(core.clone()).expect("Failed to open session");
        let events = session.subscribe();
        let snap = session.snapshot();
        let ui_state: UiState = fs::read_to_string(core.settings().state_file())
            .context("")
            .and_then(|s| serde_json::from_str(&s).context(""))
            .unwrap_or_default();
        ui_state.theme.set_theme(&cc.egui_ctx);
        let (send, recv) = flume::unbounded();
        // Bridge one-click IPC messages into the app message channel
        {
            let send = send.clone();
            thread::spawn(move || {
                while let Ok(msg) = ipc.recv() {
                    let msg = match msg {
                        uk_app::host::oneclick::IpcMessage::OpenMod(path) => Message::OpenMod(path),
                        uk_app::host::oneclick::IpcMessage::Error(e) => {
                            Message::Error(anyhow_ext::anyhow!(e))
                        }
                        uk_app::host::oneclick::IpcMessage::Starting(name) => {
                            Message::SetDownloading(name)
                        }
                    };
                    if send.send(msg).is_err() {
                        break;
                    }
                }
            });
        }
        let temp_settings = core.settings().clone();
        let platform = core.settings().current_mode;
        LOCALIZATION.write().update_language(&temp_settings.lang);
        let (changelog, check_updates) = {
            let settings = core.settings();
            let is_first_run = settings.last_version.is_none();
            let needs_config = settings.platform_config().is_none();
            drop(settings);
            // Show intro if first run OR if platform not configured
            if is_first_run || needs_config {
                (Some("Intro_Message".localize().to_string()), false)
            } else {
                (None, true)
            }
        };
        let mut app = Self {
            selected: snap.entries.first().map(|e| e.id).into_iter().collect(),
            drag: None,
            hover: None,
            package_builder: RefCell::new(ModPackerBuilder::new(platform)),
            list_state: ui_state.list_state,
            picker_state: ui_state.picker_state,
            profiles_state: RefCell::new(profiles::ProfileManagerState::new(&core)),
            meta_input: MetaInputModal::new(send.clone()),
            mod_list_filter: "".parse().unwrap(),
            temp_settings,
            changelog,
            channel: (send, recv),
            closed_tabs: Default::default(),
            focused: FocusedPane::None,
            error: None,
            new_profile: None,
            confirm: None,
            show_about: false,
            show_package_deps: false,
            opt_folders: None,
            options_modal: None,
            downloading: None,
            tree: Rc::new(RefCell::new(ui_state.tree)),
            toasts: egui_notify::Toasts::new().with_anchor(egui_notify::Anchor::BottomRight),
            theme: ui_state.theme,
            dock_style: visuals::style_dock(&cc.egui_ctx.style()),
            new_version: None,
            session,
            events,
            snap,
            visible: vec![],
            visible_dirty: true,
            core,
        };
        app.refresh_visible();
        if check_updates {
            app.session.dispatch(Command::CheckForUpdates);
        }
        app
    }

    #[inline(always)]
    fn platform(&self) -> Platform {
        self.core.settings().current_mode
    }

    /// Rebuilds the visible row list (sorted + filtered ids) from the
    /// snapshot. Display indices never leave the frontend.
    fn refresh_visible(&mut self) {
        let sorted = sort_entries(
            &self.snap.entries,
            self.list_state.sort,
            self.list_state.descending,
        );
        self.visible = filter_entries(&sorted, &self.snap.entries, &self.mod_list_filter);
        self.visible_dirty = false;
    }

    #[inline(always)]
    fn modal_open(&self) -> bool {
        self.error.is_some()
            || self.snap.busy.is_some()
            || self.downloading.is_some()
            || self.options_modal.is_some()
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

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        if !(self.modal_open() || files.is_empty()) {
            let paths = files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                self.session.dispatch(Command::InstallBatch {
                    paths,
                    replace: None,
                });
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
        // let layer_id = LayerId::background();
        // let max_rect = ctx.available_rect();
        //let clip_rect = ctx.available_rect();
        let id = Id::new("egui_dock::DockArea");
        let mut ui_builder = egui::UiBuilder::new();
        ui_builder.layer_id = Some(LayerId::background());
        ui_builder.max_rect = Some(ctx.available_rect());
        ui_builder.ui_stack_info = UiStackInfo::new(egui::UiKind::CentralPanel);
        let mut ui = Ui::new(ctx.clone(), id, ui_builder);
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
            list_state: std::mem::take(&mut self.list_state),
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

pub fn main(ipc: Receiver<uk_app::host::oneclick::IpcMessage>) -> Result<(), eframe::Error> {
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
                min_inner_size: Some(Vec2::new(850.0, 500.0)),
                inner_size: Some(Vec2::new(1200.0, 800.0)),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc, ipc)))),
    )
}
