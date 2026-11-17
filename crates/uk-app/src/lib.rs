//! The operations a UKMM frontend performs, independent of any UI toolkit.
//!
//! Nothing here may depend on a GUI, a dialog library, or a frontend's message
//! type: inputs a frontend must collect (paths, metadata, option picks) are
//! explicit parameters, and outcomes that need further input are returned as
//! data (see [`open::Opened`]).

mod apply;
mod import;
pub mod open;
pub mod package;
pub mod sniff;

pub use apply::apply_changes;
pub use import::{import_cemu_settings, migrate_bcml};
pub use open::{open_mod, Opened};
pub use package::{dev_update_mod, extract_mods, package_mod, parse_meta};
