//! Events: everything the session emits. Frontends drain all of them per
//! frame.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uk_mod::Meta;

use crate::{
    error::AppError,
    host::updates::VersionResponse,
    profile::ProfileName,
    session::{InstallCandidate, JobId, ModId, OptionsPrompt},
};

/// A `(name, version)` pair for a missing master mod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterRef {
    pub name:    String,
    pub version: String,
}

/// Input the session needs from the user, returned as data. The frontend
/// answers with a follow-up [`Command`](crate::session::Command).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Inspection {
    /// The path opened cleanly and can be installed as-is.
    Ready(InstallCandidate),
    /// The mod has option groups; resolve them via [`OptionsPrompt::resolve`].
    NeedsOptions(OptionsPrompt),
    /// A legacy mod without metadata; ask for a `Meta` and re-`Open`.
    NeedsMeta {
        path: PathBuf,
        suggested_name: String,
    },
    /// The mod targets another platform; do not install.
    WrongPlatform { mod_is: String, current: String },
    /// Non-blocking: required master mods are not installed.
    MissingMasters(Vec<MasterRef>),
}

/// Everything the session emits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// The snapshot changed; re-read it.
    Revision(u64),
    /// A command needs user input.
    Inspection(Inspection),
    Installed {
        id: ModId,
    },
    /// A mod was replaced by an update; `from` is the old id (its hash
    /// changed), `to` the new one.
    Updated {
        from: ModId,
        to:   ModId,
    },
    Removed {
        ids: Vec<ModId>,
    },
    PendingChanged {
        profile: ProfileName,
        files:   usize,
    },
    ProfilesChanged,
    JobStarted(JobId),
    Progress {
        job:     JobId,
        message: String,
    },
    JobDone {
        job:    JobId,
        result: Result<(), AppError>,
    },
    Error(AppError),
    /// Settings were reloaded from disk; re-read them from the core.
    SettingsChanged,
    /// Result of [`Command::ParseMeta`](crate::session::Command::ParseMeta).
    PackageMeta(Meta),
    /// A mod was packaged; the packer can reset.
    Packaged,
    /// The newest release's changelog (we are ahead of it).
    Changelog(String),
    /// A newer release is available.
    UpdateAvailable(VersionResponse),
    /// The self-update finished; the frontend should restart the app.
    Restart,
}
