mod cli;
mod core;
mod deploy;
mod gui;
mod logger;
mod mods;
mod settings;
mod util;

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
