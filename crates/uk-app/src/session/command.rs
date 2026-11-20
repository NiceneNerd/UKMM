//! Commands: everything a frontend can ask the session to do. Data-only and
//! serde, so a CLI or remote frontend can send the same commands. The enum
//! grows per refactor phase.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uk_manager::mods::Mod;
use uk_mod::{Meta, ModOption, OptionGroup};

use crate::{
    host::updates::VersionResponse, options::required_satisfied, profile::ProfileName,
    session::ModId,
};

/// A mod that passed inspection and is ready to install. Constructible only
/// through [`OptionsPrompt::resolve`] or the no-options path, so installing
/// with an unmet required option group is unrepresentable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallCandidate {
    pub(crate) mod_:    Mod,
    pub(crate) picks:   Vec<ModOption>,
    pub(crate) replace: Option<ModId>,
}

impl InstallCandidate {
    /// The no-options path: a mod with no option groups installs as-is.
    pub(crate) fn ready(mod_: Mod, replace: Option<ModId>) -> Self {
        Self {
            mod_,
            picks: vec![],
            replace,
        }
    }

    /// The mod being installed as a replacement for this id, if any.
    pub fn replace_target(&self) -> Option<ModId> {
        self.replace
    }
}

/// The option-picker prompt: the staged mod plus its default picks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsPrompt {
    mod_:     Mod,
    defaults: Vec<ModOption>,
    replace:  Option<ModId>,
}

impl OptionsPrompt {
    pub(crate) fn new(mut mod_: Mod, replace: Option<ModId>, picks: Vec<ModOption>) -> Self {
        // A fresh install starts from the mod's default options; an update
        // starts from the picks the replaced mod had.
        let defaults = if replace.is_none() {
            mod_.enable_default_options();
            mod_.enabled_options.clone()
        } else {
            picks
        };
        Self {
            mod_,
            defaults,
            replace,
        }
    }

    /// The mod's option groups.
    pub fn groups(&self) -> &[OptionGroup] {
        &self.mod_.meta.options
    }

    /// The pre-selected picks (defaults for a fresh install, current picks
    /// for an update).
    pub fn defaults(&self) -> &[ModOption] {
        &self.defaults
    }

    /// Resolves the prompt with the user's picks. `None` when a required
    /// option group has no pick — the install is refused.
    pub fn resolve(self, picks: Vec<ModOption>) -> Option<InstallCandidate> {
        if !required_satisfied(&self.mod_.meta.options, &picks) {
            return None;
        }
        Some(InstallCandidate {
            mod_: self.mod_,
            picks,
            replace: self.replace,
        })
    }
}

/// A folder the frontend may ask the host to reveal in the OS file manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderKind {
    /// The config directory (`settings.yml`, `ui-*.json`, `log.txt`).
    Config,
    /// The mod storage directory.
    Storage,
    /// The current deploy output folder (if configured).
    DeployOutput,
    /// The folder of this mod (or the archive's parent folder).
    Mod(ModId),
}

