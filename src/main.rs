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
            } else if let Err(e) = std::panic::catch_unwind(gui::main) {
                println!(
                    "An unrecoverable error occured. Error details: {}",
                    e.downcast::<String>()
                        .or_else(|e| e.downcast::<&'static str>().map(|s| Box::new((*s).into())))
                        .unwrap_or_else(|_| {
                            Box::new(
                                "An unknown error occured, check the log for possible details."
                                    .to_string(),
                            )
                        })
                );
                if let Some(file) = logger::LOGGER.log_path() {
                    logger::LOGGER.save_log();
                    println!(
                        "More information may be available in the log file at {}",
                        file.display()
                    );
                }
            }
        }
    }
    Ok(())
}
