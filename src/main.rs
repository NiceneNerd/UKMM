mod cli;
mod core;
mod deploy;
mod mods;
mod settings;
mod util;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    env_logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    let mut core_manager = core::Manager::init()?;
    let cli = Cli::parse();
    if cli.command.is_some() {
        cli::Runner::new(&mut core_manager, cli).run()?;
    } else {
        todo!("Let's make a GUI");
    }
    Ok(())
}
