use anyhow::{Context, Result};
use parking_lot::RwLock;
use settings::Settings;
use std::sync::Arc;
mod deploy;
mod mods;
mod settings;
mod util;

#[derive(Debug)]
pub struct CoreState {
    mod_manager: Arc<mods::Manager>,
    deploy_manager: Arc<deploy::Manager>,
    settings: Arc<RwLock<Settings>>,
}

impl CoreState {
    pub fn init() -> Result<Self> {
        let settings = Settings::load();
        let mod_manager =
            Arc::new(mods::Manager::init(&settings).context("Failed to initialize mod manager")?);
        Ok(Self {
            deploy_manager: Arc::new(
                deploy::Manager::init(&settings, &mod_manager)
                    .context("Failed to initialize deployment manager")?,
            ),
            mod_manager,
            settings,
        })
    }
}

fn main() -> Result<()> {
    env_logger::init();
    log::debug!("Logger initialized");
    log::info!("Started ukmm");
    let core_state = CoreState::init()?;
    log::debug!("{:?}", core_state);
    Ok(())
}
