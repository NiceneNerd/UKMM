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
        Self::init_with(Settings::load())
    }

    /// Initializes a manager on top of an explicitly provided settings
    /// instance, e.g. one loaded from a custom path with
    /// [`Settings::load_from`] (used by tests to get a temp config dir).
    pub fn init_with(settings: Arc<RwLock<Settings>>) -> Result<Self> {
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

    /// Runs `f` with read access to the mod manager. The guard never escapes
    /// this call, so lock ordering and hold times stay under `Manager`'s
    /// control (phase 5 seal).
    #[inline(always)]
    pub fn with_mod_manager<R>(&self, f: impl FnOnce(&mods::Manager) -> R) -> R {
        f(&self.mod_manager.read())
    }

    /// Runs `f` with write access to the mod manager. See [`Self::with_mod_manager`].
    #[inline(always)]
    pub fn with_mod_manager_mut<R>(&self, f: impl FnOnce(&mut mods::Manager) -> R) -> R {
        f(&mut self.mod_manager.write())
    }

    /// Runs `f` with read access to the deployment manager. See
    /// [`Self::with_mod_manager`].
    #[inline(always)]
    pub fn with_deploy_manager<R>(&self, f: impl FnOnce(&deploy::Manager) -> R) -> R {
        f(&self.deploy_manager.read())
    }
}
