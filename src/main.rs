use anyhow::{Error, Result};
mod mods;
mod settings;

fn main() -> Result<()> {
    env_logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    Ok(())
}
