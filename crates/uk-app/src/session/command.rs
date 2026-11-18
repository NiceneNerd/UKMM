//! Commands: everything a frontend can ask the session to do. Data-only and
//! serde, so a CLI or remote frontend can send the same commands. The enum
//! grows per refactor phase.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uk_manager::mods::Mod;
use uk_mod::{Meta, ModOption, OptionGroup};

use crate::{options::required_satisfied, profile::ProfileName, session::ModId};

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

/// Everything a frontend can ask the session to do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Classify + inspect a path; never blocks on input — answers with
    /// [`Inspection`](crate::session::Event::Inspection) events.
    Open {
        path: PathBuf,
        meta: Option<Meta>,
    },
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
}
