//! Re-exports of the core data types a frontend needs, so a frontend's
//! `Cargo.toml` only lists `uk-app` (+ its toolkit + `uk-localization`) and
//! never touches `uk-content`/`uk-mod`/`uk-manager`/`uk-util` directly.
//!
//! These are plain data types (mod metadata, option groups, settings structs,
//! the `Manager` handle); behaviour lives in [`crate::session`] (commands) and
//! the other `uk-app` modules.

// uk-content: game-file constants/types used by frontends
// (comment order matches rustfmt's alphabetical import sorting)
pub use uk_content::{UKError, constants::Language, util::HashMap};
// uk-manager: runtime handle + core settings/mod types
pub use uk_manager::{
    core::Manager,
    mods::{LookupMod, Mod, Profile},
    settings::{
        DeployConfig, DeployLayout, DeployMethod, Platform, PlatformSettings, Settings,
        UpdatePreference,
    },
    util::clear_temp,
};
// uk-mod: mod format types
pub use uk_mod::{
    ExclusiveOptionGroup, Manifest, Meta, ModCategory, ModOption, ModOptionGroup, ModPlatform,
    MultipleOptionGroup, OptionGroup, pack,
};
// uk-util helpers used by frontends
pub use uk_util::OptionResultExt;
