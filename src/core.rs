use crate::{deploy, mods, settings::Settings};
use anyhow::{Context, Result};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{path::Path, sync::Arc};
use uk_mod::ModOption;

#[derive(Debug)]
pub struct Manager {
    mod_manager: Arc<mods::Manager>,
    deploy_manager: Arc<deploy::Manager>,
    settings: Arc<RwLock<Settings>>,
}

impl Manager {
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

    pub fn settings(&self) -> RwLockReadGuard<Settings> {
        self.settings.read()
    }

    pub fn settings_mut(&self) -> RwLockWriteGuard<Settings> {
        self.settings.write()
    }
}
