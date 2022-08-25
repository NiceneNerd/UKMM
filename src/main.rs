use anyhow::Result;
mod deploy;
mod mods;
mod settings;
mod util;

fn main() -> Result<()> {
    env_logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    Ok(())
}
