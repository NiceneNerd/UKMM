#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![allow(stable_features)]
#![feature(
    const_result_drop,
    let_else,
    let_chains,
    option_get_or_insert_default,
    option_result_contains,
    result_option_inspect,
    once_cell
)]
mod cli;
mod gui;
mod logger;

use anyhow::Result;
use cli::Ukmm;

fn main() -> Result<()> {
    match Ukmm::from_env() {
        Ok(cmd) => {
            cli::Runner::new(cmd).run()?;
        }
        Err(e) => {
            if e.is_help() {
                e.exit();
            } else {
                gui::main();
            }
        }
    }
    Ok(())
}
