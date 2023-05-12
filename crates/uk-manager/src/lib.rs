#![feature(
    arbitrary_self_types,
    let_chains,
    result_option_inspect,
    option_get_or_insert_default,
    once_cell
)]
#![deny(clippy::unwrap_used)]

pub mod bnp;
pub mod core;
pub mod deploy;
pub mod mods;
pub mod settings;
pub mod util;
