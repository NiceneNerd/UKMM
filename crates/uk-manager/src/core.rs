use std::sync::Arc;

use anyhow_ext::{Context, Result};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{deploy, mods, settings::Settings};

#[derive(Debug, Clone)]
pub struct Manager {
    mod_manager: Arc<RwLock<mods::Manager>>,
    deploy_manager: Arc<RwLock<deploy::Manager>>,
    settings: Arc<RwLock<Settings>>,
}

impl std::panic::RefUnwindSafe for Manager {}

impl Manager {
    pub fn init() -> Result<Self> {
        let settings = Settings::load();
        let mod_manager = Arc::new(RwLock::new(
            mods::Manager::init(&settings).context("Failed to initialize mod manager")?,
        ));
        Ok(Self {
            deploy_manager: Arc::new(RwLock::new(
                deploy::Manager::init(&settings, &mod_manager)
                    .context("Failed to initialize deployment manager")?,
            )),
            mod_manager,
            settings,
        })
    }

    pub fn reload(&self) -> Result<()> {
        self.settings.write().reload();
        *self.mod_manager.write() =
            mods::Manager::init(&self.settings).context("Failed to initialize mod manager")?;
        *self.deploy_manager.write() = deploy::Manager::init(&self.settings, &self.mod_manager)
            .context("Failed to initialize deployment manager")?;
        Ok(())
    }

    pub fn change_profile(&self, profile: impl AsRef<str>) -> Result<()> {
        self.mod_manager.write().set_profile(profile.as_ref())?;
        if let Some(config) = self.settings.write().platform_config_mut() {
            config.profile = profile.as_ref().into();
        }
        Ok(())
    }

    #[inline(always)]
    pub fn settings(&self) -> RwLockReadGuard<'_, Settings> {
        self.settings.read()
    }

    #[inline(always)]
    pub fn settings_mut(&self) -> RwLockWriteGuard<'_, Settings> {
        self.settings.write()
    }

    #[inline(always)]
    pub fn mod_manager(&self) -> RwLockReadGuard<'_, mods::Manager> {
        self.mod_manager.read()
    }

    #[inline(always)]
    pub fn mod_manager_mut(&self) -> RwLockWriteGuard<'_, mods::Manager> {
        self.mod_manager.write()
    }

    #[inline(always)]
    pub fn deploy_manager(&self) -> RwLockReadGuard<'_, deploy::Manager> {
        self.deploy_manager.read()
    }
}
