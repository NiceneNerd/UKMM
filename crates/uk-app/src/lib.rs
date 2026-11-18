//! The operations a UKMM frontend performs, independent of any UI toolkit.
//!
//! Nothing here may depend on a GUI, a dialog library, or a frontend's message
//! type: inputs a frontend must collect (paths, metadata, option picks) are
//! explicit parameters, and outcomes that need further input are returned as
//! data (see [`open::Opened`]).

mod apply;
pub mod error;
pub mod host;
pub mod import;
pub mod open;
pub mod options;
pub mod order;
pub mod package;
pub mod profile;
pub mod session;
pub mod sniff;
pub mod view;

pub use apply::apply_changes;
pub use import::{import_cemu_settings, migrate_bcml, parse_bcml_settings};
pub use lenient_semver;
pub use open::{Opened, open_mod};
pub use options::required_satisfied;
pub use order::move_selected;
pub use package::{dev_update_mod, dev_update_mods, extract_mods, package_mod, parse_meta};
pub use profile::ProfileName;
pub use session::{
    Command, Event, Inspection, InstallCandidate, JobId, MasterRef, ModEntry, ModId, OptionsPrompt,
    PendingSummary, Session, Snapshot,
};
pub use view::Sort;
