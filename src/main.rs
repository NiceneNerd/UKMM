#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![feature(
    const_result_drop,
    let_chains,
    option_get_or_insert_default,
    option_result_contains,
    result_option_inspect
)]
mod cli;
mod gui;
mod logger;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.command.is_some() {
        cli::Runner::new(cli).run()?;
    } else {
        gui::main();
    }
    Ok(())
}