/// Everything a frontend can ask the session to do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Classify + inspect a path; never blocks on input — answers with
    /// [`Inspection`](crate::session::Event::Inspection) events.
    Open {
        path: PathBuf,
        meta: Option<Meta>,
    },
    /// Download a GameBanana one-click URL and inspect the result, as one
    /// job (busy/progress come from the job lifecycle, not a stray flag).
    OneClick(String),
    /// Install an inspected mod (from `Inspection::Ready` or a resolved
    /// `OptionsPrompt`).
    Install(InstallCandidate),
    /// Inspect and install a batch of paths; per-file progress events, one
    /// aggregated error at the end. `replace` turns installs into updates.
    InstallBatch {
        paths:   Vec<PathBuf>,
        replace: Option<ModId>,
    },
    SetEnabled {
        ids:     Vec<ModId>,
        enabled: bool,
    },
    SetOptions {
        id:    ModId,
        picks: Vec<ModOption>,
    },
    /// Move `ids` so they start at load-order `index` (clamped).
    MoveTo {
        ids:   Vec<ModId>,
        index: usize,
    },
    /// Commit staged changes and re-merge.
    Apply,
    /// Discard the current profile's pending manifest.
    CancelPending,
    /// Deploy the merged output.
    Deploy,
    /// Re-merge without changing the mod configuration.
    Remerge,
    /// Reset the deployment pending log.
    ResetPending,
    Uninstall(Vec<ModId>),
    AddToProfile {
        ids:     Vec<ModId>,
        profile: ProfileName,
    },
    SwitchProfile(ProfileName),
    /// Create (and switch to) a new profile.
    CreateProfile(ProfileName),
    DuplicateProfile(ProfileName),
    RenameProfile {
        from: ProfileName,
        to:   ProfileName,
    },
    DeleteProfile(ProfileName),
    /// Inspect `path` as a replacement for mod `id`.
    UpdateFromPath {
        id:   ModId,
        path: PathBuf,
        meta: Option<Meta>,
    },
    /// Repack the given mods in place from loose folders (same order as `ids`).
    DevUpdate {
        ids:     Vec<ModId>,
        folders: Vec<PathBuf>,
    },
    Extract {
        ids:  Vec<ModId>,
        dest: PathBuf,
    },
    /// Re-read settings from disk and rebuild the managers, then refresh all
    /// session state. The frontend saves settings itself (phase 4 owns the
    /// settings draft) and dispatches this afterwards.
    Reload,
    /// Import settings from a Cemu `settings.xml`, then reload.
    ImportCemu(PathBuf),
    /// Migrate a BCML installation into the current config, then reload.
    MigrateBcml,
    /// Package a mod zip from a loose source folder.
    Package {
        source: PathBuf,
        dest:   PathBuf,
        meta:   Meta,
    },
    /// Parse a meta file (`info.json` / `rules.txt` / `meta.yml`); answers
    /// with [`Event::PackageMeta`](crate::session::Event::PackageMeta).
    ParseMeta(PathBuf),
    /// Check GitHub for releases; answers with
    /// [`Event::UpdateAvailable`](crate::session::Event::UpdateAvailable) or
    /// [`Event::Changelog`](crate::session::Event::Changelog).
    CheckForUpdates,
    /// Download and install a release; answers with
    /// [`Event::Restart`](crate::session::Event::Restart).
    InstallUpdate(VersionResponse),
    /// Launch the configured emulator (deploy settings → executable). Runs on
    /// the worker thread, never in frontend render code.
    LaunchEmulator,
    /// Ask the host OS to open a folder in the file manager.
    OpenFolder(FolderKind),
    /// Register the `bcml:` URL scheme so GameBanana one-click links reach
    /// UKMM.
    RegisterUrlHandler,
    /// Validate and save a settings draft (wiping the old deploy output and
    /// resetting pending when the deploy layout/method/output changed);
    /// answers with [`Event::SettingsSaved`](crate::session::Event::SettingsSaved)
    /// after the reload.
    SaveSettings(crate::settings::SettingsDraft),
}

impl Command {
    /// The variant name. Exhaustive by construction: adding a variant without
    /// a match arm here fails to compile, which is what forces frontends
    /// (e.g. the CLI in `uk-cli`) to cover every command.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::Open { .. } => "Open",
            Self::OneClick(_) => "OneClick",
            Self::Install(_) => "Install",
            Self::InstallBatch { .. } => "InstallBatch",
            Self::SetEnabled { .. } => "SetEnabled",
            Self::SetOptions { .. } => "SetOptions",
            Self::MoveTo { .. } => "MoveTo",
            Self::Apply => "Apply",
            Self::CancelPending => "CancelPending",
            Self::Deploy => "Deploy",
            Self::Remerge => "Remerge",
            Self::ResetPending => "ResetPending",
            Self::Uninstall(_) => "Uninstall",
            Self::AddToProfile { .. } => "AddToProfile",
            Self::SwitchProfile(_) => "SwitchProfile",
            Self::CreateProfile(_) => "CreateProfile",
            Self::DuplicateProfile(_) => "DuplicateProfile",
            Self::RenameProfile { .. } => "RenameProfile",
            Self::DeleteProfile(_) => "DeleteProfile",
            Self::UpdateFromPath { .. } => "UpdateFromPath",
            Self::DevUpdate { .. } => "DevUpdate",
            Self::Extract { .. } => "Extract",
            Self::Reload => "Reload",
            Self::ImportCemu(_) => "ImportCemu",
            Self::MigrateBcml => "MigrateBcml",
            Self::Package { .. } => "Package",
            Self::ParseMeta(_) => "ParseMeta",
            Self::CheckForUpdates => "CheckForUpdates",
            Self::InstallUpdate(_) => "InstallUpdate",
            Self::LaunchEmulator => "LaunchEmulator",
            Self::OpenFolder(_) => "OpenFolder",
            Self::RegisterUrlHandler => "RegisterUrlHandler",
            Self::SaveSettings(_) => "SaveSettings",
        }
    }

    /// All variant names, in declaration order.
    pub fn variant_names() -> &'static [&'static str] {
        &[
            "Open",
            "OneClick",
            "Install",
            "InstallBatch",
            "SetEnabled",
            "SetOptions",
            "MoveTo",
            "Apply",
            "CancelPending",
            "Deploy",
            "Remerge",
            "ResetPending",
            "Uninstall",
            "AddToProfile",
            "SwitchProfile",
            "CreateProfile",
            "DuplicateProfile",
            "RenameProfile",
            "DeleteProfile",
            "UpdateFromPath",
            "DevUpdate",
            "Extract",
            "Reload",
            "ImportCemu",
            "MigrateBcml",
            "Package",
            "ParseMeta",
            "CheckForUpdates",
            "InstallUpdate",
            "LaunchEmulator",
            "OpenFolder",
            "RegisterUrlHandler",
            "SaveSettings",
        ]
    }
}

