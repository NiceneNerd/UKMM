//! Identity and view types: [`ModId`], [`JobId`], [`ModEntry`], [`Snapshot`].

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uk_manager::mods::Mod;
use uk_mod::{Manifest, Meta, ModOption};

use crate::profile::ProfileName;

/// Stable identity of an installed mod: the mod's content hash. Selection,
/// drag, and commands carry these — never row indices or `Mod` clones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModId(pub usize);

/// A command dispatched to the session worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

/// One mod in a [`Snapshot`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub id: ModId,
    pub meta: Meta,
    pub enabled: bool,
    pub options: Vec<ModOption>,
    pub path: PathBuf,
}

impl ModEntry {
    pub fn from_mod(mod_: &Mod) -> Self {
        Self {
            id: ModId(mod_.hash()),
            meta: mod_.meta.clone(),
            enabled: mod_.enabled,
            options: mod_.enabled_options.clone(),
            path: mod_.path.clone(),
        }
    }
}

/// Pending (staged for the next merge) state of the current profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSummary {
    pub manifest: Manifest,
}

impl PendingSummary {
    /// Number of files the pending manifest touches.
    pub fn files(&self) -> usize {
        self.manifest.content_files.len() + self.manifest.aoc_files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files() == 0
    }
}

/// Immutable, revisioned view of the session state. The worker rebuilds and
/// republishes it after every job; frontends read it via
/// [`Session::snapshot`](crate::session::Session::snapshot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub revision: u64,
    pub profile: ProfileName,
    /// All mods of the current profile; vec order IS load order.
    pub entries: Vec<ModEntry>,
    pub pending: PendingSummary,
    /// The currently running *blocking* job, if any. Background jobs
    /// ([`Session::dispatch_background`](crate::session::Session::dispatch_background))
    /// never set this.
    pub busy: Option<JobId>,
    /// Latest progress text of the running job (for busy overlays).
    pub progress: Option<String>,
    /// Remaining files in a running install batch.
    pub installing: usize,
}
