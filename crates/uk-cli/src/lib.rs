//! The command-line frontend of UKMM. A thin frontend over [`uk_app`]'s
//! [`Session`]: every subcommand becomes one or more [`Command`]s, and
//! [`Inspection`] prompts are answered automatically (default options,
//! `--meta`). No GUI toolkit is allowed here — CI checks
//! `cargo tree -p uk-cli` for egui/eframe/rfd/winit/uk-ui.
//!
//! [`COMMAND_COVERAGE`] maps every [`Command`] variant to the subcommand that
//! exercises it; `tests/coverage.rs` asserts the table matches
//! `Command::variant_names()` exactly.

use std::{
    io::{Write, stdin, stdout},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow_ext::{Context, Result};
use uk_app::{
    Command, Event, FolderKind, Inspection, ModEntry, ModId, OptionsPrompt, ProfileName, Session,
    model::{Manager, Meta, ModOptionGroup, ModPlatform, Platform},
    settings::SettingsDraft,
};

xflags::xflags! {
    src "./src/lib.rs"

    /// Command line interface for U-King Mod Manager
    cmd ukmm {
        /// Verbose logging for debugging
        optional -d, --debug
        /// Run using settings in same folder as executable
        optional -p, --portable
        /// Automatically deploy after running command (redunant with `deploy` command)
        optional -D, --deploy
        /// Emit session events and snapshots as JSON lines on stdout
        optional -j, --json
        /// List installed mods in the current profile
        cmd list {}
        /// Inspect a mod path without installing it
        cmd open {
            /// Path to the mod
            required path: PathBuf
        }
        /// Handle a one-click install URL
        cmd one-click {
            /// The bcml:// URL
            required url: String
        }
        /// Install a mod
        cmd install {
            /// Path to the mod to install
            required path: PathBuf
            /// The profile to install the mod in
            optional profile: String
            /// Path to a meta file (meta.yml, info.json, or rules.txt) for legacy mods
            optional -m, --meta meta: PathBuf
        }
        /// Install several mods at once, optionally replacing one installed mod
        cmd install-batch {
            /// Paths to the mods to install
            repeated paths: PathBuf
            /// 1-based index of an installed mod to replace
            optional -r, --replace replace: usize
        }
        /// Enable mods
        cmd enable {
            /// 1-based indices of the mods to enable (as shown by `list`)
            repeated ids: usize
        }
        /// Disable mods
        cmd disable {
            /// 1-based indices of the mods to disable
            repeated ids: usize
        }
        /// Set the enabled options of a mod
        cmd options {
            /// 1-based index of the mod
            required index: usize
            /// Option paths to enable (one per option group)
            repeated picks: String
        }
        /// Move mods to a new position in the load order
        cmd move {
            /// 1-based index of the mod(s) to move
            required index: usize
            /// 1-based target position
            required to: usize
        }
        /// Apply pending changes (merge)
        cmd apply {}
        /// Cancel pending changes
        cmd cancel-pending {}
        /// Deploy mods
        cmd deploy {}
        /// Refresh merge
        cmd remerge {}
        /// Reset the pending merge to the last deployed state
        cmd reset-pending {}
        /// Uninstall a mod
        cmd uninstall {
            /// The 1-based index of the mod to uninstall (as shown by `list`); omit to pick interactively
            optional index: usize
            /// The profile to uninstall the mod from
            optional profile: String
        }
        /// Copy mods into another profile
        cmd add-to-profile {
            /// The target profile
            required profile: String
            /// 1-based indices of the mods to copy
            repeated ids: usize
        }
        /// List profiles
        cmd profiles {}
        /// Switch the current profile
        cmd profile-switch {
            required name: String
        }
        /// Create a profile
        cmd profile-create {
            required name: String
        }
        /// Duplicate a profile
        cmd profile-duplicate {
            required name: String
        }
        /// Rename a profile
        cmd profile-rename {
            required from: String
            required to: String
        }
        /// Delete a profile
        cmd profile-delete {
            required name: String
        }
        /// Replace an installed mod with a new version from a path
        cmd update-from {
            /// 1-based index of the mod to replace
            required index: usize
            /// Path to the new mod
            required path: PathBuf
            /// Path to a meta file (meta.yml, info.json, or rules.txt) for legacy mods
            optional -m, --meta meta: PathBuf
        }
        /// Repack installed mods in place from loose folders (dev)
        cmd dev-update {
            /// 1-based indices of the mods to update
            repeated ids: usize
            /// The loose mod folder
            required -f, --folder folder: PathBuf
        }
        /// Extract mods to a folder
        cmd extract {
            /// Destination folder
            required dest: PathBuf
            /// 1-based indices of the mods to extract
            repeated ids: usize
        }
        /// Re-read settings from disk
        cmd reload {}
        /// Import settings from a Cemu settings.xml
        cmd import-cemu {
            required path: PathBuf
        }
        /// Migrate a BCML installation
        cmd migrate-bcml {}
        /// Package a mod
        cmd package {
            /// Path to the mod root directory
            required path: PathBuf
            /// Path to the output mod archive
            required output: PathBuf
            /// Path to the meta file for the mod
            required meta: PathBuf
        }
        /// Parse a meta file and print it
        cmd parse-meta {
            required path: PathBuf
        }
        /// Check for UKMM updates
        cmd check-updates {
            /// Install the update immediately if one is available
            optional -y, --yes
        }
        /// Launch the configured emulator
        cmd launch-emulator {}
        /// Open a folder in the OS file manager
        cmd open-folder {
            /// Which folder: config, storage, or deploy
            optional kind: String
        }
        /// Open the folder of an installed mod
        cmd open-mod-folder {
            /// 1-based index of the mod
            required index: usize
        }
        /// Register the bcml:// URL handler
        cmd register-url {}
        /// Save a settings draft (YAML) and reload
        cmd save-settings {
            required path: PathBuf
        }
        /// Change current mode (Switch or Wii U)
        cmd mode {
            /// Mode to activate (Switch or Wii U)
            required platform: Platform
        }
    }
}
// generated start
// The following code is generated by `xflags` macro.
// Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
#[derive(Debug)]
pub struct Ukmm {
    pub debug: bool,
    pub portable: bool,
    pub deploy: bool,
    pub json: bool,
    pub subcommand: UkmmCmd,
}

#[derive(Debug)]
pub enum UkmmCmd {
    List(List),
    Open(Open),
    OneClick(OneClick),
    Install(Install),
    InstallBatch(InstallBatch),
    Enable(Enable),
    Disable(Disable),
    Options(Options),
    Move(Move),
    Apply(Apply),
    CancelPending(CancelPending),
    Deploy(Deploy),
    Remerge(Remerge),
    ResetPending(ResetPending),
    Uninstall(Uninstall),
    AddToProfile(AddToProfile),
    Profiles(Profiles),
    ProfileSwitch(ProfileSwitch),
    ProfileCreate(ProfileCreate),
    ProfileDuplicate(ProfileDuplicate),
    ProfileRename(ProfileRename),
    ProfileDelete(ProfileDelete),
    UpdateFrom(UpdateFrom),
    DevUpdate(DevUpdate),
    Extract(Extract),
    Reload(Reload),
    ImportCemu(ImportCemu),
    MigrateBcml(MigrateBcml),
    Package(Package),
    ParseMeta(ParseMeta),
    CheckUpdates(CheckUpdates),
    LaunchEmulator(LaunchEmulator),
    OpenFolder(OpenFolder),
    OpenModFolder(OpenModFolder),
    RegisterUrl(RegisterUrl),
    SaveSettings(SaveSettings),
    Mode(Mode),
}

#[derive(Debug)]
pub struct List;

#[derive(Debug)]
pub struct Open {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct OneClick {
    pub url: String,
}

#[derive(Debug)]
pub struct Install {
    pub path:    PathBuf,
    pub profile: Option<String>,

    pub meta: Option<PathBuf>,
}

#[derive(Debug)]
pub struct InstallBatch {
    pub paths: Vec<PathBuf>,

    pub replace: Option<usize>,
}

#[derive(Debug)]
pub struct Enable {
    pub ids: Vec<usize>,
}

#[derive(Debug)]
pub struct Disable {
    pub ids: Vec<usize>,
}

#[derive(Debug)]
pub struct Options {
    pub index: usize,
    pub picks: Vec<String>,
}

#[derive(Debug)]
pub struct Move {
    pub index: usize,
    pub to:    usize,
}

#[derive(Debug)]
pub struct Apply;

#[derive(Debug)]
pub struct CancelPending;

#[derive(Debug)]
pub struct Deploy;

#[derive(Debug)]
pub struct Remerge;

#[derive(Debug)]
pub struct ResetPending;

#[derive(Debug)]
pub struct Uninstall {
    pub index:   Option<usize>,
    pub profile: Option<String>,
}

#[derive(Debug)]
pub struct AddToProfile {
    pub profile: String,
    pub ids:     Vec<usize>,
}

#[derive(Debug)]
pub struct Profiles;

#[derive(Debug)]
pub struct ProfileSwitch {
    pub name: String,
}

#[derive(Debug)]
pub struct ProfileCreate {
    pub name: String,
}

#[derive(Debug)]
pub struct ProfileDuplicate {
    pub name: String,
}

#[derive(Debug)]
pub struct ProfileRename {
    pub from: String,
    pub to:   String,
}

#[derive(Debug)]
pub struct ProfileDelete {
    pub name: String,
}

#[derive(Debug)]
pub struct UpdateFrom {
    pub index: usize,
    pub path:  PathBuf,

    pub meta: Option<PathBuf>,
}

#[derive(Debug)]
pub struct DevUpdate {
    pub ids: Vec<usize>,

    pub folder: PathBuf,
}

#[derive(Debug)]
pub struct Extract {
    pub dest: PathBuf,
    pub ids:  Vec<usize>,
}

#[derive(Debug)]
pub struct Reload;

#[derive(Debug)]
pub struct ImportCemu {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct MigrateBcml;

#[derive(Debug)]
pub struct Package {
    pub path:   PathBuf,
    pub output: PathBuf,
    pub meta:   PathBuf,
}

#[derive(Debug)]
pub struct ParseMeta {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct CheckUpdates {
    pub yes: bool,
}

#[derive(Debug)]
pub struct LaunchEmulator;

#[derive(Debug)]
pub struct OpenFolder {
    pub kind: Option<String>,
}

#[derive(Debug)]
pub struct OpenModFolder {
    pub index: usize,
}

#[derive(Debug)]
pub struct RegisterUrl;

#[derive(Debug)]
pub struct SaveSettings {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Mode {
    pub platform: Platform,
}

impl Ukmm {
    #[allow(dead_code)]
    pub fn from_env_or_exit() -> Self {
        Self::from_env_or_exit_()
    }

    #[allow(dead_code)]
    pub fn from_env() -> xflags::Result<Self> {
        Self::from_env_()
    }

    #[allow(dead_code)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
        Self::from_vec_(args)
    }
}
// generated end

/// Maps every [`Command`] variant to the subcommand that exercises it. The
/// test in `tests/coverage.rs` asserts this covers `Command::variant_names()`
/// exactly — add a variant to `Command` and both the exhaustive
/// `Command::variant_name` match and this table must grow.
pub const COMMAND_COVERAGE: &[(&str, &str)] = &[
    ("Open", "open"),
    ("OneClick", "one-click"),
    ("Install", "install"),
    ("InstallBatch", "install-batch"),
    ("SetEnabled", "enable"),
    ("SetOptions", "options"),
    ("MoveTo", "move"),
    ("Apply", "apply"),
    ("CancelPending", "cancel-pending"),
    ("Deploy", "deploy"),
    ("Remerge", "remerge"),
    ("ResetPending", "reset-pending"),
    ("Uninstall", "uninstall"),
    ("AddToProfile", "add-to-profile"),
    ("SwitchProfile", "profile-switch"),
    ("CreateProfile", "profile-create"),
    ("DuplicateProfile", "profile-duplicate"),
    ("RenameProfile", "profile-rename"),
    ("DeleteProfile", "profile-delete"),
    ("UpdateFromPath", "update-from"),
    ("DevUpdate", "dev-update"),
    ("Extract", "extract"),
    ("Reload", "reload"),
    ("ImportCemu", "import-cemu"),
    ("MigrateBcml", "migrate-bcml"),
    ("Package", "package"),
    ("ParseMeta", "parse-meta"),
    ("CheckForUpdates", "check-updates"),
    ("InstallUpdate", "check-updates --yes"),
    ("LaunchEmulator", "launch-emulator"),
    ("OpenFolder", "open-folder"),
    ("RegisterUrlHandler", "register-url"),
    ("SaveSettings", "save-settings"),
];

/// All subcommand names. The root binary uses this to tell a malformed CLI
/// invocation apart from a GUI/one-click launch (a parse error with a
/// subcommand present must exit non-zero, not silently start the GUI).
pub const SUBCOMMANDS: &[&str] = &[
    "list",
    "open",
    "one-click",
    "install",
    "install-batch",
    "enable",
    "disable",
    "options",
    "move",
    "apply",
    "cancel-pending",
    "deploy",
    "remerge",
    "reset-pending",
    "uninstall",
    "add-to-profile",
    "profiles",
    "profile-switch",
    "profile-create",
    "profile-duplicate",
    "profile-rename",
    "profile-delete",
    "update-from",
    "dev-update",
    "extract",
    "reload",
    "import-cemu",
    "migrate-bcml",
    "package",
    "parse-meta",
    "check-updates",
    "launch-emulator",
    "open-folder",
    "open-mod-folder",
    "register-url",
    "save-settings",
    "mode",
];

/// Runs the CLI: builds the core manager + session and executes the subcommand.
pub fn run(cli: Ukmm) -> Result<()> {
    if cli.debug {
        env_logger::init();
        log::set_max_level(log::LevelFilter::Debug);
    }
    let core = Arc::new(Manager::init().context("Failed to initialize the mod manager")?);
    let session = Session::open(core)?;
    Runner {
        session,
        deploy_after: cli.deploy,
        json: cli.json,
        meta: None,
        last_installed: None,
        pending_replace: None,
    }
    .run_subcommand(&cli.subcommand)
}

pub struct Runner {
    session: Session,
    deploy_after: bool,
    /// `--json`: every event is printed as one JSON line, plus a
    /// `{"snapshot": …}` line after each revision.
    json: bool,
    /// The `--meta` file, used when a mod needs metadata (`NeedsMeta`).
    meta: Option<PathBuf>,
    last_installed: Option<ModId>,
    /// Set while an `update-from` is in flight: if the archive turns out to
    /// be a legacy mod (`NeedsMeta`), the re-inspect must keep the replace
    /// target instead of installing a new mod.
    pending_replace: Option<ModId>,
}

impl Runner {
    /// Human-readable progress output: stdout normally, stderr in `--json`
    /// mode so stdout stays pure JSONL.
    fn say(&self, msg: &str) {
        if self.json {
            eprintln!("{msg}");
        } else {
            println!("{msg}");
        }
    }

    fn run_subcommand(&mut self, sub: &UkmmCmd) -> Result<()> {
        self.do_subcommand(sub)?;
        if self.deploy_after {
            self.deploy()?;
        }
        Ok(())
    }

    fn do_subcommand(&mut self, sub: &UkmmCmd) -> Result<()> {
        match sub {
            UkmmCmd::List(_) => self.list(),
            UkmmCmd::Open(Open { path }) => {
                let events = self.exec_inner(
                    Command::Open {
                        path: path.clone(),
                        meta: None,
                    },
                    false,
                )?;
                if events.iter().any(|e| {
                    matches!(
                        e,
                        Event::Inspection(Inspection::WrongPlatform { .. })
                            | Event::Inspection(Inspection::MissingMasters(_))
                    )
                }) {
                    anyhow_ext::bail!(
                        "The mod cannot be installed with the current configuration (see above)"
                    );
                }
                Ok(())
            }
            UkmmCmd::OneClick(OneClick { url }) => {
                self.exec(Command::OneClick(url.clone()))?;
                Ok(())
            }
            UkmmCmd::Install(Install {
                path,
                profile,
                meta,
            }) => {
                self.meta = meta.clone();
                let current = self.session.snapshot().profile.clone();
                let target = profile.as_ref().map(ProfileName::parse).transpose()?;
                match &target {
                    // Install directly into the target profile (the old CLI
                    // semantics): create it if needed, switch there, install +
                    // merge, switch back. The current profile is untouched.
                    Some(t) if *t != current => {
                        if !self.session.snapshot().profiles.contains(t) {
                            self.exec(Command::CreateProfile(t.clone()))?;
                        }
                        self.exec(Command::SwitchProfile(t.clone()))?;
                        self.install_mod(path)?;
                        self.exec(Command::SwitchProfile(current))?;
                    }
                    _ => self.install_mod(path)?,
                }
                Ok(())
            }
            UkmmCmd::InstallBatch(InstallBatch { paths, replace }) => {
                let replace = replace.map(|i| self.entry_id(i)).transpose()?;
                self.exec(Command::InstallBatch {
                    paths: paths.clone(),
                    replace,
                })?;
                self.exec(Command::Apply)?;
                self.say("Done!");
                Ok(())
            }
            UkmmCmd::Enable(Enable { ids }) => {
                let ids = self.ids(ids)?;
                self.exec(Command::SetEnabled { ids, enabled: true })?;
                self.merge_staged()
            }
            UkmmCmd::Disable(Disable { ids }) => {
                let ids = self.ids(ids)?;
                self.exec(Command::SetEnabled {
                    ids,
                    enabled: false,
                })?;
                self.merge_staged()
            }
            UkmmCmd::Options(Options { index, picks }) => {
                let entry = self.entry(*index)?;
                if picks.is_empty() {
                    // Discovery mode: show the option groups, their paths, and
                    // the current selection, without changing anything.
                    if entry.meta.options.is_empty() {
                        self.say("This mod has no options");
                        return Ok(());
                    }
                    for group in &entry.meta.options {
                        self.say(&format!(
                            "{} (required: {})",
                            group.name(),
                            group.required()
                        ));
                        for o in group.options() {
                            let selected = entry.options.iter().any(|e| e.path == o.path);
                            self.say(&format!(
                                "  {} {} - {}",
                                if selected { "*" } else { " " },
                                o.path.display(),
                                o.name
                            ));
                        }
                    }
                    self.say("Change the selection with: ukmm options <index> <path>...");
                    return Ok(());
                }
                // Every pick must be a known option path (exact match).
                let all: Vec<_> = entry
                    .meta
                    .options
                    .iter()
                    .flat_map(|g| g.options().iter())
                    .collect();
                for p in picks {
                    if !all.iter().any(|o| o.path.to_string_lossy() == *p) {
                        let valid: Vec<String> = all
                            .iter()
                            .map(|o| format!("  {}", o.path.display()))
                            .collect();
                        anyhow_ext::bail!(
                            "Unknown option path '{p}'. Available paths:\n{}",
                            valid.join("\n")
                        );
                    }
                }
                let mut resolved = vec![];
                for group in &entry.meta.options {
                    let matches: Vec<_> = picks
                        .iter()
                        .filter_map(|p| {
                            group
                                .options()
                                .iter()
                                .find(|o| o.path.to_string_lossy() == *p)
                        })
                        .collect();
                    match matches.as_slice() {
                        [o] => resolved.push((*o).clone()),
                        [] if group.required() => {
                            anyhow_ext::bail!(
                                "Required option group '{}' has no pick",
                                group.name()
                            );
                        }
                        [] => {}
                        _ => {
                            anyhow_ext::bail!(
                                "Option group '{}' takes exactly one pick, got {}",
                                group.name(),
                                matches.len()
                            );
                        }
                    }
                }
                self.exec(Command::SetOptions {
                    id:    entry.id,
                    picks: resolved,
                })?;
                self.merge_staged()
            }
            UkmmCmd::Move(Move { index, to }) => {
                let ids = vec![self.entry_id(*index)?];
                self.exec(Command::MoveTo {
                    ids,
                    index: to
                        .checked_sub(1)
                        .context("Target position must be >= 1 (1-based, as shown by `list`")?,
                })?;
                self.merge_staged()
            }
            UkmmCmd::Apply(_) => self.exec(Command::Apply),
            UkmmCmd::CancelPending(_) => self.exec(Command::CancelPending),
            UkmmCmd::Deploy(_) => self.deploy(),
            UkmmCmd::Remerge(_) => {
                self.say("Remerging...");
                self.exec(Command::Remerge)?;
                self.say("Done!");
                Ok(())
            }
            UkmmCmd::ResetPending(_) => self.exec(Command::ResetPending),
            UkmmCmd::Uninstall(Uninstall { index, profile }) => {
                let ids = self.uninstall_ids(*index)?;
                let current = self.session.snapshot().profile.clone();
                let target = profile.as_ref().map(ProfileName::parse).transpose()?;
                if let Some(target) = &target
                    && *target != current
                {
                    // The session uninstalls from the current profile; switch
                    // there and back to honour the profile argument.
                    self.exec(Command::SwitchProfile(target.clone()))?;
                    self.exec(Command::Uninstall(ids))?;
                    self.exec(Command::Apply)?;
                    self.exec(Command::SwitchProfile(current))?;
                } else {
                    self.exec(Command::Uninstall(ids))?;
                    self.exec(Command::Apply)?;
                }
                self.say("Done!");
                Ok(())
            }
            UkmmCmd::AddToProfile(AddToProfile { profile, ids }) => {
                let ids = self.ids(ids)?;
                let target = ProfileName::parse(profile)?;
                let current = self.session.snapshot().profile.clone();
                self.exec(Command::AddToProfile {
                    ids,
                    profile: target.clone(),
                })?;
                if target != current {
                    // The staged diff lives in the target profile; merge it
                    // there before switching back.
                    self.exec(Command::SwitchProfile(target))?;
                    self.exec(Command::Apply)?;
                    self.exec(Command::SwitchProfile(current))?;
                }
                Ok(())
            }
            UkmmCmd::Profiles(_) => {
                let snap = self.session.snapshot();
                if self.json {
                    println!(
                        "{{\"profiles\":{}}}",
                        serde_json::to_string(&*snap.profiles)?
                    );
                } else {
                    for p in &snap.profiles {
                        let current = if *p == snap.profile { " (current)" } else { "" };
                        self.say(&format!("{p}{current}"));
                    }
                }
                Ok(())
            }
            UkmmCmd::ProfileSwitch(ProfileSwitch { name }) => {
                let profile = ProfileName::parse(name)?;
                // `SwitchProfile` changes the in-memory profile; persist it so
                // the next process starts in it (same pattern as `mode`).
                self.exec(Command::SwitchProfile(profile))?;
                let draft = SettingsDraft::try_from_settings(&self.session.core().settings())
                    .context("Failed to read the current settings")?;
                self.exec(Command::SaveSettings(draft))
            }
            UkmmCmd::ProfileCreate(ProfileCreate { name }) => {
                self.exec(Command::CreateProfile(ProfileName::parse(name)?))
            }
            UkmmCmd::ProfileDuplicate(ProfileDuplicate { name }) => {
                self.exec(Command::DuplicateProfile(ProfileName::parse(name)?))
            }
            UkmmCmd::ProfileRename(ProfileRename { from, to }) => {
                self.exec(Command::RenameProfile {
                    from: ProfileName::parse(from)?,
                    to:   ProfileName::parse(to)?,
                })
            }
            UkmmCmd::ProfileDelete(ProfileDelete { name }) => {
                self.exec(Command::DeleteProfile(ProfileName::parse(name)?))
            }
            UkmmCmd::UpdateFrom(UpdateFrom { index, path, meta }) => {
                let id = self.entry_id(*index)?;
                self.meta = meta.clone();
                self.last_installed = None;
                let meta_parsed = meta.as_ref().map(uk_app::parse_meta).transpose()?;
                self.pending_replace = Some(id);
                let result = self.exec(Command::UpdateFromPath {
                    id,
                    path: path.clone(),
                    meta: meta_parsed,
                });
                self.pending_replace = None;
                result?;
                self.exec(Command::Apply)?;
                self.say("Done!");
                Ok(())
            }
            UkmmCmd::DevUpdate(DevUpdate { ids, folder }) => {
                let ids = self.ids(ids)?;
                self.exec(Command::DevUpdate {
                    ids,
                    folders: vec![folder.clone()],
                })?;
                Ok(())
            }
            UkmmCmd::Extract(Extract { dest, ids }) => {
                let ids = self.ids(ids)?;
                self.exec(Command::Extract {
                    ids,
                    dest: dest.clone(),
                })?;
                Ok(())
            }
            UkmmCmd::Reload(_) => self.exec(Command::Reload),
            UkmmCmd::ImportCemu(ImportCemu { path }) => {
                self.exec(Command::ImportCemu(path.clone()))
            }
            UkmmCmd::MigrateBcml(_) => self.exec(Command::MigrateBcml),
            UkmmCmd::Package(pkg) => {
                self.say("Packaging mod...");
                let meta = uk_app::parse_meta(&pkg.meta)?;
                self.exec(Command::Package {
                    source: pkg.path.clone(),
                    dest: pkg.output.clone(),
                    meta,
                })?;
                self.say("Done!");
                Ok(())
            }
            UkmmCmd::ParseMeta(ParseMeta { path }) => {
                let events = self.exec_inner(Command::ParseMeta(path.clone()), false)?;
                for ev in &events {
                    if let Event::PackageMeta(meta) = ev {
                        self.say(&format!(
                            "{} v{} by {}",
                            meta.name, meta.version, meta.author
                        ));
                    }
                }
                Ok(())
            }
            UkmmCmd::CheckUpdates(c) => {
                let events = self.exec_inner(Command::CheckForUpdates, false)?;
                match events.into_iter().find_map(|e| {
                    match e {
                        Event::UpdateAvailable(v) => Some(v),
                        _ => None,
                    }
                }) {
                    Some(v) => {
                        self.say(&format!("Update available: {}", v.name));
                        if c.yes {
                            self.exec(Command::InstallUpdate(v))?;
                            self.say("Update installed; restart UKMM to apply");
                        }
                    }
                    None => self.say("No update available"),
                }
                Ok(())
            }
            UkmmCmd::LaunchEmulator(_) => self.exec(Command::LaunchEmulator),
            UkmmCmd::OpenFolder(OpenFolder { kind }) => {
                let kind = match kind.as_deref().unwrap_or("config") {
                    "config" => FolderKind::Config,
                    "storage" => FolderKind::Storage,
                    "deploy" | "deploy-output" => FolderKind::DeployOutput,
                    other => {
                        anyhow_ext::bail!(
                            "Unknown folder kind '{other}' (expected config, storage, or deploy)"
                        )
                    }
                };
                self.exec(Command::OpenFolder(kind))
            }
            UkmmCmd::OpenModFolder(OpenModFolder { index }) => {
                self.exec(Command::OpenFolder(FolderKind::Mod(self.entry_id(*index)?)))
            }
            UkmmCmd::RegisterUrl(_) => self.exec(Command::RegisterUrlHandler),
            UkmmCmd::SaveSettings(SaveSettings { path }) => {
                let text = fs_err::read_to_string(path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                let draft: SettingsDraft = serde_yaml::from_str(&text).with_context(|| {
                    format!("Failed to parse {} as a settings draft", path.display())
                })?;
                self.exec(Command::SaveSettings(draft))
            }
            UkmmCmd::Mode(Mode { platform }) => {
                let mut draft = SettingsDraft::try_from_settings(&self.session.core().settings())
                    .context("Failed to read the current settings")?;
                draft.current_mode = *platform;
                self.exec(Command::SaveSettings(draft))?;
                self.say(&format!("Mode changed to {platform:?}"));
                Ok(())
            }
        }
    }

    /// The install flow in the current profile: inspect, install, enable,
    /// merge.
    fn install_mod(&mut self, path: &Path) -> Result<()> {
        self.say(&format!("Installing {}...", path.display()));
        self.last_installed = None;
        self.exec(Command::Open {
            path: path.to_path_buf(),
            meta: None,
        })?;
        let id = self.last_installed.context("The mod was not installed")?;
        self.exec(Command::SetEnabled {
            ids:     vec![id],
            enabled: true,
        })?;
        self.exec(Command::Apply)?;
        self.say("Done!");
        Ok(())
    }

    /// Merges staged changes in the same invocation. One CLI call is one
    /// transaction: the session's staged diff is in-memory only, so leaving
    /// it unapplied would lose it when this process exits (a later `apply`
    /// would silently merge nothing).
    fn merge_staged(&mut self) -> Result<()> {
        self.exec(Command::Apply)
    }

    fn list(&self) -> Result<()> {
        let snap = self.session.snapshot();
        if self.json {
            println!("{{\"snapshot\":{}}}", serde_json::to_string(&*snap)?);
            return Ok(());
        }
        self.say(&format!("Profile: {}", snap.profile.as_str()));
        for (i, entry) in snap.entries.iter().enumerate() {
            self.say(&format!(
                "{}. {} v{} by {} [{}]",
                i + 1,
                entry.meta.name,
                entry.meta.version,
                entry.meta.author,
                if entry.enabled { "enabled" } else { "disabled" },
            ));
        }
        self.say(&format!("Pending: {} files", snap.pending.files()));
        Ok(())
    }

    /// Resolves a 1-based load-order index to a [`ModId`].
    fn entry_id(&self, index: usize) -> Result<ModId> {
        Ok(self.entry(index)?.id)
    }

    fn entry(&self, index: usize) -> Result<ModEntry> {
        let entries = &self.session.snapshot().entries;
        entries
            .get(index.checked_sub(1).context("Mod index must be >= 1")?)
            .cloned()
            .with_context(|| format!("Mod {index} does not exist"))
    }

    /// Resolves several 1-based indices.
    fn ids(&self, indices: &[usize]) -> Result<Vec<ModId>> {
        indices.iter().map(|&i| self.entry_id(i)).collect()
    }

    /// Resolves the uninstall target: a 1-based load-order index, or an
    /// interactive comma-separated list when omitted.
    fn uninstall_ids(&self, index: Option<usize>) -> Result<Vec<ModId>> {
        match index {
            Some(i) => {
                let entry = self.entry(i)?;
                self.say(&format!("Removing mod {}...", entry.meta.name));
                Ok(vec![entry.id])
            }
            None => {
                let entries = &self.session.snapshot().entries;
                self.say("Installed mods:");
                for (i, entry) in entries.iter().enumerate() {
                    self.say(&format!(
                        "{}. {} (v{}) by {}",
                        i + 1,
                        entry.meta.name,
                        entry.meta.version,
                        entry.meta.author
                    ));
                }
                self.say("Enter mod(s) to uninstall, separated by commas:");
                stdout().flush()?;
                let mut input = String::new();
                stdin().read_line(&mut input)?;
                let mut ids = vec![];
                for id in input.replace(' ', "").split(',') {
                    if id.is_empty() {
                        continue;
                    }
                    let n: usize = id.trim().parse().context("Invalid mod number")?;
                    let entry = self.entry(n)?;
                    self.say(&format!("Removing mod {}...", entry.meta.name));
                    ids.push(entry.id);
                }
                Ok(ids)
            }
        }
    }

    fn deploy(&mut self) -> Result<()> {
        let pending = self.session.core().with_deploy_manager(|dm| dm.pending());
        if pending {
            self.say("Deploying changes...");
            self.exec(Command::Deploy)?;
            self.say("Deployment complete");
        } else {
            self.say("No changes pending deployment");
        }
        Ok(())
    }

    /// Dispatches `cmd` and drains its events until the job finishes, answering
    /// [`Inspection`] prompts automatically (default option picks, `--meta`).
    fn exec(&mut self, cmd: Command) -> Result<()> {
        self.exec_inner(cmd, true).map(|_| ())
    }

    /// Like [`Runner::exec`], but `auto` controls whether [`Inspection`]
    /// prompts are answered automatically (`install` & co.) or just reported
    /// (`open`, `parse-meta`, `check-updates`). Returns the events seen.
    fn exec_inner(&mut self, cmd: Command, auto: bool) -> Result<Vec<Event>> {
        let rx = self.session.subscribe();
        let job = self.session.dispatch(cmd);
        let mut seen = vec![];
        loop {
            let ev = rx.recv().context("The session worker stopped")?;
            if self.json {
                println!("{{\"event\":{}}}", serde_json::to_string(&ev)?);
                if matches!(ev, Event::Revision(_)) {
                    let snap = self.session.snapshot();
                    println!("{{\"snapshot\":{}}}", serde_json::to_string(&*snap)?);
                }
            }
            let done = matches!(&ev, Event::JobDone { job: j, .. } if *j == job);
            let result = if let Event::JobDone { job: j, result } = &ev
                && *j == job
            {
                Some(result.clone())
            } else {
                None
            };
            match ev {
                Event::Inspection(insp) => {
                    seen.push(Event::Inspection(insp.clone()));
                    if auto {
                        match insp {
                            Inspection::Ready(candidate) => {
                                return self.exec_inner(Command::Install(candidate), auto);
                            }
                            Inspection::NeedsOptions(prompt) => {
                                let picks = prompt.defaults().to_vec();
                                let candidate = prompt
                                    .resolve(picks)
                                    .context("A required option group has no default selection")?;
                                return self.exec_inner(Command::Install(candidate), auto);
                            }
                            Inspection::NeedsMeta {
                                path,
                                suggested_name,
                            } => {
                                let meta = match &self.meta {
                                    Some(p) => uk_app::parse_meta(p)?,
                                    None => {
                                        let mut meta = Meta::blank(ModPlatform::Universal);
                                        meta.name = suggested_name.as_str().into();
                                        meta
                                    }
                                };
                                // An update-from on a legacy archive must
                                // replace the mod, not install a second copy.
                                let cmd = match self.pending_replace.take() {
                                    Some(id) => {
                                        Command::UpdateFromPath {
                                            id,
                                            path,
                                            meta: Some(meta),
                                        }
                                    }
                                    None => {
                                        Command::Open {
                                            path,
                                            meta: Some(meta),
                                        }
                                    }
                                };
                                return self.exec_inner(cmd, auto);
                            }
                            Inspection::WrongPlatform { mod_is, current } => {
                                anyhow_ext::bail!(
                                    "Mod is for {mod_is}, but the current mode is {current}"
                                );
                            }
                            Inspection::MissingMasters(masters) => {
                                anyhow_ext::bail!(
                                    "Missing required mod(s): {}",
                                    masters
                                        .iter()
                                        .map(|m| format!("{} {}", m.name, m.version))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                );
                            }
                        }
                    } else {
                        self.describe_inspection(&insp);
                    }
                }
                Event::Installed { id } => self.last_installed = Some(id),
                Event::Updated { to, .. } => self.last_installed = Some(to),
                Event::PackageMeta(_) | Event::UpdateAvailable(_) => seen.push(ev),
                Event::JobDone { .. } => {}
                _ => {}
            }
            if done {
                // `JobDone` carries the same error; report it there only.
                result
                    .context("no JobDone result")?
                    .map_err(|e| anyhow_ext::anyhow!("{e}"))?;
                return Ok(seen);
            }
        }
    }

    /// Human-readable summary of an inspection (used by `open`).
    fn describe_inspection(&self, insp: &Inspection) {
        match insp {
            Inspection::Ready(_) => self.say("Ready to install"),
            Inspection::NeedsOptions(OptionsPrompt { .. }) => {
                self.say("Mod has option groups; use `install` to pick defaults");
            }
            Inspection::NeedsMeta { suggested_name, .. } => {
                self.say(&format!(
                    "Legacy mod without metadata (suggested name: {suggested_name}); use `install \
                     --meta` to install"
                ));
            }
            Inspection::WrongPlatform { mod_is, current } => {
                self.say(&format!("Mod is for {mod_is}, current mode is {current}"));
            }
            Inspection::MissingMasters(masters) => {
                self.say(&format!(
                    "Missing required mod(s): {}",
                    masters
                        .iter()
                        .map(|m| format!("{} {}", m.name, m.version))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }
}
