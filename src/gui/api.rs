use crate::{core::Manager, mods::Mod};
use anyhow::Result;
use rustc_hash::FxHashMap;
use sciter::Value;
use std::ops::Deref;
use uk_mod::Manifest;

impl Manager {
    pub fn api(&self) -> Value {
        let mods = |_args: &[Value]| -> Value {
            let mods = self
                .mod_manager()
                .all_mods()
                .map(|m| m.to_value())
                .collect();
            log::debug!("Mods: {:?}", &mods);
            mods
        };

        let profiles = |_args: &[Value]| -> Value {
            let profiles = self
                .settings()
                .profiles()
                .map(|p| Value::from(p.as_str()))
                .collect();
            log::debug!("Profiles: {:?}", &profiles);
            profiles
        };

        let current_profile = |_args: &[Value]| -> Value {
            self.settings()
                .platform_config()
                .map(|config| Value::from(config.profile.as_str()))
                .unwrap_or_else(|| Value::from("Default"))
        };

        let preview = |args: &[Value]| -> Value {
            let mod_manager = self.mod_manager();
            let hash = args[0].as_string().unwrap().parse::<usize>().unwrap();
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
        };

        let settings =
            |_args: &[Value]| -> Value { sciter_serde::to_value(self.settings().deref()).unwrap() };

        let apply = |args: &[Value]| -> Value {
            let mods: Vec<Mod> = serde_json::from_str(&args[0].as_string().unwrap()).unwrap();
            self.apply_changes(mods).into()
        };

        let check_hash = |args: &[Value]| {
            println!("{}", args[0].to_float().unwrap());
        };

        let mut api = Value::new();
        api.set_item("mods", mods);
        api.set_item("profiles", profiles);
        api.set_item("preview", preview);
        api.set_item("current_profile", current_profile);
        api.set_item("check_hash", check_hash);
        api.set_item("settings", settings);
        api.set_item("apply", apply);
        api
    }

    fn apply_changes(&self, changed_mods: Vec<Mod>) -> Result<()> {
        let manager = self.mod_manager();
        let mods = manager.all_mods().map(|m| m.clone()).collect::<Vec<_>>();
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
        manager.save()?;
        self.deploy_manager().apply(Some(manifest))?;
        Ok(())
    }
}