#[cfg(test)]
mod tests {
    use uk_manager::settings::Platform;
    use uk_mod::ModPlatform;

    use super::*;
    use crate::{host::updates::VersionAsset, profile::ProfileName, settings::SettingsDraft};

    fn sample_mod() -> Mod {
        // `Mod::hash` is pub(crate) in uk-manager, so build one through serde.
        serde_json::from_value(serde_json::json!({
            "hash": "0",
            "meta": Meta::blank(ModPlatform::Universal),
            "enabled_options": [],
            "path": "",
            "enabled": false,
        }))
        .unwrap()
    }

    /// One instance of every variant, serde-round-tripped: this is what makes
    /// `--json` output (and remote frontends) able to carry any command.
    #[test]
    fn every_variant_round_trips_through_serde() {
        let id = ModId(42);
        let profile = ProfileName::parse("Test").unwrap();
        let version = VersionResponse {
            body: "notes".into(),
            name: "v1.0.0".into(),
            tag_name: "v1.0.0".into(),
            prerelease: false,
            assets: vec![VersionAsset {
                name: "ukmm.zip".into(),
                browser_download_url: "https://example.invalid/ukmm.zip".into(),
            }],
        };
        let draft = SettingsDraft {
            current_mode: Platform::WiiU,
            system_7z: true,
            storage_dir: PathBuf::from("/tmp/ukmm"),
            check_updates: Default::default(),
            show_changelog: true,
            wiiu: None,
            switch: None,
        };
        let commands = vec![
            Command::Open {
                path: PathBuf::from("/mod.zip"),
                meta: None,
            },
            Command::OneClick("bcml://example".into()),
            Command::Install(InstallCandidate::ready(sample_mod(), None)),
            Command::InstallBatch {
                paths:   vec![PathBuf::from("/a.zip")],
                replace: None,
            },
            Command::SetEnabled {
                ids:     vec![id],
                enabled: true,
            },
            Command::SetOptions { id, picks: vec![] },
            Command::MoveTo {
                ids:   vec![id],
                index: 0,
            },
            Command::Apply,
            Command::CancelPending,
            Command::Deploy,
            Command::Remerge,
            Command::ResetPending,
            Command::Uninstall(vec![id]),
            Command::AddToProfile {
                ids:     vec![id],
                profile: profile.clone(),
            },
            Command::SwitchProfile(profile.clone()),
            Command::CreateProfile(profile.clone()),
            Command::DuplicateProfile(profile.clone()),
            Command::RenameProfile {
                from: profile.clone(),
                to:   profile.clone(),
            },
            Command::DeleteProfile(profile.clone()),
            Command::UpdateFromPath {
                id,
                path: PathBuf::from("/new.zip"),
                meta: None,
            },
            Command::DevUpdate {
                ids:     vec![id],
                folders: vec![PathBuf::from("/src")],
            },
            Command::Extract {
                ids:  vec![id],
                dest: PathBuf::from("/out"),
            },
            Command::Reload,
            Command::ImportCemu(PathBuf::from("/settings.xml")),
            Command::MigrateBcml,
            Command::Package {
                source: PathBuf::from("/src"),
                dest:   PathBuf::from("/out.zip"),
                meta:   Meta::blank(ModPlatform::Universal),
            },
            Command::ParseMeta(PathBuf::from("/meta.yml")),
            Command::CheckForUpdates,
            Command::InstallUpdate(version),
            Command::LaunchEmulator,
            Command::OpenFolder(FolderKind::Config),
            Command::RegisterUrlHandler,
            Command::SaveSettings(draft),
        ];
        assert_eq!(commands.len(), Command::variant_names().len());
        for cmd in commands {
            let json = serde_json::to_string(&cmd).unwrap();
            let back: Command = serde_json::from_str(&json).unwrap();
            assert_eq!(
                back.variant_name(),
                cmd.variant_name(),
                "round trip: {json}"
            );
        }
    }
}
