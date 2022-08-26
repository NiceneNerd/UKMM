mod cli;
mod core;
mod deploy;
mod mods;
mod settings;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Install a mod
    Install { path: PathBuf },
    /// Uninstall a mod
    Uninstall,
    /// Deploy mods
    Deploy,
    /// Change current mode (Switch or Wii U)
    Mode { platform: settings::Platform },
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

fn main() -> Result<()> {
    env_logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    let core_manager = core::Manager::init()?;
    let cli = Cli::parse();
    if let Some(command) = cli.command {
        cli::process_command(&core_manager, command)?;
    } else {
        todo!("Let's make a GUI");
    }
    Ok(())
}
