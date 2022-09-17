use super::EventHandler;
use crate::{core::Manager, mods::Mod};
use anyhow::Result;
use rustc_hash::FxHashMap;
use sciter::{make_args, Value};
use std::{ops::Deref, sync::Arc};
use uk_mod::Manifest;

macro_rules! res {
    ($action:expr, $callback:expr) => {
        std::thread::spawn(move || {
            let _res = match $action {
                Ok(_res) => _res.into(),
                Err(_e) => {
                    log::error!("{:?}", &_e);
                    let _trace = _e.backtrace().to_string();
                    log::error!("{:?}", &_trace);
                    let mut _err = Value::error(&format!("{:?}", &_e));
                    _err.set_item("error", Value::from(&format!("{:?}", &_e)));
                    _err.set_item("source", Value::from(_trace));
                    _err
                }
            };
            $callback.call(None, &make_args!(_res), None).unwrap();
        });
    };
}

#[allow(non_snake_case)]
impl EventHandler {
    pub fn mods(&self) -> Value {
        let mods = self
            .core
            .mod_manager()
            .all_mods()
            .map(|m| m.to_value())
            .collect();
        log::debug!("Mods: {:?}", &mods);
        mods
    }

    pub fn profiles(&self) -> Value {
        let profiles = self
            .core
            .settings()
            .profiles()
            .map(|p| Value::from(p.as_str()))
            .collect();
        log::debug!("Profiles: {:?}", &profiles);
        profiles
    }

    pub fn currentProfile(&self) -> Value {
        self.core
            .settings()
            .platform_config()
            .map(|config| Value::from(config.profile.as_str()))
            .unwrap_or_else(|| Value::from("Default"))
    }

    pub fn preview(&self, hash: String) -> Value {
        let mod_manager = self.core.mod_manager();
        let hash = hash.parse::<usize>().unwrap();
        let mod_ = mod_manager.get_mod(hash).unwrap();
        if let Ok(Some(data)) = mod_.preview() {
            match &data[..4] {
                [0xff, 0xd8, 0xff, 0xe0] => {
                    Value::from(format!("data:image/jpeg;base64,{}", base64::encode(&data)))
                }
                [0x89, 0x50, 0x4e, 0x47] => {
                    Value::from(format!("data:image/png;base64,{}", base64::encode(&data)))
                }
                _ => {
                    log::debug!("Unsupported preview image, ignoring");
                    Value::null()
                }
            }
        } else {
            Value::null()
        }
    }

    pub fn settings(&self) -> Value {
        sciter_serde::to_value(self.core.settings().deref()).unwrap()
    }

    pub fn apply(&self, mods: String, callback: Value) {
        let mods: Vec<Mod> = serde_json::from_str(&mods).unwrap();
        let core = self.core.clone();
        res!(Self::apply_changes(core, mods), callback);
    }

    fn apply_changes(core: Arc<Manager>, changed_mods: Vec<Mod>) -> Result<()> {
        let manager = core.mod_manager();
        let mods = manager.all_mods().map(|m| m.clone()).collect::<Vec<_>>();
        let order = changed_mods.iter().map(|m| m.hash).collect();
        let mut manifest = Manifest::default();
        let mut modified = FxHashMap::default();
        for (i, mod_) in changed_mods.into_iter().enumerate() {
            if modified.contains_key(&mod_.hash) {
                modified.remove(&mod_.hash);
                continue;
            }
            let original = &mods[i - modified.len()];
            if &mod_ != original || !mod_.state_eq(original) {
                modified.insert(mod_.hash, mod_.clone());
                if mod_.enabled_options != original.enabled_options {
                    manifest.extend(&manager.set_enabled_options(mod_.hash, mod_.enabled_options)?);
                    continue;
                }
                if mod_.enabled != original.enabled {
                    manager.set_enabled(mod_.hash, mod_.enabled)?;
                }
                manifest.extend(&mod_.manifest);
            }
        }
        manager.set_order(order);
        manager.save()?;
        core.deploy_manager().apply(Some(manifest))?;
        Ok(())
    }
}
