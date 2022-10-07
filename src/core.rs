use crate::{deploy, mods, settings::Settings};
use anyhow::{Context, Result};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Manager {
    mod_manager: Arc<mods::Manager>,
    deploy_manager: Arc<deploy::Manager>,
    settings: Arc<RwLock<Settings>>,
}

impl std::panic::RefUnwindSafe for Manager {}

impl Manager {
    pub fn init() -> Result<Self> {
        let settings = Settings::load();
        let mod_manager = Arc::new(
            mods::Manager::open_current_profile(&settings)
                .context("Failed to initialize mod manager")?,
        );
        Ok(Self {
            deploy_manager: Arc::new(
                deploy::Manager::init(&settings, &mod_manager)
                    .context("Failed to initialize deployment manager")?,
            ),
            mod_manager,
            settings,
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        *self = Self::init()?;
        Ok(())
    }

    pub fn change_profile(&mut self, profile: impl AsRef<str>) -> Result<()> {
        let profile_path = self.settings.read().profiles_dir().join(profile.as_ref());
        self.mod_manager = Arc::new(mods::Manager::open_profile(&profile_path, &self.settings)?);
        if let Some(config) = self.settings.write().platform_config_mut() {
            config.profile = profile.as_ref().into();
        }
        Ok(())
    }

    #[inline(always)]
    pub fn settings(&self) -> RwLockReadGuard<Settings> {
        self.settings.read()
    }

    #[inline(always)]
    pub fn settings_mut(&self) -> RwLockWriteGuard<Settings> {
        self.settings.write()
    }

    #[inline(always)]
    pub fn mod_manager(&self) -> Arc<mods::Manager> {
        self.mod_manager.clone()
    }

    #[inline(always)]
    pub fn deploy_manager(&self) -> Arc<deploy::Manager> {
        self.deploy_manager.clone()
    }
}
